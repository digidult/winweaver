//! Opening files of other kinds as profiles (`docs/spec/03-features.md` section 3): an answer file
//! with the profile it embeds (built by WinWeaver or by WinKickOff), and a profile of WinKickOff,
//! the predecessor, so that its users keep their work.

use indexmap::IndexMap;
use serde_json::Value;

use crate::jsonfile;
use crate::profile::{self, Account, Choice, Install, Languages, Profile};

/// The start and the end of the profile embedded in an answer file.
const EMBED_START: &str = "<Profile format=\"json\"><![CDATA[";
const EMBED_END: &str = "]]></Profile>";
/// The newest profile format of WinKickOff this program converts.
pub const WINKICKOFF_FORMAT: u64 = 5;

/// The kind of a profile text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    WinWeaver,
    WinKickOff,
}

/// What a JSON object holds: a WinWeaver profile, a WinKickOff profile, or neither.
pub fn kind_of(value: &Value) -> Option<Kind> {
    if value.get("format").and_then(Value::as_str) == Some(profile::FORMAT) {
        Some(Kind::WinWeaver)
    } else if value.get("format_version").is_some_and(Value::is_u64)
        && value.get("rules").is_some_and(Value::is_object)
    {
        Some(Kind::WinKickOff)
    } else {
        None
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A profile of WinKickOff as a WinWeaver profile; the notes say what was dropped.
pub fn from_winkickoff(value: &Value, name: &str) -> Result<(Profile, Vec<String>), String> {
    let mut notes = Vec::new();
    let version = value
        .get("format_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{name}: not a WinKickOff profile"))?;
    if version > WINKICKOFF_FORMAT {
        notes.push(format!(
            "the WinKickOff profile format {version} is newer than {WINKICKOFF_FORMAT}; fields this program does not know were left out"
        ));
    }
    let mut p = Profile::new(&text(value, "name"), &text(value, "catalog_version"));
    p.author = text(value, "author");
    p.comment = text(value, "comment");
    p.created = text(value, "created");
    p.modified = text(value, "modified");
    if let Some(install) = value.get("install") {
        match serde_json::from_value::<Install>(install.clone()) {
            Ok(i) => p.install = i,
            Err(e) => notes.push(format!("the installation form was not read: {e}")),
        }
    }
    if let Some(languages) = value.get("languages") {
        match serde_json::from_value::<Languages>(languages.clone()) {
            Ok(l) => p.languages = l,
            Err(e) => notes.push(format!("the languages were not read: {e}")),
        }
    }
    if let Some(accounts) = value.get("accounts") {
        match serde_json::from_value::<Vec<Account>>(accounts.clone()) {
            Ok(a) => p.accounts = a,
            Err(e) => notes.push(format!("the accounts were not read: {e}")),
        }
    }
    let mut rules = IndexMap::new();
    if let Some(Value::Object(entries)) = value.get("rules") {
        for (id, entry) in entries {
            let on = match entry.get("enabled") {
                Some(Value::Bool(b)) => *b,
                Some(Value::Number(n)) => n.as_i64() == Some(1),
                _ => false,
            };
            let params: IndexMap<String, Value> = match entry.get("params") {
                Some(Value::Object(map)) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => IndexMap::new(),
            };
            let source = entry.get("source").and_then(Value::as_str).map(str::to_owned);
            if on || !params.is_empty() || source.is_some() {
                rules.insert(id.clone(), Choice { on, params, source });
            }
        }
    }
    p.rules = rules;
    if let Some(Value::Object(unknown)) = value.get("unknown") {
        for (id, entry) in unknown {
            p.unknown.insert(id.clone(), entry.clone());
        }
    }
    Ok((p, notes))
}

/// The profile of a JSON text of either kind.
pub fn profile_from_json(text: &str, name: &str) -> Result<(Profile, Vec<String>), String> {
    let value: Value = jsonfile::parse(text, name).map_err(|e| e.to_string())?;
    match kind_of(&value) {
        Some(Kind::WinWeaver) => Profile::parse(text, name)
            .map(|p| (p, Vec::new()))
            .map_err(|e| e.to_string()),
        Some(Kind::WinKickOff) => {
            let (p, mut notes) = from_winkickoff(&value, name)?;
            notes.insert(0, "converted from a WinKickOff profile".to_owned());
            Ok((p, notes))
        }
        None => Err(format!("{name}: not a profile of WinWeaver or WinKickOff")),
    }
}

/// The profile an answer file embeds.
pub fn profile_from_answer_file(text: &str, name: &str) -> Result<(Profile, Vec<String>), String> {
    let start = text.find(EMBED_START).ok_or_else(|| {
        format!(
            "{name}: the answer file has no embedded profile; only files built by WinWeaver or WinKickOff open as a profile"
        )
    })?;
    let body = &text[start + EMBED_START.len()..];
    let end = body
        .find(EMBED_END)
        .ok_or_else(|| format!("{name}: the embedded profile has no end"))?;
    profile_from_json(body[..end].trim(), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_winkickoff_profile_converts() {
        let text = r#"{"format_version": 5, "catalog_version": "0.11", "name": "Old", "install": {"edition": "Pro", "workgroup": "LAB"},
            "languages": {"ui_language": "uk-UA", "input": ["en-US"]}, "accounts": [{"name": "Admin", "group": "Administrators"}],
            "rules": {"a.b": {"enabled": true}, "c.d": {"enabled": false}, "e.f": {"enabled": false, "params": {"x": 1}}},
            "unknown": {"admx.x": {"enabled": true}}}"#;
        let (p, notes) = profile_from_json(text, "old.json").unwrap();
        assert!(notes[0].contains("WinKickOff"));
        assert_eq!(p.format, profile::FORMAT);
        assert_eq!(p.name, "Old");
        assert_eq!(p.install.workgroup, "LAB");
        assert_eq!(p.install.disk_mode, "ask", "missing fields take the defaults");
        assert_eq!(p.languages.input, vec!["en-US".to_owned()]);
        assert_eq!(p.accounts.len(), 1);
        assert!(p.is_on("a.b") && !p.is_on("c.d") && !p.rules.contains_key("c.d"));
        assert_eq!(p.rules["e.f"].params["x"], Value::from(1));
        assert!(p.unknown.contains_key("admx.x"));
    }

    #[test]
    fn answer_files_without_a_profile_are_refused() {
        assert!(profile_from_answer_file("<unattend/>", "a.xml").is_err());
        assert!(profile_from_json("{\"a\": 1}", "x.json").is_err());
    }
}
