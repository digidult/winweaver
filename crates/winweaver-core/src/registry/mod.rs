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

/// A registry value a rule writes or removes: its key in the form of regedit and its name, `None`
/// for a list, which owns every value of its key.
struct Written {
    key: String,
    name: Option<String>,
}

impl Written {
    fn of(rule: &Rule) -> Vec<Self> {
        rule.actions
            .iter()
            .filter_map(|a| {
                let (path, name) = match a {
                    Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. } => {
                        (path, Some(name.clone()))
                    }
                    Action::RegList { path, .. } => (path, None),
                    _ => return None,
                };
                normalize(path).map(|key| Self { key, name })
            })
            .collect()
    }

    fn meets(&self, other: &Self) -> bool {
        self.key.eq_ignore_ascii_case(&other.key)
            && match (&self.name, &other.name) {
                (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                _ => true,
            }
    }

    fn text(&self) -> String {
        match &self.name {
            Some(name) if name.is_empty() => format!("{}\\(Default)", self.key),
            Some(name) => format!("{}\\{name}", self.key),
            None => self.key.clone(),
        }
    }
}

/// The other rules of the catalog (built in or imported) that write a registry value this rule
/// writes, each with the values they share, in the order of the catalog. It works both ways: a
/// rule is in the list of every rule in its own list. The two rules of one policy (`<id>` and
/// `<id>.off`) are one, and so are the copies of a policy in several imports.
pub fn same_values<'a>(catalog: &'a Catalog, rule: &Rule) -> Vec<(&'a Rule, Vec<String>)> {
    let mine = Written::of(rule);
    if mine.is_empty() {
        return Vec::new();
    }
    let base = rule.id.trim_end_matches(".off");
    catalog
        .rules()
        .iter()
        .filter(|r| r.id.trim_end_matches(".off") != base)
        .filter_map(|r| {
            let mut shared: Vec<String> = Vec::new();
            for w in Written::of(r) {
                if mine.iter().any(|m| m.meets(&w)) {
                    let text = w.text();
                    if !shared.contains(&text) {
                        shared.push(text);
                    }
                }
            }
            (!shared.is_empty()).then_some((r, shared))
        })
        .collect()
}

/// The path of a key renamed: its parent and the new name, which has no backslash.
pub fn renamed_path(path: &str, new_name: &str) -> Result<String, String> {
    let full = normalize(path).ok_or_else(|| format!("not a path of the registry: {path}"))?;
    let new_name = new_name.trim();
    if new_name.is_empty() || new_name.contains('\\') {
        return Err("a name without backslashes is needed".to_owned());
    }
    match full.rsplit_once('\\') {
        Some((parent, _)) => Ok(format!("{parent}\\{new_name}")),
        None => Err(format!("{full}: a hive is not renamed")),
    }
}

/// Copies a key with its values and every key under it, at most 64 levels deep.
fn copy_key<R: Registry + ?Sized>(
    reg: &R,
    from: &str,
    to: &str,
    depth: usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err(format!("{from}: the keys go deeper than 64 levels"));
    }
    let key = reg.read(from)?;
    reg.create_key(to)?;
    for value in &key.values {
        reg.set(to, &value.name, &value.data)?;
    }
    for sub in &key.subkeys {
        copy_key(
            reg,
            &format!("{from}\\{sub}"),
            &format!("{to}\\{sub}"),
            depth + 1,
        )?;
    }
    Ok(())
}

/// Renames a value of a key: its data under the new name, the old name deleted. A change of the
/// case alone deletes first, since the names of values ignore case.
pub fn rename_value(reg: &dyn Registry, path: &str, old: &str, new: &str) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    let key = reg.read(path)?;
    let value = key
        .values
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(old))
        .ok_or_else(|| format!("{path}: no value {old}"))?;
    if old.eq_ignore_ascii_case(new) {
        reg.delete_value(path, old)?;
        return reg.set(path, new, &value.data);
    }
    if key.values.iter().any(|v| v.name.eq_ignore_ascii_case(new)) {
        return Err(format!("{path}: a value {new} is there already"));
    }
    reg.set(path, new, &value.data)?;
    reg.delete_value(path, old)
}

/// Reading and changing the registry: the native one of Windows, or one in memory.
pub trait Registry: Send + Sync {
    fn read(&self, path: &str) -> Result<Key, String>;
    fn set(&self, path: &str, name: &str, data: &Data) -> Result<(), String>;
    fn delete_value(&self, path: &str, name: &str) -> Result<(), String>;
    fn create_key(&self, path: &str) -> Result<(), String>;
    /// Deletes a key with everything under it.
    fn delete_key(&self, path: &str) -> Result<(), String>;
    /// Renames a key, `new_name` the new last part of its path, and gives its new path. This one
    /// copies the key with everything under it and deletes the old one; Windows renames it in
    /// place.
    fn rename_key(&self, path: &str, new_name: &str) -> Result<String, String> {
        let target = renamed_path(path, new_name)?;
        if self.read(&target).is_ok() {
            return Err(format!("{target}: a key of this name is there already"));
        }
        copy_key(self, path, &target, 0)?;
        self.delete_key(path)?;
        Ok(target)
    }
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
