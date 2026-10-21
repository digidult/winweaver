//! The results of the checks and the data of the forms, as the audit script writes them and
//! as a native read gives them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::values;
use crate::jsonfile;

use super::REPORT_MAX_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Differs,
    Unknown,
}

/// The result of one check, as the audit script writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub rule: String,
    pub check: String,
    pub status: Status,
    #[serde(default)]
    pub current: String,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub note: String,
}

/// An input language read from Windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputLanguage {
    pub tag: String,
    #[serde(default)]
    pub tips: Vec<String>,
}

/// A local account read from Windows (never a password).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReadAccount {
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub group: String,
}

/// The data of the forms read from Windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub computer_name: String,
    pub edition_id: String,
    pub build: String,
    pub display_version: String,
    pub time_zone: String,
    pub ui_language: String,
    pub system_locale: String,
    pub user_locale: String,
    pub input: Vec<InputLanguage>,
    pub accounts: Vec<ReadAccount>,
    /// Why the accounts are missing, when they are.
    pub accounts_error: String,
}

/// An audit: the results of the checks and, optionally, the data of the forms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub computer: String,
    pub admin: bool,
    pub time: String,
    pub system: Option<SystemInfo>,
    pub results: Vec<CheckResult>,
}

fn field(value: &Value, key: &str) -> String {
    value.get(key).map(values::text).unwrap_or_default()
}

/// A list, also when PowerShell wrote a single object or nothing.
fn items(value: &Value, key: &str) -> Vec<Value> {
    match value.get(key) {
        Some(Value::Array(list)) => list.clone(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => vec![other.clone()],
    }
}

impl SystemInfo {
    fn from_value(v: &Value) -> Self {
        Self {
            computer_name: field(v, "computer_name"),
            edition_id: field(v, "edition_id"),
            build: field(v, "build"),
            display_version: field(v, "display_version"),
            time_zone: field(v, "time_zone"),
            ui_language: field(v, "ui_language"),
            system_locale: field(v, "system_locale"),
            user_locale: field(v, "user_locale"),
            input: items(v, "input")
                .iter()
                .map(|i| match i {
                    Value::String(tag) => InputLanguage {
                        tag: tag.clone(),
                        tips: Vec::new(),
                    },
                    other => InputLanguage {
                        tag: field(other, "tag"),
                        tips: items(other, "tips").iter().map(values::text).collect(),
                    },
                })
                .filter(|i| !i.tag.is_empty())
                .collect(),
            accounts: items(v, "accounts")
                .iter()
                .map(|a| ReadAccount {
                    name: field(a, "name"),
                    full_name: field(a, "full_name"),
                    description: field(a, "description"),
                    group: field(a, "group"),
                })
                .filter(|a| !a.name.is_empty())
                .collect(),
            accounts_error: field(v, "accounts_error"),
        }
    }
}

impl Report {
    /// Parses the JSON report of the audit script (PowerShell writes it with a byte order mark).
    pub fn parse(text: &str, name: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let value: Value = jsonfile::parse(text, name).map_err(|e| e.to_string())?;
        if !value.is_object() || value.get("results").is_none() {
            return Err(format!("{name}: not an audit report of WinWeaver"));
        }
        let mut results = Vec::new();
        for item in items(&value, "results") {
            let status = match field(&item, "status").as_str() {
                "ok" => Status::Ok,
                "differs" => Status::Differs,
                _ => Status::Unknown,
            };
            results.push(CheckResult {
                rule: field(&item, "rule"),
                check: field(&item, "check"),
                status,
                current: field(&item, "current"),
                expected: field(&item, "expected"),
                note: field(&item, "note"),
            });
        }
        Ok(Self {
            computer: field(&value, "computer"),
            admin: value.get("admin").and_then(Value::as_bool).unwrap_or(false),
            time: field(&value, "time"),
            system: value
                .get("system")
                .filter(|s| s.is_object())
                .map(SystemInfo::from_value),
            results,
        })
    }

    /// Reads a report file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = jsonfile::read_text_bom(path, REPORT_MAX_BYTES).map_err(|e| e.to_string())?;
        Self::parse(&text, &jsonfile::display_name(path))
    }
}
