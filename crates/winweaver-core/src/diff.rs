//! Diff files (`.wwdiff`): the differences between two profiles, applied to a profile as a
//! merge (`docs/spec/02-documents.md` section 4). A diff holds only what differs.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::jsonfile::{self, JsonError};
use crate::profile::{Choice, Profile};

pub const FORMAT: &str = "winweaver-diff";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "wwdiff";

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error("{name}: {source}")]
    Io {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{name}: not a WinWeaver diff (format {found})")]
    Format { name: String, found: String },
    #[error("{name}: diff version {found} is newer than this program knows ({VERSION})")]
    Version { name: String, found: u32 },
}

/// One of the two profiles of a diff, as it was when the diff was made.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Side {
    pub name: String,
    pub path: String,
    pub modified: String,
}

impl Side {
    pub fn of(profile: &Profile, path: Option<&Path>) -> Self {
        Self {
            name: profile.name.clone(),
            path: path.map(|p| p.display().to_string()).unwrap_or_default(),
            modified: profile.modified.clone(),
        }
    }
}

/// The two states of a rule; `None` is "off with the defaults" (not listed in the profile).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleDiff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<Choice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<Choice>,
}

/// The two values of a field of a form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDiff {
    pub base: Value,
    pub other: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    pub format: String,
    pub version: u32,
    pub catalog: String,
    pub base: Side,
    pub other: Side,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub rules: IndexMap<String, RuleDiff>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub install: IndexMap<String, FieldDiff>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub languages: IndexMap<String, FieldDiff>,
    /// The whole account list of both sides when it differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<FieldDiff>,
    /// Comments on the differences, by the key of an item ([`Diff::keys`]).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub comments: IndexMap<String, String>,
}

/// A choice that is worth listing: on, or with parameters, or with a source.
fn listed(choice: Option<&Choice>) -> Option<Choice> {
    choice
        .filter(|c| c.on || !c.params.is_empty() || c.source.is_some())
        .cloned()
}

/// The fields of a form as JSON, in the order of the struct.
fn fields(value: &impl Serialize) -> IndexMap<String, Value> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map.into_iter().collect(),
        _ => IndexMap::new(),
    }
}

fn field_diffs(
    base: &IndexMap<String, Value>,
    other: &IndexMap<String, Value>,
) -> IndexMap<String, FieldDiff> {
    base.iter()
        .filter_map(|(k, b)| {
            let o = other.get(k).cloned().unwrap_or(Value::Null);
            (*b != o).then(|| {
                (
                    k.clone(),
                    FieldDiff {
                        base: b.clone(),
                        other: o,
                    },
                )
            })
        })
        .collect()
}

fn merged<T: Serialize + serde::de::DeserializeOwned>(
    current: &T,
    changes: &IndexMap<String, FieldDiff>,
) -> Option<T> {
    let mut map = fields(current);
    for (k, d) in changes {
        map.insert(k.clone(), d.other.clone());
    }
    serde_json::from_value(Value::Object(map.into_iter().collect())).ok()
}

impl Diff {
    /// The differences from `base` to `other`.
    pub fn between(base: &Profile, other: &Profile, base_side: Side, other_side: Side) -> Self {
        let mut rules = IndexMap::new();
        let ids = base
            .rules
            .keys()
            .chain(other.rules.keys().filter(|k| !base.rules.contains_key(*k)));
        for id in ids {
            let b = listed(base.rules.get(id));
            let o = listed(other.rules.get(id));
            if b != o {
                rules.insert(id.clone(), RuleDiff { base: b, other: o });
            }
        }
        let accounts = (base.accounts != other.accounts).then(|| FieldDiff {
            base: serde_json::to_value(&base.accounts).unwrap_or(Value::Null),
            other: serde_json::to_value(&other.accounts).unwrap_or(Value::Null),
        });
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            catalog: other.catalog.clone(),
            base: base_side,
            other: other_side,
            rules,
            install: field_diffs(&fields(&base.install), &fields(&other.install)),
            languages: field_diffs(&fields(&base.languages), &fields(&other.languages)),
            accounts,
            comments: IndexMap::new(),
        }
    }

    /// The keys of the items in their order: `install:<field>`, `languages:<field>`, `accounts`
    /// and `rule:<id>`.
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .install
            .keys()
            .map(|k| format!("install:{k}"))
            .collect();
        keys.extend(self.languages.keys().map(|k| format!("languages:{k}")));
        if self.accounts.is_some() {
            keys.push("accounts".to_owned());
        }
        keys.extend(self.rules.keys().map(|k| format!("rule:{k}")));
        keys
    }

    /// Sets one item of a profile to the value of the other side.
    pub fn apply_item(&self, key: &str, profile: &mut Profile) {
        if let Some(id) = key.strip_prefix("rule:") {
            if let Some(d) = self.rules.get(id) {
                match &d.other {
                    Some(c) => {
                        profile.rules.insert(id.to_owned(), c.clone());
                    }
                    None => {
                        profile.rules.shift_remove(id);
                    }
                }
            }
        } else if let Some(field) = key.strip_prefix("install:") {
            if let Some(f) = self.install.get(field) {
                let one: IndexMap<String, FieldDiff> = [(field.to_owned(), f.clone())].into();
                if let Some(install) = merged(&profile.install, &one) {
                    profile.install = install;
                }
            }
        } else if let Some(field) = key.strip_prefix("languages:") {
            if let Some(f) = self.languages.get(field) {
                let one: IndexMap<String, FieldDiff> = [(field.to_owned(), f.clone())].into();
                if let Some(languages) = merged(&profile.languages, &one) {
                    profile.languages = languages;
                }
            }
        } else if key == "accounts"
            && let Some(d) = &self.accounts
            && let Ok(accounts) = serde_json::from_value(d.other.clone())
        {
            profile.accounts = accounts;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// The number of differences: rules, fields and the account list.
    pub fn count(&self) -> usize {
        self.rules.len()
            + self.install.len()
            + self.languages.len()
            + usize::from(self.accounts.is_some())
    }

    /// Sets the `other` side of every difference in the profile (the merge).
    pub fn apply_to(&self, profile: &mut Profile) {
        for (id, d) in &self.rules {
            match &d.other {
                Some(c) => {
                    profile.rules.insert(id.clone(), c.clone());
                }
                None => {
                    profile.rules.shift_remove(id);
                }
            }
        }
        if let Some(install) = merged(&profile.install, &self.install) {
            profile.install = install;
        }
        if let Some(languages) = merged(&profile.languages, &self.languages) {
            profile.languages = languages;
        }
        if let Some(d) = &self.accounts
            && let Ok(accounts) = serde_json::from_value(d.other.clone())
        {
            profile.accounts = accounts;
        }
    }

    pub fn load(path: &Path) -> Result<Self, DiffError> {
        let name = jsonfile::display_name(path);
        let text = jsonfile::read_text(path, jsonfile::MAX_BYTES)?;
        Self::parse(&text, &name)
    }

    pub fn parse(text: &str, name: &str) -> Result<Self, DiffError> {
        let diff: Self = jsonfile::parse(text, name)?;
        if diff.format != FORMAT {
            return Err(DiffError::Format {
                name: name.to_owned(),
                found: diff.format,
            });
        }
        if diff.version > VERSION {
            return Err(DiffError::Version {
                name: name.to_owned(),
                found: diff.version,
            });
        }
        Ok(diff)
    }

    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    pub fn save(&self, path: &Path) -> Result<(), DiffError> {
        let name = jsonfile::display_name(path);
        let io = |source: std::io::Error| DiffError::Io {
            name: name.clone(),
            source,
        };
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_json()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }
}
