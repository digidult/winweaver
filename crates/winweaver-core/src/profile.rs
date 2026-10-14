//! A profile: which rules are on, with which parameters, and the installation forms.
//!
//! The file format is `docs/spec/02-documents.md` section 3. Only the rules whose state differs
//! from "off" are listed; parameters only when they differ from the defaults.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::Catalog;
use crate::jsonfile::{self, JsonError};

pub const FORMAT: &str = "winweaver-profile";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "wwprofile";

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error("{name}: {source}")]
    Io {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{name}: not a WinWeaver profile (format {found})")]
    Format { name: String, found: String },
    #[error("{name}: profile version {found} is newer than this program knows ({VERSION})")]
    Version { name: String, found: u32 },
}

/// The form "Installation".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Install {
    pub edition: String,
    /// `generic`, `ask` or `key`.
    pub product_key_mode: String,
    pub product_key: String,
    pub time_zone: String,
    /// `file` (the accounts of the profile) or `ask` (Windows Setup asks for one).
    pub account_mode: String,
    /// `random`, `fixed` or `template`.
    pub computer_name_mode: String,
    pub computer_name: String,
    pub workgroup: String,
    /// `ask` or `partition`.
    pub disk_mode: String,
    pub disk_id: String,
    pub partition_id: String,
}

impl Default for Install {
    fn default() -> Self {
        Self {
            edition: "Pro".into(),
            product_key_mode: "generic".into(),
            product_key: String::new(),
            time_zone: "FLE Standard Time".into(),
            account_mode: "file".into(),
            computer_name_mode: "random".into(),
            computer_name: String::new(),
            workgroup: String::new(),
            disk_mode: "ask".into(),
            disk_id: "0".into(),
            partition_id: "3".into(),
        }
    }
}

/// The form "Languages".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Languages {
    pub ui_language: String,
    pub system_locale: String,
    pub user_locale: String,
    /// Input languages in order; the first is the default.
    pub input: Vec<String>,
}

impl Default for Languages {
    fn default() -> Self {
        Self {
            ui_language: "uk-UA".into(),
            system_locale: "uk-UA".into(),
            user_locale: "uk-UA".into(),
            input: vec!["en-US".into(), "uk-UA".into(), "ru-UA".into()],
        }
    }
}

/// An account of the form "Accounts".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    pub name: String,
    pub display_name: String,
    /// `Administrators` or `Users`.
    pub group: String,
    pub description: String,
    pub password: String,
}

/// The choice of one rule.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Choice {
    pub on: bool,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Value>,
    /// The kind of import a choice of an imported policy was made in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub format: String,
    pub version: u32,
    /// The version of the catalog the profile was saved with.
    pub catalog: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub modified: String,
    #[serde(default)]
    pub install: Install,
    #[serde(default)]
    pub languages: Languages,
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub rules: IndexMap<String, Choice>,
    /// Choices of rules the loaded catalog does not have, kept unchanged.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub unknown: IndexMap<String, Value>,
}

impl Profile {
    /// An empty profile: every rule off, the default forms.
    pub fn new(name: &str, catalog_version: &str) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            catalog: catalog_version.into(),
            name: name.into(),
            author: String::new(),
            comment: String::new(),
            created: String::new(),
            modified: String::new(),
            install: Install::default(),
            languages: Languages::default(),
            accounts: vec![
                Account {
                    name: "Admin".into(),
                    display_name: "Admin".into(),
                    group: "Administrators".into(),
                    description: "Local administrator (starter account)".into(),
                    password: String::new(),
                },
                Account {
                    name: "User".into(),
                    display_name: "User".into(),
                    group: "Users".into(),
                    description: "Standard user (starter account)".into(),
                    password: String::new(),
                },
            ],
            rules: IndexMap::new(),
            unknown: IndexMap::new(),
        }
    }

    /// A profile with the catalog defaults: every rule that is on by default is on.
    pub fn from_defaults(name: &str, catalog: &Catalog) -> Self {
        let mut profile = Self::new(name, catalog.version());
        for rule in catalog.rules() {
            if rule.default {
                profile.rules.insert(
                    rule.id.clone(),
                    Choice {
                        on: true,
                        ..Choice::default()
                    },
                );
            }
        }
        profile
    }

    /// Whether the rule is chosen on (before the resolver).
    pub fn is_on(&self, id: &str) -> bool {
        self.rules.get(id).is_some_and(|c| c.on)
    }

    /// Switches a rule on or off; a rule switched off keeps its parameters.
    pub fn set_on(&mut self, id: &str, on: bool) {
        let choice = self.rules.entry(id.to_owned()).or_default();
        choice.on = on;
        if !on && choice.params.is_empty() && choice.source.is_none() {
            self.rules.shift_remove(id);
        }
    }

    /// The value of a parameter of a rule: the choice, else the default of the catalog.
    pub fn param(&self, catalog: &Catalog, id: &str, name: &str) -> Option<Value> {
        if let Some(v) = self.rules.get(id).and_then(|c| c.params.get(name)) {
            return Some(v.clone());
        }
        catalog
            .rule(id)
            .and_then(|r| r.params.get(name))
            .map(|p| p.default_value())
    }

    /// The ids of the rules the catalog does not have, moved into `unknown`.
    pub fn split_unknown(&mut self, catalog: &Catalog) -> Vec<String> {
        let unknown: Vec<String> = self
            .rules
            .keys()
            .filter(|id| catalog.rule(id).is_none())
            .cloned()
            .collect();
        for id in &unknown {
            if let Some(choice) = self.rules.shift_remove(id)
                && let Ok(value) = serde_json::to_value(choice)
            {
                self.unknown.insert(id.clone(), value);
            }
        }
        unknown
    }

    /// Reads a profile file strictly.
    pub fn load(path: &Path) -> Result<Self, ProfileError> {
        let name = jsonfile::display_name(path);
        let text = jsonfile::read_text(path, jsonfile::MAX_BYTES)?;
        Self::parse(&text, &name)
    }

    /// Parses profile text.
    pub fn parse(text: &str, name: &str) -> Result<Self, ProfileError> {
        let profile: Self = jsonfile::parse(text, name)?;
        if profile.format != FORMAT {
            return Err(ProfileError::Format {
                name: name.to_owned(),
                found: profile.format,
            });
        }
        if profile.version > VERSION {
            return Err(ProfileError::Version {
                name: name.to_owned(),
                found: profile.version,
            });
        }
        Ok(profile)
    }

    /// The canonical text of the profile: two-space indentation, a trailing line break.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    /// Writes the profile through a temporary file that replaces the target, so a failed save
    /// leaves the old file whole.
    pub fn save(&self, path: &Path) -> Result<(), ProfileError> {
        let name = jsonfile::display_name(path);
        let io = |source: std::io::Error| ProfileError::Io {
            name: name.clone(),
            source,
        };
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_json()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }
}
