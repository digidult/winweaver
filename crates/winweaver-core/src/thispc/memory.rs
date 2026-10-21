//! A machine kept in memory, for the tests and for a look at a report without a PC.

use super::report::SystemInfo;
use super::{Components, Machine, RegData};

/// A machine kept in memory, for the tests and for a look at a report without a PC.
#[derive(Debug, Clone, Default)]
pub struct MemoryMachine {
    /// Values by `(path, name)`, both in lower case.
    pub values: std::collections::HashMap<(String, String), RegData>,
    /// Keys that exist, in lower case, with their subkey names.
    pub keys: std::collections::HashMap<String, Vec<String>>,
    /// Keys that cannot be read, in lower case.
    pub denied: Vec<String>,
    pub admin: bool,
    pub system: SystemInfo,
    /// The optional features with their states.
    pub features: Vec<(String, String)>,
    /// The capabilities with their states.
    pub capabilities: Vec<(String, String)>,
    /// When not empty, the features and capabilities cannot be read and this is the error.
    pub components_error: String,
}

impl MemoryMachine {
    pub fn set(&mut self, path: &str, name: &str, data: RegData) {
        self.values
            .insert((path.to_lowercase(), name.to_lowercase()), data);
    }
}

impl Machine for MemoryMachine {
    fn value(&self, path: &str, name: &str) -> Result<Option<RegData>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        Ok(self.values.get(&(p, name.to_lowercase())).cloned())
    }

    fn subkeys(&self, path: &str) -> Result<Option<Vec<String>>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        Ok(self.keys.get(&p).cloned())
    }

    fn values(&self, path: &str) -> Result<Option<Vec<(String, RegData)>>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        let mut found: Vec<(String, RegData)> = self
            .values
            .iter()
            .filter(|((key, _), _)| *key == p)
            .map(|((_, name), data)| (name.clone(), data.clone()))
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        let exists = !found.is_empty() || self.keys.contains_key(&p);
        Ok(exists.then_some(found))
    }

    fn is_admin(&self) -> bool {
        self.admin
    }

    fn computer_name(&self) -> String {
        "MEMORY".to_owned()
    }

    fn system_info(&self) -> SystemInfo {
        self.system.clone()
    }

    fn components(&self) -> Components {
        let list = |items: &[(String, String)]| {
            if self.components_error.is_empty() {
                Ok(items.to_vec())
            } else {
                Err(self.components_error.clone())
            }
        };
        Components {
            features: list(&self.features),
            capabilities: list(&self.capabilities),
        }
    }
}
