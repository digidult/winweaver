//! Translations of the catalog texts: `lang/<code>.json` next to the catalog data.
//!
//! The file holds `_language` (the native name), an optional `_comment`, `_groups` (titles and
//! summaries by group id) and one entry per rule id. A missing text falls back to English.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{EnumValue, Group, Rule, embedded};
use crate::jsonfile::{self, JsonError};

/// The texts of a group in one language: of the translation files of the catalog and of the
/// catalog files alike.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupText {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// The texts of a rule in one language: of the translation files of the catalog and of the
/// catalog files alike. A text left out is the English one.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleText {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Parameter titles by parameter name.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, String>,
    /// Titles of enum values by parameter name and value (as text).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub values: IndexMap<String, IndexMap<String, String>>,
}

impl RuleText {
    /// Whether the text holds nothing, so that a file leaves it out.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Every text of the translation, for a search.
    pub fn all_texts(&self) -> impl Iterator<Item = &str> {
        [
            &self.title,
            &self.summary,
            &self.effect,
            &self.risk,
            &self.versions,
            &self.verify,
            &self.rollback,
            &self.note,
        ]
        .into_iter()
        .filter_map(|t| t.as_deref())
        .chain(self.tags.iter().flatten().map(String::as_str))
        .chain(self.params.values().map(String::as_str))
        .chain(
            self.values
                .values()
                .flat_map(|v| v.values().map(String::as_str)),
        )
    }
}

/// The key of a value of an enum parameter in a translation: its text (`1`, `Warn`).
pub fn value_key(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The translation of the catalog into one language.
#[derive(Debug, Clone, Default)]
pub struct CatalogTexts {
    /// The language code (`ru`, `uk`).
    pub code: String,
    /// The native name of the language.
    pub language: String,
    /// The word after the title of the Disabled rule of an imported policy (`<title>: <word>`).
    pub disabled: String,
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
                "_disabled" => {
                    texts.disabled = value.as_str().unwrap_or_default().to_owned();
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

    /// A text of a rule in this language, or the English one the file lacks.
    fn rule_field<'a>(
        &'a self,
        rule: &'a Rule,
        field: fn(&RuleText) -> Option<&str>,
        english: &'a str,
    ) -> &'a str {
        self.rules.get(&rule.id).and_then(field).unwrap_or(english)
    }

    pub fn rule_effect<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rule_field(rule, |t| t.effect.as_deref(), &rule.effect)
    }

    pub fn rule_risk<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rule_field(rule, |t| t.risk.as_deref(), &rule.risk)
    }

    pub fn rule_versions<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rule_field(rule, |t| t.versions.as_deref(), &rule.versions)
    }

    pub fn rule_verify<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rule_field(rule, |t| t.verify.as_deref(), &rule.verify)
    }

    pub fn rule_rollback<'a>(&'a self, rule: &'a Rule) -> &'a str {
        self.rule_field(rule, |t| t.rollback.as_deref(), &rule.rollback)
    }

    /// The search words of a rule in this language (the English ones are searched too).
    pub fn rule_tags<'a>(&'a self, rule: &'a Rule) -> &'a [String] {
        self.rules
            .get(&rule.id)
            .and_then(|t| t.tags.as_deref())
            .unwrap_or(&[])
    }

    /// The title of a value of an enum parameter of a rule; the file keys the values by their
    /// text (`1`, `Warn`).
    pub fn value_title<'a>(&'a self, rule: &'a Rule, name: &str, value: &'a EnumValue) -> &'a str {
        let key = value_key(&value.value);
        self.rules
            .get(&rule.id)
            .and_then(|t| t.values.get(name))
            .and_then(|v| v.get(&key))
            .map_or(value.title.as_str(), String::as_str)
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
    pub fn param_title<'a>(&'a self, rule: &'a Rule, name: &'a str) -> &'a str {
        self.rules
            .get(&rule.id)
            .and_then(|t| t.params.get(name))
            .map(String::as_str)
            .or_else(|| rule.params.get(name).map(|p| p.title()))
            .unwrap_or(name)
    }
}
