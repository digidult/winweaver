//! The input languages and keyboard layouts of this Windows (`resources::Locales`): every locale
//! Windows ships with its locale id, its name in the language of Windows, its parent and the
//! input profile it installs first, and every keyboard layout of the registry with its name.

use windows_sys::Win32::Globalization::{
    EnumSystemLocalesEx, GetLocaleInfoEx, LOCALE_ALLOW_NEUTRAL_NAMES, LOCALE_SKEYBOARDSTOINSTALL,
    LOCALE_SLOCALIZEDDISPLAYNAME, LOCALE_SPARENT, LOCALE_WINDOWS, LocaleNameToLCID,
};
use windows_sys::Win32::System::Registry::{HKEY, RegLoadMUIStringW};
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winweaver_core::resources::{InputLanguage, KeyboardLayout, Locales};

/// The keyboard layouts of Windows, one key a layout id.
const LAYOUTS: &str = r"SYSTEM\CurrentControlSet\Control\Keyboard Layouts";
/// The locale id of a locale without one of its own (`LOCALE_CUSTOM_UNSPECIFIED`).
const NO_ID: u32 = 0x1000;

/// The languages and layouts, each list by its names.
pub fn read() -> Locales {
    let mut languages: Vec<InputLanguage> = names().into_iter().map(language).collect();
    languages.sort_by_cached_key(|l| l.name.to_lowercase());
    let mut layouts = layouts();
    layouts.sort_by_cached_key(|l| l.name.to_lowercase());
    Locales { languages, layouts }
}

/// The names of the locales Windows ships.
fn names() -> Vec<String> {
    unsafe extern "system" fn each(name: *mut u16, _flags: u32, data: isize) -> i32 {
        // SAFETY: `data` is the address of the vector `names` passes, alive during the whole
        // enumeration, and `name` a null-terminated string Windows hands to the callback.
        unsafe {
            let names = &mut *(data as *mut Vec<String>);
            names.push(crate::native::wide_text(name));
        }
        1
    }
    let mut names: Vec<String> = Vec::new();
    let data = &mut names as *mut Vec<String> as isize;
    // SAFETY: the callback only pushes into `names`, which outlives the call.
    unsafe {
        EnumSystemLocalesEx(Some(each), LOCALE_WINDOWS, data, std::ptr::null());
    }
    names.retain(|n| !n.is_empty());
    names
}

/// A text of a locale, empty when Windows has none.
fn info(name: &[u16], kind: u32) -> String {
    let mut buffer = [0u16; 512];
    let (out, len) = (buffer.as_mut_ptr(), buffer.len() as i32);
    // SAFETY: the name is null-terminated and the buffer valid for the length passed.
    let written = unsafe { GetLocaleInfoEx(name.as_ptr(), kind, out, len) };
    if written <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..written as usize - 1])
}

/// A locale with its id, its names and the first input profile it installs.
fn language(name: String) -> InputLanguage {
    let wide = wide(&name);
    // SAFETY: the name is null-terminated.
    let id = unsafe { LocaleNameToLCID(wide.as_ptr(), LOCALE_ALLOW_NEUTRAL_NAMES) };
    let lcid = if id == 0 || id == NO_ID {
        String::new()
    } else {
        format!("{id:04X}")
    };
    let profiles = info(&wide, LOCALE_SKEYBOARDSTOINSTALL);
    InputLanguage {
        tag: name,
        lcid,
        name: info(&wide, LOCALE_SLOCALIZEDDISPLAYNAME),
        parent: info(&wide, LOCALE_SPARENT),
        default: profiles.split(';').next().unwrap_or("").trim().to_owned(),
    }
}

/// The keyboard layouts of the registry with their names; the ids that stand for an input
/// method (`E0010411`) are left out.
fn layouts() -> Vec<KeyboardLayout> {
    let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(LAYOUTS) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for klid in root.enum_keys().filter_map(Result::ok) {
        let hex = klid.len() == 8 && klid.chars().all(|c| c.is_ascii_hexdigit());
        if !hex || klid.starts_with(['e', 'E']) {
            continue;
        }
        let Ok(key) = root.open_subkey(&klid) else {
            continue;
        };
        let name = display_name(&key)
            .or_else(|| key.get_value::<String, _>("Layout Text").ok())
            .unwrap_or_default();
        if !name.is_empty() {
            found.push(KeyboardLayout {
                klid: klid.to_uppercase(),
                name,
            });
        }
    }
    found
}

/// The name of a layout in the language of Windows, from the resource its `Layout Display Name`
/// points at.
fn display_name(key: &RegKey) -> Option<String> {
    let value = wide("Layout Display Name");
    let mut buffer = [0u16; 256];
    let mut size = 0u32;
    let bytes = (buffer.len() * 2) as u32;
    // SAFETY: the key is open while `key` lives, the name of the value is null-terminated and
    // the buffer is valid for the size in bytes passed.
    let status = unsafe {
        RegLoadMUIStringW(
            key.raw_handle() as HKEY,
            value.as_ptr(),
            buffer.as_mut_ptr(),
            bytes,
            &mut size,
            0,
            std::ptr::null(),
        )
    };
    if status != 0 {
        return None;
    }
    let len = buffer.iter().take_while(|&&c| c != 0).count();
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..len]))
}

/// A text as a null-terminated wide string for the Windows API.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_languages_and_layouts_of_this_windows_are_read() {
        let l = super::read();
        assert!(l.languages.len() > 100, "{} languages", l.languages.len());
        assert!(l.layouts.len() > 50, "{} layouts", l.layouts.len());
        let us = l.language("en-US").unwrap();
        assert_eq!(us.lcid, "0409");
        let default = &us.default;
        assert!(default.eq_ignore_ascii_case("0409:00000409"), "{default}");
        assert!(!us.name.is_empty());
        assert!(l.layout("00000409").is_some());
        assert!(l.layout("00020422").is_some());
        let (uk, profile) = l.input("uk-UA").unwrap();
        assert_eq!(uk.lcid, "0422");
        assert_eq!(profile, "0422:00020422");
        if let Some(ru) = l.language("ru-UA") {
            assert!(ru.lcid.is_empty(), "{}", ru.lcid);
            assert_eq!(ru.parent, "ru");
        }
    }
}
