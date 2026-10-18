//! Registry files (`.reg`) as regedit writes and reads them: the header, the keys to make or to
//! delete, the values of every type and their deletions. Comment and blank lines keep their places
//! and a line the program does not read is kept as it is, so a file opened and saved again loses
//! no line. A file is also made from an apply or return plan (`apply::Plan`): its registry steps
//! become keys and values, the steps a registry file cannot hold become comments.

use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::apply::{self, Op, Plan};
use crate::catalog::RegKind;
use crate::jsonfile;
use crate::registry::{self, Data};

pub const EXTENSION: &str = "reg";
pub const HEADER: &str = "Windows Registry Editor Version 5.00";
pub const HEADER_4: &str = "REGEDIT4";
/// The largest file the program reads.
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;
/// The width at which a long hexadecimal value goes on in the next line, as regedit writes it.
const WIDTH: usize = 76;

/// One line of a registry file (a value of several lines joined by `\` is one).
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Blank,
    /// A comment with its `;`.
    Comment(String),
    /// A key to make (`[path]`) or to delete with everything under it (`[-path]`).
    Key {
        path: String,
        delete: bool,
    },
    /// A value of the key above it; `None` deletes it (`"name"=-`). The name is empty for the
    /// default value (`@`).
    Value {
        name: String,
        data: Option<Data>,
    },
    /// A line the program does not read, kept as it is.
    Other(String),
}

impl Line {
    /// A line of a file from its text, as the file is read.
    pub fn parse(text: &str) -> Self {
        line_of(text)
    }
}

/// A registry file: its header and its lines.
#[derive(Debug, Clone, PartialEq)]
pub struct RegFile {
    pub header: String,
    pub lines: Vec<Line>,
}

impl Default for RegFile {
    fn default() -> Self {
        Self {
            header: HEADER.to_owned(),
            lines: Vec::new(),
        }
    }
}

/// A quoted text of a registry file from its start: the text and the rest after the closing quote.
fn quoted(text: &str) -> Option<(String, &str)> {
    let rest = text.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?.1),
            '"' => return Some((out, &rest[i + 1..])),
            c => out.push(c),
        }
    }
    None
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    text.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| u8::from_str_radix(p, 16).ok())
        .collect()
}

/// The data after the `=` of a value line; `Some(None)` for a deletion.
fn data_of_text(text: &str) -> Option<Option<Data>> {
    let text = text.trim();
    if text == "-" {
        return Some(None);
    }
    if text.starts_with('"') {
        let (s, rest) = quoted(text)?;
        return rest.trim().is_empty().then_some(Some(Data::Sz(s)));
    }
    let lower = text.to_ascii_lowercase();
    if let Some(n) = lower.strip_prefix("dword:") {
        return u32::from_str_radix(n.trim(), 16)
            .ok()
            .map(|n| Some(Data::Dword(n)));
    }
    if let Some(bytes) = lower.strip_prefix("hex:") {
        return hex_bytes(bytes).map(|b| Some(Data::Binary(b)));
    }
    let rest = lower.strip_prefix("hex(")?;
    let (kind, bytes) = rest.split_once("):")?;
    let kind = u32::from_str_radix(kind, 16).ok()?;
    hex_bytes(bytes).map(|b| Some(Data::from_raw(kind, &b)))
}

fn value_of_line(line: &str) -> Option<Line> {
    let (name, rest) = match line.strip_prefix('@') {
        Some(rest) => (String::new(), rest),
        None => quoted(line)?,
    };
    let data = rest.trim_start().strip_prefix('=')?;
    Some(Line::Value {
        name,
        data: data_of_text(data)?,
    })
}

fn line_of(text: &str) -> Line {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Line::Blank;
    }
    if trimmed.starts_with(';') {
        return Line::Comment(text.trim_end().to_owned());
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        return match inner.strip_prefix('-') {
            Some(path) => Line::Key {
                path: path.to_owned(),
                delete: true,
            },
            None => Line::Key {
                path: inner.to_owned(),
                delete: false,
            },
        };
    }
    if (trimmed.starts_with('"') || trimmed.starts_with('@'))
        && let Some(value) = value_of_line(trimmed)
    {
        return value;
    }
    Line::Other(text.trim_end().to_owned())
}

/// `hex(k):aa,bb,...` of a value, in lines of at most `WIDTH` characters after the name.
fn hex_text(prefix: &str, kind: &str, bytes: &[u8]) -> String {
    let mut out = format!("{prefix}{kind}");
    let mut width = out.len();
    for (i, b) in bytes.iter().enumerate() {
        let part = if i + 1 < bytes.len() {
            format!("{b:02x},")
        } else {
            format!("{b:02x}")
        };
        if width + part.len() > WIDTH && i + 1 < bytes.len() {
            out.push_str("\\\r\n  ");
            width = 2;
        }
        width += part.len();
        out.push_str(&part);
    }
    out
}

/// The text of a value line, as regedit writes it.
pub fn value_text(name: &str, data: Option<&Data>) -> String {
    let name = if name.is_empty() {
        "@".to_owned()
    } else {
        format!("\"{}\"", escape(name))
    };
    let prefix = format!("{name}=");
    match data {
        None => format!("{prefix}-"),
        // A text of several lines has no quoted form: regedit writes it as hex(1).
        Some(Data::Sz(s)) if !s.contains(['\r', '\n']) => format!("{prefix}\"{}\"", escape(s)),
        Some(Data::Dword(n)) => format!("{prefix}dword:{n:08x}"),
        Some(Data::Binary(b)) => hex_text(&prefix, "hex:", b),
        Some(other) => {
            let (kind, bytes) = other.to_raw();
            hex_text(&prefix, &format!("hex({kind:x}):"), &bytes)
        }
    }
}

impl RegFile {
    /// Reads the text of a registry file; it starts with the header of regedit.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut raw = text.lines();
        let header = loop {
            match raw.next() {
                Some(l) if l.trim().is_empty() => continue,
                Some(l) => break l.trim().to_owned(),
                None => return Err("an empty file".to_owned()),
            }
        };
        if header != HEADER && header != HEADER_4 {
            return Err(
                "not a registry file: the first line is not the header of regedit".to_owned(),
            );
        }
        let mut lines = Vec::new();
        let mut pending: Option<String> = None;
        for l in raw {
            let joined = match pending.take() {
                Some(start) => format!("{start}{}", l.trim_start()),
                None => l.to_owned(),
            };
            let end = joined.trim_end();
            if end.ends_with('\\') && !end.trim_start().starts_with(';') && end.contains('=') {
                pending = Some(end[..end.len() - 1].to_owned());
                continue;
            }
            lines.push(line_of(&joined));
        }
        if let Some(rest) = pending {
            lines.push(line_of(&rest));
        }
        Ok(Self { header, lines })
    }

    /// Reads a file in UTF-16 with its mark (as regedit writes it) or in UTF-8.
    pub fn load(path: &Path) -> Result<Self, String> {
        let name = jsonfile::display_name(path);
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes))
            .map_err(|e| format!("{name}: {e}"))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(format!(
                "{name}: larger than a registry file the program reads"
            ));
        }
        let text = jsonfile::decode_bom(&bytes)
            .ok_or_else(|| format!("{name}: neither UTF-8 nor UTF-16 with its mark"))?;
        Self::parse(&text).map_err(|e| format!("{name}: {e}"))
    }

    /// The text of the file, lines ended by CR LF as regedit ends them.
    pub fn to_text(&self) -> String {
        let mut out = format!("{}\r\n", self.header);
        for line in &self.lines {
            match line {
                Line::Blank => {}
                Line::Comment(t) | Line::Other(t) => out.push_str(t),
                Line::Key { path, delete } => {
                    out.push('[');
                    if *delete {
                        out.push('-');
                    }
                    out.push_str(path);
                    out.push(']');
                }
                Line::Value { name, data } => out.push_str(&value_text(name, data.as_ref())),
            }
            out.push_str("\r\n");
        }
        out
    }

    /// Writes the file: UTF-16 LE with its mark for the header of version 5, as regedit does.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = self.to_text();
        let bytes: Vec<u8> = if self.header == HEADER {
            let mut b = vec![0xFF, 0xFE];
            b.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
            b
        } else {
            text.into_bytes()
        };
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    /// The registry steps of a plan as a registry file; the steps a registry file cannot hold
    /// and the skipped ones as comments. The default user profile is the hive the apply script
    /// loads at `HKEY_USERS\UnattendDefault`, which the comment at the top tells how to load.
    pub fn from_plan(plan: &Plan, title: &str) -> Self {
        let mut file = Self::default();
        file.note(title);
        file.note(&format!("Made by WinWeaver {}", crate::BUILD));
        if plan.uses_default_user() {
            for text in DEFAULT_USER {
                file.note(text);
            }
        }
        let mut key: Option<String> = None;
        let mut rule = String::new();
        for step in &plan.steps {
            if step.rule != rule {
                rule = step.rule.clone();
                file.lines.push(Line::Blank);
                file.note(&format!("Rule {rule}"));
                key = None;
            }
            let value = |name: &str, data: Option<Data>| Line::Value {
                name: name.to_owned(),
                data,
            };
            match &step.op {
                Op::SetValue {
                    path,
                    name,
                    kind,
                    data,
                } => {
                    if file.open(&mut key, path) {
                        file.lines.push(value(name, Some(data_of(*kind, data))));
                    }
                }
                Op::DeleteValue { path, name } => {
                    if file.open(&mut key, path) {
                        file.lines.push(value(name, None));
                    }
                }
                Op::SetList {
                    path,
                    kind,
                    names,
                    values,
                    additive,
                } => {
                    // Without `additive` the other values of the key go first.
                    let clear = registry::normalize(path).filter(|_| !additive);
                    if let Some(full) = clear {
                        file.note("The list replaces the values of its key: the key is made anew");
                        file.lines.push(Line::Key {
                            path: full,
                            delete: true,
                        });
                        key = None;
                    }
                    if file.open(&mut key, path) {
                        for (n, v) in names.iter().zip(values) {
                            let data = data_of(*kind, &Value::from(v.as_str()));
                            file.lines.push(value(n, Some(data)));
                        }
                    }
                }
                Op::ServiceStart { name, start } => {
                    if file.open(&mut key, &apply::service_key(name)) {
                        file.lines.push(value("Start", Some(Data::Dword(*start))));
                    }
                }
                Op::Run { file: exe, args } => {
                    file.note(&format!("Not in a registry file: {exe} {}", args.join(" ")));
                    key = None;
                }
                Op::RemoveCapability { pattern } => {
                    file.note(&format!(
                        "Not in a registry file: removing the capabilities {pattern}"
                    ));
                    key = None;
                }
                Op::RemoveApps { names } => {
                    file.note(&format!(
                        "Not in a registry file: removing the apps {}",
                        names.join(", ")
                    ));
                    key = None;
                }
            }
        }
        if !plan.skipped.is_empty() {
            file.lines.push(Line::Blank);
            file.note("Left to the stand-alone scripts:");
            for (rule, why) in &plan.skipped {
                file.note(&format!("  {rule}: {why}"));
            }
        }
        file.lines.push(Line::Blank);
        file
    }

    /// Adds a comment line; a line break of the text would end the comment, so it is a space.
    fn note(&mut self, text: &str) {
        let text = text.replace(['\r', '\n'], " ");
        self.lines.push(Line::Comment(format!("; {text}")));
    }

    /// Opens the key of a step, unless it is the key open already; false, with a comment, for a
    /// path that is not one of the registry.
    fn open(&mut self, key: &mut Option<String>, path: &str) -> bool {
        let Some(path) = registry::normalize(path) else {
            self.note(&format!("Not a key of the registry: {path}"));
            return false;
        };
        if key.as_deref() != Some(path.as_str()) {
            self.lines.push(Line::Key {
                path: path.clone(),
                delete: false,
            });
            *key = Some(path);
        }
        true
    }
}

/// How to bring the default user profile into a registry file of a plan.
const DEFAULT_USER: [&str; 3] = [
    "The values of HKEY_USERS\\UnattendDefault go to the default user profile: load it first with",
    "  reg load HKU\\UnattendDefault C:\\Users\\Default\\NTUSER.DAT",
    "and unload it after the import with  reg unload HKU\\UnattendDefault",
];

/// A number of the data of a plan.
fn number(data: &Value) -> u64 {
    match data {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_i64().map(|i| i as u64))
            .unwrap_or(0),
        Value::String(s) => s.trim().parse().unwrap_or(0),
        Value::Bool(b) => u64::from(*b),
        _ => 0,
    }
}

fn text(data: &Value) -> String {
    match data {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The data of a value of a plan in the types of regedit.
fn data_of(kind: RegKind, data: &Value) -> Data {
    match kind {
        RegKind::DWord => Data::Dword(number(data) as u32),
        RegKind::QWord => Data::Qword(number(data)),
        RegKind::String => Data::Sz(text(data)),
        RegKind::ExpandString => Data::ExpandSz(text(data)),
        RegKind::MultiString => Data::MultiSz(match data {
            Value::Array(items) => items.iter().map(text).collect(),
            Value::String(s) if s.is_empty() => Vec::new(),
            other => vec![text(other)],
        }),
        RegKind::Binary => Data::Binary(
            data.as_array()
                .map(|items| items.iter().map(|v| number(v) as u8).collect())
                .unwrap_or_default(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as regedit writes it, with a comment, a line the program does not read, a value
    /// that goes on in the next line and the deletions.
    const SAMPLE: [&str; 13] = [
        "Windows Registry Editor Version 5.00",
        "",
        "; a comment that stays",
        "[HKEY_CURRENT_USER\\Software\\WinWeaver]",
        "@=\"default \\\"quoted\\\" \\\\ text\"",
        "\"Number\"=dword:0000002a",
        "\"Bytes\"=hex:01,02,\\",
        "  ff",
        "\"Lines\"=hex(7):61,00,00,00,62,00,00,00,00,00",
        "\"Gone\"=-",
        "something else",
        "",
        "[-HKEY_CURRENT_USER\\Software\\Old]",
    ];

    #[test]
    fn a_file_is_read_with_its_comments_and_written_back() {
        let file = RegFile::parse(&(SAMPLE.join("\r\n") + "\r\n")).unwrap();
        assert_eq!(file.header, HEADER);
        assert_eq!(file.lines[0], Line::Blank);
        assert_eq!(
            file.lines[1],
            Line::Comment("; a comment that stays".to_owned())
        );
        assert_eq!(
            file.lines[3],
            Line::Value {
                name: String::new(),
                data: Some(Data::Sz("default \"quoted\" \\ text".to_owned())),
            }
        );
        assert_eq!(
            file.lines[4],
            Line::Value {
                name: "Number".to_owned(),
                data: Some(Data::Dword(42)),
            }
        );
        assert_eq!(
            file.lines[5],
            Line::Value {
                name: "Bytes".to_owned(),
                data: Some(Data::Binary(vec![1, 2, 255])),
            }
        );
        assert_eq!(
            file.lines[6],
            Line::Value {
                name: "Lines".to_owned(),
                data: Some(Data::MultiSz(vec!["a".to_owned(), "b".to_owned()])),
            }
        );
        assert!(matches!(&file.lines[7], Line::Value { data: None, .. }));
        assert_eq!(file.lines[8], Line::Other("something else".to_owned()));
        assert!(matches!(&file.lines[10], Line::Key { delete: true, .. }));
        // Written and read again, nothing is lost.
        let again = RegFile::parse(&file.to_text()).unwrap();
        assert_eq!(again, file);
        assert!(RegFile::parse("[HKEY_CURRENT_USER]\r\n").is_err());
        // A text of two lines goes as hex(1) and comes back.
        let two = Data::Sz("a\nb".to_owned());
        let line = value_text("Two", Some(&two));
        assert!(line.starts_with("\"Two\"=hex(1):"), "{line}");
        let text = format!("{HEADER}\r\n[HKEY_CURRENT_USER\\X]\r\n{line}\r\n");
        assert_eq!(
            RegFile::parse(&text).unwrap().lines[1],
            Line::Value {
                name: "Two".to_owned(),
                data: Some(two),
            }
        );
    }

    #[test]
    fn a_plan_becomes_keys_values_and_comments() {
        let catalog = crate::catalog::Catalog::embedded().unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("../../data/presets/office.wwprofile");
        let profile = crate::profile::Profile::load(&path).unwrap();
        let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
        let plan = apply::plan_apply(&catalog, &profile, &ids, "n");
        let file = RegFile::from_plan(&plan, "Apply Office");
        let text = file.to_text();
        assert!(text.starts_with(HEADER));
        // The machine, the default user profile, a service, and the app removals as comments.
        assert!(text.contains("[HKEY_LOCAL_MACHINE\\"));
        assert!(text.contains("[HKEY_USERS\\UnattendDefault\\"));
        assert!(text.contains("\"Start\"=dword:"));
        let apps = "; Not in a registry file: removing the apps";
        assert!(text.contains(apps), "{text}");
        assert_eq!(RegFile::parse(&text).unwrap(), file);
    }
}
