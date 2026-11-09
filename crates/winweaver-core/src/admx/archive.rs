//! Zip archives of policy templates or of a catalog file, read in memory.

use std::collections::HashMap;
use std::path::Path;

use crate::jsonfile;

use super::file::CatalogFile;
use super::templates::{Templates, read_pair};
use super::{CATALOG_EXTENSION, CATALOG_MAX, CULTURES, FILE_MAX};

/// The extension of an archive of policy templates or of a catalog file.
pub const ARCHIVE_EXTENSION: &str = "zip";
/// The limits of an archive: its size, the count of its entries and all it unpacks to. An entry is
/// read up to the limit of its kind (`FILE_MAX`, `CATALOG_MAX`) whatever size it declares.
const ARCHIVE_MAX: u64 = 256 * 1024 * 1024;
const ARCHIVE_ENTRIES: usize = 20_000;
const UNPACKED_MAX: u64 = 512 * 1024 * 1024;

/// What a zip archive holds.
#[derive(Debug)]
pub enum Archive {
    /// Its ADMX files, each with the ADML of the folders of the cultures next to it, as a folder of
    /// templates gives them.
    Templates(Templates),
    /// Its one catalog file, checked.
    Catalog(Box<CatalogFile>),
}

/// Reads a zip archive of policy templates (as Chrome, Edge or Firefox ship them) or of a catalog
/// file in memory, nothing unpacked to the disk: its ADMX files when it has any, else its one
/// `.wwcatalog` file. Its size, its entries and what it unpacks to are limited, so a crafted archive
/// gives an error; an ADMX that cannot be read is listed among the skipped ones.
pub fn read_archive(path: &Path) -> Result<Archive, String> {
    let name = jsonfile::display_name(path);
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{name}: {e}"))?
        .len();
    if size > ARCHIVE_MAX {
        return Err(format!("{name}: larger than {} MB", ARCHIVE_MAX >> 20));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{name}: {e}"))?;
    read_zip(std::io::BufReader::new(file), &name)
}

/// [`read_archive`] of the bytes of an archive under its name: a file a browser hands over.
pub fn read_archive_bytes(bytes: &[u8], name: &str) -> Result<Archive, String> {
    if bytes.len() as u64 > ARCHIVE_MAX {
        return Err(format!("{name}: larger than {} MB", ARCHIVE_MAX >> 20));
    }
    read_zip(std::io::Cursor::new(bytes), name)
}

fn read_zip<R: std::io::Read + std::io::Seek>(reader: R, name: &str) -> Result<Archive, String> {
    let mut zip =
        zip::ZipArchive::new(reader).map_err(|e| format!("{name}: not a zip archive: {e}"))?;
    if zip.len() > ARCHIVE_ENTRIES {
        return Err(format!("{name}: more than {ARCHIVE_ENTRIES} entries"));
    }
    let names: Vec<String> = zip
        .file_names()
        .map(|n| n.map(std::borrow::Cow::into_owned))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{name}: {e}"))?;
    // The entries by their path with forward slashes in lower case, to find the ADML of an ADMX.
    let lower: HashMap<String, String> = names
        .iter()
        .map(|n| (n.replace('\\', "/").to_ascii_lowercase(), n.clone()))
        .collect();
    let mut unpacked = 0;
    let mut admx: Vec<&String> = names
        .iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".admx"))
        .collect();
    admx.sort();
    if !admx.is_empty() {
        let mut out = Templates::default();
        for entry in admx {
            let path = entry.replace('\\', "/");
            let (dir, file) = path.rsplit_once('/').unwrap_or(("", path.as_str()));
            let stem = &file[..file.len() - ".admx".len()];
            let text = match entry_text(&mut zip, entry, FILE_MAX, &mut unpacked) {
                Ok(t) => t,
                Err(e) => {
                    out.skipped.push((file.to_owned(), e));
                    continue;
                }
            };
            let mut adml = Vec::new();
            for (culture, _) in CULTURES {
                let key = if dir.is_empty() {
                    format!("{culture}/{stem}.adml")
                } else {
                    format!("{dir}/{culture}/{stem}.adml")
                };
                if let Some(original) = lower.get(&key.to_ascii_lowercase())
                    && let Ok(t) = entry_text(&mut zip, original, FILE_MAX, &mut unpacked)
                {
                    adml.push((culture.to_owned(), t));
                }
            }
            if let Err(e) = read_pair(&text, &adml, file, &mut out) {
                out.skipped.push((file.to_owned(), e));
            }
        }
        if unpacked > UNPACKED_MAX {
            return Err(format!(
                "{name}: unpacks to more than {} MB",
                UNPACKED_MAX >> 20
            ));
        }
        return Ok(Archive::Templates(out));
    }
    let ending = format!(".{CATALOG_EXTENSION}");
    let catalogs: Vec<&String> = names
        .iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(&ending))
        .collect();
    match catalogs.as_slice() {
        [one] => {
            let text = entry_text(&mut zip, one, CATALOG_MAX, &mut unpacked)?;
            let file: CatalogFile = jsonfile::parse(&text, one).map_err(|e| e.to_string())?;
            file.check()?;
            Ok(Archive::Catalog(Box::new(file)))
        }
        [] => Err(format!(
            "{name}: no ADMX file and no catalog file in the archive"
        )),
        many => Err(format!(
            "{name}: {} catalog files in the archive, one is imported at a time",
            many.len()
        )),
    }
}

/// The text of an entry of an archive: at most `limit` bytes whatever it declares, UTF-8 with or
/// without a byte order mark; its bytes count towards what the archive may unpack.
fn entry_text<R: std::io::Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
    limit: u64,
    unpacked: &mut u64,
) -> Result<String, String> {
    use std::io::Read;
    if *unpacked > UNPACKED_MAX {
        return Err(format!("{name}: the archive unpacks to too much"));
    }
    let entry = zip.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    if entry.size() > limit {
        return Err(format!("{name}: larger than {} MB", limit >> 20));
    }
    let mut bytes = Vec::new();
    entry
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{name}: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("{name}: larger than {} MB", limit >> 20));
    }
    *unpacked += bytes.len() as u64;
    jsonfile::decode_bom(&bytes).ok_or_else(|| format!("{name}: neither UTF-8 nor UTF-16 text"))
}
