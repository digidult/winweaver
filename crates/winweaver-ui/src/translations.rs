//! The tests of the translations: every text of the interface, of WW too, has one in each
//! embedded language; the example language file reads; no answer of a dialog reads as Cancel.

use std::path::{Path, PathBuf};

use ww::i18n::{languages, parse, tr, trf};

use crate::LANGUAGES;

#[test]
fn the_embedded_files_translate_and_english_is_the_fallback() {
    ww::i18n::install(&LANGUAGES);
    assert_eq!(tr("A text nobody translated"), "A text nobody translated");
    assert_eq!(
        trf("{n} rules for {name}", &[("n", &3), ("name", &"PC")]),
        tr("{n} rules for {name}")
            .replace("{n}", "3")
            .replace("{name}", "PC")
    );
    for (code, text) in LANGUAGES {
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

/// The literals of the `tr` and `trf` calls of a source text: the literal right after the
/// bracket, or on the next line as rustfmt puts a long one. A call whose first argument is no
/// literal names no text.
fn literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pattern in ["tr(", "trf("] {
        let mut rest = text;
        while let Some(i) = rest.find(pattern) {
            let before = rest[..i].chars().last();
            let after = rest[i + pattern.len()..].trim_start();
            rest = &rest[i + pattern.len()..];
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let Some(after) = after.strip_prefix('"') else {
                continue;
            };
            let mut literal = String::new();
            let mut chars = after.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        // A line of the literal that goes on: the break and the indentation
                        // of the next line are no part of the text.
                        Some('\n') => while chars.next_if(|c| c.is_whitespace()).is_some() {},
                        Some('n') => literal.push('\n'),
                        Some('t') => literal.push('\t'),
                        Some(other) => literal.push(other),
                        None => break,
                    },
                    c => literal.push(c),
                }
            }
            out.push(literal);
        }
    }
    out
}

/// The literals are found wherever rustfmt puts them.
#[test]
fn the_literals_of_a_source_are_found() {
    let source = concat!(
        "let a = tr(\"One\"); let b = trf(\n    \"Two {n} \\\"x\\\"\",\n    &[(\"n\", &1)],\n);\n",
        "let c = tr(\n        \"Three \\\n         more\",\n    );\n",
        "attr(\"no\"); str(\"no\"); tr(name); trf(key, &[]);\n",
    );
    let mut found = literals(source);
    found.sort();
    assert_eq!(found, vec!["One", "Three more", "Two {n} \"x\""]);
}

/// Every Rust source under a folder, in its subfolders too.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn placeholders(t: &str) -> Vec<String> {
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
}

fn example_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../package/lang/be/ui.json")
}

/// The Belarusian file shipped in `lang/be/` of the archive, the example of a translation: it
/// reads, its placeholders match and it has no text the interface does not know.
#[test]
fn the_example_translation_reads() {
    let text = std::fs::read_to_string(example_file()).unwrap();
    let (name, table) = parse(&text, "be/ui.json").unwrap();
    assert!(!name.is_empty());
    let (_, known) = parse(LANGUAGES[1].1, "ru/ui.json").unwrap();
    for (key, value) in &table {
        assert!(
            known.contains_key(key),
            "be: {key:?} is not a text of the interface"
        );
        assert_eq!(placeholders(key), placeholders(value), "be: {key:?}");
        assert!(
            !value.contains('\u{2013}') && !value.contains('\u{2014}'),
            "be: a dash in {key:?}"
        );
    }
}

/// The files of the core whose messages the window shows translated: each makes its messages
/// with the `tr` and `trf` of `winweaver_core::message`, which keep the English text as the key
/// of the translation, so their literals are texts of the interface like the ones of the window.
const CORE_MESSAGES: [&str; 1] = ["validate.rs"];

/// The texts of `winweaver-ui`, of `ww` and of `wwmcp`, and the messages of the core the window
/// shows (`CORE_MESSAGES`): this file is left out, since it names texts that are not of the
/// interface.
#[test]
fn every_text_of_the_interface_is_translated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    for name in CORE_MESSAGES {
        let file = root.join("../winweaver-core/src").join(name);
        assert!(file.is_file(), "{}", file.display());
        files.push(file);
    }
    // W#, a git dependency: its checkout names its sources, and the tab of the MCP server of W#.
    sources(Path::new(ww::SOURCES), &mut files);
    #[cfg(feature = "mcp")]
    sources(Path::new(wwmcp::SOURCES), &mut files);
    let mut keys = std::collections::BTreeSet::new();
    for path in files {
        if path.file_name().is_some_and(|n| n == "translations.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        keys.extend(literals(&text));
    }
    assert!(keys.len() > 100, "{} texts found", keys.len());
    for (code, text) in LANGUAGES {
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

/// A dialog shows its answer beside Cancel, so no translation of an answer may read as Cancel:
/// the Ukrainian Discard once said Cancel too. The answers of `WinWeaver::ask`, of the confirmation of
/// an apply, of the editors and of the permissions.
#[test]
fn the_answers_of_a_dialog_do_not_read_as_cancel() {
    let answers = [
        "Remove",
        "Save",
        "Don't save",
        "Open",
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
    let be = std::fs::read_to_string(example_file()).unwrap();
    let mut files: Vec<(&str, &str)> = LANGUAGES.to_vec();
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
