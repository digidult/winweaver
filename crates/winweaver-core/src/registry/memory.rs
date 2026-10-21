//! A registry in memory, for the tests and where there is no Windows.

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::{Data, Key, Registry, Value, normalize};

/// A registry in memory, for the tests and where there is no Windows.
#[derive(Default)]
pub struct MemoryRegistry {
    /// Keys by their path in lower case: the path as written and the values.
    keys: Mutex<BTreeMap<String, (String, Vec<Value>)>>,
    /// The rights of keys written by `set_security`, by their path in lower case.
    rights: Mutex<BTreeMap<String, String>>,
}

/// The rights of a key of the registry in memory that has none of its own: what it inherits.
pub const MEMORY_RIGHTS: &str = "O:BAG:SYD:AI(A;CIID;KA;;;BA)(A;CIID;KR;;;BU)";

impl MemoryRegistry {
    fn keys(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, (String, Vec<Value>)>> {
        self.keys.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn rights(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.rights.lock().unwrap_or_else(|e| e.into_inner())
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

    fn security(&self, path: &str) -> Result<String, String> {
        let path = Self::checked(path)?;
        self.read(&path)?;
        let own = self.rights().get(&path.to_lowercase()).cloned();
        Ok(own.unwrap_or_else(|| MEMORY_RIGHTS.to_owned()))
    }

    fn set_security(&self, path: &str, dacl: &str, protected: bool) -> Result<(), String> {
        let path = Self::checked(path)?;
        self.read(&path)?;
        crate::security::Rights::parse(dacl)?;
        // What the key inherits comes after its own entries unless it is protected.
        let inherited = if protected { "" } else { "(A;CIID;KA;;;BA)" };
        let text = format!("O:BA{dacl}{inherited}");
        self.rights().insert(path.to_lowercase(), text);
        Ok(())
    }

    fn account_name(&self, sid: &str) -> Option<String> {
        crate::security::alias_title(sid).map(str::to_owned)
    }

    fn account_sid(&self, name: &str) -> Option<String> {
        use crate::security::alias_title;
        let titled = |a: &&str| alias_title(a).is_some_and(|t| t.eq_ignore_ascii_case(name));
        ["BA", "BU", "SY", "WD", "AU"]
            .into_iter()
            .find(titled)
            .map(str::to_owned)
    }
}
