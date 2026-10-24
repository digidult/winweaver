# 04. Accounts and .NET Framework 3.5

The accounts of the profile, the password age, and what `Setup-System.ps1` takes from the installation media.

## AdminAccount, UserAccount

- Value: `Admin`, `User` (the accounts of a new profile).
- Where applied: the list `$accounts` of `Post-OOBE.ps1`, which the build writes from the accounts of the profile
  (account mode `file`; in mode `ask` the list is empty).
- What it does: tells Post-OOBE.ps1 the names of the accounts whose password expiration must be removed
  (`post-oobe.password-never-expires`). The accounts themselves are created by the XML (section 03).
- Expected effect: `Set-LocalUser -PasswordNeverExpires $true` for both names after OOBE completes.
- Cross-links: the list and `<Name>` in `<LocalAccounts>` come from the same accounts of the profile; an account
  that is not found is written to the log and keeps its expiration, but the global `net accounts` command (see
  below) still takes effect.
- Version differences: none.
- Verification: the line `$accounts = @(...)` of `C:\ProgramData\Unattend\Scripts\Post-OOBE.ps1`.
- Rollback: not applicable.

## PasswordNeverExpires

- Value: on (the rules `accounts.password-never-expires` and `post-oobe.password-never-expires`, both on by
  default).
- Where applied: two places. `Setup-System.ps1` (specialize, always runs) and `Post-OOBE.ps1` (after OOBE, for
  each account).
- What it does:
  1. `net.exe accounts /maxpwage:unlimited`: the maximum password age for all local
     accounts is removed (the default is 42 days).
  2. `Set-LocalUser -Name <name> -PasswordNeverExpires $true`: the «Срок действия пароля не ограничен»
     (Password never expires) flag on the Admin and User accounts themselves.
- Expected effect: Windows will never show "Your password has expired and must be changed". For empty
  passwords this is critical: a non-professional user will not understand what is being asked of them.
- Cross-links:
  - Starter accounts without passwords (section 03). A users file (`.wwusers`) assigns passwords; password
    expiration can then be restored (`net accounts /maxpwage:90`).
  - `AccountLockout` (section 08) is set by the same `net accounts` utility; the order of calls does not matter.
  - If Post-OOBE.ps1 does not run (the PC was switched off before it completed), the global setting from
    specialize still applies.
- Version differences: `net accounts` works in all versions. `Set-LocalUser` is available in Windows 10 1607+
  (the Microsoft.PowerShell.LocalAccounts module). On Windows 11 22H2+ the default of 42 days is retained.
- Verification: `net accounts` → "Maximum password age: Unlimited"; `Get-LocalUser Admin | Select PasswordExpires`
  (empty).
- Rollback: `net accounts /maxpwage:42`; `Set-LocalUser -Name Admin -PasswordNeverExpires $false`.

## EnableNetFx3

- Value: on (the rule `install.netfx3`, on by default).
- Where applied: `Setup-System.ps1`, specialize, SYSTEM.
- What it does: goes through all ready drives looking for a `sources\sxs` folder with `.cab` files inside (the installation
  media: USB, DVD, an ISO mounted by Ventoy). If found:
  `dism.exe /Online /Enable-Feature /FeatureName:NetFx3 /All /LimitAccess /Source:<path> /NoRestart /Quiet`.
  If not found: a WARN is written to the log and the feature is not enabled.
- Expected effect: .NET Framework 3.5 (including 2.0 and 3.0) is available right after installation without
  internet access. It is needed by old accounting and banking programs and old versions of digital signature key drivers.
- Cross-links:
  - The media must stay connected during specialize (the first reboot). If the USB drive
    was removed right after the files were copied, the feature will not be installed; it can then be added later
    via Settings → Optional features (requires internet) or with a DISM command from the media.
  - `/LimitAccess` prevents contacting Windows Update while the feature is being installed: there is no network
    in specialize anyway, and without this switch DISM would wait for a timeout.
  - Security updates for .NET 3.5 arrive through Windows Update (section 06).
- Version differences: the `sources\sxs` folder is present in all official Windows 10/11 ISOs. On images
  stripped down by third-party tools it may be missing. In Windows 11 24H2 the feature is still
  optional and disabled by default.
- Verification: `Get-WindowsOptionalFeature -Online -FeatureName NetFx3` → State Enabled;
  the log contains the line `dism.exe ... -> exit 0`.
- Rollback: `Disable-WindowsOptionalFeature -Online -FeatureName NetFx3`.

## DriversFromMedia

- Rule: `install.drivers-from-media` (off by default; the folder is `\Drivers` next to `autounattend.xml`).
- What it does: `Setup-System.ps1` (specialize) searches every ready drive for a `Drivers` folder that holds an `.inf`
  file, the same walk as `EnableNetFx3` uses for `sources\sxs`, and runs
  `pnputil.exe /add-driver <drive>:\Drivers\*.inf /subdirs /install`: every driver package of the folder and its
  subfolders goes into the driver store and is installed for the present devices. Without such a folder the script
  logs a WARN line and goes on.
- Expected effect: network, chipset and storage drivers of a new PC work right after installation without internet
  access, and Windows Update has nothing to fetch for them.
- Limits: a driver that windowsPE needs to see the disk at all (RAID, VMD) cannot come this way: it must be in the
  image (`DISM /Add-Driver`) or in `$WinPEDriver$`. `pnputil` returns 3010 when a restart is needed; `Invoke-Exe` logs
  it as WARN, and the restart after specialize completes the installation.
- Risk: every `.inf` of the folder is installed as trusted. Keep the folder to the signed packages of the vendor for
  the model.
- Verification: `pnputil /enum-drivers`; `C:\ProgramData\Unattend\Logs\Setup-System.log`.
- Rollback: `pnputil /delete-driver oemNN.inf /uninstall` per driver.
- Pending: acceptance in a VM (a USB stick with a `Drivers` folder of a virtual network adapter).
