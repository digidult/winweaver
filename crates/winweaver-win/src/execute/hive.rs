//! The hive of the default user profile, loaded under HKEY_USERS for the steps that write it and unloaded after the last one.

use super::*;

/// The key of the default user profile under HKEY_USERS (`UnattendDefault`).
pub(crate) fn default_user_subkey() -> &'static str {
    apply::DEFAULT_USER_KEY
        .strip_prefix("HKU:\\")
        .unwrap_or(apply::DEFAULT_USER_KEY)
}

/// Enables a privilege the token of the process holds, such as `SeRestorePrivilege`.
pub(crate) fn enable_privilege(name: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID,
    };
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    let name_w = wide(name);
    // SAFETY: the name is null-terminated and the LUID is written by the call.
    if unsafe { LookupPrivilegeValueW(std::ptr::null(), name_w.as_ptr(), &mut luid) } == 0 {
        return Err(format!("the privilege {name} is not known"));
    }
    let mut token: HANDLE = std::ptr::null_mut();
    let access = TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY;
    // SAFETY: the pseudo handle of the process is always valid; the token handle is closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut token) } == 0 {
        return Err("the token of the process cannot be opened".to_owned());
    }
    let state = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let none = std::ptr::null_mut();
    // SAFETY: the state holds one privilege; the previous state is not asked for.
    let ok = unsafe { AdjustTokenPrivileges(token, 0, &state, 0, none, std::ptr::null_mut()) };
    // SAFETY: the error of the call above, read before any other call of the thread.
    let error = unsafe { GetLastError() };
    // SAFETY: the token handle came from OpenProcessToken and is closed once.
    unsafe { CloseHandle(token) };
    if ok == 0 || error == ERROR_NOT_ALL_ASSIGNED {
        return Err(format!("the process does not hold the privilege {name}"));
    }
    Ok(())
}

/// The hive file of the default user profile, such as `C:\Users\Default\NTUSER.DAT`.
pub(crate) fn default_user_hive() -> Result<PathBuf, String> {
    use windows_sys::Win32::UI::Shell::GetDefaultUserProfileDirectoryW;
    let mut buffer = [0u16; 512];
    let mut size = buffer.len() as u32;
    // SAFETY: the buffer is valid for the size passed, in characters.
    if unsafe { GetDefaultUserProfileDirectoryW(buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err("the folder of the default user profile is not known".to_owned());
    }
    Ok(PathBuf::from(buffer_text(&buffer)).join("NTUSER.DAT"))
}

/// The hive of the default user profile, loaded at the key of the stand-alone apply script while
/// a plan writes it.
pub(crate) struct DefaultUser {
    /// Whether this process loaded the hive; a hive loaded before stays loaded.
    loaded_here: bool,
}

impl DefaultUser {
    /// Loads the hive with the backup and restore privileges, or takes the one already loaded
    /// at the key.
    pub(super) fn mount() -> Result<Self, String> {
        use windows_sys::Win32::System::Registry::{HKEY_USERS as USERS, RegLoadKeyW};
        let sub = default_user_subkey();
        if RegKey::predef(HKEY_USERS).open_subkey(sub).is_ok() {
            return Ok(Self { loaded_here: false });
        }
        enable_privilege("SeRestorePrivilege")?;
        enable_privilege("SeBackupPrivilege")?;
        let file = default_user_hive()?;
        let (sub_w, file_w) = (wide(sub), wide(&file.display().to_string()));
        // SAFETY: both texts are null-terminated and HKEY_USERS is a predefined key.
        let status = unsafe { RegLoadKeyW(USERS, sub_w.as_ptr(), file_w.as_ptr()) };
        if status != 0 {
            let file = file.display();
            return Err(format!(
                "the default user profile {file} was not loaded (error {status})"
            ));
        }
        Ok(Self { loaded_here: true })
    }

    /// Unloads the hive when this process loaded it; every key of the hive must be closed.
    pub(super) fn unload(&mut self) -> Result<(), String> {
        use windows_sys::Win32::System::Registry::{HKEY_USERS as USERS, RegUnLoadKeyW};
        if !self.loaded_here {
            return Ok(());
        }
        self.loaded_here = false;
        let sub_w = wide(default_user_subkey());
        // SAFETY: the text is null-terminated and HKEY_USERS is a predefined key.
        let status = unsafe { RegUnLoadKeyW(USERS, sub_w.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "the default user profile was not unloaded (error {status})"
            ))
        }
    }

    pub(super) fn unmount(mut self) -> Result<(), String> {
        self.unload()
    }
}

impl Drop for DefaultUser {
    fn drop(&mut self) {
        let _ = self.unload();
    }
}
