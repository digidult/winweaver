//! Diff files (`.wwdiff`): the differences between two profiles, applied to a profile as a
//! merge (`docs/spec/02-documents.md` section 4). A diff holds only what differs.

use std::fmt;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::jsonfile::{self, DocError, Document};
use crate::paths;
use crate::profile::{Choice, Profile};

pub const FORMAT: &str = "winweaver-diff";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "wwdiff";

/// What went wrong while reading or writing a diff: the errors of every document.
pub type DiffError = DocError;

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

impl Document for Diff {
    const FORMAT: &'static str = FORMAT;
    const VERSION: u32 = VERSION;
    const EXTENSION: &'static str = EXTENSION;
    const KIND: &'static str = "diff";

    fn format(&self) -> &str {
        &self.format
    }

    fn version(&self) -> u32 {
        self.version
    }
}

/// The key of an item of a diff, what its comment is kept by: `install:<field>`,
/// `languages:<field>`, `accounts` or `rule:<id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKey {
    Install(String),
    Languages(String),
    Accounts,
    Rule(String),
}

impl ItemKey {
    const INSTALL: &str = "install:";
    const LANGUAGES: &str = "languages:";
    const ACCOUNTS: &str = "accounts";
    const RULE: &str = "rule:";

    pub fn parse(key: &str) -> Option<Self> {
        if let Some(id) = key.strip_prefix(Self::RULE) {
            Some(Self::Rule(id.to_owned()))
        } else if let Some(field) = key.strip_prefix(Self::INSTALL) {
            Some(Self::Install(field.to_owned()))
        } else if let Some(field) = key.strip_prefix(Self::LANGUAGES) {
            Some(Self::Languages(field.to_owned()))
        } else if key == Self::ACCOUNTS {
            Some(Self::Accounts)
        } else {
            None
        }
    }

    /// The id of the rule a key names, for the keys of rules.
    pub fn rule(key: &str) -> Option<&str> {
        key.strip_prefix(Self::RULE)
    }
}

impl fmt::Display for ItemKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Install(field) => write!(f, "{}{field}", Self::INSTALL),
            Self::Languages(field) => write!(f, "{}{field}", Self::LANGUAGES),
            Self::Accounts => f.write_str(Self::ACCOUNTS),
            Self::Rule(id) => write!(f, "{}{id}", Self::RULE),
        }
    }
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

    /// The keys of the items in their order (`ItemKey`).
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .install
            .keys()
            .map(|k| ItemKey::Install(k.clone()).to_string())
            .collect();
        keys.extend(
            self.languages
                .keys()
                .map(|k| ItemKey::Languages(k.clone()).to_string()),
        );
        if self.accounts.is_some() {
            keys.push(ItemKey::Accounts.to_string());
        }
        keys.extend(
            self.rules
                .keys()
                .map(|k| ItemKey::Rule(k.clone()).to_string()),
        );
        keys
    }

    /// Sets one item of a profile to the value of the other side.
    pub fn apply_item(&self, key: &str, profile: &mut Profile) {
        match ItemKey::parse(key) {
            Some(ItemKey::Rule(id)) => {
                if let Some(d) = self.rules.get(&id) {
                    match &d.other {
                        Some(c) => {
                            profile.rules.insert(id, c.clone());
                        }
                        None => {
                            profile.rules.shift_remove(&id);
                        }
                    }
                }
            }
            Some(ItemKey::Install(field)) => {
                if let Some(f) = self.install.get(&field) {
                    let one: IndexMap<String, FieldDiff> = [(field, f.clone())].into();
                    if let Some(install) = merged(&profile.install, &one) {
                        profile.install = install;
                    }
                }
            }
            Some(ItemKey::Languages(field)) => {
                if let Some(f) = self.languages.get(&field) {
                    let one: IndexMap<String, FieldDiff> = [(field, f.clone())].into();
                    if let Some(languages) = merged(&profile.languages, &one) {
                        profile.languages = languages;
                    }
                }
            }
            Some(ItemKey::Accounts) => {
                if let Some(d) = &self.accounts
                    && let Ok(accounts) = serde_json::from_value(d.other.clone())
                {
                    profile.accounts = accounts;
                }
            }
            None => {}
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

    /// The file with the profiles inside the program folder named by absolute paths again.
    pub fn load(path: &Path) -> Result<Self, DiffError> {
        let mut diff = Self::load_checked(path)?;
        diff.resolve_in(&paths::program_dir());
        Ok(diff)
    }

    /// The diff as a file keeps it: a profile inside `base` relative to it.
    pub fn stored_in(&self, base: &Path) -> Self {
        let mut diff = self.clone();
        for side in [&mut diff.base, &mut diff.other] {
            side.path = paths::stored_text(&side.path, base);
        }
        diff
    }

    /// The diff of a file with the profiles kept relative to `base` made absolute.
    pub fn resolve_in(&mut self, base: &Path) {
        for side in [&mut self.base, &mut self.other] {
            side.path = paths::resolved_text(&side.path, base);
        }
    }

    pub fn parse(text: &str, name: &str) -> Result<Self, DiffError> {
        Self::parse_checked(text, name)
    }

    pub fn to_json(&self) -> String {
        self.json_text()
    }

    pub fn save(&self, path: &Path) -> Result<(), DiffError> {
        let text = self.stored_in(&paths::program_dir()).to_json();
        jsonfile::write_atomic(path, &text)
    }
}
