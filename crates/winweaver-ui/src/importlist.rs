//! The catalog files of the imports next to the executable: the list, the ones the settings load, their reading.
//! The import of policy templates itself is here too, for the window and the command line.

use crate::*;

/// The folder of the policy templates of this Windows.
const SYSTEM_TEMPLATES: &str = "C:\\Windows\\PolicyDefinitions";

/// The build of this Windows, as the name of an import of its templates gives it; empty where
/// Windows is not.
pub fn windows_build() -> String {
    winweaver_win::this_pc()
        .map(|m| m.system_info().build)
        .unwrap_or_default()
}

/// The id an imported catalog file takes: its own, or with the prefix `package-` when an import
/// of `taken` has it, so the import does not replace that one.
pub fn package_id(id: &str, taken: &[String]) -> String {
    if taken.iter().any(|t| t == id) && !id.starts_with("package-") {
        format!("package-{id}")
    } else {
        id.to_owned()
    }
}

/// The catalog file of an import of policy templates: the templates of this Windows (the id
/// `system`, named with its `build`), a folder of ADMX files (`folder-<fnv of its kept path>`), a
/// catalog file, or a zip archive of templates (`archive-<fnv>`) or of a catalog file; a catalog
/// file takes its [`package_id`] among the ids of `taken`.
pub fn import_source(
    source: ImportSource,
    path: Option<&Path>,
    build: &str,
    taken: &[String],
) -> Result<CatalogFile, String> {
    let now = clock::now_utc_iso();
    let package = |mut f: CatalogFile| {
        f.id = package_id(&f.id, taken);
        f
    };
    match (source, path) {
        (ImportSource::ThisWindows, _) => {
            let t = admx::read_folder(Path::new(SYSTEM_TEMPLATES));
            let name = if build.is_empty() {
                "Policy templates of this Windows".to_owned()
            } else {
                format!("Policy templates of this Windows ({build})")
            };
            Ok(admx::to_catalog_file(&t, "system", &name, build, &now))
        }
        (ImportSource::Folder, Some(dir)) => {
            let t = admx::read_folder(dir);
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Templates".to_owned());
            let id = format!("folder-{:08x}", session::fnv(&kept_name(dir)) as u32);
            Ok(admx::to_catalog_file(&t, &id, &name, "", &now))
        }
        (ImportSource::CatalogFile, Some(file)) => CatalogFile::load(file).map(package),
        (ImportSource::Archive, Some(file)) => match admx::read_archive(file)? {
            admx::Archive::Templates(t) => {
                let name = file
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Templates".to_owned());
                let id = format!("archive-{:08x}", session::fnv(&kept_name(file)) as u32);
                Ok(admx::to_catalog_file(&t, &id, &name, "", &now))
            }
            admx::Archive::Catalog(f) => Ok(package(*f)),
        },
        _ => Err("nothing to import".to_owned()),
    }
}

/// The catalog file of one ADMX file with the ADML of the folders of its cultures, as its tab
/// shows it before an import: the id `file-<fnv of its kept path>` and the name of the file; the
/// reason when it holds no policy.
pub fn admx_file_catalog(path: &Path) -> Result<CatalogFile, String> {
    let templates = admx::read_file(path);
    if templates.policies() == 0 {
        let why = templates
            .skipped
            .first()
            .map(|s| s.1.clone())
            .unwrap_or_else(|| tr("no policies").to_owned());
        return Err(why);
    }
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Templates".to_owned());
    let id = format!("file-{:08x}", session::fnv(&kept_name(path)) as u32);
    let now = clock::now_utc_iso();
    Ok(admx::to_catalog_file(&templates, &id, &name, "", &now))
}

/// Saves the catalog file of an import as `<id>.wwcatalog` in the imports folder `dir`, made
/// first when missing (the file layer does it; in a browser the folder is the storage of the
/// page); the path of the file.
pub fn save_import(dir: &Path, file: &CatalogFile) -> Result<PathBuf, String> {
    let path = dir.join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
    file.save(&path)?;
    Ok(path)
}

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
    // Through the file layer: in a browser the catalog files of the folder of the page.
    let mut files: Vec<(String, PathBuf)> = ww_base::files::list(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == admx::CATALOG_EXTENSION))
        .filter_map(|p| Some((p.file_stem()?.to_str()?.to_owned(), p)))
        .collect();
    files.sort_by_key(|(id, _)| import_order(id));
    files
}

/// The ids of the imports the settings load. Settings of an earlier version name the hidden
/// ones instead: then every catalog file of the folder but those.
pub fn loaded_import_ids(settings: &Settings, dir: &Path) -> Vec<String> {
    match &settings.extra.loaded_imports {
        Some(ids) => ids.clone(),
        None => import_files(dir)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !settings.extra.hidden_imports.contains(id))
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
            let bytes = admx::file_size(&path);
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
