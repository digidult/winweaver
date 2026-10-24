# Runtime templates

The fixed parts of the answer file and of the stand-alone scripts. The build (`crates/winweaver-core/src/render/`
for the answer file, `scripts.rs` and `users.rs` for the stand-alone scripts) fills the `{{name}}` slots with the
blocks of the enabled rules and with profile data; everything else is copied as is.

| File | Content | Slots |
|---|---|---|
| `autounattend.template.xml` | Skeleton of the answer file and the extraction script (ExtractScript) | `header`, `settings`, `files`, `profile_json` |
| `Setup-System.runtime.ps1` | Functions Write-Log, Set-Reg, Remove-Reg, Open-RegKey, Set-RegList (a key with a list of values: `-Names` and `-Values` pair up by position, without `-Additive` the other values of the key are deleted first), Set-ServiceStart, Invoke-Exe, Set-Feature, Remove-Capability, Remove-Apps; trap; log header | `build_label`, `computer_name_section`, `blocks`, `default_user_section`, `active_setup_section`, `post_oobe_section` |
| `section-computer-name.ps1` | The computer name of a template (`install.computer_name_mode` `template`): computes it and starts a hidden process that writes it until the restart after specialize (reference card 02) | `name_parts` |
| `section-default-user.ps1` | Mounting of the default user hive (`$du`) around the blocks of the default-user phase | `blocks` |
| `section-active-setup.ps1` | Active Setup registration of `Setup-User.ps1` (added when the user-first-logon phase has rules) | none |
| `section-post-oobe-task.ps1` | Scheduled task for `Post-OOBE.ps1` (added when the post-oobe phase has rules or an account text needs it) | none |
| `Setup-User.runtime.ps1` | Per-user log, Set-Reg and Remove-Reg (HKCU of the user), input language lists from the profile | `input_languages`, `input_tips`, `input_fallback`, `transient_languages`, `blocks` |
| `Post-OOBE.runtime.ps1` | Waiting for OOBE to finish, Set-Reg and Remove-Reg, list of accounts, Set-AccountText and ConvertFrom-Base64Text (display names and descriptions outside ASCII, set through ADSI), removal of the task | `accounts`, `account_texts`, `blocks` |
| `Audit.runtime.ps1` | Read-only audit of selected rules on a running Windows: Test-Reg, Test-RegList, Test-ServiceStart, Test-Feature, Test-AppAbsent and others; JSON report, with `system` when the data forms are read | `build_label`, `blocks`, `system_section` |
| `section-read-system.ps1` | The data forms of the audit report, which WinWeaver turns into a profile: edition, time zone, computer name, languages, local accounts and their groups (no passwords, no keys) | none |
| `Apply.runtime.ps1` | Applying selected rules to a running Windows, or returning them to the Windows defaults: administrator check, the same helpers as Setup-System with the previous state saved to `backup-*.json` (Set-RegList saves every value of the key it touches), default user hive mounting | `build_label`, `accounts`, `blocks` |
| `Undo.runtime.ps1` | Restoring from `backup-*.json` of the apply script or of the native apply; a list of values comes back value by value | `build_label` |
| `Create-Users.runtime.ps1` | The stand-alone script of a users file (`.wwusers`): asks the master password, creates or updates the local accounts of the file next to it | `users_label`, `users_file_b64`, `build` |

`VERSION` holds the version of the runtime templates (`render::runtime_version`), the same as the version of the
catalog (`CATALOG_VERSION`); the header of the answer file names both, and a profile stores the catalog version
as `catalog`. Any change of a template or of the catalog that changes the output file needs a new version and a
run of the tests of `crates/winweaver-core` (`tests/render.rs`, `tests/scripts.rs`).

Everything the generator itself writes (header, block markers, embedded profile) is ASCII, so the answer
file does not depend on how Windows Setup treats national characters.
