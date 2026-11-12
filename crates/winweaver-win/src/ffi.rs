//! The small wrappers of the Windows API every module of this crate needs, each written once:
//! texts to and from UTF-16, the buffers of the network management API, security identifiers and
//! the accounts they name, the accounts of `NetUserEnum`, the access list of an object, the hives
//! of the paths of the catalog, the folders of the system and the messages of the status codes.

use std::path::PathBuf;

use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, LocalFree};
use windows_sys::Win32::NetworkManagement::NetManagement::{
    FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree, NetUserEnum,
    USER_INFO_20,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW, SDDL_REVISION_1,
    SE_OBJECT_TYPE, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, LookupAccountSidW,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SID_NAME_USE,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemDirectoryW, GetSystemWindowsDirectoryW,
};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS};

/// The error of an account that does not exist (NERR_UserNotFound).
pub(crate) const USER_NOT_FOUND: u32 = 2221;

/// A text as a null-terminated wide string for the Windows API.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// The UTF-16 bytes of a text with its terminator, as a registry value holds them.
pub(crate) fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// The text of a null-terminated wide string the system wrote, empty for a null pointer.
///
/// # Safety
///
/// `p` is null or points to a null-terminated UTF-16 string that stays valid during the call.
pub(crate) unsafe fn wide_text(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    // SAFETY: the caller guarantees a terminating null, so every unit up to it is readable.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: the `len` units before the null were read just above.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

/// The text of a buffer the system filled, up to its first null (the whole buffer without one).
pub(crate) fn buffer_text(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// A text Windows allocated, read up to its null and released with LocalFree.
///
/// # Safety
///
/// `p` is a null-terminated text allocated by Windows for LocalFree, released once.
pub(crate) unsafe fn take_text(p: *mut u16) -> String {
    // SAFETY: the caller guarantees a terminating null.
    let text = unsafe { wide_text(p) };
    // SAFETY: the caller guarantees the text is released by LocalFree, once.
    unsafe { LocalFree(p.cast()) };
    text
}

/// A buffer the network management API allocates, freed when dropped.
#[derive(Default)]
pub(crate) struct NetBuffer(*mut u8);

impl NetBuffer {
    /// Where a call writes the address of the buffer.
    pub(crate) fn out(&mut self) -> *mut *mut u8 {
        &mut self.0
    }

    /// The buffer, null until a call filled it.
    pub(crate) fn get(&self) -> *mut u8 {
        self.0
    }
}

impl Drop for NetBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the buffer came from a network management call and is freed once.
            unsafe { NetApiBufferFree(self.0.cast_const().cast()) };
        }
    }
}

/// A security identifier as the API wants it, freed when dropped.
pub(crate) struct Sid(pub(crate) PSID);

impl Sid {
    /// The identifier of its text form (`S-1-5-32-544`), or an alias of SDDL (`BA`).
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let text_w = wide(text);
        let mut psid: PSID = std::ptr::null_mut();
        // SAFETY: the text is null-terminated; the identifier is allocated by the call.
        if unsafe { ConvertStringSidToSidW(text_w.as_ptr(), &mut psid) } == 0 {
            return Err(format!("{text} is not an identifier"));
        }
        Ok(Self(psid))
    }
}

impl Drop for Sid {
    fn drop(&mut self) {
        // SAFETY: the identifier was allocated by ConvertStringSidToSidW and is freed once.
        unsafe { LocalFree(self.0) };
    }
}

/// The name and the domain of the account of an identifier, as this Windows calls them.
///
/// # Safety
///
/// `sid` points to a valid security identifier during the call.
pub(crate) unsafe fn account_of_sid(sid: PSID) -> Option<(String, String)> {
    let (mut name, mut domain) = ([0u16; 256], [0u16; 256]);
    let (mut name_len, mut domain_len) = (256u32, 256u32);
    let mut kind: SID_NAME_USE = 0;
    // SAFETY: the identifier is valid by the contract of the caller; both buffers are valid for
    // the lengths passed, in characters.
    let found = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    };
    if found == 0 {
        return None;
    }
    Some((buffer_text(&name), buffer_text(&domain)))
}

/// Every normal local account of this PC (level 20 of `NetUserEnum`), page by page; the status
/// of the call when the list could not be read.
pub(crate) fn each_user(mut each: impl FnMut(&USER_INFO_20)) -> Result<(), u32> {
    let mut resume = 0u32;
    loop {
        let mut buffer = NetBuffer::default();
        let (mut read, mut total) = (0u32, 0u32);
        // SAFETY: the call allocates the buffer and writes its counts and the resume handle.
        let status = unsafe {
            NetUserEnum(
                std::ptr::null(),
                20,
                FILTER_NORMAL_ACCOUNT,
                buffer.out(),
                MAX_PREFERRED_LENGTH,
                &mut read,
                &mut total,
                &mut resume,
            )
        };
        let more = status == ERROR_MORE_DATA;
        if (status == NERR_Success || more) && !buffer.get().is_null() {
            // SAFETY: the buffer holds `read` entries of level 20.
            let entries = unsafe {
                std::slice::from_raw_parts(buffer.get().cast::<USER_INFO_20>(), read as usize)
            };
            for entry in entries {
                each(entry);
            }
        }
        if status == NERR_Success {
            return Ok(());
        }
        if !more || read == 0 {
            return Err(status);
        }
    }
}

/// The text of a status of the network management API the operator can act on.
pub(crate) fn net_error(status: u32) -> String {
    match status {
        5 => "access denied: WinWeaver needs to run as administrator".to_owned(),
        2224 => "the name is taken by an account".to_owned(),
        2223 => "the name is taken by a group".to_owned(),
        2245 => {
            "the password does not meet the password policy of this PC (its length, its complexity, a part of the name of the account in it, or a password used before)"
                .to_owned()
        }
        USER_NOT_FOUND => "the account does not exist".to_owned(),
        other => format!("error {other}"),
    }
}

/// Why an access list was not written.
pub(crate) enum DaclError {
    /// Windows does not read the SDDL text.
    NotTaken,
    /// The text has no entries.
    Empty,
    /// The status of `SetNamedSecurityInfoW`.
    Status(u32),
}

/// Writes the access list of an SDDL text (`D:...`) on an object named for the security
/// functions of Windows (a key as `MACHINE\SOFTWARE`, a folder by its path); `protected` stops the
/// inheritance from the object above.
pub(crate) fn set_dacl(
    object: SE_OBJECT_TYPE,
    name: &str,
    dacl: &str,
    protected: bool,
) -> Result<(), DaclError> {
    let mut name = wide(name);
    let text = wide(dacl);
    let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let size = std::ptr::null_mut();
    // SAFETY: the text is null-terminated; the descriptor is allocated by the call.
    let made = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            size,
        )
    };
    if made == 0 {
        return Err(DaclError::NotTaken);
    }
    let (mut present, mut defaulted) = (0, 0);
    let mut list: *mut ACL = std::ptr::null_mut();
    // SAFETY: the descriptor came from the call above; the list points into it.
    let ok = unsafe { GetSecurityDescriptorDacl(sd, &mut present, &mut list, &mut defaulted) };
    let inherit = if protected {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    let status = if ok == 0 || present == 0 {
        None
    } else {
        let none = std::ptr::null_mut();
        // SAFETY: the name is null-terminated and the list lives in the descriptor, which is
        // freed only below.
        Some(unsafe {
            SetNamedSecurityInfoW(
                name.as_mut_ptr(),
                object,
                DACL_SECURITY_INFORMATION | inherit,
                none,
                none,
                list,
                std::ptr::null(),
            )
        })
    };
    // SAFETY: the descriptor was allocated by the conversion above and is freed once.
    unsafe { LocalFree(sd) };
    match status {
        Some(0) => Ok(()),
        Some(code) => Err(DaclError::Status(code)),
        None => Err(DaclError::Empty),
    }
}

/// The root hive and the subkey of a path of the catalog, such as `HKLM:\SOFTWARE\X`.
pub(crate) fn hive_path(path: &str) -> Result<(RegKey, String), String> {
    let (root, sub) = path
        .split_once(":\\")
        .ok_or_else(|| format!("unsupported registry path {path}"))?;
    let hive = match root.to_ascii_uppercase().as_str() {
        "HKLM" => HKEY_LOCAL_MACHINE,
        "HKCU" => HKEY_CURRENT_USER,
        "HKU" => HKEY_USERS,
        _ => return Err(format!("unsupported registry path {path}")),
    };
    Ok((RegKey::predef(hive), sub.to_owned()))
}

/// A folder of Windows as a function of the system names it (the functions that fill a buffer
/// and give the length they wrote); `None` when the call fails or the buffer is too small.
fn folder_of(call: unsafe extern "system" fn(*mut u16, u32) -> u32) -> Option<PathBuf> {
    let mut buffer = [0u16; 1024];
    // SAFETY: the buffer holds the number of units the call is told, and the call writes no more.
    let len = unsafe { call(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    (len > 0 && len < buffer.len()).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..len])))
}

/// The system folder (`C:\Windows\System32`), asked of Windows itself (`GetSystemDirectoryW`)
/// and never read from the environment: `SystemRoot` is set by whoever starts a process, and
/// the elevated helper runs the programs of this folder as administrator.
pub(crate) fn system_dir() -> PathBuf {
    folder_of(GetSystemDirectoryW).unwrap_or_else(|| PathBuf::from("C:\\Windows\\System32"))
}

/// The folder of Windows (`C:\Windows`), asked of Windows itself (`GetSystemWindowsDirectoryW`).
pub(crate) fn windows_dir() -> PathBuf {
    folder_of(GetSystemWindowsDirectoryW).unwrap_or_else(|| PathBuf::from("C:\\Windows"))
}

/// The letter of the drive Windows is on, in upper case.
pub(crate) fn system_drive() -> Option<u8> {
    let dir = system_dir();
    let text = dir.to_string_lossy();
    let bytes = text.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| bytes[0].to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The folders of the system come from Windows, whatever the environment says.
    #[test]
    fn the_system_folder_is_the_one_of_windows() {
        let system = system_dir();
        assert!(system.is_absolute() && system.join("kernel32.dll").is_file());
        let windows = windows_dir();
        assert!(windows.join("explorer.exe").is_file(), "{windows:?}");
        assert!(system.starts_with(&windows), "{system:?}");
        let drive = system_drive().expect("a drive letter");
        assert!(drive.is_ascii_uppercase());
        assert_eq!(
            Some(drive),
            windows
                .to_string_lossy()
                .bytes()
                .next()
                .map(|b| b.to_ascii_uppercase())
        );
    }

    #[test]
    fn texts_go_to_utf16_and_back() {
        let w = wide("ab");
        assert_eq!(w, vec![97, 98, 0]);
        assert_eq!(utf16_bytes("a"), vec![97, 0, 0, 0]);
        // SAFETY: the vector is null-terminated.
        assert_eq!(unsafe { wide_text(w.as_ptr()) }, "ab");
        // SAFETY: a null pointer gives an empty text.
        assert_eq!(unsafe { wide_text(std::ptr::null()) }, "");
        assert_eq!(buffer_text(&[97, 0, 98]), "a");
        assert_eq!(buffer_text(&[97, 98]), "ab");
        assert!(hive_path("HKLM:\\SOFTWARE").is_ok());
        assert!(hive_path("C:\\x").is_err());
        assert!(net_error(0).starts_with("error"));
        let admins = Sid::parse("BA").unwrap();
        // SAFETY: the identifier is valid while `admins` lives.
        let (name, _) = unsafe { account_of_sid(admins.0) }.unwrap();
        assert!(!name.is_empty());
        assert!(Sid::parse("not a sid").is_err());
    }
}
