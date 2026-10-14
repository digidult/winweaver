//! Strict reading of JSON files: a size limit, no byte order mark, a nesting limit.
//!
//! Every file the program reads (profiles, diffs, catalog files, templates, settings) goes through
//! this module, so a defect becomes a short message and never a crash (principle P5).

use std::path::Path;

use serde::de::DeserializeOwned;

/// The default size limit of a document file.
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// The deepest nesting of objects and arrays a file may have.
pub const MAX_DEPTH: usize = 32;

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// What went wrong while reading a JSON file. `name` is the file name only, never the full path.
#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("{name}: {source}")]
    Io {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{name}: larger than {limit} bytes")]
    TooLarge { name: String, limit: u64 },
    #[error("{name}: a byte order mark is not allowed")]
    Bom { name: String },
    #[error("{name}: not UTF-8")]
    Utf8 { name: String },
    #[error("{name}: nesting deeper than {limit} levels")]
    TooDeep { name: String, limit: usize },
    #[error("{name}: {source}")]
    Syntax {
        name: String,
        #[source]
        source: serde_json::Error,
    },
}

/// The file name of a path for messages.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Reads a text file of at most `limit` bytes, UTF-8 without a byte order mark.
pub fn read_text(path: &Path, limit: u64) -> Result<String, JsonError> {
    let name = display_name(path);
    let meta = std::fs::metadata(path).map_err(|source| JsonError::Io {
        name: name.clone(),
        source,
    })?;
    if meta.len() > limit {
        return Err(JsonError::TooLarge { name, limit });
    }
    let bytes = std::fs::read(path).map_err(|source| JsonError::Io {
        name: name.clone(),
        source,
    })?;
    if bytes.len() as u64 > limit {
        return Err(JsonError::TooLarge { name, limit });
    }
    if bytes.starts_with(&BOM) {
        return Err(JsonError::Bom { name });
    }
    String::from_utf8(bytes).map_err(|_| JsonError::Utf8 { name })
}

/// Reads a text file of at most `limit` bytes written by another program: a UTF-8 byte order
/// mark (Windows PowerShell writes one) is dropped.
pub fn read_text_bom(path: &Path, limit: u64) -> Result<String, JsonError> {
    let name = display_name(path);
    let meta = std::fs::metadata(path).map_err(|source| JsonError::Io {
        name: name.clone(),
        source,
    })?;
    if meta.len() > limit {
        return Err(JsonError::TooLarge { name, limit });
    }
    let bytes = std::fs::read(path).map_err(|source| JsonError::Io {
        name: name.clone(),
        source,
    })?;
    let bytes = bytes.strip_prefix(&BOM[..]).unwrap_or(&bytes).to_vec();
    String::from_utf8(bytes).map_err(|_| JsonError::Utf8 { name })
}

/// Parses JSON text into `T` after the checks of this module.
pub fn parse<T: DeserializeOwned>(text: &str, name: &str) -> Result<T, JsonError> {
    if text.as_bytes().starts_with(&BOM) {
        return Err(JsonError::Bom {
            name: name.to_owned(),
        });
    }
    let depth = nesting_depth(text);
    if depth > MAX_DEPTH {
        return Err(JsonError::TooDeep {
            name: name.to_owned(),
            limit: MAX_DEPTH,
        });
    }
    serde_json::from_str(text).map_err(|source| JsonError::Syntax {
        name: name.to_owned(),
        source,
    })
}

/// Reads and parses a JSON file of at most `limit` bytes.
pub fn read<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T, JsonError> {
    let text = read_text(path, limit)?;
    parse(&text, &display_name(path))
}

/// The deepest nesting of objects and arrays in JSON text, counted outside strings.
pub fn nesting_depth(text: &str) -> usize {
    let mut depth = 0usize;
    let mut deepest = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' | '[' => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_ignores_brackets_in_strings() {
        assert_eq!(nesting_depth(r#"{"a": "[[[", "b": [1, [2]]}"#), 3);
        assert_eq!(nesting_depth(r#"{"a": "\"[", "b": 1}"#), 1);
        assert_eq!(nesting_depth("12"), 0);
    }

    #[test]
    fn too_deep_is_refused() {
        let text = format!(
            "{}1{}",
            "[".repeat(MAX_DEPTH + 1),
            "]".repeat(MAX_DEPTH + 1)
        );
        let err = parse::<serde_json::Value>(&text, "deep.json").unwrap_err();
        assert!(matches!(err, JsonError::TooDeep { .. }), "{err}");
    }

    #[test]
    fn bom_is_refused() {
        let err = parse::<serde_json::Value>("\u{feff}{}", "bom.json").unwrap_err();
        assert!(matches!(err, JsonError::Bom { .. }), "{err}");
    }
}
