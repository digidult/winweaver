//! The flags of local accounts an apply plan changes ("password never expires", disabled), through
//! the network management API of Windows, as the script steps of the rules do with PowerShell. No
//! password is read or written.

use windows_sys::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NetUserGetInfo, NetUserSetInfo, UF_ACCOUNTDISABLE, UF_DONT_EXPIRE_PASSWD,
    USER_INFO_1, USER_INFO_1008,
};

use crate::ffi::{NetBuffer, USER_NOT_FOUND, each_user, net_error, wide, wide_text};

/// The flag of an account whose password never expires.
pub const NEVER_EXPIRES: u32 = UF_DONT_EXPIRE_PASSWD;
/// The flag of a disabled account.
pub const DISABLED: u32 = UF_ACCOUNTDISABLE;

/// The flags of an account; `None` when it does not exist.
pub(crate) fn flags(name: &str) -> Result<Option<u32>, String> {
    let name_w = wide(name);
    let mut buffer = NetBuffer::default();
    // SAFETY: the name is null-terminated; the call allocates the buffer of level 1.
    let status = unsafe { NetUserGetInfo(std::ptr::null(), name_w.as_ptr(), 1, buffer.out()) };
    if status == USER_NOT_FOUND {
        return Ok(None);
    }
    if status != NERR_Success || buffer.get().is_null() {
        let e = format!("the account {name} was not read ({})", net_error(status));
        return Err(e);
    }
    // SAFETY: on success the buffer holds one entry of level 1.
    let flags = unsafe { (*buffer.get().cast::<USER_INFO_1>()).usri1_flags };
    Ok(Some(flags))
}

/// Sets or clears a flag of an account: whether it changed, `None` when the account does not
/// exist.
pub fn set_flag(name: &str, flag: u32, on: bool) -> Result<Option<bool>, String> {
    let Some(now) = flags(name)? else {
        return Ok(None);
    };
    let wanted = if on { now | flag } else { now & !flag };
    if wanted == now {
        return Ok(Some(false));
    }
    let info = USER_INFO_1008 {
        usri1008_flags: wanted,
    };
    let name_w = wide(name);
    let mut bad = 0u32;
    let buffer = (&raw const info).cast::<u8>();
    // SAFETY: the name is null-terminated and the structure is the one of level 1008.
    let status =
        unsafe { NetUserSetInfo(std::ptr::null(), name_w.as_ptr(), 1008, buffer, &mut bad) };
    if status != NERR_Success {
        let e = format!("the account {name} was not changed ({})", net_error(status));
        return Err(e);
    }
    Ok(Some(true))
}

/// The names of the built-in accounts Administrator and Guest (relative ids 500 and 501), in the
/// language of this Windows.
pub fn builtin() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    each_user(|entry| {
        if matches!(entry.usri20_user_id, 500 | 501) {
            // SAFETY: the name is a null-terminated string inside the buffer of the call.
            names.push(unsafe { wide_text(entry.usri20_name) });
        }
    })
    .map_err(|status| format!("the local accounts were not read ({})", net_error(status)))?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_accounts_are_found_and_a_missing_one_is_none() {
        let names = builtin().unwrap();
        assert!(!names.is_empty(), "Windows has a built-in Administrator");
        let missing = set_flag("WinWeaverNoSuchAccount", NEVER_EXPIRES, true);
        assert_eq!(missing, Ok(None));
    }
}
