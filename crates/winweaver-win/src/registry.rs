//! The registry of this PC for the registry tab and the MCP server, with winreg, in the paths of
//! regedit (`winweaver_core::registry`). A change the process has no right to is an error that
//! says so. The rights of a key are read and written as SDDL through the security functions of
//! Windows (`winweaver_core::security` reads them).

use std::borrow::Cow;
use std::io;

use winreg::enums::{
    HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS,
    KEY_READ, KEY_SET_VALUE, RegType,
};
use winreg::{RegKey, RegValue};
use winweaver_core::registry::{self, Data, Key, Registry, Value};

/// The registry of Windows.
pub struct NativeRegistry;

/// The hive of a path and the rest of it, with the path in the form of regedit.
fn split(path: &str) -> Result<(RegKey, String, String), String> {
    let full =
        registry::normalize(path).ok_or_else(|| format!("not a path of the registry: {path}"))?;
    let (hive, sub) = full.split_once('\\').unwrap_or((full.as_str(), ""));
    let root = match hive {
        "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
        "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        "HKEY_USERS" => HKEY_USERS,
        _ => HKEY_CURRENT_CONFIG,
    };
    let sub = sub.to_owned();
    Ok((RegKey::predef(root), sub, full))
}

fn error(e: io::Error, path: &str) -> String {
    match e.kind() {
        io::ErrorKind::PermissionDenied => format!(
            "{path}: access denied; a change of this key needs WinWeaver started as administrator"
        ),
        io::ErrorKind::NotFound => format!("{path}: the key does not exist"),
        _ => format!("{path}: {e}"),
    }
}

fn vtype(kind: u32) -> RegType {
    match kind {
        0 => RegType::REG_NONE,
        1 => RegType::REG_SZ,
        2 => RegType::REG_EXPAND_SZ,
        4 => RegType::REG_DWORD,
        5 => RegType::REG_DWORD_BIG_ENDIAN,
        6 => RegType::REG_LINK,
        7 => RegType::REG_MULTI_SZ,
        8 => RegType::REG_RESOURCE_LIST,
        9 => RegType::REG_FULL_RESOURCE_DESCRIPTOR,
        10 => RegType::REG_RESOURCE_REQUIREMENTS_LIST,
        11 => RegType::REG_QWORD,
        _ => RegType::REG_BINARY,
    }
}

impl NativeRegistry {
    fn open(path: &str, rights: u32) -> Result<(RegKey, String), String> {
        let (root, sub, full) = split(path)?;
        if sub.is_empty() {
            return Ok((root, full));
        }
        let key = root
            .open_subkey_with_flags(&sub, rights)
            .map_err(|e| error(e, &full))?;
        Ok((key, full))
    }
}

impl Registry for NativeRegistry {
    fn read(&self, path: &str) -> Result<Key, String> {
        let (key, full) = Self::open(path, KEY_READ)?;
        let mut subkeys: Vec<String> = key.enum_keys().filter_map(Result::ok).collect();
        subkeys.sort_by_key(|s| s.to_lowercase());
        let mut values: Vec<Value> = key
            .enum_values()
            .filter_map(Result::ok)
            .map(|(name, v)| Value {
                name,
                data: Data::from_raw(v.vtype as u32, &v.bytes),
            })
            .collect();
        values.sort_by_key(|v| v.name.to_lowercase());
        Ok(Key {
            path: full,
            subkeys,
            values,
        })
    }

    fn set(&self, path: &str, name: &str, data: &Data) -> Result<(), String> {
        if name.len() > registry::NAME_MAX {
            return Err("the name of the value is too long".to_owned());
        }
        let (key, full) = Self::open(path, KEY_SET_VALUE)?;
        let (kind, bytes) = data.to_raw();
        let value = RegValue {
            bytes: Cow::Owned(bytes),
            vtype: vtype(kind),
        };
        key.set_raw_value(name, &value).map_err(|e| error(e, &full))
    }

    fn delete_value(&self, path: &str, name: &str) -> Result<(), String> {
        let (key, full) = Self::open(path, KEY_SET_VALUE)?;
        key.delete_value(name).map_err(|e| error(e, &full))
    }

    fn create_key(&self, path: &str) -> Result<(), String> {
        let (root, sub, full) = split(path)?;
        if sub.is_empty() {
            return Err(format!("{full}: a hive is there already"));
        }
        root.create_subkey(&sub)
            .map(|_| ())
            .map_err(|e| error(e, &full))
    }

    fn delete_key(&self, path: &str) -> Result<(), String> {
        let (root, sub, full) = split(path)?;
        if sub.is_empty() {
            return Err(format!("{full}: a hive is not deleted"));
        }
        root.delete_subkey_all(&sub).map_err(|e| error(e, &full))
    }

    fn security(&self, path: &str) -> Result<String, String> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetNamedSecurityInfoW,
            SDDL_REVISION_1, SE_REGISTRY_KEY,
        };
        use windows_sys::Win32::Security::{
            DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        };
        let (mut name, full) = security_name(path)?;
        let what = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let (mut owner, mut group) = (std::ptr::null_mut(), std::ptr::null_mut());
        let (mut dacl, mut sacl) = (std::ptr::null_mut(), std::ptr::null_mut());
        // SAFETY: the name is null-terminated; the descriptor is allocated by the call and freed
        // below, the other pointers point into it.
        let status = unsafe {
            GetNamedSecurityInfoW(
                name.as_mut_ptr(),
                SE_REGISTRY_KEY,
                what,
                &mut owner,
                &mut group,
                &mut dacl,
                &mut sacl,
                &mut sd,
            )
        };
        if status != 0 {
            return Err(rights_error(status, &full));
        }
        let mut text: *mut u16 = std::ptr::null_mut();
        let none = std::ptr::null_mut();
        // SAFETY: the descriptor came from the call above; the text is allocated by this call.
        let ok = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                sd,
                SDDL_REVISION_1,
                what,
                &mut text,
                none,
            )
        };
        // SAFETY: the descriptor was allocated by GetNamedSecurityInfoW and is freed once.
        unsafe { LocalFree(sd) };
        if ok == 0 || text.is_null() {
            return Err(format!("{full}: the rights were not read"));
        }
        // SAFETY: the text is null-terminated and allocated by the call above.
        Ok(unsafe { take_text(text) })
    }

    fn set_security(&self, path: &str, dacl: &str, protected: bool) -> Result<(), String> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_REGISTRY_KEY,
            SetNamedSecurityInfoW,
        };
        use windows_sys::Win32::Security::{
            ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl,
            PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
            UNPROTECTED_DACL_SECURITY_INFORMATION,
        };
        let (mut name, full) = security_name(path)?;
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
            return Err(format!("{full}: Windows does not take this access list"));
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
                    SE_REGISTRY_KEY,
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
            Some(code) => Err(rights_error(code, &full)),
            None => {
                let why = format!("{full}: the access list has no entries Windows takes");
                Err(why)
            }
        }
    }

    fn account_name(&self, sid: &str) -> Option<String> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
        use windows_sys::Win32::Security::{LookupAccountSidW, PSID, SID_NAME_USE};
        let text = wide(sid);
        let mut psid: PSID = std::ptr::null_mut();
        // SAFETY: the text is null-terminated; the identifier is allocated by the call.
        if unsafe { ConvertStringSidToSidW(text.as_ptr(), &mut psid) } == 0 {
            return None;
        }
        let (mut name, mut domain) = ([0u16; 256], [0u16; 256]);
        let (mut name_len, mut domain_len) = (256u32, 256u32);
        let mut kind: SID_NAME_USE = 0;
        // SAFETY: the buffers are valid for the sizes passed, in characters.
        let ok = unsafe {
            LookupAccountSidW(
                std::ptr::null(),
                psid,
                name.as_mut_ptr(),
                &mut name_len,
                domain.as_mut_ptr(),
                &mut domain_len,
                &mut kind,
            )
        };
        // SAFETY: the identifier was allocated by the conversion above and is freed once.
        unsafe { LocalFree(psid) };
        if ok == 0 {
            return None;
        }
        let name = String::from_utf16_lossy(&name[..name_len as usize]);
        let domain = String::from_utf16_lossy(&domain[..domain_len as usize]);
        Some(if domain.is_empty() {
            name
        } else {
            format!("{domain}\\{name}")
        })
    }

    fn account_sid(&self, name: &str) -> Option<String> {
        use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
        use windows_sys::Win32::Security::{LookupAccountNameW, SECURITY_MAX_SID_SIZE, SID_NAME_USE};
        let text = wide(name);
        // A SID of the largest size, aligned for its 32-bit sub-authorities.
        let mut sid = [0u32; SECURITY_MAX_SID_SIZE as usize / 4];
        let mut size = SECURITY_MAX_SID_SIZE;
        let mut domain = [0u16; 256];
        let mut domain_len = 256u32;
        let mut kind: SID_NAME_USE = 0;
        // SAFETY: the buffers are valid for the sizes passed.
        let ok = unsafe {
            LookupAccountNameW(
                std::ptr::null(),
                text.as_ptr(),
                sid.as_mut_ptr().cast(),
                &mut size,
                domain.as_mut_ptr(),
                &mut domain_len,
                &mut kind,
            )
        };
        if ok == 0 {
            return None;
        }
        let mut out: *mut u16 = std::ptr::null_mut();
        // SAFETY: the identifier was written by the call above; the text is allocated here.
        if unsafe { ConvertSidToStringSidW(sid.as_mut_ptr().cast(), &mut out) } == 0 {
            return None;
        }
        // SAFETY: the text is null-terminated and allocated by the call above.
        Some(unsafe { take_text(out) })
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// A key as the security functions of Windows name it (`MACHINE\SOFTWARE`), null-terminated,
/// with the path in the form of regedit for the messages.
fn security_name(path: &str) -> Result<(Vec<u16>, String), String> {
    let full =
        registry::normalize(path).ok_or_else(|| format!("not a path of the registry: {path}"))?;
    let (hive, rest) = full.split_once('\\').unwrap_or((full.as_str(), ""));
    let root = match hive {
        "HKEY_CLASSES_ROOT" => "CLASSES_ROOT",
        "HKEY_CURRENT_USER" => "CURRENT_USER",
        "HKEY_LOCAL_MACHINE" => "MACHINE",
        "HKEY_USERS" => "USERS",
        _ => "CURRENT_CONFIG",
    };
    let name = if rest.is_empty() {
        root.to_owned()
    } else {
        format!("{root}\\{rest}")
    };
    Ok((wide(&name), full))
}

/// A text Windows allocated, read up to its null and released with LocalFree.
///
/// # Safety
/// `p` is a null-terminated text allocated by Windows for LocalFree.
unsafe fn take_text(p: *mut u16) -> String {
    use windows_sys::Win32::Foundation::LocalFree;
    let mut len = 0usize;
    // SAFETY: the caller guarantees a terminating null.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: the `len` units before the null were read just above.
    let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) });
    // SAFETY: the caller guarantees the text is released by LocalFree, once.
    unsafe { LocalFree(p.cast()) };
    text
}

/// The message of an error of the security functions of Windows.
fn rights_error(code: u32, path: &str) -> String {
    match code {
        2 => format!("{path}: the key does not exist"),
        5 => format!(
            "{path}: access denied; the rights of this key change only with WinWeaver started as administrator"
        ),
        n => format!("{path}: the rights were not read or changed (error {n})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winweaver_core::security::{Entry, Rights, Scope};

    #[test]
    fn the_rights_of_a_test_key_are_read_and_written() {
        let pid = std::process::id();
        let key = format!("HKEY_CURRENT_USER\\Software\\WinWeaverRights-{pid}");
        let reg = NativeRegistry;
        reg.create_key(&key).unwrap();
        let mut rights = Rights::parse(&reg.security(&key).unwrap()).unwrap();
        assert!(!rights.entries.is_empty());
        rights.entries.push(Entry::new("BU", false, false, Scope::Key));
        reg.set_security(&key, &rights.dacl(), rights.protected)
            .unwrap();
        let again = Rights::parse(&reg.security(&key).unwrap()).unwrap();
        let added = again
            .entries
            .iter()
            .any(|e| e.sid == "BU" && !e.inherited && e.read());
        assert!(added, "{again:?}");
        let admins = reg.account_name("BA").unwrap();
        assert_eq!(reg.account_sid(&admins).as_deref(), Some("S-1-5-32-544"));
        reg.delete_key(&key).unwrap();
    }
}
