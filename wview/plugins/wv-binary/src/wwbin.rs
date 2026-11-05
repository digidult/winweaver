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

use sha2::{Digest, Sha256};
use wwrt::serde_json::{Map, Value, json};

use crate::engine::{Data, Source, Tree};

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
    }
}
