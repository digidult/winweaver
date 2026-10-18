//! The registry of this PC for the registry tab and the MCP server, with winreg, in the paths of
//! regedit (`winweaver_core::registry`). A change the process has no right to is an error that
//! says so; the rights of keys are not changed here.

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
    let full = registry::normalize(path)
        .ok_or_else(|| format!("not a path of the registry: {path}"))?;
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
}
