//! Translations of the interface (principle P3): the English text is the key, a language is a
//! file `lang/<code>/ui.json` with `_language` (the native name) and the translations. Ukrainian
//! and Russian are embedded; a folder next to the executable adds a language or overrides an
//! embedded one. A missing text falls back to English.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use winweaver_core::jsonfile;

/// The embedded translations: the language code and the file. Every text of the interface has a
/// translation in each of them (`every_text_of_the_interface_is_translated`).
pub const EMBEDDED: [(&str, &str); 8] = [
    ("uk", include_str!("../../../data/lang/uk/ui.json")),
    ("ru", include_str!("../../../data/lang/ru/ui.json")),
    ("de", include_str!("../../../data/lang/de/ui.json")),
    ("es", include_str!("../../../data/lang/es/ui.json")),
    ("fr", include_str!("../../../data/lang/fr/ui.json")),
    ("id", include_str!("../../../data/lang/id/ui.json")),
    ("it", include_str!("../../../data/lang/it/ui.json")),
    ("tr", include_str!("../../../data/lang/tr/ui.json")),
];
const MAX_BYTES: u64 = 4 * 1024 * 1024;

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
    for (code, text) in EMBEDDED {
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
    if let Some((_, text)) = EMBEDDED.iter().find(|(c, _)| *c == code)
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
    fn the_embedded_files_translate_and_english_is_the_fallback() {
        assert_eq!(tr("A text nobody translated"), "A text nobody translated");
        assert_eq!(
            trf("{n} rules for {name}", &[("n", &3), ("name", &"PC")]),
            tr("{n} rules for {name}")
                .replace("{n}", "3")
                .replace("{name}", "PC")
        );
        for (code, text) in EMBEDDED {
            let (name, table) = parse(text, code).unwrap();
            assert!(!name.is_empty(), "{code}: _language");
            assert!(
                table.get("Save").is_some_and(|t| t != "Save"),
                "{code}: Save"
            );
        }
        let dir = std::env::temp_dir().join("winweaver-no-lang-folder");
        let codes: Vec<String> = languages(&dir).into_iter().map(|l| l.code).collect();
        assert_eq!(
            codes,
            vec!["en", "uk", "ru", "de", "es", "fr", "id", "it", "tr"]
        );
    }

    /// The literals of the `tr` and `trf` calls of a source text.
    fn literals(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for pattern in ["tr(\"", "trf(\""] {
            let mut rest = text;
            while let Some(i) = rest.find(pattern) {
                let before = rest[..i].chars().last();
                let after = &rest[i + pattern.len()..];
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    rest = after;
                    continue;
                }
                let mut literal = String::new();
                let mut escaped = false;
                let mut used = 0;
                for c in after.chars() {
                    used += c.len_utf8();
                    if escaped {
                        literal.push(c);
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    } else {
                        literal.push(c);
                    }
                }
                out.push(literal);
                rest = &after[used..];
            }
        }
        out
    }

    /// The Belarusian file shipped in `lang/be/` of the archive, the example of a translation:
    /// it reads, its placeholders match and it has no text the interface does not know.
    #[test]
    fn the_example_translation_reads() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../package/lang/be/ui.json");
        let text = std::fs::read_to_string(&path).unwrap();
        let (name, table) = parse(&text, "be/ui.json").unwrap();
        assert!(!name.is_empty());
        let (_, known) = parse(EMBEDDED[1].1, "ru/ui.json").unwrap();
        let marks = |t: &str| {
            let mut found = Vec::new();
            let mut rest = t;
            while let Some(i) = rest.find('{') {
                let Some(j) = rest[i..].find('}') else {
                    break;
                };
                found.push(rest[i..i + j + 1].to_owned());
                rest = &rest[i + j + 1..];
            }
            found.sort();
            found
        };
        for (key, value) in &table {
            assert!(
                known.contains_key(key),
                "be: {key:?} is not a text of the interface"
            );
            assert_eq!(marks(key), marks(value), "be: {key:?}");
            assert!(
                !value.contains('\u{2013}') && !value.contains('\u{2014}'),
                "be: a dash in {key:?}"
            );
        }
    }

    #[test]
    fn every_text_of_the_interface_is_translated() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut keys = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(&src).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|n| n != "i18n.rs")
            {
                let text = std::fs::read_to_string(&path).unwrap();
                keys.extend(literals(&text));
            }
        }
        assert!(keys.len() > 100, "{} texts found", keys.len());
        let placeholders = |t: &str| {
            let mut found: Vec<String> = Vec::new();
            let mut rest = t;
            while let Some(i) = rest.find('{') {
                match rest[i..].find('}') {
                    Some(j) => {
                        found.push(rest[i..i + j + 1].to_owned());
                        rest = &rest[i + j + 1..];
                    }
                    None => break,
                }
            }
            found.sort();
            found
        };
        for (code, text) in EMBEDDED {
            let (_, table) = parse(text, code).unwrap();
            for key in &keys {
                let translated = table
                    .get(key)
                    .unwrap_or_else(|| panic!("{code}/ui.json has no translation of {key:?}"));
                assert_eq!(
                    placeholders(key),
                    placeholders(translated),
                    "{code}: {key:?}"
                );
                assert!(
                    !translated.contains('\u{2013}') && !translated.contains('\u{2014}'),
                    "{code}: a dash in {key:?}"
                );
            }
        }
    }

    /// A dialog shows its answer beside Cancel, so no translation of an answer may read as
    /// Cancel: the Ukrainian Discard once said Cancel too. The answers of `App::ask`, of the
    /// confirmation of an apply, of the editors and of the permissions.
    #[test]
    fn the_answers_of_a_dialog_do_not_read_as_cancel() {
        let answers = [
            "Remove",
            "Discard",
            "Forget",
            "Renew",
            "Change",
            "Delete",
            "Change this PC",
            "OK",
            "Add",
            "Apply",
        ];
        let start = |t: &str| t.to_lowercase().chars().take(4).collect::<String>();
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../package/lang/be/ui.json");
        let be = std::fs::read_to_string(&path).unwrap();
        let mut files: Vec<(&str, &str)> = EMBEDDED.to_vec();
        files.push(("be", be.as_str()));
        for (code, text) in files {
            let (_, table) = parse(text, code).unwrap();
            let cancel = start(&table["Cancel"]);
            for key in answers {
                let answer = start(&table[key]);
                assert_ne!(answer, cancel, "{code}: {key:?} reads as Cancel");
            }
        }
    }
}
