//! Policy templates (`docs/spec/03-features.md` section 6): ADMX files with their ADML texts read
//! into rules, kept as catalog files (`.wwcatalog`) and merged into the catalog.
//!
//! Every template is untrusted input: no DTD, size and depth limits, safe characters in keys,
//! value names and values (they reach the registry, the scripts and a CDATA section of the answer
//! file). A policy the program cannot represent is skipped and listed, never fatal. A catalog
//! file holds rules of every kind the built-in catalog has (one made in the editor too) and is
//! checked again when it is loaded, with the checks of the built-in catalog and its own; one
//! whose rules run code is imported only after a question.
//!
//! The folder: `xml` (a small tree of XML elements), `templates` (the model of the templates and
//! their reading from ADMX and ADML), `rules` (the conversion into rules), `file` (the catalog
//! file), `compare`, `archive` (zip archives of templates) and `merge` (into the catalog).

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

use crate::catalog::check;
use crate::jsonfile;

mod archive;
mod compare;
mod file;
mod merge;
mod rules;
mod templates;
#[cfg(test)]
mod tests;
mod xml;

pub use archive::{ARCHIVE_EXTENSION, Archive, read_archive, read_archive_bytes};
pub use compare::{Comparison, PolicyDifference, compare};
pub use file::{CatalogFile, CopyWhat, LangTexts};
pub use merge::{merge, merge_texts};
pub use rules::to_catalog_file;
pub use templates::{Templates, Texts, Val, read_file, read_folder};

pub const CATALOG_FORMAT: &str = "winweaver-catalog";
pub const CATALOG_VERSION: u32 = 2;
pub const CATALOG_EXTENSION: &str = "wwcatalog";
/// The prefix of the ids of the rules and groups an import brings.
pub const RULE_PREFIX: &str = "admx.";
pub(super) const FILE_MAX: u64 = 8 * 1024 * 1024;
pub(super) const CATALOG_MAX: u64 = 64 * 1024 * 1024;
pub(super) const DEPTH_MAX: usize = 64;
pub(super) const SUMMARY_MAX: usize = 4000;
pub(super) const NAME_MAX: usize = 255;
/// The cultures whose texts an import keeps, with the language code of the catalog texts.
pub const CULTURES: [(&str, &str); 9] = [
    ("en-US", "en"),
    ("uk-UA", "uk"),
    ("ru-RU", "ru"),
    ("de-DE", "de"),
    ("es-ES", "es"),
    ("fr-FR", "fr"),
    ("id-ID", "id"),
    ("it-IT", "it"),
    ("tr-TR", "tr"),
];

/// A key of the registry in the form a catalog file writes it: `HKLM:\...` for the machine and
/// `DU:\...` for the default user profile (the current user of regedit), from any form of
/// `registry::normalize`. `None` for another root.
pub fn catalog_path(path: &str) -> Option<String> {
    let full = crate::registry::normalize(path)?;
    let (hive, rest) = full.split_once('\\')?;
    match hive {
        "HKEY_LOCAL_MACHINE" => Some(format!("HKLM:\\{rest}")),
        "HKEY_CURRENT_USER" => Some(format!("DU:\\{rest}")),
        _ => None,
    }
}

/// What the list of the imports shows of a catalog file that is not loaded, read from the first
/// bytes of the file only, so that its rules are never read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CatalogHeader {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub windows: String,
    #[serde(default)]
    pub created: String,
}

/// How much of the start of a catalog file its header may take.
const HEADER_BYTES: u64 = 16 * 1024;

/// The first `limit` bytes of a catalog file. A disk gives them without the rest of the file;
/// in a browser the files are the ones of the file layer (the storage of the page, the folder
/// of the page), which reads a file whole.
fn first_bytes(path: &Path, limit: u64) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Read;
        let mut bytes = Vec::new();
        let file = std::fs::File::open(path).ok()?;
        file.take(limit).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut bytes = ww_base::files::read(path, CATALOG_MAX).ok()?;
        bytes.truncate(limit as usize);
        Some(bytes)
    }
}

/// The size of a catalog file in bytes for the list of the imports, 0 when it cannot be read.
pub fn file_size(path: &Path) -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    let size = std::fs::metadata(path).map(|m| m.len());
    #[cfg(target_arch = "wasm32")]
    let size = ww_base::files::read(path, CATALOG_MAX).map(|b| b.len() as u64);
    size.unwrap_or(0)
}

impl CatalogHeader {
    /// The header of a catalog file as WinWeaver writes it: the fields before the groups. `None`
    /// for another file, a file of another version, or one whose fields come in another order.
    pub fn read(path: &Path) -> Option<Self> {
        #[derive(Deserialize)]
        struct Head {
            format: String,
            version: u32,
            kind: String,
            id: String,
            name: String,
            #[serde(default)]
            windows: String,
            #[serde(default)]
            created: String,
        }
        let bytes = first_bytes(path, HEADER_BYTES)?;
        let text = String::from_utf8_lossy(&bytes);
        // A quote inside a string of JSON is escaped, so this marks the end of the header.
        let end = text.find(",\"groups\":")?;
        let head: Head = serde_json::from_str(&format!("{}}}", &text[..end])).ok()?;
        let fits = head.format == CATALOG_FORMAT
            && head.version <= CATALOG_VERSION
            && head.kind == "admx"
            && check::is_safe_id(&head.id)
            && !head.name.chars().any(char::is_control);
        fits.then(|| Self {
            id: head.id,
            name: head.name.chars().take(120).collect(),
            windows: head.windows.chars().take(40).collect(),
            created: head.created.chars().take(40).collect(),
        })
    }
}

/// The ids of the rules of a catalog file, read as a stream without keeping the file in memory
/// (in a browser through the file layer, which gives the file whole): what the search for the
/// catalog of an option the loaded catalog does not have needs.
pub fn rule_ids(path: &Path) -> Result<HashSet<String>, String> {
    #[derive(Deserialize)]
    struct Id {
        id: String,
    }
    #[derive(Deserialize)]
    struct Ids {
        format: String,
        rules: Vec<Id>,
    }
    let name = jsonfile::display_name(path);
    #[cfg(not(target_arch = "wasm32"))]
    let ids: Ids = {
        let file = std::fs::File::open(path).map_err(|e| format!("{name}: {e}"))?;
        let reader = std::io::BufReader::new(std::io::Read::take(file, CATALOG_MAX));
        serde_json::from_reader(reader).map_err(|e| format!("{name}: {e}"))?
    };
    #[cfg(target_arch = "wasm32")]
    let ids: Ids = {
        let bytes = ww_base::files::read(path, CATALOG_MAX).map_err(|e| format!("{name}: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("{name}: {e}"))?
    };
    if ids.format != CATALOG_FORMAT {
        return Err(format!("{name}: not a catalog file"));
    }
    Ok(ids.rules.into_iter().map(|r| r.id).collect())
}
