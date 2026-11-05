//! Binary structures: one plugin, many formats. The engine reads a file by a description of its
//! format (`.wwformat`, data and never code: docs/plugins.md section 6.4) into a tree of nodes,
//! the bytes left in the file, read in blocks with a cache of a fixed size (`wwrt::blocks`). The
//! Portable Executable (`formats/pe.wwformat`) is built in; the descriptions of the folder
//! `formats` next to the DLL add formats, and one of the same name replaces the built-in one.
//!
//! `expr`: the expressions of a description; `format`: the description checked and compiled;
//! `engine`: the reader and the tree; `hex`: the bytes; `view`: the tree, the bytes and the text.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use wwrt::blocks::BlockFile;
use wwrt::egui;
use wwrt::plugin::{self, Plugin, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::i18n::{tr, trf};

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
        .map(|Described { format: f, .. }| plugin::Format {
            name: Box::leak(f.name.clone().into_boxed_str()),
            extensions: Box::leak(
                f.extensions
                    .iter()
                    .map(|e| &*Box::leak(e.clone().into_boxed_str()))
                    .collect::<Vec<&'static str>>()
                    .into_boxed_slice(),
            ),
        })
        .collect();
    Formats {
        list,
        problems,
        shown: Box::leak(shown.into_boxed_slice()),
    }
}

fn formats() -> &'static Formats {
    FORMATS.get_or_init(load)
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
        let block: usize = setting(settings, "block_kb", 64);
        let cache: usize = setting(settings, "cache_mb", 16);
        let per_row: u64 = setting(settings, "bytes_per_row", 16);
        let block = block.clamp(BLOCK_STEPS[0], BLOCK_STEPS[2]) << 10;
        let cache = cache.clamp(CACHE_STEPS[0], CACHE_STEPS[2]) << 20;
        let per_row = per_row.clamp(ROW_STEPS[0], ROW_STEPS[2]);
        let mut file = BlockFile::open(path, block, cache).map_err(|e| e.to_string())?;
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let candidates: Vec<&'static Described> = formats()
            .list
            .iter()
            .filter(|d| d.format.extensions.contains(&extension))
            .collect();
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
        Ok(Box::new(view::BinaryView::new(
            path,
            &found.format,
            found.sha256.clone(),
            tree,
            file,
            per_row,
        )))
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
