//! The structure of a file saved without its bytes (`.wwbin`, docs/plugins.md section 6.4): every
//! node with its name, key, offset, size and value, the names of an enum or of flags, the SHA-256
//! of every range of raw bytes, the file with its size and SHA-256, the description with its name,
//! version and SHA-256. A comparison of a file with its `.wwbin` (phase 6) reads it back.
//!
//! ```json
//! { "format": "wwbin", "version": 1, "by": "wv-binary 0.5.0",
//!   "file": { "name": "kernel32.dll", "size": 853648, "sha256": "..." },
//!   "description": { "name": "Portable Executable", "version": 1, "sha256": "..." },
//!   "tree": { "name": "kernel32.dll", "offset": 0, "size": 853648, "type": "...",
//!             "children": [ { "name": "e_magic", "offset": 0, "size": 2, "value": 23117 } ] } }
//! ```
//!
//! A value is a number of JSON (the integers of 64 bits whole), a `text` or the `sha256` of raw
//! bytes; `names` the names of its enum or flags; `error` the reason a node stops, in the language
//! of the window that saved it.

use wwrt::serde_json::{Map, Value, json};
use wwrt::sha2::{Digest, Sha256};

use crate::engine::{Data, Node, Source, Tree};
use crate::format::Show;

/// The bytes of a range read at once to hash them.
const CHUNK: usize = 1 << 20;

/// The SHA-256 of a range of the file as hexadecimal; the bytes past its end are left out.
pub fn sha256(source: &mut dyn Source, offset: u64, size: u64) -> String {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut done = 0u64;
    while done < size {
        let want = (size - done).min(CHUNK as u64) as usize;
        let n = source.read(offset + done, &mut buffer[..want]);
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        done += n as u64;
    }
    hex(&hasher.finalize())
}

/// Bytes as hexadecimal in small letters.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn number(v: i128) -> Value {
    if let Ok(u) = u64::try_from(v) {
        json!(u)
    } else if let Ok(i) = i64::try_from(v) {
        json!(i)
    } else {
        json!(v.to_string())
    }
}

fn node(tree: &Tree, index: usize, source: &mut dyn Source) -> Value {
    let n = &tree.nodes[index];
    let mut out = Map::new();
    out.insert("name".to_owned(), json!(n.name));
    if let Some(k) = &n.key {
        out.insert("key".to_owned(), json!(k));
    }
    out.insert("offset".to_owned(), json!(n.offset));
    out.insert("size".to_owned(), json!(n.size));
    match &n.data {
        Data::Struct(t) => {
            out.insert("type".to_owned(), json!(t));
        }
        Data::Array(count) => {
            out.insert("items".to_owned(), json!(count));
        }
        Data::Int(v) => {
            out.insert("value".to_owned(), number(*v));
        }
        Data::Float(f) => {
            out.insert("value".to_owned(), json!(f));
        }
        Data::Text(t) => {
            out.insert("text".to_owned(), json!(t));
        }
        Data::Raw => {
            out.insert("sha256".to_owned(), json!(sha256(source, n.offset, n.size)));
        }
        Data::Missing => {}
    }
    if n.error {
        out.insert("error".to_owned(), json!(n.note));
    } else if !n.note.is_empty() {
        out.insert("names".to_owned(), json!(n.note));
    }
    if !n.children.is_empty() {
        let children: Vec<Value> = n.children.iter().map(|c| node(tree, *c, source)).collect();
        out.insert("children".to_owned(), Value::Array(children));
    }
    Value::Object(out)
}

/// The document of a tree: the file of `name` read by the description `format` (with the SHA-256
/// of its text).
pub fn document(
    tree: &Tree,
    source: &mut dyn Source,
    name: &str,
    format: &str,
    description: &str,
) -> Value {
    let size = source.len();
    json!({
        "format": "wwbin",
        "version": 1,
        "by": concat!("wv-binary ", env!("CARGO_PKG_VERSION")),
        "file": { "name": name, "size": size, "sha256": sha256(source, 0, size) },
        "description": { "name": format, "version": 1, "sha256": description },
        "tree": node(tree, 0, source),
    })
}

/// The largest `.wwbin` read, in bytes.
pub const LARGEST: u64 = 512 << 20;
/// The nodes of a `.wwbin` read at most, and how deep.
const MOST_NODES: usize = 2_000_000;
const DEEPEST: usize = 200;

/// A structure read back from its `.wwbin`.
pub struct Saved {
    /// The name, the size and the SHA-256 of the file it was saved of.
    pub name: String,
    pub size: u64,
    pub sha256: String,
    /// The name of its description.
    pub format: String,
    /// The tree: a raw range has its SHA-256 as its note.
    pub tree: Tree,
}

fn text<'a>(v: &'a Value, name: &str) -> Result<&'a str, String> {
    v.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no text {name}"))
}

fn whole(v: &Value, name: &str) -> Result<u64, String> {
    v.get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("no number {name}"))
}

/// The nodes of a node of a document, depth first, into `nodes`.
fn read_node(
    v: &Value,
    parent: Option<usize>,
    depth: usize,
    nodes: &mut Vec<Node>,
) -> Result<usize, String> {
    if depth > DEEPEST || nodes.len() >= MOST_NODES {
        return Err("a structure deeper or larger than a .wwbin may be".to_owned());
    }
    let data = if let Some(t) = v.get("type").and_then(Value::as_str) {
        Data::Struct(t.to_owned())
    } else if let Some(n) = v.get("items").and_then(Value::as_u64) {
        Data::Array(n)
    } else if let Some(n) = v.get("value") {
        match (n.as_u64(), n.as_i64(), n.as_f64(), n.as_str()) {
            (Some(u), _, _, _) => Data::Int(i128::from(u)),
            (_, Some(i), _, _) => Data::Int(i128::from(i)),
            (_, _, _, Some(s)) => Data::Int(s.parse().map_err(|_| format!("a value {s}"))?),
            (_, _, Some(f), _) => Data::Float(f),
            _ => return Err("a value of no kind".to_owned()),
        }
    } else if let Some(t) = v.get("text").and_then(Value::as_str) {
        Data::Text(t.to_owned())
    } else if v.get("sha256").is_some() {
        Data::Raw
    } else {
        Data::Missing
    };
    let error = v.get("error").and_then(Value::as_str);
    let note = match (&data, error) {
        (_, Some(e)) => e.to_owned(),
        (Data::Raw, None) => text(v, "sha256")?.to_owned(),
        _ => v
            .get("names")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    };
    let index = nodes.len();
    nodes.push(Node {
        name: text(v, "name")?.to_owned(),
        key: v.get("key").and_then(Value::as_str).map(str::to_owned),
        offset: whole(v, "offset")?,
        size: whole(v, "size")?,
        data,
        parent,
        children: Vec::new(),
        note,
        error: error.is_some(),
        show: Show::Number,
        pending: false,
        content: None,
    });
    if let Some(children) = v.get("children").and_then(Value::as_array) {
        for c in children {
            let child = read_node(c, Some(index), depth + 1, nodes)?;
            nodes[index].children.push(child);
        }
    }
    Ok(index)
}

/// A `.wwbin` read back: a file that is not one, or larger than `LARGEST`, is a message.
pub fn read(path: &std::path::Path) -> Result<Saved, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > LARGEST {
        return Err(format!("larger than {LARGEST} bytes"));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let doc: Value = wwrt::serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if text(&doc, "format")? != "wwbin" || whole(&doc, "version")? != 1 {
        return Err("not a .wwbin of version 1".to_owned());
    }
    let file = doc.get("file").ok_or("no file")?;
    let mut nodes = Vec::new();
    read_node(doc.get("tree").ok_or("no tree")?, None, 0, &mut nodes)?;
    let errors = nodes.iter().filter(|n| n.error).count();
    Ok(Saved {
        name: text(file, "name")?.to_owned(),
        size: whole(file, "size")?,
        sha256: text(file, "sha256")?.to_owned(),
        format: text(doc.get("description").ok_or("no description")?, "name")?.to_owned(),
        tree: Tree { nodes, errors },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Limits, read};
    use crate::format::Format;

    #[test]
    fn a_structure_is_saved_without_its_bytes() {
        let description = r#"{ "format": "wwformat", "version": 1, "name": "Pair",
            "extensions": [], "root": "file", "types": { "file": { "fields": [
                { "name": "kind", "type": "u8", "enum": "kinds" },
                { "name": "name", "type": "text", "size": "3" },
                { "name": "body", "type": "raw", "size": "4" },
                { "name": "big", "type": "u64" },
                { "name": "lost", "type": "u32" } ] } },
            "enums": { "kinds": { "1": "One" } } }"#;
        let format = Format::parse(description).unwrap();
        let mut bytes = vec![1u8, b'a', b'b', b'c', 9, 8, 7, 6];
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        let tree = read(&format, &mut bytes, Limits::default());
        let doc = document(&tree, &mut bytes, "pair.bin", "Pair", "00");
        assert_eq!(doc["format"], "wwbin");
        assert_eq!(doc["file"]["size"], 16);
        // The SHA-256 of "abc" is the vector of FIPS 180-2.
        let mut abc = b"abc".to_vec();
        assert_eq!(
            sha256(&mut abc, 0, 3),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let children = doc["tree"]["children"].as_array().unwrap();
        assert_eq!(children[0]["value"], 1);
        assert_eq!(children[0]["names"], "One");
        assert_eq!(children[1]["text"], "abc");
        assert_eq!(children[2]["sha256"], sha256(&mut bytes.clone(), 4, 4));
        assert!(
            children[2].get("value").is_none(),
            "no bytes in the document"
        );
        assert_eq!(children[3]["value"], u64::MAX, "64 bits whole");
        assert!(children[4]["error"].as_str().is_some_and(|e| !e.is_empty()));
        // The document reads back into the same tree.
        let dir = std::env::temp_dir().join(format!("wv-binary-wwbin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pair.bin.wwbin");
        std::fs::write(&path, wwrt::serde_json::to_string(&doc).unwrap()).unwrap();
        let saved = super::read(&path).unwrap();
        assert_eq!(
            (saved.name.as_str(), saved.size, saved.format.as_str()),
            ("pair.bin", 16, "Pair")
        );
        assert_eq!(saved.tree.nodes.len(), tree.nodes.len());
        assert_eq!(saved.tree.nodes[4].data, Data::Int(i128::from(u64::MAX)));
        assert_eq!(
            saved.tree.nodes[3].note,
            sha256(&mut bytes.clone(), 4, 4),
            "raw bytes keep their hash"
        );
        assert!(saved.tree.nodes[5].error);
        std::fs::write(&path, b"{\"format\": \"other\"}").unwrap();
        assert!(super::read(&path).is_err());
    }
}
