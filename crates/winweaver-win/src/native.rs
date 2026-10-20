//! The native reader: winreg for the registry, windows-sys for the token and the locale names.

use std::io;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, RegType};
use winweaver_core::thispc::{
    Components, InputLanguage, Machine, ReadAccount, RegData, SystemInfo,
};

/// This PC, read through the Windows API. It never writes.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeMachine;

/// The root hive and the subkey of a path such as `HKLM:\SOFTWARE\X`.
fn split(path: &str) -> Option<(RegKey, &str)> {
    let (root, sub) = path.split_once(":\\")?;
    let hive = match root.to_ascii_uppercase().as_str() {
        "HKLM" => HKEY_LOCAL_MACHINE,
        "HKCU" => HKEY_CURRENT_USER,
        "HKU" => HKEY_USERS,
        _ => return None,
    };
    Some((RegKey::predef(hive), sub))
}

fn open(path: &str) -> Result<Option<RegKey>, String> {
    let Some((root, sub)) = split(path) else {
        return Err(format!("unsupported registry path {path}"));
    };
    match root.open_subkey_with_flags(sub, KEY_READ) {
        Ok(key) => Ok(Some(key)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&units)
}

fn convert(vtype: &RegType, bytes: &[u8]) -> RegData {
    match vtype {
        RegType::REG_DWORD if bytes.len() >= 4 => {
            RegData::Dword(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        RegType::REG_QWORD if bytes.len() >= 8 => {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[..8]);
            RegData::Qword(u64::from_le_bytes(b))
        }
        RegType::REG_SZ => RegData::Str(utf16(bytes).trim_end_matches('\0').to_owned()),
        RegType::REG_EXPAND_SZ => {
            RegData::ExpandStr(utf16(bytes).trim_end_matches('\0').to_owned())
        }
        RegType::REG_MULTI_SZ => {
            let text = utf16(bytes);
            RegData::Multi(
                text.trim_end_matches('\0')
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
            )
        }
        RegType::REG_BINARY => RegData::Binary(bytes.to_vec()),
        _ => RegData::Other,
    }
}

fn text(machine: &NativeMachine, path: &str, name: &str) -> String {
    match machine.value(path, name) {
        Ok(Some(RegData::Str(s))) | Ok(Some(RegData::ExpandStr(s))) => s,
        Ok(Some(other)) => other.text(),
        _ => String::new(),
    }
}

/// The culture name of a locale id, such as `uk-UA` for 0x0422.
fn locale_name(lcid: u32) -> Option<String> {
    use windows_sys::Win32::Globalization::LCIDToLocaleName;
    let mut buffer = [0u16; 85];
    // SAFETY: the buffer is valid for its length, which is passed; the function writes at most
    // that many characters and returns how many, the terminating null included.
    let written = unsafe { LCIDToLocaleName(lcid, buffer.as_mut_ptr(), buffer.len() as i32, 0) };
    if written <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..written as usize - 1]))
}

/// A locale id written in hexadecimal, such as `00000422` or `0422`.
fn hex_locale(text: &str) -> Option<String> {
    u32::from_str_radix(text.trim(), 16)
        .ok()
        .and_then(locale_name)
}

/// The language of the Windows interface of the current user, such as `uk`.
pub fn ui_language() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // SAFETY: the function takes no arguments and only returns a language id.
    let lang = unsafe { GetUserDefaultUILanguage() };
    let name = locale_name(u32::from(lang))?;
    name.split('-').next().map(str::to_lowercase)
}

/// Whether the process runs elevated.
pub(crate) fn elevated() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: the pseudo handle of the current process is always valid; the token handle is
    // written by the call and closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size = 0u32;
    // SAFETY: the buffer is a TOKEN_ELEVATION and its size is passed.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
    };
    // SAFETY: the token handle came from OpenProcessToken.
    unsafe { CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
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

/// A text as a null-terminated wide string for the Windows API.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The name of a built-in group in the language of this Windows, such as `Administratoren` on a
/// German Windows.
pub(crate) fn builtin_group(
    kind: windows_sys::Win32::Security::WELL_KNOWN_SID_TYPE,
) -> Option<String> {
    use windows_sys::Win32::Security::{
        CreateWellKnownSid, LookupAccountSidW, SECURITY_MAX_SID_SIZE, SID_NAME_USE,
    };
    // a SID of the largest size, aligned for its 32-bit sub-authorities
    let mut sid = [0u32; SECURITY_MAX_SID_SIZE as usize / 4];
    let mut size = SECURITY_MAX_SID_SIZE;
    // SAFETY: the buffer holds the largest SID and its size in bytes is passed.
    let made = unsafe {
        CreateWellKnownSid(
            kind,
            std::ptr::null_mut(),
            sid.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if made == 0 {
        return None;
    }
    let mut name = [0u16; 256];
    let mut name_len = name.len() as u32;
    let mut domain = [0u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut kind_of_name: SID_NAME_USE = 0;
    // SAFETY: the SID was written above; both buffers are valid for the lengths passed.
    let found = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid.as_mut_ptr().cast(),
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind_of_name,
        )
    };
    if found == 0 {
        return None;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    Some(String::from_utf16_lossy(&name[..end]))
}

/// The local groups an account belongs to directly, in lower case; none when they cannot be read.
pub(crate) fn groups_of(user: &str) -> Vec<String> {
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        LOCALGROUP_USERS_INFO_0, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree,
        NetUserGetLocalGroups,
    };
    let name = wide(user);
    let mut buffer: *mut u8 = std::ptr::null_mut();
    let (mut read, mut total) = (0u32, 0u32);
    // SAFETY: the name is null-terminated; the call allocates the buffer and writes its counts.
    let status = unsafe {
        NetUserGetLocalGroups(
            std::ptr::null(),
            name.as_ptr(),
            0,
            0,
            &mut buffer,
            MAX_PREFERRED_LENGTH,
            &mut read,
            &mut total,
        )
    };
    let mut groups = Vec::new();
    if status == NERR_Success && !buffer.is_null() {
        // SAFETY: on success the buffer holds `read` entries of level 0.
        let entries = unsafe {
            std::slice::from_raw_parts(buffer.cast::<LOCALGROUP_USERS_INFO_0>(), read as usize)
        };
        for entry in entries {
            // SAFETY: each name is a null-terminated string inside the buffer.
            groups.push(unsafe { wide_text(entry.lgrui0_name) }.to_lowercase());
        }
    }
    if !buffer.is_null() {
        // SAFETY: the buffer came from the call above and is freed once.
        unsafe { NetApiBufferFree(buffer.cast_const().cast()) };
    }
    groups
}

/// The enabled local accounts but the built-in ones, with their group, the way the audit script
/// reads them: the relative ids 500, 501, 503 and 504 are Administrator, Guest, DefaultAccount
/// and WDAGUtilityAccount. A password is never read.
fn local_accounts() -> Result<Vec<ReadAccount>, String> {
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree, NetUserEnum,
        UF_ACCOUNTDISABLE, USER_INFO_20,
    };
    use windows_sys::Win32::Security::{WinBuiltinAdministratorsSid, WinBuiltinUsersSid};
    let admins = builtin_group(WinBuiltinAdministratorsSid)
        .ok_or("the name of the group Administrators could not be read")?
        .to_lowercase();
    let users = builtin_group(WinBuiltinUsersSid)
        .ok_or("the name of the group Users could not be read")?
        .to_lowercase();
    let mut accounts = Vec::new();
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
                if entry.usri20_flags & UF_ACCOUNTDISABLE != 0
                    || matches!(entry.usri20_user_id, 500 | 501 | 503 | 504)
                {
                    continue;
                }
                // SAFETY: the texts are null-terminated strings inside the buffer.
                let (name, full_name, description) = unsafe {
                    (
                        wide_text(entry.usri20_name),
                        wide_text(entry.usri20_full_name),
                        wide_text(entry.usri20_comment),
                    )
                };
                let groups = groups_of(&name);
                let group = if groups.contains(&admins) {
                    "Administrators"
                } else if groups.contains(&users) {
                    "Users"
                } else {
                    ""
                };
                accounts.push(ReadAccount {
                    name,
                    full_name,
                    description,
                    group: group.to_owned(),
                });
            }
        }
        if !buffer.is_null() {
            // SAFETY: the buffer came from the call above and is freed once.
            unsafe { NetApiBufferFree(buffer.cast_const().cast()) };
        }
        if status == NERR_Success {
            return Ok(accounts);
        }
        if !more || read == 0 {
            let error = format!("the local accounts could not be read (error {status})");
            return Err(error);
        }
    }
}

impl Machine for NativeMachine {
    fn value(&self, path: &str, name: &str) -> Result<Option<RegData>, String> {
        let Some(key) = open(path)? else {
            return Ok(None);
        };
        let name = if name == "(Default)" { "" } else { name };
        match key.get_raw_value(name) {
            Ok(v) => Ok(Some(convert(&v.vtype, &v.bytes))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn subkeys(&self, path: &str) -> Result<Option<Vec<String>>, String> {
        let Some(key) = open(path)? else {
            return Ok(None);
        };
        key.enum_keys()
            .collect::<io::Result<Vec<String>>>()
            .map(Some)
            .map_err(|e| e.to_string())
    }

    fn values(&self, path: &str) -> Result<Option<Vec<(String, RegData)>>, String> {
        let Some(key) = open(path)? else {
            return Ok(None);
        };
        key.enum_values()
            .map(|v| v.map(|(name, value)| (name, convert(&value.vtype, &value.bytes))))
            .collect::<io::Result<Vec<_>>>()
            .map(Some)
            .map_err(|e| e.to_string())
    }

    fn is_admin(&self) -> bool {
        elevated()
    }

    fn computer_name(&self) -> String {
        std::env::var("COMPUTERNAME").unwrap_or_default()
    }

    fn components(&self) -> Components {
        crate::dism::components()
    }

    fn system_info(&self) -> SystemInfo {
        const CV: &str = "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        let input = match self.value(
            "HKCU:\\Control Panel\\International\\User Profile",
            "Languages",
        ) {
            Ok(Some(RegData::Multi(tags))) => tags
                .into_iter()
                .map(|tag| {
                    // The layouts of a language are the values named like 0422:00020422 of its
                    // key, as Get-WinUserLanguageList lists them in InputMethodTips.
                    let mut tips: Vec<(i64, String)> = self
                        .values(&format!(
                            "HKCU:\\Control Panel\\International\\User Profile\\{tag}"
                        ))
                        .ok()
                        .flatten()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|(name, _)| name.contains(':'))
                        .map(|(name, data)| match data {
                            RegData::Dword(order) => (i64::from(order), name),
                            _ => (i64::MAX, name),
                        })
                        .collect();
                    tips.sort();
                    InputLanguage {
                        tag,
                        tips: tips.into_iter().map(|(_, name)| name).collect(),
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        let (accounts, accounts_error) = match local_accounts() {
            Ok(list) => (list, String::new()),
            Err(e) => (Vec::new(), e),
        };
        SystemInfo {
            computer_name: self.computer_name(),
            edition_id: text(self, CV, "EditionID"),
            build: text(self, CV, "CurrentBuild"),
            display_version: text(self, CV, "DisplayVersion"),
            time_zone: text(
                self,
                "HKLM:\\SYSTEM\\CurrentControlSet\\Control\\TimeZoneInformation",
                "TimeZoneKeyName",
            ),
            ui_language: hex_locale(&text(
                self,
                "HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Nls\\Language",
                "InstallLanguage",
            ))
            .unwrap_or_default(),
            system_locale: hex_locale(&text(
                self,
                "HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Nls\\Locale",
                "(Default)",
            ))
            .unwrap_or_default(),
            user_locale: text(self, "HKCU:\\Control Panel\\International", "LocaleName"),
            input,
            accounts,
            accounts_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winweaver_core::catalog::Catalog;
    use winweaver_core::profile::Profile;
    use winweaver_core::thispc;

    #[test]
    fn reads_the_registry_of_this_pc() {
        let m = NativeMachine;
        let build = m.value(
            "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "CurrentBuild",
        );
        assert!(matches!(build, Ok(Some(RegData::Str(_)))), "{build:?}");
        assert_eq!(
            m.value("HKLM:\\SOFTWARE\\WinWeaverNoSuchKey", "x"),
            Ok(None)
        );
        assert_eq!(
            m.value(
                "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
                "NoSuchValue"
            ),
            Ok(None)
        );
        let services = m
            .subkeys("HKLM:\\SYSTEM\\CurrentControlSet\\Services")
            .unwrap();
        assert!(services.is_some_and(|s| s.len() > 10));
        assert!(m.value("XX:\\a", "b").is_err());
        let info = m.system_info();
        assert!(
            !info.build.is_empty() && !info.edition_id.is_empty(),
            "{info:?}"
        );
        assert_eq!(locale_name(0x0409).as_deref(), Some("en-US"));
        assert_eq!(hex_locale("00000422").as_deref(), Some("uk-UA"));
    }

    #[test]
    fn reads_the_local_accounts_without_a_password() {
        let accounts = local_accounts().expect("the local accounts");
        assert!(accounts.iter().all(|a| !a.name.is_empty()), "{accounts:?}");
        let groups = ["Administrators", "Users", ""];
        assert!(
            accounts.iter().all(|a| groups.contains(&a.group.as_str())),
            "{accounts:?}"
        );
        let admins = builtin_group(windows_sys::Win32::Security::WinBuiltinAdministratorsSid);
        assert!(admins.is_some_and(|n| !n.is_empty()));
        let info = NativeMachine.system_info();
        assert!(info.accounts_error.is_empty(), "{}", info.accounts_error);
        assert_eq!(info.accounts, accounts);
    }

    #[test]
    fn the_whole_catalog_reads_without_a_failure() {
        let catalog = Catalog::embedded().unwrap();
        let profile = Profile::from_defaults("Office", &catalog);
        let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
        let plan = thispc::plan(&catalog, &profile, &ids);
        let report = thispc::run(&NativeMachine, &plan, true, "now");
        assert_eq!(report.results.len(), plan.checks.len());
        let readings = thispc::judge(&catalog, &plan, &report.results);
        assert_eq!(readings.len(), plan.rules.len());
        let (_, summary) = thispc::capture(&catalog, &profile, &readings, report.system.as_ref());
        assert!(!summary.line().is_empty());
    }
}
