//! The registry as the registry tab, the bookmarks and the MCP server see it: keys with their
//! values in the data types of regedit, paths in the form of regedit (`HKEY_LOCAL_MACHINE\Software`),
//! the rules of a catalog that write a key, and a registry in memory for the tests. The Windows API is
//! in winweaver-win.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::catalog::{Action, Catalog, Rule};

/// The roots of the registry: the name regedit shows and its short form.
pub const HIVES: [(&str, &str); 5] = [
    ("HKEY_CLASSES_ROOT", "HKCR"),
    ("HKEY_CURRENT_USER", "HKCU"),
    ("HKEY_LOCAL_MACHINE", "HKLM"),
    ("HKEY_USERS", "HKU"),
    ("HKEY_CURRENT_CONFIG", "HKCC"),
];

/// The longest path, value name and value the program reads or writes.
pub const PATH_MAX: usize = 1024;
pub const NAME_MAX: usize = 16383;
pub const DATA_MAX: usize = 1024 * 1024;

/// The data of a value in the types of regedit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Data {
    #[serde(rename = "REG_SZ")]
    Sz(String),
    #[serde(rename = "REG_EXPAND_SZ")]
    ExpandSz(String),
    #[serde(rename = "REG_MULTI_SZ")]
    MultiSz(Vec<String>),
    #[serde(rename = "REG_DWORD")]
    Dword(u32),
    #[serde(rename = "REG_QWORD")]
    Qword(u64),
    #[serde(rename = "REG_BINARY")]
    Binary(Vec<u8>),
    #[serde(rename = "REG_NONE")]
    None(Vec<u8>),
    /// Another type (REG_LINK, REG_RESOURCE_LIST and the like): its number and bytes, shown, not
    /// edited.
    #[serde(rename = "OTHER")]
    Other { kind: u32, bytes: Vec<u8> },
}

/// The types a new value can take, as regedit offers them.
pub const NEW_TYPES: [&str; 6] = [
    "REG_SZ",
    "REG_BINARY",
    "REG_DWORD",
    "REG_QWORD",
    "REG_MULTI_SZ",
    "REG_EXPAND_SZ",
];

fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let text = String::from_utf16_lossy(&units);
    text.trim_end_matches('\0').to_owned()
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

impl Data {
    /// The name of the type, as regedit shows it.
    pub fn type_name(&self) -> String {
        match self {
            Data::Sz(_) => "REG_SZ".to_owned(),
            Data::ExpandSz(_) => "REG_EXPAND_SZ".to_owned(),
            Data::MultiSz(_) => "REG_MULTI_SZ".to_owned(),
            Data::Dword(_) => "REG_DWORD".to_owned(),
            Data::Qword(_) => "REG_QWORD".to_owned(),
            Data::Binary(_) => "REG_BINARY".to_owned(),
            Data::None(_) => "REG_NONE".to_owned(),
            Data::Other { kind, .. } => match kind {
                6 => "REG_LINK".to_owned(),
                8 => "REG_RESOURCE_LIST".to_owned(),
                9 => "REG_FULL_RESOURCE_DESCRIPTOR".to_owned(),
                10 => "REG_RESOURCE_REQUIREMENTS_LIST".to_owned(),
                n => format!("0x{n:x}"),
            },
        }
    }

    /// Whether the editor of the tab changes it: every type but the rare ones.
    pub fn editable(&self) -> bool {
        !matches!(self, Data::Other { .. })
    }

    /// The data in one line, as regedit lists it.
    pub fn display(&self) -> String {
        match self {
            Data::Sz(s) | Data::ExpandSz(s) => s.clone(),
            Data::MultiSz(items) => items.join(" "),
            Data::Dword(n) => format!("0x{n:08x} ({n})"),
            Data::Qword(n) => format!("0x{n:016x} ({n})"),
            Data::Binary(b) | Data::None(b) | Data::Other { bytes: b, .. } => hex(b),
        }
    }

    /// The text the editor starts from: lines for a multi-string, the number for a dword or
    /// qword, hexadecimal pairs for bytes.
    pub fn edit_text(&self) -> String {
        match self {
            Data::Sz(s) | Data::ExpandSz(s) => s.clone(),
            Data::MultiSz(items) => items.join("\n"),
            Data::Dword(n) => n.to_string(),
            Data::Qword(n) => n.to_string(),
            Data::Binary(b) | Data::None(b) | Data::Other { bytes: b, .. } => hex(b),
        }
    }

    /// The data of a type from the text of the editor: a dword or qword in decimal or with `0x` in
    /// hexadecimal, bytes as hexadecimal pairs (spaces and commas ignored), a multi-string as lines.
    pub fn parse(type_name: &str, text: &str) -> Result<Self, String> {
        let number = |text: &str, max: u64| -> Result<u64, String> {
            let t = text.trim();
            let n = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                Some(h) => u64::from_str_radix(h, 16),
                None => t.parse::<u64>(),
            }
            .map_err(|_| format!("not a number: {t}"))?;
            if n > max {
                return Err(format!("{t} is larger than {max}"));
            }
            Ok(n)
        };
        let data = match type_name {
            "REG_SZ" => Data::Sz(text.to_owned()),
            "REG_EXPAND_SZ" => Data::ExpandSz(text.to_owned()),
            "REG_MULTI_SZ" => Data::MultiSz(
                text.lines()
                    .map(str::to_owned)
                    .filter(|l| !l.is_empty())
                    .collect(),
            ),
            "REG_DWORD" => Data::Dword(number(text, u64::from(u32::MAX))? as u32),
            "REG_QWORD" => Data::Qword(number(text, u64::MAX)?),
            "REG_BINARY" => Data::Binary(unhex(text)?),
            "REG_NONE" => Data::None(unhex(text)?),
            other => return Err(format!("the type {other} is not edited here")),
        };
        if data.to_raw().1.len() > DATA_MAX {
            return Err(format!("larger than {} KB", DATA_MAX / 1024));
        }
        Ok(data)
    }

    /// The data of a type number and its bytes as the registry gives them.
    pub fn from_raw(kind: u32, bytes: &[u8]) -> Self {
        match kind {
            0 => Data::None(bytes.to_vec()),
            1 => Data::Sz(utf16_text(bytes)),
            2 => Data::ExpandSz(utf16_text(bytes)),
            3 => Data::Binary(bytes.to_vec()),
            4 if bytes.len() == 4 => {
                Data::Dword(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            }
            7 => Data::MultiSz(
                utf16_text(bytes)
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            11 if bytes.len() == 8 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(bytes);
                Data::Qword(u64::from_le_bytes(b))
            }
            other => Data::Other {
                kind: other,
                bytes: bytes.to_vec(),
            },
        }
    }

    /// The type number and the bytes to write.
    pub fn to_raw(&self) -> (u32, Vec<u8>) {
        match self {
            Data::None(b) => (0, b.clone()),
            Data::Sz(s) => (1, utf16_bytes(s)),
            Data::ExpandSz(s) => (2, utf16_bytes(s)),
            Data::Binary(b) => (3, b.clone()),
            Data::Dword(n) => (4, n.to_le_bytes().to_vec()),
            Data::MultiSz(items) => {
                let mut bytes: Vec<u8> = Vec::new();
                for item in items {
                    bytes.extend(item.encode_utf16().flat_map(u16::to_le_bytes));
                    bytes.extend([0, 0]);
                }
                bytes.extend([0, 0]);
                (7, bytes)
            }
            Data::Qword(n) => (11, n.to_le_bytes().to_vec()),
            Data::Other { kind, bytes } => (*kind, bytes.clone()),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn unhex(text: &str) -> Result<Vec<u8>, String> {
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err("an odd count of hexadecimal digits".to_owned());
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&digits[i..i + 2], 16)
                .map_err(|_| format!("not hexadecimal: {}", &digits[i..i + 2]))
        })
        .collect()
}

/// A value of a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    /// The name; empty for the default value of the key.
    pub name: String,
    pub data: Data,
}

/// A key: its path, the names of its subkeys and its values, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub path: String,
    pub subkeys: Vec<String>,
    pub values: Vec<Value>,
}

/// A path of the registry in the form of regedit (`HKEY_LOCAL_MACHINE\Software\X`) from the forms
/// people and the catalog write: `HKLM:\...`, `HKLM\...`, `HKEY_LOCAL_MACHINE\...`, `Computer\...`,
/// with slashes or backslashes. The default user profile of the catalog (`DU:`) is shown as the
/// current user, as the audit reads it. `None` for another root or a path too long or with a
/// control character.
pub fn normalize(path: &str) -> Option<String> {
    if path.len() > PATH_MAX || path.chars().any(char::is_control) {
        return None;
    }
    let path = path.trim().replace('/', "\\");
    let path = path.strip_prefix("Computer\\").unwrap_or(&path);
    let (root, rest) = path.split_once('\\').unwrap_or((path, ""));
    let root = root.trim_end_matches(':').to_ascii_uppercase();
    let hive = if root == "DU" {
        "HKEY_CURRENT_USER"
    } else {
        HIVES
            .iter()
            .find(|(long, short)| root == *long || root == *short)
            .map(|(long, _)| *long)?
    };
    let rest: Vec<&str> = rest.split('\\').filter(|p| !p.is_empty()).collect();
    if rest.iter().any(|p| *p == "." || *p == "..") {
        return None;
    }
    Some(if rest.is_empty() {
        hive.to_owned()
    } else {
        format!("{hive}\\{}", rest.join("\\"))
    })
}

/// The key above a path, `None` for a hive.
pub fn parent(path: &str) -> Option<&str> {
    path.rsplit_once('\\').map(|(p, _)| p)
}

/// The rules of a catalog with an action on a key (or a value of it when `value` is given): the
/// catalogs a registry bookmark leads to.
pub fn writers<'a>(catalog: &'a Catalog, key: &str, value: Option<&str>) -> Vec<&'a Rule> {
    let Some(key) = normalize(key) else {
        return Vec::new();
    };
    catalog
        .rules()
        .iter()
        .filter(|r| {
            r.actions.iter().any(|a| {
                let (path, name) = match a {
                    Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. } => {
                        (path, Some(name.as_str()))
                    }
                    Action::RegList { path, .. } => (path, None),
                    _ => return false,
                };
                normalize(path).is_some_and(|p| p.eq_ignore_ascii_case(&key))
                    && value.is_none_or(|v| name.is_none_or(|n| n.eq_ignore_ascii_case(v)))
            })
        })
        .collect()
}

/// Reading and changing the registry: the native one of Windows, or one in memory.
pub trait Registry: Send + Sync {
    fn read(&self, path: &str) -> Result<Key, String>;
    fn set(&self, path: &str, name: &str, data: &Data) -> Result<(), String>;
    fn delete_value(&self, path: &str, name: &str) -> Result<(), String>;
    fn create_key(&self, path: &str) -> Result<(), String>;
    /// Deletes a key with everything under it.
    fn delete_key(&self, path: &str) -> Result<(), String>;
}

/// A registry in memory, for the tests and where there is no Windows.
#[derive(Default)]
pub struct MemoryRegistry {
    /// Keys by their path in lower case: the path as written and the values.
    keys: Mutex<BTreeMap<String, (String, Vec<Value>)>>,
}

impl MemoryRegistry {
    fn keys(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, (String, Vec<Value>)>> {
        self.keys.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn checked(path: &str) -> Result<String, String> {
        normalize(path).ok_or_else(|| format!("not a path of the registry: {path}"))
    }
}

impl Registry for MemoryRegistry {
    fn read(&self, path: &str) -> Result<Key, String> {
        let path = Self::checked(path)?;
        let keys = self.keys();
        let lower = path.to_lowercase();
        let hive = !path.contains('\\');
        let Some((written, values)) = keys
            .get(&lower)
            .cloned()
            .or_else(|| hive.then(|| (path.clone(), Vec::new())))
        else {
            return Err(format!("{path}: the key does not exist"));
        };
        let prefix = format!("{lower}\\");
        let mut subkeys: Vec<String> = keys
            .iter()
            .filter_map(|(k, (w, _))| {
                k.strip_prefix(&prefix)
                    .filter(|rest| !rest.contains('\\'))
                    .map(|_| w.rsplit('\\').next().unwrap_or(w).to_owned())
            })
            .collect();
        subkeys.sort_by_key(|s| s.to_lowercase());
        let mut values = values;
        values.sort_by_key(|v| v.name.to_lowercase());
        Ok(Key {
            path: written,
            subkeys,
            values,
        })
    }

    fn set(&self, path: &str, name: &str, data: &Data) -> Result<(), String> {
        let path = Self::checked(path)?;
        let mut keys = self.keys();
        let entry = keys
            .get_mut(&path.to_lowercase())
            .ok_or_else(|| format!("{path}: the key does not exist"))?;
        entry.1.retain(|v| !v.name.eq_ignore_ascii_case(name));
        entry.1.push(Value {
            name: name.to_owned(),
            data: data.clone(),
        });
        Ok(())
    }

    fn delete_value(&self, path: &str, name: &str) -> Result<(), String> {
        let path = Self::checked(path)?;
        let mut keys = self.keys();
        let entry = keys
            .get_mut(&path.to_lowercase())
            .ok_or_else(|| format!("{path}: the key does not exist"))?;
        entry.1.retain(|v| !v.name.eq_ignore_ascii_case(name));
        Ok(())
    }

    fn create_key(&self, path: &str) -> Result<(), String> {
        let path = Self::checked(path)?;
        let mut keys = self.keys();
        let mut cur = String::new();
        for part in path.split('\\') {
            if !cur.is_empty() {
                cur.push('\\');
            }
            cur.push_str(part);
            if cur.contains('\\') {
                keys.entry(cur.to_lowercase())
                    .or_insert_with(|| (cur.clone(), Vec::new()));
            }
        }
        Ok(())
    }

    fn delete_key(&self, path: &str) -> Result<(), String> {
        let path = Self::checked(path)?;
        if !path.contains('\\') {
            return Err("a hive is not deleted".to_owned());
        }
        let lower = path.to_lowercase();
        let prefix = format!("{lower}\\");
        self.keys()
            .retain(|k, _| k != &lower && !k.starts_with(&prefix));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_take_the_form_of_regedit() {
        let long = "HKEY_LOCAL_MACHINE\\Software\\Policies";
        for p in [
            "HKLM:\\Software\\Policies",
            "hklm\\Software\\Policies\\",
            "Computer\\HKEY_LOCAL_MACHINE\\Software\\Policies",
            "HKLM:/Software/Policies",
        ] {
            assert_eq!(normalize(p).as_deref(), Some(long), "{p}");
        }
        assert_eq!(
            normalize("DU:\\Software\\X").as_deref(),
            Some("HKEY_CURRENT_USER\\Software\\X")
        );
        assert_eq!(normalize("HKU").as_deref(), Some("HKEY_USERS"));
        assert_eq!(normalize("C:\\Windows"), None);
        assert_eq!(normalize("HKLM\\a\\..\\b"), None);
        assert_eq!(parent(long), Some("HKEY_LOCAL_MACHINE\\Software"));
    }

    #[test]
    fn every_type_goes_to_bytes_and_back_and_parses_from_its_editor() {
        for data in [
            Data::Sz("text \u{439}".to_owned()),
            Data::ExpandSz("%SystemRoot%\\x".to_owned()),
            Data::MultiSz(vec!["a".to_owned(), "b c".to_owned()]),
            Data::Dword(0xdead_beef),
            Data::Qword(u64::MAX),
            Data::Binary(vec![0, 1, 0xab]),
            Data::None(Vec::new()),
        ] {
            let (kind, bytes) = data.to_raw();
            assert_eq!(Data::from_raw(kind, &bytes), data);
            assert_eq!(
                Data::parse(&data.type_name(), &data.edit_text()).unwrap(),
                data
            );
        }
        assert_eq!(Data::parse("REG_DWORD", "0x10").unwrap(), Data::Dword(16));
        assert!(Data::parse("REG_DWORD", "4294967296").is_err());
        assert!(Data::parse("REG_BINARY", "abc").is_err());
        assert_eq!(Data::Dword(1).display(), "0x00000001 (1)");
        let link = Data::from_raw(6, &[1, 2]);
        assert_eq!(link.type_name(), "REG_LINK");
        assert!(!link.editable());
    }

    #[test]
    fn the_registry_in_memory_keeps_keys_and_values() {
        let reg = MemoryRegistry::default();
        reg.create_key("HKCU\\Software\\WinWeaver\\Test").unwrap();
        reg.set("HKCU\\Software\\WinWeaver\\Test", "N", &Data::Dword(1))
            .unwrap();
        let key = reg.read("HKEY_CURRENT_USER\\Software\\WinWeaver").unwrap();
        assert_eq!(key.subkeys, vec!["Test"]);
        let test = reg.read("hkcu\\software\\winweaver\\test").unwrap();
        assert_eq!(test.values[0].data, Data::Dword(1));
        reg.delete_value("HKCU\\Software\\WinWeaver\\Test", "n")
            .unwrap();
        assert!(
            reg.read("HKCU\\Software\\WinWeaver\\Test")
                .unwrap()
                .values
                .is_empty()
        );
        reg.delete_key("HKCU\\Software\\WinWeaver").unwrap();
        assert!(reg.read("HKCU\\Software\\WinWeaver\\Test").is_err());
        assert!(reg.read("HKCU").is_ok(), "a hive is always there");
    }

    #[test]
    fn the_rules_that_write_a_key_are_found() {
        let catalog = Catalog::embedded().unwrap();
        let (path, name) = catalog
            .rules()
            .iter()
            .find_map(|r| {
                r.actions.iter().find_map(|a| match a {
                    Action::Reg { path, name, .. } => Some((path.clone(), name.clone())),
                    _ => None,
                })
            })
            .unwrap();
        let found = writers(&catalog, &path, Some(&name));
        assert!(!found.is_empty(), "{path} {name}");
        assert!(writers(&catalog, "HKLM\\Software\\NobodyWritesThis", None).is_empty());
    }
}
