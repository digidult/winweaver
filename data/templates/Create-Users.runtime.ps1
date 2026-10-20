# Create-Users: the local accounts of a WinWeaver users file ({{users_label}}).
# Made by WinWeaver {{build}}.
#
# Run from a command prompt opened as administrator:
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File "C:\path\to\this script.ps1"
#
# The users file it was made for lies next to the script (its name is kept in Base64, so the
# script stays ASCII whatever the name); -UsersFile names another one. The
# master password of its passwords is asked for here, never taken from a parameter, a variable
# or a file. Switches:
#
#   -UpdateExisting   also apply the password expiry, the missing group membership, the full
#                     name, the description and the profile path to accounts that exist
#   -PasswordsOnly    set the passwords of the accounts that exist and change nothing else
#   -UserName a,b     only these accounts of the file
#   -NoPause          do not wait for a key before the window closes
#
# One status an account: CREATED, UPDATED, ALREADY_EXISTS, PASSWORD_SET, NOT_FOUND, ERROR.
# An account that exists is changed only by the orders of its row (apply_password, state,
# force_group) or by -UpdateExisting; no account is ever deleted and no profile is moved.
# Exit codes: 0 done, 1 some accounts failed, 2 cancelled or wrong master password, 3 the file
# was refused, 4 not an administrator.

#Requires -Version 5.1

[CmdletBinding()]
param(
    [string] $UsersFile,
    [switch] $UpdateExisting,
    [switch] $PasswordsOnly,
    [string[]] $UserName,
    [string] $LogFile,
    [switch] $NoPause
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

if (-not $UsersFile) {
    $fileName = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{users_file_b64}}'))
    $UsersFile = Join-Path $PSScriptRoot $fileName
}
if (-not $LogFile) {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $stem = [IO.Path]::GetFileNameWithoutExtension($UsersFile)
    $LogFile = Join-Path $PSScriptRoot ($stem + '-create-users-' + $stamp + '.log')
}

$ADMINISTRATORS_SID = 'S-1-5-32-544'
$USERS_SID = 'S-1-5-32-545'
$SYSTEM_SID = 'S-1-5-18'
$PROFILE_LIST = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList'
$script:results = New-Object System.Collections.ArrayList

function Write-Log {
    param([string] $Action, [string] $Name, [string] $Result, [string] $Detail)
    $line = '{0} | {1} | {2} | {3}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $Action, $(if ($Name) { $Name } else { '-' }), $Result
    if ($Detail) { $line += ' | ' + $Detail }
    try { Add-Content -LiteralPath $LogFile -Value $line -Encoding UTF8 } catch {}
}

function Add-Result {
    param([string] $Name, [string] $Status, [string] $Message)
    [void]$script:results.Add(@{ name = $Name; status = $Status; message = $Message })
    $colour = switch ($Status) { 'ERROR' { 'Red' } 'NOT_FOUND' { 'Red' } 'ALREADY_EXISTS' { 'Gray' } default { 'Green' } }
    $text = '{0,-20} {1}' -f $Name, $Status
    if ($Message) { $text += ': ' + $Message }
    Write-Host $text -ForegroundColor $colour
    Write-Log -Action $(if ($PasswordsOnly) { 'APPLY_PASSWORD' } else { 'CREATE_USER' }) -Name $Name -Result $Status -Detail $Message
}

function Stop-Run {
    param([int] $ExitCode)
    Write-Log -Action 'RUN_END' -Result ('EXIT ' + $ExitCode)
    if (-not $NoPause) {
        Write-Host ''
        Write-Host 'Press any key to close this window.'
        $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
    }
    exit $ExitCode
}

function Test-Administrator {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-Field {
    param($Object, [string] $Name, $Default)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) { return $Default }
    return $property.Value
}

function Test-AccountName {
    param([string] $Name)
    if (-not $Name) { return 'the name is empty' }
    if ($Name.Length -gt 20) { return 'the name is longer than 20 characters' }
    if ($Name -match '["/\\\[\]:;|=,+*?<>@]') { return 'the name holds a character Windows refuses' }
    if ($Name.EndsWith('.')) { return 'the name ends with a dot' }
    return $null
}

# --- the passwords: PBKDF2-SHA256, HMAC-SHA256 over salt, iv and data, then AES-256-CBC ----------

function Unprotect-Passwords {
    param($Container, [System.Security.SecureString] $Master)
    if ((Get-Field $Container 'kdf' '') -ne 'pbkdf2-sha256') { throw 'The passwords use an unknown key derivation.' }
    $iterations = [int](Get-Field $Container 'iterations' 0)
    if ($iterations -lt 100000) { throw 'The passwords have too few iterations.' }
    $salt = [Convert]::FromBase64String([string](Get-Field $Container 'salt' ''))
    $iv = [Convert]::FromBase64String([string](Get-Field $Container 'iv' ''))
    $mac = [Convert]::FromBase64String([string](Get-Field $Container 'mac' ''))
    $data = [Convert]::FromBase64String([string](Get-Field $Container 'data' ''))
    if ($salt.Length -ne 32 -or $iv.Length -ne 16 -or $mac.Length -ne 32 -or $data.Length -eq 0 -or ($data.Length % 16) -ne 0) {
        throw 'The passwords of the file are damaged.'
    }
    $pointer = [System.Runtime.InteropServices.Marshal]::SecureStringToGlobalAllocUnicode($Master)
    $chars = New-Object char[] $Master.Length
    $masterBytes = $null
    $derived = $null
    $plain = $null
    try {
        for ($i = 0; $i -lt $Master.Length; $i++) {
            $chars[$i] = [char]([System.Runtime.InteropServices.Marshal]::ReadInt16($pointer, $i * 2) -band 0xFFFF)
        }
        $masterBytes = [System.Text.Encoding]::UTF8.GetBytes($chars)
        $kdf = New-Object System.Security.Cryptography.Rfc2898DeriveBytes($masterBytes, $salt, $iterations, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
        try { $derived = $kdf.GetBytes(64) } finally { $kdf.Dispose() }
        $encKey = $derived[0..31]
        $macKey = $derived[32..63]
        $hmac = New-Object System.Security.Cryptography.HMACSHA256 -ArgumentList @(, [byte[]]$macKey)
        try { $actual = $hmac.ComputeHash([byte[]]($salt + $iv + $data)) } finally { $hmac.Dispose() }
        $difference = 0
        for ($i = 0; $i -lt 32; $i++) { $difference = $difference -bor ($actual[$i] -bxor $mac[$i]) }
        if ($difference -ne 0) { throw 'Wrong master password or a damaged users file.' }
        $aes = [System.Security.Cryptography.Aes]::Create()
        try {
            $aes.KeySize = 256
            $aes.Mode = [System.Security.Cryptography.CipherMode]::CBC
            $aes.Padding = [System.Security.Cryptography.PaddingMode]::PKCS7
            $aes.Key = [byte[]]$encKey
            $aes.IV = $iv
            $decryptor = $aes.CreateDecryptor()
            try { $plain = $decryptor.TransformFinalBlock($data, 0, $data.Length) } finally { $decryptor.Dispose() }
        }
        finally { $aes.Dispose() }
        $parsed = [System.Text.Encoding]::UTF8.GetString($plain) | ConvertFrom-Json
        $map = New-Object System.Collections.Hashtable ([System.StringComparer]::OrdinalIgnoreCase)
        foreach ($property in $parsed.PSObject.Properties) { $map[$property.Name] = [string]$property.Value }
        return $map
    }
    finally {
        [Array]::Clear($chars, 0, $chars.Length)
        [System.Runtime.InteropServices.Marshal]::ZeroFreeGlobalAllocUnicode($pointer)
        if ($null -ne $masterBytes) { [Array]::Clear($masterBytes, 0, $masterBytes.Length) }
        if ($null -ne $derived) { [Array]::Clear($derived, 0, $derived.Length) }
        if ($null -ne $plain) { [Array]::Clear($plain, 0, $plain.Length) }
    }
}

# --- groups by SID, the profile folder and its registration ------------------------------------

function Get-CurrentSid { return [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value }

function Test-Member {
    param([string] $GroupSid, [string] $UserSid)
    $members = @(Get-LocalGroupMember -SID $GroupSid -ErrorAction SilentlyContinue)
    foreach ($member in $members) { if ($member.SID.Value -eq $UserSid) { return $true } }
    return $false
}

function Set-Membership {
    # Strict: exactly one of the two groups. Additive: the missing membership only.
    param([string] $UserSid, [bool] $IsAdmin, [bool] $Strict)
    $changes = @()
    $wanted = $(if ($IsAdmin) { $ADMINISTRATORS_SID } else { $USERS_SID })
    $other = $(if ($IsAdmin) { $USERS_SID } else { $ADMINISTRATORS_SID })
    if (-not (Test-Member -GroupSid $wanted -UserSid $UserSid)) {
        Add-LocalGroupMember -SID $wanted -Member $UserSid -ErrorAction Stop
        $changes += $(if ($IsAdmin) { '+Administrators' } else { '+Users' })
    }
    if ($Strict -and (Test-Member -GroupSid $other -UserSid $UserSid)) {
        if ($other -eq $ADMINISTRATORS_SID -and $UserSid -eq (Get-CurrentSid)) {
            Write-Log -Action 'GROUP' -Name $UserSid -Result 'KEPT' -Detail 'Administrators kept (current user)'
            $changes += 'Administrators kept (current user)'
        }
        else {
            Remove-LocalGroupMember -SID $other -Member $UserSid -ErrorAction Stop
            $changes += $(if ($IsAdmin) { '-Users' } else { '-Administrators' })
        }
    }
    return ,$changes
}

function New-ProfileFolder {
    # Only SYSTEM, Administrators and the account itself may enter the folder.
    param([string] $ProfilePath, [string] $UserSid)
    if (-not (Test-Path -LiteralPath $ProfilePath)) { New-Item -ItemType Directory -Path $ProfilePath -Force | Out-Null }
    $acl = Get-Acl -LiteralPath $ProfilePath
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($rule in @($acl.Access)) { if (-not $rule.IsInherited) { [void]$acl.RemoveAccessRuleSpecific($rule) } }
    $inherit = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
    foreach ($sid in @($SYSTEM_SID, $ADMINISTRATORS_SID, $UserSid)) {
        $identity = New-Object System.Security.Principal.SecurityIdentifier($sid)
        $acl.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($identity, 'FullControl', $inherit, 'None', 'Allow')))
    }
    Set-Acl -LiteralPath $ProfilePath -AclObject $acl
    try {
        $owner = Get-Acl -LiteralPath $ProfilePath
        $owner.SetOwner((New-Object System.Security.Principal.SecurityIdentifier($UserSid)))
        Set-Acl -LiteralPath $ProfilePath -AclObject $owner
    }
    catch { Write-Log -Action 'PROFILE_OWNER' -Name $UserSid -Result 'SKIPPED' -Detail $_.Exception.Message }
}

function Test-ProfileElsewhere {
    param([string] $UserSid, [string] $ProfilePath)
    $key = Join-Path $PROFILE_LIST $UserSid
    if (-not (Test-Path -LiteralPath $key)) { return $false }
    $current = (Get-ItemProperty -LiteralPath $key -Name 'ProfileImagePath' -ErrorAction SilentlyContinue).ProfileImagePath
    return ([bool]$current -and $current -ne $ProfilePath)
}

function Register-Profile {
    param([string] $UserSid, [string] $ProfilePath)
    $key = Join-Path $PROFILE_LIST $UserSid
    if (-not (Test-Path -LiteralPath $key)) { New-Item -Path $key -Force | Out-Null }
    New-ItemProperty -LiteralPath $key -Name 'ProfileImagePath' -Value $ProfilePath -PropertyType ExpandString -Force | Out-Null
    New-ItemProperty -LiteralPath $key -Name 'Flags' -Value 0 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -LiteralPath $key -Name 'State' -Value 0 -PropertyType DWord -Force | Out-Null
}

# --- one account -------------------------------------------------------------------------------

function New-Account {
    param($Entry, [System.Security.SecureString] $Password)
    $parameters = @{ Name = $Entry.name; Password = $Password; AccountNeverExpires = $true; ErrorAction = 'Stop' }
    if ($Entry.full_name) { $parameters['FullName'] = $Entry.full_name }
    if ($Entry.description) { $parameters['Description'] = $Entry.description }
    if ($Entry.password_never_expires) { $parameters['PasswordNeverExpires'] = $true }
    New-LocalUser @parameters | Out-Null
    try {
        if ($Entry.state -eq 'disabled') { Disable-LocalUser -Name $Entry.name } else { Enable-LocalUser -Name $Entry.name }
        $sid = (Get-LocalUser -Name $Entry.name).SID.Value
        [void](Set-Membership -UserSid $sid -IsAdmin $Entry.admin -Strict $true)
        if ($Entry.profile_path) {
            New-ProfileFolder -ProfilePath $Entry.profile_path -UserSid $sid
            Register-Profile -UserSid $sid -ProfilePath $Entry.profile_path
        }
    }
    catch { throw ('The account was created, but its configuration failed: ' + $_.Exception.Message) }
}

function Update-Account {
    param($Existing, $Entry, [System.Security.SecureString] $Password)
    $changes = @()
    $name = $Existing.Name
    $sid = $Existing.SID.Value
    if ($Entry.apply_password) { Set-LocalUser -Name $name -Password $Password; $changes += 'Password' }
    if ($Entry.state -eq 'enabled' -and -not $Existing.Enabled) { Enable-LocalUser -Name $name; $changes += 'Enabled' }
    elseif ($Entry.state -eq 'disabled' -and $Existing.Enabled) { Disable-LocalUser -Name $name; $changes += 'Disabled' }
    if ($Entry.force_group) { $changes += @(Set-Membership -UserSid $sid -IsAdmin $Entry.admin -Strict $true) }
    elseif ($UpdateExisting) { $changes += @(Set-Membership -UserSid $sid -IsAdmin $Entry.admin -Strict $false) }
    if (-not $UpdateExisting) { return ($changes -join ', ') }
    $neverExpires = ($null -eq $Existing.PasswordExpires)
    if ($Entry.password_never_expires -ne $neverExpires) {
        Set-LocalUser -Name $name -PasswordNeverExpires $Entry.password_never_expires
        $changes += 'PasswordNeverExpires'
    }
    if ($Entry.full_name -and $Entry.full_name -ne $Existing.FullName) { Set-LocalUser -Name $name -FullName $Entry.full_name; $changes += 'FullName' }
    if ($Entry.description -and $Entry.description -ne $Existing.Description) { Set-LocalUser -Name $name -Description $Entry.description; $changes += 'Description' }
    if ($Entry.profile_path) {
        if (Test-ProfileElsewhere -UserSid $sid -ProfilePath $Entry.profile_path) {
            Write-Log -Action 'PROFILE_PATH' -Name $name -Result 'KEPT' -Detail 'An existing profile was found and was not moved.'
        }
        else {
            New-ProfileFolder -ProfilePath $Entry.profile_path -UserSid $sid
            Register-Profile -UserSid $sid -ProfilePath $Entry.profile_path
            $changes += 'ProfilePath'
        }
    }
    return ($changes -join ', ')
}

# --- main --------------------------------------------------------------------------------------

Write-Host ''
Write-Host ('WinWeaver users: ' + $(if ($PasswordsOnly) { 'apply the passwords' } else { 'create the accounts' })) -ForegroundColor Cyan
Write-Host ('Users file: ' + $UsersFile)
Write-Host ''
Write-Log -Action 'RUN_START' -Result 'STARTED' -Detail ('file=' + $UsersFile + ' update=' + [bool]$UpdateExisting + ' passwords_only=' + [bool]$PasswordsOnly)

if (-not (Test-Administrator)) {
    Write-Host 'This script needs a command prompt opened as administrator.' -ForegroundColor Red
    Stop-Run -ExitCode 4
}
if (-not (Test-Path -LiteralPath $UsersFile -PathType Leaf)) {
    Write-Host ('The users file does not exist: ' + $UsersFile) -ForegroundColor Red
    Stop-Run -ExitCode 3
}

$passwords = $null
try {
    try { $file = Get-Content -LiteralPath $UsersFile -Raw -Encoding UTF8 | ConvertFrom-Json }
    catch { Write-Host 'The users file is not valid JSON.' -ForegroundColor Red; Stop-Run -ExitCode 3 }
    if ((Get-Field $file 'format' '') -ne 'winweaver-users') {
        Write-Host 'This is not a WinWeaver users file.' -ForegroundColor Red
        Stop-Run -ExitCode 3
    }
    $container = Get-Field $file 'passwords' $null
    if ($null -eq $container) {
        Write-Host 'The users file holds no passwords: set a master password in WinWeaver first.' -ForegroundColor Red
        Stop-Run -ExitCode 3
    }

    $entries = @()
    $invalid = $false
    $seen = New-Object System.Collections.Hashtable ([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($raw in @(Get-Field $file 'users' @())) {
        $entry = @{
            name = [string](Get-Field $raw 'name' '')
            full_name = [string](Get-Field $raw 'full_name' '')
            description = [string](Get-Field $raw 'description' '')
            admin = [bool](Get-Field $raw 'admin' $false)
            password_never_expires = [bool](Get-Field $raw 'password_never_expires' $false)
            profile_path = [string](Get-Field $raw 'profile_path' '')
            apply_password = [bool](Get-Field $raw 'apply_password' $false)
            state = ([string](Get-Field $raw 'state' 'keep')).ToLowerInvariant()
            force_group = [bool](Get-Field $raw 'force_group' $false)
        }
        $problem = Test-AccountName -Name $entry.name
        if (-not $problem -and $seen.ContainsKey($entry.name)) { $problem = 'the name is used twice' }
        if (-not $problem -and $entry.state -notin @('keep', 'enabled', 'disabled')) { $problem = 'the state is not keep, enabled or disabled' }
        if ($problem) {
            Write-Host ('Invalid account "' + $entry.name + '": ' + $problem) -ForegroundColor Red
            $invalid = $true
            continue
        }
        $seen[$entry.name] = $true
        $entries += $entry
    }
    if ($invalid) {
        Write-Host 'The users file holds invalid accounts, nothing was done.' -ForegroundColor Red
        Stop-Run -ExitCode 3
    }
    if ($UserName) {
        $wanted = New-Object System.Collections.Hashtable ([System.StringComparer]::OrdinalIgnoreCase)
        foreach ($n in $UserName) { $wanted[$n.Trim()] = $true }
        $entries = @($entries | Where-Object { $wanted.ContainsKey($_.name) })
        foreach ($n in $wanted.Keys) {
            if (-not ($entries | Where-Object { $_.name -eq $n })) {
                Write-Host ('The account is not in the file: ' + $n) -ForegroundColor Red
                Stop-Run -ExitCode 3
            }
        }
    }
    if ($entries.Count -eq 0) {
        Write-Host 'The users file holds no accounts, there is nothing to do.' -ForegroundColor Yellow
        Stop-Run -ExitCode 0
    }

    $master = Read-Host -Prompt 'Master password' -AsSecureString
    try { $passwords = Unprotect-Passwords -Container $container -Master $master }
    catch { Write-Host $_.Exception.Message -ForegroundColor Red; Stop-Run -ExitCode 2 }
    finally { $master.Dispose() }

    $missing = @($entries | Where-Object { -not $passwords.ContainsKey($_.name) } | ForEach-Object { $_.name })
    if ($missing.Count -gt 0) {
        Write-Host ('No password for: ' + ($missing -join ', ') + '. Nothing was done.') -ForegroundColor Red
        Stop-Run -ExitCode 3
    }

    foreach ($entry in $entries) {
        try {
            $existing = Get-LocalUser -Name $entry.name -ErrorAction SilentlyContinue
            $secure = ConvertTo-SecureString -String ([string]$passwords[$entry.name]) -AsPlainText -Force
            try {
                if ($PasswordsOnly) {
                    if ($null -eq $existing) { Add-Result -Name $entry.name -Status 'NOT_FOUND' -Message 'Create the account first.' }
                    else { Set-LocalUser -Name $existing.Name -Password $secure; Add-Result -Name $entry.name -Status 'PASSWORD_SET' }
                }
                elseif ($null -ne $existing) {
                    $changed = Update-Account -Existing $existing -Entry $entry -Password $secure
                    if ($changed) { Add-Result -Name $entry.name -Status 'UPDATED' -Message $changed }
                    else { Add-Result -Name $entry.name -Status 'ALREADY_EXISTS' -Message 'The account was left as it is.' }
                }
                else {
                    New-Account -Entry $entry -Password $secure
                    Add-Result -Name $entry.name -Status 'CREATED'
                }
            }
            finally { $secure.Dispose() }
        }
        catch { Add-Result -Name $entry.name -Status 'ERROR' -Message $_.Exception.Message }
    }
}
finally {
    if ($null -ne $passwords) { $passwords.Clear() }
}

$failed = @($script:results | Where-Object { $_.status -eq 'ERROR' -or $_.status -eq 'NOT_FOUND' }).Count
Write-Host ''
Write-Host ('Done: {0} accounts, {1} failed' -f $script:results.Count, $failed)
Stop-Run -ExitCode $(if ($failed -gt 0) { 1 } else { 0 })
