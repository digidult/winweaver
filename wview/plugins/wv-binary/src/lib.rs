//! Binary structures: one plugin, many formats. The engine reads a file by a description of its
//! format (`.wwformat`, data and never code: docs/plugins.md section 6.4) into a tree of nodes,
//! the bytes left in the file, read in blocks with a cache of a fixed size (`wwrt::blocks`). The
//! Portable Executable (`formats/pe.wwformat`) is built in; the descriptions of the folder
//! `formats` next to the DLL add formats, and one of the same name replaces the built-in one.
//!
//! A description names the MIME types of its files (a program inside the resources of another
//! opens with it) and of the bytes of its fields (`content`): the view shows the content of a
//! node with the plugin of its type inside itself.
//!
//! `expr`: the expressions of a description; `format`: the description checked and compiled;
//! `engine`: the reader and the tree; `hex`: the bytes; `content`: what a node holds; `view`: the
//! tree, the bytes, the text and the content.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use wwrt::blocks::BlockFile;
use wwrt::egui;
use wwrt::mime;
use wwrt::plugin::{self, Host, Plugin, Request, Tool, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::icons;

mod compareview;
mod content;
mod diff;
mod engine;
mod expr;
mod format;
mod hex;
mod view;
mod wwbin;

use format::Format;

/// The description of the Portable Executable, built in.
pub const PE: &str = include_str!("../formats/pe.wwformat");

/// The name of the plugin (translated by the host).
const NAME: &str = "Binary structures";

/// The settings and the values they offer.
const ROW_STEPS: [u64; 3] = [8, 16, 32];
const BLOCK_STEPS: [usize; 3] = [4, 64, 1024];
const CACHE_STEPS: [usize; 3] = [1, 16, 64];
/// The descriptions the folder `formats` may hold.
const MOST_FILES: usize = 100;
/// The files of the plugin itself: a structure saved and a comparison of structures.
const SAVED: plugin::Format = plugin::Format {
    name: "Structure of a file",
    extensions: &["wwbin"],
    mimes: &[],
};
const COMPARED: plugin::Format = plugin::Format {
    name: "Comparison of structures",
    extensions: &["wwbindiff"],
    mimes: &[],
};

/// The texts of the plugin in the languages of the window.
static TRANSLATIONS: [(&str, &str); 8] = [
    ("uk", include_str!("../lang/uk.json")),
    ("ru", include_str!("../lang/ru.json")),
    ("de", include_str!("../lang/de.json")),
    ("es", include_str!("../lang/es.json")),
    ("fr", include_str!("../lang/fr.json")),
    ("id", include_str!("../lang/id.json")),
    ("it", include_str!("../lang/it.json")),
    ("tr", include_str!("../lang/tr.json")),
];

/// A description read: the format, where it came from and the SHA-256 of its text.
struct Described {
    format: Format,
    from: String,
    sha256: String,
}

/// The descriptions read once: the formats and the files that did not read with the reason.
struct Formats {
    list: Vec<Described>,
    problems: Vec<(String, String)>,
    shown: &'static [plugin::Format],
}

static FORMATS: OnceLock<Formats> = OnceLock::new();

/// The folder of the DLL of the plugin (of the executable, when the plugin is a library).
#[cfg(windows)]
fn module_dir() -> Option<PathBuf> {
    use wwrt::windows::Win32::Foundation::HMODULE;
    use wwrt::windows::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExW,
    };
    use wwrt::windows::core::PCWSTR;
    let mut module = HMODULE::default();
    let mut buffer = vec![0u16; 4096];
    // SAFETY: the address is one of this module, which stays loaded while its code runs; the
    // name goes into a buffer of its length.
    let n = unsafe {
        let address = PCWSTR(module_dir as *const () as *const u16);
        let flags =
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        GetModuleHandleExW(flags, address, &raw mut module).ok()?;
        GetModuleFileNameW(Some(module), &mut buffer) as usize
    };
    let path = PathBuf::from(String::from_utf16_lossy(&buffer[..n.min(buffer.len())]));
    path.parent().map(Path::to_path_buf)
}

#[cfg(not(windows))]
fn module_dir() -> Option<PathBuf> {
    None
}

fn load() -> Formats {
    let mut list = Vec::new();
    let mut problems = Vec::new();
    let sha = |text: &str| wwbin::hex(&<sha2::Sha256 as sha2::Digest>::digest(text.as_bytes()));
    match Format::parse(PE) {
        Ok(format) => list.push(Described {
            format,
            from: tr("built in").to_owned(),
            sha256: sha(PE),
        }),
        Err(e) => problems.push(("pe.wwformat".to_owned(), e)),
    }
    if let Some(dir) = module_dir().map(|d| d.join("formats"))
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("wwformat"))
            })
            .collect();
        files.sort();
        for file in files.into_iter().take(MOST_FILES) {
            let shown = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let text = match std::fs::metadata(&file) {
                Ok(m) if m.len() > format::LARGEST as u64 => {
                    Err(format!("larger than {} bytes", format::LARGEST))
                }
                _ => std::fs::read_to_string(&file).map_err(|e| e.to_string()),
            };
            match text.and_then(|t| Format::parse(&t).map(|f| (f, sha(&t)))) {
                Ok((format, sha256)) => {
                    list.retain(|d| d.format.name != format.name);
                    list.push(Described {
                        format,
                        from: shown,
                        sha256,
                    });
                }
                Err(e) => problems.push((shown, e)),
            }
        }
    }
    let shown: Vec<plugin::Format> = list
        .iter()
        .map(|Described { format: f, .. }| {
            let leaked = |list: &[String]| -> &'static [&'static str] {
                Box::leak(
                    list.iter()
                        .map(|e| &*Box::leak(e.clone().into_boxed_str()))
                        .collect::<Vec<&'static str>>()
                        .into_boxed_slice(),
                )
            };
            plugin::Format {
                name: Box::leak(f.name.clone().into_boxed_str()),
                extensions: leaked(&f.extensions),
                mimes: leaked(&f.mimes),
            }
        })
        .collect();
    let mut shown = shown;
    shown.extend([SAVED, COMPARED]);
    Formats {
        list,
        problems,
        shown: Box::leak(shown.into_boxed_slice()),
    }
}

fn formats() -> &'static Formats {
    FORMATS.get_or_init(load)
}

/// The block, the cache and the bytes of a row of the settings.
fn sizes(settings: &Value) -> (usize, usize, u64) {
    let block: usize = setting(settings, "block_kb", 64);
    let cache: usize = setting(settings, "cache_mb", 16);
    let per_row: u64 = setting(settings, "bytes_per_row", 16);
    (
        block.clamp(BLOCK_STEPS[0], BLOCK_STEPS[2]) << 10,
        cache.clamp(CACHE_STEPS[0], CACHE_STEPS[2]) << 20,
        per_row.clamp(ROW_STEPS[0], ROW_STEPS[2]),
    )
}

/// The view of a binary file by the first description of its extension whose magic it has.
fn open_binary(path: &Path, settings: &Value) -> Result<view::BinaryView, String> {
    let (block, cache, per_row) = sizes(settings);
    let file = BlockFile::open(path, block, cache).map_err(|e| e.to_string())?;
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut candidates: Vec<&'static Described> = formats()
        .list
        .iter()
        .filter(|d| d.format.extensions.contains(&extension))
        .collect();
    // An extension no description names (a file the host opened by its first bytes): every
    // description, its magic decides.
    if candidates.is_empty() {
        candidates = formats().list.iter().collect();
    }
    view_of(path, file, &candidates, per_row)
}

/// The view of a source by the first description of the candidates whose magic it has.
fn view_of(
    path: &Path,
    mut file: BlockFile,
    candidates: &[&'static Described],
    per_row: u64,
) -> Result<view::BinaryView, String> {
    let Some(found) = candidates.iter().find(|d| {
        d.format
            .knows(&mut |at, out| engine::Source::read(&mut file, at, out))
    }) else {
        let name = candidates.first().map_or("?", |d| d.format.name.as_str());
        return Err(trf(
            "not a file of the format {format}",
            &[("format", &name)],
        ));
    };
    let tree = engine::read(&found.format, &mut file, engine::Limits::default());
    Ok(view::BinaryView::new(
        path,
        &found.format,
        found.sha256.clone(),
        tree,
        file,
        per_row,
    ))
}

/// The comparison a `.wwbindiff` names: its file compared with its structure, with its notes.
fn open_comparison(path: &Path, settings: &Value) -> Result<view::BinaryView, String> {
    let file = diff::DiffFile::read(path)?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let binary = diff::resolved(&file.file, dir);
    let structure = diff::resolved(&file.structure, dir);
    let named =
        |p: &Path, why: String| trf("{path}: {why}", &[("path", &p.display()), ("why", &why)]);
    let mut view = open_binary(&binary, settings).map_err(|e| named(&binary, e))?;
    view.compare_with(&structure)
        .map_err(|e| named(&structure, e))?;
    view.set_notes(file.notes, path.to_path_buf());
    Ok(view)
}

/// Structures and comparisons made for the tests of a program that takes the plugin as a
/// library (the feature `library`), without its dialogs.
#[cfg(any(test, feature = "library"))]
pub mod sample {
    use std::path::Path;

    use wwrt::serde_json::Value;

    /// The view of a file with the content of the node a path of names or keys leads to (`*`
    /// for the first child) shown inside it.
    pub fn content(file: &Path, path: &[&str]) -> Result<Box<dyn wwrt::plugin::View>, String> {
        let mut view = crate::open_binary(file, &Value::Null)?;
        view.show_content(path)?;
        Ok(Box::new(view))
    }

    /// Saves the structure of a file as a `.wwbin`.
    pub fn save_structure(file: &Path, out: &Path) -> Result<(), String> {
        crate::open_binary(file, &Value::Null)?.save_to(out)
    }

    /// Saves the comparison of a file with a structure as a `.wwbindiff`, with notes of a key, a
    /// comment and a bookmark.
    pub fn save_comparison(
        file: &Path,
        structure: &Path,
        out: &Path,
        notes: &[(&str, &str, bool)],
    ) -> Result<(), String> {
        let mut view = crate::open_binary(file, &Value::Null)?;
        view.compare_with(structure)?;
        let notes = notes
            .iter()
            .map(|(key, comment, bookmark)| crate::diff::Note {
                key: (*key).to_owned(),
                comment: (*comment).to_owned(),
                bookmark: *bookmark,
            })
            .collect();
        view.set_notes(notes, out.to_path_buf());
        view.save_comparison_to(out)
    }
}

/// The view of some bytes by the first description of their MIME type whose magic they have.
fn open_memory(
    name: &str,
    mime: &str,
    bytes: Vec<u8>,
    settings: &Value,
) -> Result<view::BinaryView, String> {
    let wanted = mime::canonical(mime);
    let candidates: Vec<&'static Described> = formats()
        .list
        .iter()
        .filter(|d| d.format.mimes.iter().any(|m| mime::canonical(m) == wanted))
        .collect();
    if candidates.is_empty() {
        return Err(trf("no description of {mime}", &[("mime", &wanted)]));
    }
    let (_, _, per_row) = sizes(settings);
    let file = BlockFile::from_bytes(bytes);
    Ok(view_of(Path::new(""), file, &candidates, per_row)?.named(name))
}

pub struct BinaryPlugin;

impl Plugin for BinaryPlugin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn formats(&self) -> &'static [plugin::Format] {
        formats().shown
    }

    fn open(&self, path: &Path, settings: &Value) -> Result<Box<dyn View>, String> {
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match extension.as_str() {
            "wwbin" => {
                let saved = wwbin::read(path)?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Box::new(compareview::SavedView::new(name, saved)))
            }
            "wwbindiff" => open_comparison(path, settings).map(|v| Box::new(v) as Box<dyn View>),
            _ => open_binary(path, settings).map(|v| Box::new(v) as Box<dyn View>),
        }
    }

    fn open_bytes(
        &self,
        name: &str,
        mime: &str,
        bytes: Vec<u8>,
        settings: &Value,
    ) -> Result<Box<dyn View>, String> {
        open_memory(name, mime, bytes, settings).map(|v| Box::new(v) as Box<dyn View>)
    }

    fn menu(&self) -> Vec<Tool> {
        vec![Tool {
            id: "formats",
            icon: icons::FOLDER,
            label: "Folder of the descriptions",
        }]
    }

    /// The folder `formats` next to the DLL in Explorer, made when it is not there.
    fn act(&self, id: &str, host: &mut Host) {
        if id != "formats" {
            return;
        }
        let Some(dir) = module_dir().map(|d| d.join("formats")) else {
            return;
        };
        let shown = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|()| wwrt::ww::folders::show_in_folder(&dir));
        if let Err(why) = shown {
            host.requests.push(Request::Status(trf(
                "{path}: {why}",
                &[("path", &dir.display()), ("why", &why)],
            )));
        }
    }

    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &TRANSLATIONS
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&self, ui: &mut egui::Ui, settings: &mut Value) -> bool {
        let mut changed = false;
        let mut radios = |ui: &mut egui::Ui,
                          label: &str,
                          name: &str,
                          steps: [u64; 3],
                          default: u64,
                          unit: &'static str| {
            let mut value: u64 = setting(settings, name, default);
            ui.horizontal(|ui| {
                ui.label(label);
                for s in steps {
                    let text = if unit.is_empty() {
                        s.to_string()
                    } else {
                        trf(unit, &[("n", &s)])
                    };
                    if ui.radio(value == s, text).clicked() && value != s {
                        value = s;
                        set_setting(settings, name, s);
                        changed = true;
                    }
                }
            });
        };
        radios(ui, tr("Bytes in a row"), "bytes_per_row", ROW_STEPS, 16, "");
        let blocks = BLOCK_STEPS.map(|b| b as u64);
        radios(
            ui,
            tr("Block of the file"),
            "block_kb",
            blocks,
            64,
            tr("{n} KB"),
        );
        let caches = CACHE_STEPS.map(|c| c as u64);
        radios(
            ui,
            tr("Cache of the blocks"),
            "cache_mb",
            caches,
            16,
            tr("{n} MB"),
        );
        ui.weak(tr("The settings apply to the files opened after a change."));
        ui.add_space(4.0);
        for d in &formats().list {
            ui.label(trf(
                "{format} ({from}): {extensions}",
                &[
                    ("format", &d.format.name),
                    ("from", &d.from),
                    ("extensions", &d.format.extensions.join(", ")),
                ],
            ));
        }
        for (file, why) in &formats().problems {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                trf(
                    "{file} did not read: {why}",
                    &[("file", file), ("why", why)],
                ),
            );
        }
        changed
    }
}

wwrt::plugin!(BinaryPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_of_the_plugin_is_translated() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let wanted = wwrt::testing::plugin_texts(&src);
        let missing = wwrt::testing::missing(&TRANSLATIONS, &wanted);
        assert!(missing.is_empty(), "{missing:#?}");
    }

    /// A DLL of this Windows: its headers, sections, imports and exports.
    #[test]
    #[cfg(windows)]
    fn a_dll_of_windows_reads_whole() {
        let system =
            std::env::var_os("SystemRoot").map_or(PathBuf::from(r"C:\Windows"), PathBuf::from);
        let path = system.join("System32").join("kernel32.dll");
        let view = BinaryPlugin.open(&path, &Value::Null).unwrap();
        assert!(
            view.status().contains("Portable Executable"),
            "{}",
            view.status()
        );
        let format = Format::parse(PE).unwrap();
        let mut file = BlockFile::open(&path, 64 << 10, 16 << 20).unwrap();
        let tree = engine::read(&format, &mut file, engine::Limits::default());
        let child =
            |at: usize, name: &str| tree.child(at, name).unwrap_or_else(|| panic!("no {name}"));
        let nt = child(0, "nt_headers");
        let machine = child(child(nt, "file_header"), "machine");
        assert!(tree.value(machine).contains("AMD64") || tree.value(machine).contains("ARM64"));
        let sections = child(0, "sections");
        let names: Vec<String> = tree.nodes[sections]
            .children
            .iter()
            .map(|s| tree.label(*s))
            .collect();
        assert!(names.iter().any(|n| n.ends_with(".text")), "{names:?}");
        let exports = child(0, "exports");
        assert_eq!(
            tree.nodes[child(exports, "dll")].data,
            engine::Data::Text("KERNEL32.dll".to_owned())
        );
        let exported: Vec<String> = tree.nodes[child(exports, "names")]
            .children
            .iter()
            .map(|n| tree.label(*n))
            .collect();
        assert!(
            exported.iter().any(|n| n.ends_with(" CreateFileW")),
            "{} names",
            exported.len()
        );
        let imports = child(0, "imports");
        assert!(!tree.nodes[imports].children.is_empty());
        let first = tree.nodes[imports].children[0];
        let functions = child(first, "functions");
        assert!(
            !tree.nodes[functions].children.is_empty(),
            "{}",
            tree.label(first)
        );
        assert_eq!(
            tree.errors,
            0,
            "{:?}",
            tree.nodes
                .iter()
                .filter(|n| n.error)
                .map(|n| (&n.name, &n.note))
                .collect::<Vec<_>>()
        );
    }

    fn system(parts: &[&str]) -> PathBuf {
        let root =
            std::env::var_os("SystemRoot").map_or(PathBuf::from(r"C:\Windows"), PathBuf::from);
        parts.iter().fold(root, |p, part| p.join(part))
    }

    /// The node of a path of names from the root, with the items of an array by their key.
    fn find(tree: &engine::Tree, path: &[&str]) -> usize {
        path.iter().fold(0, |at, name| {
            tree.nodes[at]
                .children
                .iter()
                .copied()
                .find(|c| {
                    let n = &tree.nodes[*c];
                    n.name == *name || n.key.as_deref() == Some(*name)
                })
                .unwrap_or_else(|| panic!("no {name} in {}", tree.label(at)))
        })
    }

    /// The rest of PE in a DLL of this Windows: its resources read when they open, its exceptions,
    /// relocations, debug directory with the path of its PDB, and its load configuration.
    #[test]
    #[cfg(windows)]
    fn the_rest_of_a_dll_of_windows_reads_whole() {
        let path = system(&["System32", "kernel32.dll"]);
        let format = Format::parse(PE).unwrap();
        let mut file = BlockFile::open(&path, 64 << 10, 16 << 20).unwrap();
        let mut tree = engine::read(&format, &mut file, engine::Limits::default());
        let version = find(&tree, &["resources", "entries", "VERSION"]);
        let names = find(&tree, &["resources", "entries", "VERSION", "names"]);
        assert!(tree.nodes[names].pending, "{}", tree.label(version));
        engine::expand(
            &mut tree,
            &format,
            &mut file,
            names,
            engine::Limits::default(),
        );
        assert!(!tree.nodes[names].children.is_empty());
        engine::expand_all(&mut tree, &format, &mut file, engine::Limits::default());
        assert!(tree.nodes.iter().all(|n| !n.pending));
        let pdb = find(&tree, &["debug", "CODEVIEW", "codeview", "pdb"]);
        assert_eq!(
            tree.nodes[pdb].data,
            engine::Data::Text("kernel32.pdb".to_owned())
        );
        let relocations = find(&tree, &["relocations"]);
        assert!(!tree.nodes[relocations].children.is_empty());
        let exceptions = find(&tree, &["exceptions"]);
        assert!(
            tree.nodes[exceptions].children.len() > 100,
            "{}",
            tree.value(exceptions)
        );
        let size = find(&tree, &["load_config", "size"]);
        assert!(matches!(tree.nodes[size].data, engine::Data::Int(s) if s >= 64));
        assert_eq!(
            tree.errors,
            0,
            "{:?}",
            tree.nodes
                .iter()
                .filter(|n| n.error)
                .map(|n| (&n.name, &n.note))
                .collect::<Vec<_>>()
        );
    }

    /// The header of .NET and the streams of its metadata in an assembly of this Windows.
    #[test]
    #[cfg(windows)]
    fn an_assembly_of_dotnet_shows_its_metadata() {
        let path = system(&["Microsoft.NET", "Framework64", "v4.0.30319", "System.dll"]);
        if !path.is_file() {
            eprintln!("no {} on this Windows", path.display());
            return;
        }
        let format = Format::parse(PE).unwrap();
        let mut file = BlockFile::open(&path, 64 << 10, 16 << 20).unwrap();
        let tree = engine::read(&format, &mut file, engine::Limits::default());
        let signature = find(&tree, &["clr", "metadata_root", "signature"]);
        assert_eq!(
            tree.nodes[signature].data,
            engine::Data::Int(0x424A_5342),
            "BSJB"
        );
        let streams = find(&tree, &["clr", "metadata_root", "stream_headers"]);
        let keys: Vec<String> = tree.nodes[streams]
            .children
            .iter()
            .map(|s| tree.label(*s))
            .collect();
        assert!(keys.iter().any(|k| k.ends_with("#Strings")), "{keys:?}");
        assert!(
            tree.value(find(&tree, &["clr", "flags"]))
                .contains("ILONLY")
        );
    }

    /// The structure saved as a .wwbin: the file with its SHA-256, every node read.
    #[test]
    #[cfg(windows)]
    fn a_structure_is_saved_and_reads_back() {
        let path = system(&["System32", "kernel32.dll"]);
        let described = &formats().list[0];
        let mut file = BlockFile::open(&path, 64 << 10, 16 << 20).unwrap();
        let tree = engine::read(&described.format, &mut file, engine::Limits::default());
        let mut view = view::BinaryView::new(
            &path,
            &described.format,
            described.sha256.clone(),
            tree,
            file,
            16,
        );
        let dir = std::env::temp_dir().join(format!("wv-binary-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("kernel32.dll.wwbin");
        view.save_to(&out).unwrap();
        let doc: Value =
            wwrt::serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let sha = wwbin::hex(&<sha2::Sha256 as sha2::Digest>::digest(&bytes));
        assert_eq!(doc["file"]["sha256"], sha);
        assert_eq!(doc["file"]["size"], bytes.len() as u64);
        assert_eq!(doc["description"]["sha256"], described.sha256);
        assert_eq!(doc["tree"]["name"], "kernel32.dll");
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(
            text.contains("\"kernel32.pdb\""),
            "the resources and the debug directory are in it"
        );
        assert!(!text.contains("read when it opens"));
        assert_eq!(
            view::wwbin_path(&path).file_name().unwrap(),
            "kernel32.dll.wwbin"
        );
    }

    /// A copy of a DLL saved as its structure, one field of the copy changed, then compared: the
    /// one change found, the notes kept in a .wwbindiff and the comparison opened from it.
    #[test]
    #[cfg(windows)]
    fn a_changed_copy_is_compared_with_its_structure() {
        let dir = std::env::temp_dir().join(format!("wv-binary-compare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("k.dll");
        std::fs::copy(system(&["System32", "kernel32.dll"]), &copy).unwrap();
        let structure = dir.join("k.dll.wwbin");
        open_binary(&copy, &Value::Null)
            .unwrap()
            .save_to(&structure)
            .unwrap();
        // The time stamp of the file header, four bytes after the signature and the machine.
        let mut bytes = std::fs::read(&copy).unwrap();
        let header = u32::from_le_bytes(bytes[0x3C..0x40].try_into().unwrap()) as usize;
        bytes[header + 8] ^= 0xFF;
        std::fs::write(&copy, &bytes).unwrap();
        let mut view = open_binary(&copy, &Value::Null).unwrap();
        view.compare_with(&structure).unwrap();
        let status = view.status();
        assert!(status.contains("1 changed, 0 added, 0 removed"), "{status}");
        let saved = BinaryPlugin.open(&structure, &Value::Null).unwrap();
        assert!(saved.status().contains("k.dll"), "{}", saved.status());
        // A note saved with the comparison comes back with it.
        let diff_file = dir.join("k.dll.wwbindiff");
        let note = diff::Note {
            key: "nt_headers.file_header.time_date_stamp".to_owned(),
            comment: "Built again".to_owned(),
            bookmark: true,
        };
        view.set_notes(vec![note.clone()], diff_file.clone());
        view.save_comparison_to(&diff_file).unwrap();
        let written = diff::DiffFile::read(&diff_file).unwrap();
        assert_eq!(
            (written.file.as_str(), written.structure.as_str()),
            ("k.dll", "k.dll.wwbin")
        );
        assert_eq!(written.notes, vec![note]);
        let reopened = BinaryPlugin.open(&diff_file, &Value::Null).unwrap();
        assert!(
            reopened.status().contains("1 changed"),
            "{}",
            reopened.status()
        );
        assert!(
            reopened
                .tools()
                .iter()
                .any(|t| t.label == "Save the comparison")
        );
    }

    /// The path of the bytes of the first resource of a type.
    fn resource(kind: &'static str) -> [&'static str; 10] {
        [
            "resources",
            "entries",
            kind,
            "names",
            "entries",
            "*",
            "languages",
            "entries",
            "*",
            "data",
        ]
    }

    /// The resources of a program of this Windows name the types of their bytes: its manifest
    /// a text of XML, its first icon an image made an ICO file.
    #[test]
    #[cfg(windows)]
    fn the_resources_of_a_program_name_their_content() {
        let path = system(&["System32", "notepad.exe"]);
        let mut view = open_binary(&path, &Value::Null).unwrap();
        view.show_content(&resource("MANIFEST")).unwrap();
        let (name, mime, bytes) = view.selected_content().unwrap();
        assert!(name.starts_with("notepad.exe, MANIFEST/1/"), "{name}");
        assert_eq!(mime, "application/xml");
        assert!(
            mime::sniff(&bytes).is_some_and(|m| mime::same(m, "application/xml")),
            "{:?}",
            &bytes[..bytes.len().min(16)]
        );
        view.show_content(&resource("ICON")).unwrap();
        let (_, mime, bytes) = view.selected_content().unwrap();
        assert!(
            matches!(mime.as_str(), "image/x-icon" | "image/png"),
            "{mime}"
        );
        assert_eq!(mime::sniff(&bytes), Some(mime.as_str()));
        // A group of icons is one ICO file of its images, each the ICON of the id it names.
        view.show_content(&resource("GROUP_ICON")).unwrap();
        let (_, mime, bytes) = view.selected_content().unwrap();
        assert_eq!(mime, "image/x-icon");
        assert_eq!(mime::sniff(&bytes), Some("image/x-icon"));
        let images = u16::from_le_bytes([bytes[4], bytes[5]]);
        assert!(images > 1, "{images} images");
        // A cursor directory is still no file of its own.
        assert!(PE.contains("\"12\": \"application/octet-stream\""));
    }

    /// A program in memory, a part of another file, opens by its MIME type.
    #[test]
    #[cfg(windows)]
    fn a_program_in_memory_opens_by_its_type() {
        let bytes = std::fs::read(system(&["System32", "kernel32.dll"])).unwrap();
        let view = BinaryPlugin
            .open_bytes("inside.dll", mime::PE, bytes, &Value::Null)
            .unwrap();
        assert!(
            view.status().contains("Portable Executable"),
            "{}",
            view.status()
        );
        let Err(why) = BinaryPlugin.open_bytes("x", "image/png", vec![0; 8], &Value::Null) else {
            panic!("no description of PNG");
        };
        assert!(why.contains("image/png"), "{why}");
    }

    /// A program under another extension opens by its magic.
    #[test]
    #[cfg(windows)]
    fn a_program_of_another_extension_opens_by_its_magic() {
        let dir = std::env::temp_dir().join(format!("wv-binary-magic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("kernel32.bin");
        std::fs::copy(system(&["System32", "kernel32.dll"]), &copy).unwrap();
        let view = BinaryPlugin.open(&copy, &Value::Null).unwrap();
        assert!(
            view.status().contains("Portable Executable"),
            "{}",
            view.status()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_is_not_one_stays_closed_with_the_reason() {
        let dir = std::env::temp_dir().join(format!("wv-binary-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fake.exe");
        std::fs::write(&path, b"not a program").unwrap();
        let Err(why) = BinaryPlugin.open(&path, &Value::Null) else {
            panic!("a text is no program");
        };
        assert!(why.contains("Portable Executable"), "{why}");
        let truncated = dir.join("short.exe");
        std::fs::write(&truncated, b"MZ\x90\x00").unwrap();
        let view = BinaryPlugin.open(&truncated, &Value::Null).unwrap();
        assert!(
            view.status().contains("not read whole"),
            "{}",
            view.status()
        );
    }
}
