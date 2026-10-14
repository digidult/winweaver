//! The native reader: winreg for the registry, windows-sys for the token and the locale names.

use std::io;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, RegType};
use winweaver_core::thispc::{InputLanguage, Machine, RegData, SystemInfo};

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
fn elevated() -> bool {
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

    fn is_admin(&self) -> bool {
        elevated()
    }

    fn computer_name(&self) -> String {
        std::env::var("COMPUTERNAME").unwrap_or_default()
    }

    fn system_info(&self) -> SystemInfo {
        const CV: &str = "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        let input = match self.value(
            "HKCU:\\Control Panel\\International\\User Profile",
            "Languages",
        ) {
            Ok(Some(RegData::Multi(tags))) => tags
                .into_iter()
                .map(|tag| InputLanguage {
                    tag,
                    tips: Vec::new(),
                })
                .collect(),
            _ => Vec::new(),
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
            accounts: Vec::new(),
            accounts_error: "local accounts are not read by the program yet".to_owned(),
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
