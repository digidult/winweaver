//! The registry as the registry tab, the bookmarks and the MCP server see it: keys with their
//! values in the data types of regedit, paths in the form of regedit (`HKEY_LOCAL_MACHINE\Software`),
//! the rules of a catalog that write a key, and a registry in memory for the tests. The Windows API is
//! in winweaver-win.
//!
//! The folder: `data` (the data of a value in the types of regedit), `search` (Find of regedit)
//! and `memory` (the registry in memory).

use serde::{Deserialize, Serialize};

use crate::catalog::{Action, Catalog, Rule};

mod data;
mod memory;
mod search;
#[cfg(test)]
mod tests;

pub use data::Data;
pub use memory::{MEMORY_RIGHTS, MemoryRegistry};
pub use search::{Found, Hit, Query, search};

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

/// The types a new value can take, as regedit offers them.
pub const NEW_TYPES: [&str; 6] = [
    "REG_SZ",
    "REG_BINARY",
    "REG_DWORD",
    "REG_QWORD",
    "REG_MULTI_SZ",
    "REG_EXPAND_SZ",
];

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
    /// The owner and the access list of a key in SDDL (`security::Rights::parse` reads it).
    fn security(&self, path: &str) -> Result<String, String> {
        Err(format!("{path}: the rights of keys are not read here"))
    }
    /// Writes the access list of a key from SDDL (`security::Rights::dacl`); `protected` stops the
    /// inheritance of the key above.
    fn set_security(&self, path: &str, _dacl: &str, _protected: bool) -> Result<(), String> {
        Err(format!("{path}: the rights of keys are not changed here"))
    }
    /// The name of an account by its security identifier or an alias of SDDL, as this Windows
    /// calls it.
    fn account_name(&self, _sid: &str) -> Option<String> {
        None
    }
    /// The security identifier of an account by its name.
    fn account_sid(&self, _name: &str) -> Option<String> {
        None
    }
}
