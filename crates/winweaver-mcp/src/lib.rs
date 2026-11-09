//! The tools of WinWeaver for the MCP server of W# (`wwmcp`, `docs/spec/03-features.md` section
//! 7): a source of tools (`Tools`) with its own workspace of profiles, independent of the window.
//! The server, its protocol, its transports, its configuration file, its journal, its log, its
//! tab and its command line are the ones of W#; this crate holds what is WinWeaver's: the tools of
//! the profiles, the rules, the bookmarks, the catalogs, the registry and this PC, their options,
//! their resources and the configuration files of earlier versions (`legacy`).
//!
//! The mode of the server decides what the tools may do: `read` (nothing changes, the mode of
//! every start), `edit` (workspace profiles change in memory) and `files` (profiles saved and the
//! answer file written, only into the folders of the program). The options of the source
//! `winweaver` open more: `read_pc` (reading this PC), `registry` (`read` or `write`) and
//! `edit_library` (the bookmarks and the catalog files); `profile` names the profile the workspace
//! starts with. Nothing through MCP ever applies settings, elevates, deletes or imports.

pub mod legacy;
mod tools;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use indexmap::IndexMap;
use serde_json::Value;
use winweaver_core::catalog::Catalog;
use winweaver_core::profile::Profile;
use wwmcp::{Call, Choice, Info, Mode, Outcome, ResourceDef, Source, ToolDef};

/// The id of the source of the tools of WinWeaver: the key of its branch in the configuration
/// file of the server.
pub const SOURCE: &str = "winweaver";

/// What the tools of the registry may do: off unless an option of the source allows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegistryAccess {
    #[default]
    Off,
    Read,
    /// Reading and writing.
    Write,
}

impl RegistryAccess {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Self::Off),
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// The folders and files of the program the tools read and write.
#[derive(Debug, Clone)]
pub struct Paths {
    /// The bookmark file and the folder of the catalog files of the imports.
    pub marks_path: PathBuf,
    pub imports_dir: PathBuf,
    /// The folder of the profiles the tools load and save.
    pub profiles_dir: PathBuf,
    /// The folder the answer file is written into.
    pub output_dir: PathBuf,
}

/// What a call of a tool may do: the mode of the server and the options of the source, with the
/// folders of the program.
#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub registry: RegistryAccess,
    /// Whether the tools may change the bookmarks and the catalog files; reading them is always
    /// allowed, like reading profiles.
    pub edit_library: bool,
    /// Whether the tool read_this_pc may read the settings of this PC.
    pub read_pc: bool,
    pub marks_path: PathBuf,
    pub imports_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub output_dir: PathBuf,
}

/// A profile of the workspace.
#[derive(Debug, Clone)]
pub struct WsDoc {
    pub profile: Profile,
    pub file: Option<PathBuf>,
    pub dirty: bool,
}

pub(crate) struct State {
    pub options: Options,
    pub docs: IndexMap<String, WsDoc>,
    pub current: String,
}

/// The tools of WinWeaver: the catalog, the presets, the registry they read and change (of this
/// PC, or in memory for the tests) and the workspace behind a lock.
pub struct Tools {
    pub catalog: Catalog,
    pub presets: Vec<(String, Profile)>,
    pub(crate) registry: Arc<dyn winweaver_core::registry::Registry>,
    state: Mutex<State>,
}

/// The name the tools give the struct of the server they run in.
pub(crate) type Server = Tools;

impl Tools {
    /// The tools with a workspace that starts with a copy of the first preset.
    pub fn new(catalog: Catalog, presets: &[(&str, &str)], paths: Paths) -> Self {
        // The input languages and layouts of this Windows, read at the first build or check, and
        // its regions, read at the first check or set_param of the user region.
        winweaver_core::resources::set_locale_reader(winweaver_win::locales);
        winweaver_core::resources::set_region_reader(winweaver_win::regions);
        let presets: Vec<(String, Profile)> = presets
            .iter()
            .filter_map(|(name, text)| {
                Profile::parse(text, name)
                    .ok()
                    .map(|p| ((*name).to_owned(), p))
            })
            .collect();
        let first = presets
            .first()
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| Profile::from_defaults("Office", &catalog));
        let mut docs = IndexMap::new();
        let current = first.name.clone();
        docs.insert(
            current.clone(),
            WsDoc {
                profile: first,
                file: None,
                dirty: false,
            },
        );
        Self {
            catalog,
            presets,
            registry: winweaver_win::this_registry(),
            state: Mutex::new(State {
                options: Options {
                    mode: Mode::Read,
                    registry: RegistryAccess::Off,
                    edit_library: false,
                    read_pc: false,
                    marks_path: paths.marks_path,
                    imports_dir: paths.imports_dir,
                    profiles_dir: paths.profiles_dir,
                    output_dir: paths.output_dir,
                },
                docs,
                current,
            }),
        }
    }

    /// The tools with another registry, as the tests give one in memory.
    pub fn with_registry(mut self, registry: Arc<dyn winweaver_core::registry::Registry>) -> Self {
        self.registry = registry;
        self
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The profiles of the workspace: the name, whether it changed, the file name.
    pub fn workspace(&self) -> Vec<(String, bool, String)> {
        let state = self.lock();
        state
            .docs
            .iter()
            .map(|(name, d)| {
                let file = d
                    .file
                    .as_ref()
                    .and_then(|f| f.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (name.clone(), d.dirty, file)
            })
            .collect()
    }

    /// A copy of a workspace profile, for the window to open it in a tab.
    pub fn workspace_profile(&self, name: &str) -> Option<Profile> {
        self.lock().docs.get(name).map(|d| d.profile.clone())
    }

    /// Makes a preset or a profile file of the profiles folder the current profile, whatever
    /// the mode: the choice of the person who starts the server (the option `profile`).
    pub fn select(&self, name: &str) -> Result<(), String> {
        let mut state = self.lock();
        tools::select(self, &mut state, name).map_err(|e| e.to_string())
    }
}

impl Source for Tools {
    fn id(&self) -> String {
        SOURCE.to_owned()
    }

    fn title(&self) -> String {
        "WinWeaver".to_owned()
    }

    fn tools(&self) -> Vec<ToolDef> {
        tools::definitions()
    }

    fn options(&self) -> Vec<Choice> {
        options()
    }

    fn call(&self, tool: &str, args: &Value, call: &Call<'_>) -> Outcome {
        let mut state = self.lock();
        state.options.mode = call.mode;
        state.options.read_pc = call.is_on("read_pc");
        state.options.edit_library = call.is_on("edit_library");
        state.options.registry = RegistryAccess::parse(call.option("registry")).unwrap_or_default();
        tools::call(self, &mut state, tool, args)
    }

    fn resources(&self) -> Vec<ResourceDef> {
        tools::resources()
    }

    fn read(&self, uri: &str, _call: &Call<'_>) -> Option<(String, String)> {
        let state = self.lock();
        let text = tools::resource(self, &state, uri)?;
        let mime = if uri == "winweaver://skill" {
            "text/markdown"
        } else {
            "application/json"
        };
        Some((mime.to_owned(), text))
    }

    fn instructions(&self) -> String {
        INSTRUCTIONS.to_owned()
    }

    fn secrets(&self) -> Vec<String> {
        vec!["product_key".to_owned()]
    }
}

/// The options of the tools of WinWeaver, kept in the branch `winweaver` of the configuration.
pub fn options() -> Vec<Choice> {
    vec![
        Choice::of(
            "registry",
            "Registry",
            &[
                ("off", "Off"),
                ("read", "Allow reading the registry of this PC"),
                ("write", "Allow changing the registry too"),
            ],
        ),
        Choice::switch("read_pc", "Allow reading the settings of this PC"),
        Choice::switch(
            "edit_library",
            "Allow changing the bookmarks and the catalogs",
        ),
        Choice::text("profile", "Profile at the start")
            .help("A preset or a profile of the folder profiles; empty for the first preset"),
    ]
}

/// What the server says of WinWeaver at `initialize`.
pub fn info() -> Info {
    Info {
        name: SOURCE.to_owned(),
        version: winweaver_core::BUILD.to_owned(),
        instructions: String::new(),
    }
}

/// The server of the tools of WinWeaver with a configuration: the workspace starts with the
/// profile of the option `profile`, else the first preset.
pub fn server(tools: Arc<Tools>, config: wwmcp::Config) -> Result<wwmcp::Server, String> {
    let profile = config
        .source(SOURCE)
        .options
        .get("profile")
        .cloned()
        .unwrap_or_default();
    if !profile.is_empty() {
        tools
            .select(&profile)
            .map_err(|e| format!("the profile {profile}: {e}"))?;
    }
    Ok(wwmcp::Server::new(info(), config).with(tools))
}

/// The instructions the server gives the client at `initialize`.
pub const INSTRUCTIONS: &str = "WinWeaver holds a catalog of rules for unattended Windows 11 installations and a workspace of profiles that choose them. Start with get_status. The mode decides what the tools may do: read (nothing changes), edit (workspace profiles change in memory), files (profiles are saved and the answer file is written into the folders of the program). An error text starts with its kind: not_allowed, not_found, invalid, write_failed or read_failed. Passwords and product keys are never shown.";
