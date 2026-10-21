//! The catalog files of the imports next to the executable: the list, the ones the settings load, their reading.

use crate::*;

/// A catalog file of the imports folder, loaded or not.
pub struct ImportEntry {
    /// The name of the file without its extension: the id of the import.
    pub id: String,
    /// The name of the import: of the loaded file, else of the header of the file, else the id.
    pub name: String,
    pub bytes: u64,
}

/// The order the imports are merged in: the templates of this Windows first.
pub(crate) fn import_order(id: &str) -> (bool, String) {
    (!id.starts_with("system"), id.to_owned())
}

/// The catalog files of a folder of imports with their ids, in the order of the merge.
pub fn import_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == admx::CATALOG_EXTENSION))
        .filter_map(|p| Some((p.file_stem()?.to_str()?.to_owned(), p)))
        .collect();
    files.sort_by_key(|(id, _)| import_order(id));
    files
}

/// The ids of the imports the settings load. Settings of an earlier version name the hidden
/// ones instead: then every catalog file of the folder but those.
pub fn loaded_import_ids(settings: &Settings, dir: &Path) -> Vec<String> {
    match &settings.loaded_imports {
        Some(ids) => ids.clone(),
        None => import_files(dir)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !settings.hidden_imports.contains(id))
            .collect(),
    }
}

/// Reads the catalog files of the given imports in the order of the merge, and no other file of
/// the folder; the ids of the files that could not be read.
pub fn read_imports(dir: &Path, ids: &[String]) -> (Vec<CatalogFile>, Vec<String>) {
    let mut files = Vec::new();
    let mut broken = Vec::new();
    for (id, path) in import_files(dir) {
        if !ids.contains(&id) {
            continue;
        }
        match CatalogFile::load(&path) {
            Ok(mut f) => {
                f.id = id;
                files.push(f);
            }
            Err(_) => broken.push(id),
        }
    }
    (files, broken)
}

/// The catalog files of the folder for the list of the settings: a loaded one by its name, the
/// others by the header of the file.
pub(crate) fn import_entries(dir: &Path, loaded: &[CatalogFile]) -> Vec<ImportEntry> {
    import_files(dir)
        .into_iter()
        .map(|(id, path)| {
            let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let name = match loaded.iter().find(|f| f.id == id) {
                Some(f) => f.name.clone(),
                None => admx::CatalogHeader::read(&path)
                    .map(|h| h.name)
                    .unwrap_or_else(|| id.clone()),
            };
            ImportEntry { id, name, bytes }
        })
        .collect()
}

/// The size of a file for people: kilobytes or megabytes.
pub(crate) fn size_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

/// The warning of a profile with options of catalogs that are not loaded.
pub(crate) fn unknown_issues(profile: &Profile) -> Vec<Issue> {
    if profile.unknown.is_empty() {
        return Vec::new();
    }
    vec![Issue {
        severity: Severity::Warning,
        target: Target::Unknown,
        text: trf(
            "Open: {n} options of catalogs that are not loaded are kept in the profile and left out of the build",
            &[("n", &profile.unknown.len())],
        ),
    }]
}
