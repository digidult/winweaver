//! Translations of the catalog texts: `lang/<code>.json` next to the catalog data.
//!
//! The file holds `_language` (the native name), an optional `_comment`, `_groups` (titles and
//! summaries by group id) and one entry per rule id. A missing text falls back to English.

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::catalog::{Group, Rule, embedded};
use crate::jsonfile::{self, JsonError};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupText {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleText {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub versions: Option<String>,
    #[serde(default)]
    pub verify: Option<String>,
    #[serde(default)]
    pub rollback: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Parameter titles by parameter name.
    #[serde(default)]
    pub params: IndexMap<String, String>,
    /// Titles of enum values by parameter name and value (as text).
    #[serde(default)]
    pub values: IndexMap<String, IndexMap<String, String>>,
}

/// The translation of the catalog into one language.
#[derive(Debug, Clone, Default)]
pub struct CatalogTexts {
    /// The language code (`ru`, `uk`).
    pub code: String,
    /// The native name of the language.
    pub language: String,
    pub groups: IndexMap<String, GroupText>,
    pub rules: IndexMap<String, RuleText>,
}

impl CatalogTexts {
    /// Parses a translation file. Keys that start with an underscore are the file fields; every
    /// other key is a rule id.
    pub fn parse(code: &str, text: &str) -> Result<Self, JsonError> {
        let name = format!("{code}.json");
        let raw: IndexMap<String, Value> = jsonfile::parse(text, &name)?;
        let mut texts = Self {
            code: code.to_owned(),
            ..Self::default()
        };
        let syntax = |source: serde_json::Error| JsonError::Syntax {
            name: name.clone(),
            source,
        };
        for (key, value) in raw {
            match key.as_str() {
                "_language" => {
                    texts.language = value.as_str().unwrap_or_default().to_owned();
                }
                "_comment" => {}
                "_groups" => {
                    texts.groups = serde_json::from_value(value).map_err(syntax)?;
                }
                other if other.starts_with('_') => {
                    return Err(syntax(serde::de::Error::custom(format!(
                        "unknown field {other}"
                    ))));
                }
                _ => {
                    let rule: RuleText = serde_json::from_value(value).map_err(syntax)?;
                    texts.rules.insert(key, rule);
                }
            }
        }
        Ok(texts)
    }

    /// The translations embedded in the executable.
    pub fn embedded() -> Result<Vec<Self>, JsonError> {
        embedded::TEXT_FILES
            .iter()
            .map(|(code, text)| Self::parse(code, text))
            .collect()
    }

    pub fn rule_title<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rules
            .get(&rule.id)
            .and_then(|t| t.title.as_deref())
            .unwrap_or(&rule.title)
    }

    pub fn rule_summary<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rules
            .get(&rule.id)
            .and_then(|t| t.summary.as_deref())
            .unwrap_or(&rule.summary)
    }

    pub fn group_title<'a>(&'a self, group: &'a Group) -> &'a str {
        self.groups
            .get(&group.id)
            .and_then(|t| t.title.as_deref())
            .unwrap_or(&group.title)
    }

    pub fn group_summary<'a>(&'a self, group: &'a Group) -> &'a str {
        self.groups
            .get(&group.id)
            .and_then(|t| t.summary.as_deref())
            .unwrap_or(&group.summary)
    }

    /// The title of a parameter of a rule.
    pub fn param_title<'a>(&'a self, rule: &'a Rule, name: &str) -> &'a str {
        self.rules
            .get(&rule.id)
            .and_then(|t| t.params.get(name))
            .map(String::as_str)
            .or_else(|| rule.params.get(name).map(|p| p.title()))
            .unwrap_or(name)
    }
}
