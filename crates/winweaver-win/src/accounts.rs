//! The flags of local accounts an apply plan changes ("password never expires", disabled), through
//! the network management API of Windows, as the script steps of the rules do with PowerShell. No
//! password is read or written.

use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
use windows_sys::Win32::NetworkManagement::NetManagement::{
    FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree, NetUserEnum,
    NetUserGetInfo, NetUserSetInfo, UF_ACCOUNTDISABLE, UF_DONT_EXPIRE_PASSWD, USER_INFO_1,
    USER_INFO_20, USER_INFO_1008,
};

use crate::native::wide_text;

/// The flag of an account whose password never expires.
pub const NEVER_EXPIRES: u32 = UF_DONT_EXPIRE_PASSWD;
/// The flag of a disabled account.
pub const DISABLED: u32 = UF_ACCOUNTDISABLE;
/// The error of an account that does not exist (NERR_UserNotFound).
const USER_NOT_FOUND: u32 = 2221;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

pub(crate) fn free(buffer: *mut u8) {
    if !buffer.is_null() {
        // SAFETY: the buffer came from a network management call and is freed once.
        unsafe { NetApiBufferFree(buffer.cast_const().cast()) };
    }
}

/// The flags of an account; `None` when it does not exist.
pub(crate) fn flags(name: &str) -> Result<Option<u32>, String> {
    let name_w = wide(name);
    let mut buffer: *mut u8 = std::ptr::null_mut();
    // SAFETY: the name is null-terminated; the call allocates the buffer of level 1.
    let status = unsafe { NetUserGetInfo(std::ptr::null(), name_w.as_ptr(), 1, &mut buffer) };
    if status == USER_NOT_FOUND {
        free(buffer);
        return Ok(None);
    }
    if status != NERR_Success || buffer.is_null() {
        free(buffer);
        let e = format!("the account {name} was not read (error {status})");
        return Err(e);
    }
    // SAFETY: on success the buffer holds one entry of level 1.
    let flags = unsafe { (*buffer.cast::<USER_INFO_1>()).usri1_flags };
    free(buffer);
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
        let e = format!("the account {name} was not changed (error {status})");
        return Err(e);
    }
    Ok(Some(true))
}

/// The names of the built-in accounts Administrator and Guest (relative ids 500 and 501), in the
/// language of this Windows.
pub fn builtin() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut resume = 0u32;
    loop {
        let mut buffer: *mut u8 = std::ptr::null_mut();
        let (mut read, mut total) = (0u32, 0u32);
        // SAFETY: the call allocates the buffer and writes its counts and the resume handle.
        let status = unsafe {
            NetUserEnum(
                std::ptr::null(),
                20,
                FILTER_NORMAL_ACCOUNT,
                &mut buffer,
                MAX_PREFERRED_LENGTH,
                &mut read,
                &mut total,
                &mut resume,
            )
        };
        let more = status == ERROR_MORE_DATA;
        if (status == NERR_Success || more) && !buffer.is_null() {
            // SAFETY: the buffer holds `read` entries of level 20.
            let entries =
                unsafe { std::slice::from_raw_parts(buffer.cast::<USER_INFO_20>(), read as usize) };
            for entry in entries {
                if matches!(entry.usri20_user_id, 500 | 501) {
                    // SAFETY: the name is a null-terminated string inside the buffer.
                    names.push(unsafe { wide_text(entry.usri20_name) });
                }
            }
        }
        free(buffer);
        if !more {
            if status != NERR_Success {
                let e = format!("the local accounts were not read (error {status})");
                return Err(e);
            }
            return Ok(names);
        }
    }
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
