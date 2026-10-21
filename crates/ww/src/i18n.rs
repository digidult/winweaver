//! Translations of the interface: the English text is the key, a language is a file
//! `lang/<code>/ui.json` with `_language` (the native name) and the translations. The program
//! embeds its languages (`install`); a folder next to the executable adds a language or
//! overrides an embedded one. A missing text falls back to English.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use ww_base::jsonfile;

const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// The translations the program embeds: the language code and the file, set once at the start.
static EMBEDDED: RwLock<&'static [(&'static str, &'static str)]> = RwLock::new(&[]);

/// Names the translations the program embeds.
pub fn install(list: &'static [(&'static str, &'static str)]) {
    if let Ok(mut e) = EMBEDDED.write() {
        *e = list;
    }
}

fn embedded() -> &'static [(&'static str, &'static str)] {
    EMBEDDED.read().map(|e| *e).unwrap_or(&[])
}

/// A language the interface can show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

type Table = HashMap<&'static str, &'static str>;

static CURRENT: RwLock<Option<&'static Table>> = RwLock::new(None);
static CODE: RwLock<String> = RwLock::new(String::new());

/// The code of the language the interface shows (`en` before any choice).
pub fn current() -> String {
    let code = CODE.read().map(|c| c.clone()).unwrap_or_default();
    if code.is_empty() {
        "en".to_owned()
    } else {
        code
    }
}

/// The translation of an English text in the current language, the text itself when there is
/// none.
pub fn tr(text: &'static str) -> &'static str {
    let current = CURRENT.read().map(|c| *c).unwrap_or(None);
    current.and_then(|t| t.get(text).copied()).unwrap_or(text)
}

/// `tr` of a text with `{name}` placeholders, filled from `args`.
pub fn trf(text: &'static str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = tr(text).to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

/// Parses a translation file into its native name and its table.
pub fn parse(text: &str, name: &str) -> Result<(String, HashMap<String, String>), String> {
    let raw: HashMap<String, serde_json::Value> =
        jsonfile::parse(text, name).map_err(|e| e.to_string())?;
    let mut language = String::new();
    let mut table = HashMap::new();
    for (key, value) in raw {
        match (key.as_str(), value) {
            ("_language", serde_json::Value::String(s)) => language = s,
            (k, _) if k.starts_with('_') => {}
            (_, serde_json::Value::String(s)) => {
                table.insert(key, s);
            }
            (k, _) => return Err(format!("{name}: the translation of {k:?} is not a text")),
        }
    }
    Ok((language, table))
}

/// The languages the interface can show: English, the embedded ones and the folders of `lang/`
/// next to the executable, by code.
pub fn languages(dir: &Path) -> Vec<Language> {
    let mut out = vec![Language {
        code: "en".to_owned(),
        name: "English".to_owned(),
    }];
    for (code, text) in embedded() {
        if let Ok((name, _)) = parse(text, &format!("{code}/ui.json")) {
            out.push(Language {
                code: code.to_owned(),
                name,
            });
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut found: Vec<Language> = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let code = e.file_name().to_string_lossy().into_owned();
                let file = e.path().join("ui.json");
                let text = jsonfile::read_text(&file, MAX_BYTES).ok()?;
                let (name, _) = parse(&text, &format!("{code}/ui.json")).ok()?;
                Some(Language { code, name })
            })
            .filter(|l| out.iter().all(|o| o.code != l.code))
            .collect();
        found.sort_by(|a, b| a.code.cmp(&b.code));
        out.extend(found);
    }
    out
}

/// Switches the interface to a language: the embedded file, then the file of the folder on
/// top of it. English, or a language without any file, clears the translations.
pub fn set_language(code: &str, dir: &Path) {
    let mut table: HashMap<String, String> = HashMap::new();
    if let Some((_, text)) = embedded().iter().find(|(c, _)| *c == code)
        && let Ok((_, t)) = parse(text, &format!("{code}/ui.json"))
    {
        table.extend(t);
    }
    let file = dir.join(code).join("ui.json");
    if code != "en"
        && let Ok(text) = jsonfile::read_text(&file, MAX_BYTES)
        && let Ok((_, t)) = parse(&text, &format!("{code}/ui.json"))
    {
        table.extend(t);
    }
    let leaked: Option<&'static Table> = if table.is_empty() {
        None
    } else {
        let t: Table = table
            .into_iter()
            .map(|(k, v)| {
                let k: &'static str = Box::leak(k.into_boxed_str());
                let v: &'static str = Box::leak(v.into_boxed_str());
                (k, v)
            })
            .collect();
        Some(Box::leak(Box::new(t)))
    };
    if let Ok(mut current) = CURRENT.write() {
        *current = leaked;
    }
    if let Ok(mut c) = CODE.write() {
        *c = code.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_fallback_and_a_file_parses() {
        assert_eq!(tr("A text nobody translated"), "A text nobody translated");
        assert_eq!(
            trf("{n} files for {name}", &[("n", &3), ("name", &"PC")]),
            "3 files for PC"
        );
        let (name, table) =
            parse("{\"_language\": \"Test\", \"Save\": \"Saved\"}", "t/ui.json").unwrap();
        assert_eq!(name, "Test");
        assert_eq!(table["Save"], "Saved");
        assert!(parse("{\"Save\": 1}", "t/ui.json").is_err());
    }
}
