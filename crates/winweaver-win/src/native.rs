//! The native reader: winreg for the registry, windows-sys for the token and the locale names.

use std::io;

use winreg::RegKey;
use winreg::enums::KEY_READ;
use winweaver_core::registry::Data;
use winweaver_core::thispc::{Components, InputLanguage, Machine, ReadAccount, SystemInfo};

use crate::ffi::{self, NetBuffer, each_user, hive_path, wide, wide_text};

/// This PC, read through the Windows API. It never writes.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeMachine;

fn open(path: &str) -> Result<Option<RegKey>, String> {
    let (root, sub) = hive_path(path)?;
    match root.open_subkey_with_flags(&sub, KEY_READ) {
        Ok(key) => Ok(Some(key)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn text(machine: &NativeMachine, path: &str, name: &str) -> String {
    match machine.value(path, name) {
        Ok(Some(Data::Sz(s))) | Ok(Some(Data::ExpandSz(s))) => s,
        Ok(Some(other)) => other.audit_text(),
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

/// The name of a built-in group in the language of this Windows, such as `Administratoren` on a
/// German Windows.
pub(crate) fn builtin_group(
    kind: windows_sys::Win32::Security::WELL_KNOWN_SID_TYPE,
) -> Option<String> {
    use windows_sys::Win32::Security::{CreateWellKnownSid, SECURITY_MAX_SID_SIZE};
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
    // SAFETY: the identifier was written above and lives during the call.
    unsafe { ffi::account_of_sid(sid.as_mut_ptr().cast()) }.map(|(name, _)| name)
}

/// The local groups an account belongs to directly, in lower case; none when they cannot be read.
pub(crate) fn groups_of(user: &str) -> Vec<String> {
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        LOCALGROUP_USERS_INFO_0, MAX_PREFERRED_LENGTH, NERR_Success, NetUserGetLocalGroups,
    };
    let name = wide(user);
    let mut buffer = NetBuffer::default();
    let (mut read, mut total) = (0u32, 0u32);
    // SAFETY: the name is null-terminated; the call allocates the buffer and writes its counts.
    let status = unsafe {
        NetUserGetLocalGroups(
            std::ptr::null(),
            name.as_ptr(),
            0,
            0,
            buffer.out(),
            MAX_PREFERRED_LENGTH,
            &mut read,
            &mut total,
        )
    };
    let mut groups = Vec::new();
    if status == NERR_Success && !buffer.get().is_null() {
        // SAFETY: on success the buffer holds `read` entries of level 0.
        let entries = unsafe {
            std::slice::from_raw_parts(
                buffer.get().cast::<LOCALGROUP_USERS_INFO_0>(),
                read as usize,
            )
        };
        for entry in entries {
            // SAFETY: each name is a null-terminated string inside the buffer.
            groups.push(unsafe { wide_text(entry.lgrui0_name) }.to_lowercase());
        }
    }
    groups
}

/// The enabled local accounts but the built-in ones, with their group, the way the audit script
/// reads them: the relative ids 500, 501, 503 and 504 are Administrator, Guest, DefaultAccount
/// and WDAGUtilityAccount. A password is never read.
fn local_accounts() -> Result<Vec<ReadAccount>, String> {
    use windows_sys::Win32::NetworkManagement::NetManagement::UF_ACCOUNTDISABLE;
    use windows_sys::Win32::Security::{WinBuiltinAdministratorsSid, WinBuiltinUsersSid};
    let admins = builtin_group(WinBuiltinAdministratorsSid)
        .ok_or("the name of the group Administrators could not be read")?
        .to_lowercase();
    let users = builtin_group(WinBuiltinUsersSid)
        .ok_or("the name of the group Users could not be read")?
        .to_lowercase();
    let mut accounts = Vec::new();
    each_user(|entry| {
        if entry.usri20_flags & UF_ACCOUNTDISABLE != 0
            || matches!(entry.usri20_user_id, 500 | 501 | 503 | 504)
        {
            return;
        }
        // SAFETY: the texts are null-terminated strings inside the buffer of the call.
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
    })
    .map_err(|status| {
        format!(
            "the local accounts were not read ({})",
            ffi::net_error(status)
        )
    })?;
    Ok(accounts)
}

impl Machine for NativeMachine {
    fn value(&self, path: &str, name: &str) -> Result<Option<Data>, String> {
        let Some(key) = open(path)? else {
            return Ok(None);
        };
        let name = if name == "(Default)" { "" } else { name };
        match key.get_raw_value(name) {
            Ok(v) => Ok(Some(Data::from_raw(v.vtype as u32, &v.bytes))),
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

    fn values(&self, path: &str) -> Result<Option<Vec<(String, Data)>>, String> {
        let Some(key) = open(path)? else {
            return Ok(None);
        };
        key.enum_values()
            .map(|v| {
                v.map(|(name, value)| (name, Data::from_raw(value.vtype as u32, &value.bytes)))
            })
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
            Ok(Some(Data::MultiSz(tags))) => tags
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
                            Data::Dword(order) => (i64::from(order), name),
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
        assert!(matches!(build, Ok(Some(Data::Sz(_)))), "{build:?}");
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
