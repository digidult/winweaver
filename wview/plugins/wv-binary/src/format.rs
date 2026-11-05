//! The description of a binary format (`.wwformat`, docs/plugins.md section 6.4): strict JSON,
//! read with limits like every input from outside and checked whole before a file is read by it,
//! then compiled with its expressions parsed.

use std::collections::{BTreeMap, HashMap};

use wwrt::serde::Deserialize;

use crate::expr::{self, Expr, Part};

/// The largest description, in bytes.
pub const LARGEST: usize = 1 << 20;
/// The most types of a description and fields of a type.
const TYPES: usize = 1000;
const FIELDS: usize = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(crate = "wwrt::serde", rename_all = "lowercase")]
pub enum Endian {
    #[default]
    Little,
    Big,
}

/// How a number shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(crate = "wwrt::serde", rename_all = "lowercase")]
pub enum Show {
    /// As a decimal, with its hexadecimal from 10 on.
    #[default]
    Number,
    Hex,
    /// Seconds since 1970 in UTC.
    Time,
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
struct Description {
    format: String,
    version: u32,
    name: String,
    extensions: Vec<String>,
    #[serde(default)]
    endian: Endian,
    #[serde(default)]
    magic: Vec<Magic>,
    root: String,
    #[serde(default)]
    spaces: BTreeMap<String, SpaceText>,
    types: BTreeMap<String, TypeText>,
    #[serde(default)]
    enums: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    flags: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
struct Magic {
    at: u64,
    bytes: String,
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
struct SpaceText {
    table: String,
    r#virtual: String,
    size: String,
    file: String,
    #[serde(default)]
    file_size: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
struct TypeText {
    fields: Vec<FieldText>,
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", untagged)]
enum Keys {
    One(String),
    Several(Vec<String>),
}

#[derive(Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
struct FieldText {
    name: String,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    switch: Option<String>,
    #[serde(default)]
    cases: Option<BTreeMap<String, String>>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    at: Option<String>,
    #[serde(default)]
    rva: Option<String>,
    #[serde(default)]
    count: Option<String>,
    #[serde(default)]
    terminated: bool,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    when: Option<String>,
    #[serde(default)]
    endian: Option<Endian>,
    #[serde(rename = "enum", default)]
    enumeration: Option<String>,
    #[serde(default)]
    flags: Option<String>,
    #[serde(default)]
    key: Option<Keys>,
    #[serde(default)]
    show: Show,
    #[serde(default)]
    lazy: bool,
    #[serde(default)]
    align: Option<u64>,
    #[serde(default)]
    length: Option<String>,
}

/// What a field holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Int {
        width: u8,
        signed: bool,
    },
    Float {
        width: u8,
    },
    /// A text of one byte a character (ASCII), or of two (UTF-16).
    Text {
        wide: bool,
    },
    /// Bytes shown as bytes alone.
    Raw,
    Struct(String),
}

/// Where a field is.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// After the field before it, as a part of its structure.
    Next,
    /// At an offset of the file.
    At(Expr),
    /// At an address of the space `rva`, mapped to the file by its table.
    Rva(Expr),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    /// The kind, or the cases of `switch` (a value and its kind) when it has none.
    pub kind: Option<Kind>,
    pub switch: Option<(Expr, Vec<(i128, Kind)>)>,
    /// A value computed of the fields before it, with no bytes of its own.
    pub value: Option<Expr>,
    pub place: Place,
    pub count: Option<Expr>,
    pub terminated: bool,
    pub size: Option<Expr>,
    pub when: Option<Expr>,
    pub endian: Endian,
    pub enumeration: Option<String>,
    pub flags: Option<String>,
    /// The paths whose value names an item of an array, the first one an item has.
    pub key: Vec<Vec<Part>>,
    pub show: Show,
    /// A structure read when its node opens, not with the file.
    pub lazy: bool,
    /// The field (or each item of an array) ends at a multiple of so many bytes of the file.
    pub align: u64,
    /// The bytes of an array: its items until they fill them.
    pub length: Option<Expr>,
}

pub struct Space {
    pub table: String,
    pub r#virtual: String,
    pub size: String,
    pub file: String,
    pub file_size: Option<String>,
}

/// A description checked and compiled.
pub struct Format {
    pub name: String,
    pub endian: Endian,
    pub extensions: Vec<String>,
    pub magic: Vec<(u64, Vec<u8>)>,
    pub root: String,
    pub rva: Option<Space>,
    pub types: HashMap<String, Vec<Field>>,
    pub enums: HashMap<String, Vec<(u64, String)>>,
    pub flags: HashMap<String, Vec<(u64, String)>>,
}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && text.len() <= 64
}

fn number(text: &str) -> Result<u64, String> {
    let parsed = match text.strip_prefix("0x").or(text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => text.parse(),
    };
    parsed.map_err(|_| format!("not a number: {text}"))
}

fn kind_of(name: &str, types: &BTreeMap<String, TypeText>) -> Result<Kind, String> {
    Ok(match name {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => Kind::Int {
            width: name[1..].parse::<u8>().unwrap_or(8) / 8,
            signed: name.starts_with('i'),
        },
        "f32" => Kind::Float { width: 4 },
        "f64" => Kind::Float { width: 8 },
        "text" => Kind::Text { wide: false },
        "utf16" => Kind::Text { wide: true },
        "raw" => Kind::Raw,
        other if types.contains_key(other) => Kind::Struct(other.to_owned()),
        other => return Err(format!("an unknown type {other}")),
    })
}

fn names(table: BTreeMap<String, String>, what: &str) -> Result<Vec<(u64, String)>, String> {
    let mut out = Vec::new();
    for (value, name) in table {
        out.push((number(&value).map_err(|e| format!("{what}: {e}"))?, name));
    }
    out.sort();
    Ok(out)
}

fn field(f: FieldText, d: &Description) -> Result<Field, String> {
    let at = |e: String| format!("the field {}: {e}", f.name);
    if !is_name(&f.name) {
        return Err(format!("the name of a field: {:?}", f.name));
    }
    let parse = |text: &Option<String>| text.as_deref().map(expr::parse).transpose().map_err(at);
    let kind = f
        .kind
        .as_deref()
        .map(|k| kind_of(k, &d.types))
        .transpose()
        .map_err(at)?;
    let switch = match (&f.switch, f.cases) {
        (Some(s), Some(cases)) => {
            let mut list = Vec::new();
            for (value, kind) in cases {
                let value = i128::from(number(&value).map_err(at)?);
                list.push((value, kind_of(&kind, &d.types).map_err(at)?));
            }
            Some((expr::parse(s).map_err(at)?, list))
        }
        (None, None) => None,
        _ => return Err(at("a switch needs its cases".to_owned())),
    };
    let value = parse(&f.value)?;
    let shapes = [kind.is_some(), switch.is_some(), value.is_some()];
    if shapes.iter().filter(|s| **s).count() != 1 {
        return Err(at("a field has a type, a switch or a value".to_owned()));
    }
    let place = match (parse(&f.at)?, parse(&f.rva)?) {
        (None, None) => Place::Next,
        (Some(e), None) => Place::At(e),
        (None, Some(e)) => {
            if !d.spaces.contains_key("rva") {
                return Err(at("an rva without the space rva".to_owned()));
            }
            Place::Rva(e)
        }
        _ => return Err(at("a field is at an offset or at an rva".to_owned())),
    };
    let size = parse(&f.size)?;
    let kinds: Vec<&Kind> = kind
        .iter()
        .chain(switch.iter().flat_map(|(_, c)| c.iter().map(|(_, k)| k)))
        .collect();
    if kinds.iter().any(|k| **k == Kind::Raw) && size.is_none() {
        return Err(at("raw bytes need a size".to_owned()));
    }
    for (list, what) in [(&f.enumeration, &d.enums), (&f.flags, &d.flags)] {
        if let Some(name) = list
            && !what.contains_key(name)
        {
            return Err(at(format!("no names {name}")));
        }
    }
    let key = match f.key {
        None => Vec::new(),
        Some(Keys::One(k)) => vec![k],
        Some(Keys::Several(list)) => list,
    };
    let mut paths = Vec::new();
    for k in key {
        match expr::parse(&k).map_err(at)? {
            Expr::Path { parts, .. } => paths.push(parts),
            _ => return Err(at(format!("a key is a path: {k}"))),
        }
    }
    let count = parse(&f.count)?;
    let when = parse(&f.when)?;
    let length = parse(&f.length)?;
    if length.is_some() && count.is_some() {
        return Err(at("an array has a count or a length".to_owned()));
    }
    let align = f.align.unwrap_or(1);
    if !(1..=4096).contains(&align) {
        return Err(at(format!("an alignment of {align}")));
    }
    if f.lazy {
        let structures = kinds.iter().all(|k| matches!(k, Kind::Struct(_)));
        if !structures || place == Place::Next || count.is_some() || f.terminated {
            return Err(at(
                "a lazy field is one structure at an offset or an rva".to_owned()
            ));
        }
    }
    Ok(Field {
        name: f.name,
        kind,
        switch,
        value,
        place,
        count,
        terminated: f.terminated,
        size,
        when,
        endian: f.endian.unwrap_or(d.endian),
        enumeration: f.enumeration,
        flags: f.flags,
        key: paths,
        show: f.show,
        lazy: f.lazy,
        align,
        length,
    })
}

impl Format {
    /// A description read and checked whole: a description that does not read is a message.
    pub fn parse(text: &str) -> Result<Format, String> {
        if text.len() > LARGEST {
            return Err(format!("a description larger than {LARGEST} bytes"));
        }
        let mut d: Description = wwrt::serde_json::from_str(text).map_err(|e| e.to_string())?;
        if d.format != "wwformat" || d.version != 1 {
            return Err(format!(
                "not a description of version 1 (\"format\": {:?}, \"version\": {})",
                d.format, d.version
            ));
        }
        if d.types.len() > TYPES {
            return Err(format!("more than {TYPES} types"));
        }
        if !d.types.contains_key(&d.root) {
            return Err(format!("no root type {}", d.root));
        }
        let mut magic = Vec::new();
        for m in &d.magic {
            let hex = m.bytes.replace(' ', "");
            if hex.is_empty() || hex.len() % 2 != 0 || hex.len() > 128 {
                return Err(format!("the magic bytes {:?}", m.bytes));
            }
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|_| format!("the magic bytes {:?}", m.bytes))?;
            magic.push((m.at, bytes));
        }
        let mut types = HashMap::new();
        let texts = std::mem::take(&mut d.types);
        let shapes: BTreeMap<String, TypeText> = texts
            .keys()
            .map(|k| (k.clone(), TypeText { fields: Vec::new() }))
            .collect();
        let check = Description {
            types: shapes,
            spaces: std::mem::take(&mut d.spaces),
            enums: std::mem::take(&mut d.enums),
            flags: std::mem::take(&mut d.flags),
            magic: Vec::new(),
            extensions: Vec::new(),
            format: String::new(),
            name: String::new(),
            root: String::new(),
            ..d
        };
        for (name, t) in texts {
            if !is_name(&name) {
                return Err(format!("the name of a type: {name:?}"));
            }
            if t.fields.len() > FIELDS {
                return Err(format!("the type {name}: more than {FIELDS} fields"));
            }
            let fields = t
                .fields
                .into_iter()
                .map(|f| field(f, &check))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("the type {name}: {e}"))?;
            types.insert(name, fields);
        }
        let Description {
            spaces,
            enums,
            flags,
            ..
        } = check;
        let rva = match spaces.into_iter().find(|(name, _)| name == "rva") {
            Some((_, s)) => Some(Space {
                table: s.table,
                r#virtual: s.r#virtual,
                size: s.size,
                file: s.file,
                file_size: s.file_size,
            }),
            None => None,
        };
        let names_of = |table: BTreeMap<String, BTreeMap<String, String>>| {
            table
                .into_iter()
                .map(|(k, v)| names(v, &k).map(|list| (k, list)))
                .collect::<Result<HashMap<_, _>, _>>()
        };
        let enums = names_of(enums)?;
        let flags = names_of(flags)?;
        Ok(Format {
            name: d.name,
            endian: d.endian,
            extensions: d
                .extensions
                .into_iter()
                .map(|e| e.trim_start_matches('.').to_lowercase())
                .collect(),
            magic,
            root: d.root,
            rva,
            types,
            enums,
            flags,
        })
    }

    /// Whether the first bytes of a file are the magic of the format.
    pub fn knows(&self, read: &mut dyn FnMut(u64, &mut [u8]) -> usize) -> bool {
        self.magic.iter().all(|(at, bytes)| {
            let mut found = vec![0u8; bytes.len()];
            read(*at, &mut found) == bytes.len() && found == *bytes
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = r#"{
        "format": "wwformat", "version": 1, "name": "Small", "extensions": [".sm"],
        "magic": [{ "at": 0, "bytes": "53 4D" }],
        "root": "file",
        "types": { "file": { "fields": [
            { "name": "magic", "type": "u16", "show": "hex" },
            { "name": "count", "type": "u8" },
            { "name": "items", "type": "u16", "count": "count", "enum": "kinds" }
        ] } },
        "enums": { "kinds": { "0x1": "One", "2": "Two" } }
    }"#;

    #[test]
    fn a_description_is_compiled_with_its_expressions() {
        let f = Format::parse(SMALL).unwrap();
        assert_eq!(f.extensions, vec!["sm"]);
        assert_eq!(f.magic, vec![(0, vec![0x53, 0x4D])]);
        let fields = &f.types["file"];
        assert_eq!(
            fields[2].kind,
            Some(Kind::Int {
                width: 2,
                signed: false
            })
        );
        assert!(fields[2].count.is_some());
        assert_eq!(
            f.enums["kinds"],
            vec![(1, "One".to_owned()), (2, "Two".to_owned())]
        );
        let mut bytes = |at: u64, out: &mut [u8]| {
            let data = b"SM\x02";
            let n = out.len().min(data.len().saturating_sub(at as usize));
            out[..n].copy_from_slice(&data[at as usize..at as usize + n]);
            n
        };
        assert!(f.knows(&mut bytes));
    }

    #[test]
    fn the_portable_executable_of_the_plugin_reads() {
        let f = Format::parse(crate::PE).unwrap();
        assert_eq!(f.name, "Portable Executable");
        assert!(f.extensions.iter().any(|e| e == "dll"));
        assert!(f.rva.is_some());
    }

    #[test]
    fn a_wrong_description_is_a_message() {
        let wrong = [
            (r#""version": 1"#, r#""version": 2"#),
            (r#""root": "file""#, r#""root": "none""#),
            (r#""type": "u8" }"#, r#""type": "u9" }"#),
            (r#""count": "count""#, r#""count": "count +""#),
            (r#""enum": "kinds""#, r#""enum": "sorts""#),
            (r#""show": "hex""#, r#""show": "hex", "colour": 1"#),
            (r#""bytes": "53 4D""#, r#""bytes": "5""#),
            (r#""type": "u8" }"#, r#""type": "raw" }"#),
            (r#""type": "u8" }"#, r#""type": "u8", "rva": "1" }"#),
            (r#""type": "u8" }"#, r#""type": "u8", "lazy": true }"#),
            (r#""count": "count""#, r#""count": "count", "length": "2""#),
            (r#""show": "hex""#, r#""show": "hex", "align": 0"#),
        ];
        for (from, to) in wrong {
            let text = SMALL.replacen(from, to, 1);
            assert_ne!(text, SMALL, "{from}");
            assert!(Format::parse(&text).is_err(), "{to}");
        }
    }
}
