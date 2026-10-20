//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`), built on eframe.
//!
//! This is the skeleton of variant A (the command bar): the tab row with the menu button, the
//! command bar, the navigation row (back, forward, breadcrumb, bookmark, search), the profile
//! tree with the detail panel, the Settings tab and the status bar. Every change of the profile
//! is collected as an [`Act`] while the frame is drawn and applied afterwards.

mod cataledit;
mod catalogview;
mod chrome;
mod diffview;
mod forms;
mod i18n;
mod list;
mod marksview;
mod mcpview;
mod mdview;
mod regfileview;
mod regview;
mod renderer;
mod session;
mod settings;
mod style;
mod themes;
mod themeview;
mod tree;
mod usersview;
mod views;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui;
use serde_json::Value;
use winweaver_core::admx::{self, CatalogFile};
use winweaver_core::bookmarks::{self, Bookmark, Bookmarks};
use winweaver_core::catalog::{Catalog, TREE_ROOTS, check};
use winweaver_core::clock;
use winweaver_core::diff::{self, Diff};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::import;
use winweaver_core::jsonfile;
use winweaver_core::profile::{self, Account, Install, Languages, Profile};
use winweaver_core::regfile::{self, Line, RegFile};
use winweaver_core::render;
use winweaver_core::resolver::{self, Resolution};
use winweaver_core::scripts;
use winweaver_core::thispc::{self, Report, Verdict};
use winweaver_core::users::{self, UsersFile};
use winweaver_core::validate::{self, Issue, Severity, Target};

pub use diffview::DiffDoc;
pub use forms::Form;
pub use i18n::{tr, trf};
pub use mdview::MdDoc;
pub use renderer::Renderer;
pub use settings::{Settings, ThemeChoice, exe_dir};

/// The presets embedded in the executable: the name and the profile text.
pub const PRESETS: [(&str, &str); 4] = [
    (
        "Office",
        include_str!("../../../data/presets/office.wwprofile"),
    ),
    (
        "Strict",
        include_str!("../../../data/presets/strict.wwprofile"),
    ),
    (
        "Laptop",
        include_str!("../../../data/presets/laptop.wwprofile"),
    ),
    ("Home", include_str!("../../../data/presets/home.wwprofile")),
];

const SEARCH_HISTORY: usize = 20;
/// Below this width of the window the command bar shows icons only, in the navigation row.
const COMPACT_WIDTH: f32 = 1100.0;
/// The largest Markdown file a tab shows.
const MARKDOWN_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// The key of the group of the profile tree that lists the options of catalogs that are not
/// loaded; no catalog can have it as an id.
pub const UNKNOWN_GROUP: &str = "#unknown";

/// The selected item of a profile document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sel {
    None,
    Group(String),
    Rule(String),
    Form(Form),
    /// An option of the profile whose catalog is not loaded (`Profile::unknown`).
    Unknown(String),
}

/// An open profile with its view state.
pub struct ProfileDoc {
    pub profile: Profile,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub sel: Sel,
    history: Vec<Sel>,
    pos: usize,
    pub expanded: HashSet<String>,
    pub resolution: Resolution,
    pub issues: Vec<Issue>,
    /// The name of the draft file of the unsaved changes.
    pub draft_key: String,
    /// When the profile last changed and has no draft yet.
    changed: Option<std::time::Instant>,
    pub search: Search,
    /// The selected row is scrolled to the middle at the next frame (after a search).
    pub reveal: bool,
    /// The selected row follows into view at the next frame (after a move with the keys).
    pub follow: bool,
    /// The group the selected rule is shown in, when it shows in the trees of two imports.
    pub focus: Option<String>,
}

impl ProfileDoc {
    pub fn new(profile: Profile, path: Option<PathBuf>, catalog: &Catalog) -> Self {
        let resolution = resolver::resolve(catalog, &profile);
        let issues = unknown_issues(&profile);
        let expanded = TREE_ROOTS.iter().map(|s| (*s).to_owned()).collect();
        let draft_key = match &path {
            Some(p) => session::key_for_path(p),
            None => session::new_key(),
        };
        Self {
            profile,
            path,
            dirty: false,
            sel: Sel::None,
            history: vec![Sel::None],
            pos: 0,
            expanded,
            resolution,
            issues,
            draft_key,
            changed: None,
            search: Search::default(),
            reveal: false,
            follow: false,
            focus: None,
        }
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => format!("{}.{}", self.profile.name, profile::EXTENSION),
        }
    }

    fn select(&mut self, sel: Sel) {
        if self.sel == sel {
            return;
        }
        self.history.truncate(self.pos + 1);
        self.history.push(sel.clone());
        self.pos = self.history.len() - 1;
        self.sel = sel;
    }

    fn can_go(&self, delta: isize) -> bool {
        let n = self.pos as isize + delta;
        n >= 0 && (n as usize) < self.history.len()
    }

    fn go(&mut self, delta: isize) {
        if self.can_go(delta) {
            self.pos = (self.pos as isize + delta) as usize;
            self.sel = self.history[self.pos].clone();
        }
    }

    fn refresh(&mut self, catalog: &Catalog) {
        self.resolution = resolver::resolve(catalog, &self.profile);
        self.dirty = true;
        self.changed = Some(std::time::Instant::now());
    }
}

/// A question over the window: its title, its text, the label of the yes and the act it runs.
struct Ask {
    title: String,
    text: String,
    yes: String,
    act: Act,
}

/// A tab of the window.
pub enum Doc {
    Profile(Box<ProfileDoc>),
    Diff(Box<DiffDoc>),
    Markdown(Box<MdDoc>),
    Bookmarks,
    Mcp,
    Settings,
    /// The tree of an import of policy templates.
    Catalog(Box<catalogview::CatalogDoc>),
    /// The editor of a theme.
    Theme(Box<themeview::ThemeDoc>),
    /// The registry of this PC.
    Registry(Box<regview::RegistryDoc>),
    /// The editor of a catalog file.
    CatalogEdit(Box<cataledit::EditorDoc>),
    /// A registry file (`.reg`).
    RegFile(Box<regfileview::RegFileDoc>),
    /// A users file (`.wwusers`).
    Users(Box<usersview::UsersDoc>),
}

impl Doc {
    pub fn title(&self) -> String {
        match self {
            Doc::Profile(d) => d.title(),
            Doc::Diff(d) => d.title(),
            Doc::Markdown(d) => d.title.clone(),
            Doc::Bookmarks => tr("Bookmarks").to_owned(),
            Doc::Mcp => "MCP".to_owned(),
            Doc::Settings => tr("Settings").to_owned(),
            Doc::Catalog(d) => d.name.clone(),
            Doc::Theme(d) => d.title(),
            Doc::Registry(d) => d.title(),
            Doc::CatalogEdit(d) => d.title(),
            Doc::RegFile(d) => d.title(),
            Doc::Users(d) => d.title(),
        }
    }

    pub fn dirty(&self) -> bool {
        matches!(self, Doc::Profile(d) if d.dirty)
            || matches!(self, Doc::Diff(d) if d.dirty)
            || matches!(self, Doc::Theme(d) if d.dirty)
            || matches!(self, Doc::CatalogEdit(d) if d.dirty)
            || matches!(self, Doc::RegFile(d) if d.dirty)
            || matches!(self, Doc::Users(d) if d.dirty)
    }
}

/// The search of a tab, at the right end of its navigation row.
#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub text: String,
    focus: bool,
}

impl Search {
    /// The lower case words of the search text.
    pub fn words(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect()
    }
}

/// Puts a search first in the history of the searches; `true` when the history changed.
pub(crate) fn remember_search(history: &mut Vec<String>, text: &str) -> bool {
    let text = text.trim().to_owned();
    if text.is_empty() || history.first() == Some(&text) {
        return false;
    }
    history.retain(|h| h != &text);
    history.insert(0, text);
    history.truncate(SEARCH_HISTORY);
    true
}

/// The stand-alone scripts the window saves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    Apply,
    Revert,
    Audit,
}

/// Where an import of policy templates comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportSource {
    ThisWindows,
    Folder,
    CatalogFile,
    /// A zip archive of templates or of a catalog file.
    Archive,
}

/// Another catalog to compare the one of the active tab with: a loaded import or the tab of a
/// catalog file.
pub enum CatalogRef {
    Import(String),
    Tab(usize),
}

/// The colours of the title bar: dark, caption, text, border.
type TitleColors = (bool, [u8; 3], [u8; 3], [u8; 3]);

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    Select(Sel),
    /// Selects an item of the profile in front and opens its tree down to it: a link of a view.
    Reveal(Sel),
    Toggle(String, bool),
    SetGroup(String, bool),
    SetParam(String, String, Value),
    Expand(String, bool),
    Activate(usize),
    /// Moves a tab from one place of the row to another.
    MoveTab(usize, usize),
    CloseTab(usize, bool),
    NewProfile(Option<&'static str>),
    Open,
    OpenPath(PathBuf),
    OpenSettings,
    Save,
    SaveAs,
    Back,
    Forward,
    ToggleBookmark,
    Fold,
    /// Read every rule of this PC into a new profile tab.
    ReadPc,
    /// Read the selected rules of this PC into the open profile.
    ReadSelection,
    /// Compare the selected rules with this PC in the messages panel.
    CheckOnPc,
    SaveScript(ScriptKind),
    /// Saves the registry steps of the apply (false) or the return (true) of the selection as a
    /// registry file.
    SaveRegFile(bool),
    /// Edits the selected line of the registry file in front.
    RegFileEdit,
    /// Starts a new key or a new value in the registry file in front.
    RegFileNew(regfileview::EditKind),
    /// Asks before lines leave the registry file in front, then removes them.
    AskRegFileRemove,
    RegFileRemove(Vec<usize>),
    /// A new users file, empty or of the accounts of this PC.
    NewUsersFile,
    UsersOfThisPc,
    UsersAdd,
    UsersDuplicate,
    AskUsersRemove,
    UsersRemove(usize),
    /// Moves the selected account down (`true`) or up.
    UsersMove(bool),
    UsersUnlock,
    UsersLock,
    UsersSetMaster,
    UsersSetPassword,
    UsersCheck,
    /// Adds the accounts of this PC the file lacks.
    UsersReadPc,
    /// Asks before an operation on this PC: of every account (`true`) or the selected one.
    AskUsersRun(users::Operation, bool),
    UsersRun(users::Operation, Vec<String>),
    UsersScript,
    /// The accounts of a users file into the profile in front.
    UsersIntoProfile,
    /// Import the templates of this Windows (None), of a folder or a catalog file.
    ImportTemplates(ImportSource),
    /// Loads an import that is not loaded, or forgets one that is.
    ToggleImport(String),
    RemoveImport(String),
    /// Asks before an import and its catalog file are removed.
    AskRemoveImport(String),
    /// Asks before a tab with unsaved changes is closed without them.
    AskCloseDiscard(usize),
    /// Asks before the list of the recent files is forgotten, then forgets it.
    AskForgetRecent,
    ForgetRecent,
    /// Asks before the token of the MCP server is renewed.
    AskRenewToken,
    /// Selects an item of the tree of a profile in the group it is drawn under.
    SelectIn(Sel, Option<String>),
    /// Selects a rule of a profile in the tree of another import (the group) and shows it.
    ShowIn(String, String),
    /// Opens the tab of a loaded import, or brings it to the front, at a rule.
    OpenCatalogAt(String, String),
    /// Opens the registry tab, or brings it to the front, at a key and a value of it.
    OpenRegistry(String, Option<String>),
    /// Shows a key in the registry tab, and a value of it.
    RegistryGo(String, Option<String>),
    RegistryExpand(String, bool),
    RegistryRefresh,
    /// Edits the selected value of the registry tab.
    RegistryEdit,
    /// Starts a new key or a new value of a type in the registry tab.
    RegistryNew(regview::EditKind, &'static str),
    /// Writes what the editor of the registry tab holds.
    RegistrySave,
    /// Asks before the selected value, or the key without one, is deleted.
    AskRegistryDelete,
    RegistryDelete(String, Option<String>),
    /// Bookmarks the key and the value of the registry tab.
    BookmarkRegistry,
    /// Shows or hides the search of the registry tab.
    RegistrySearch,
    /// Starts the search of the registry tab under its selected key.
    RegistryFind,
    /// Stops the search of the registry tab.
    RegistryFindStop,
    /// Opens the window of the rights of the selected key of the registry tab.
    RegistryRights,
    /// Adds an entry for the account typed in the window of the rights.
    RegistryRightsAdd,
    /// Asks before the rights of the key change, then writes them.
    AskRegistryRights,
    RegistryRightsSave,
    /// Opens the editor of a loaded import or of the file of a catalog tab.
    EditCatalog(CatalogRef),
    /// Makes an empty catalog file among the imports and opens its editor.
    NewCatalog,
    /// Asks before a policy leaves the catalog of the editor in front.
    AskCatalogEditRemove(String),
    CatalogEditRemove(String),
    /// Compares the catalog of the active tab with another one in a page.
    CompareCatalog(CatalogRef),
    /// Looks for the catalogs of the unknown options of the active profile among the imports
    /// that are not loaded, and loads the ones that have them.
    LoadCatalogs,
    ExportImport(String),
    /// Starts to rename an import: its id and its name now.
    StartRename(String, String),
    /// Opens the tab of an import.
    OpenCatalog(String),
    CatalogSelect(String),
    /// Selects a group (`g:<id>`) or a policy of the catalog tab in front and opens its tree down
    /// to it: a link of a view.
    CatalogReveal(String),
    CatalogExpand(String, bool),
    /// Shows a policy of an import (the import, the rule) in the tree of a profile.
    ShowInProfile(String, String),
    /// Shows a group in the tree of the first profile tab.
    ShowGroupInProfile(String),
    CancelRename,
    RenameImport(String, String),
    /// Prepare the native apply (false) or the return to the defaults (true) of the selection.
    PlanNative(bool),
    PlanUndo,
    ConfirmPlan,
    CancelPlan,
    OpenReport,
    OpenGuide,
    /// Opens the license, built into the executable, in a Markdown tab.
    OpenLicense,
    OpenMarks,
    OpenMcp,
    McpStart,
    McpStop,
    McpRenewToken,
    McpOpen(String),
    GoToMark(usize),
    /// Selects a bookmark in the bookmark tab.
    MarkSelect(usize),
    SetMarkComment(usize, String),
    RemoveMark(usize),
    /// Removes a bookmark, asking first when it has a comment.
    RemoveMarkAsk(usize),
    /// The answer to the question over the window: yes runs its act.
    AskYes,
    AskNo,
    Compare,
    CompareWith(&'static str),
    ApplyDiff,
    SaveDiffAs,
    /// Selects an item of the active diff by its key.
    DiffSelect(String),
    /// Sets the comment on an item of the active diff (empty removes it).
    DiffComment(String, String),
    /// Shows an item of the active diff in the profile of its base (false) or other side (true).
    DiffGoTo(String, bool),
    /// Sets one item of the active diff in the profile of its base.
    DiffApplyItem(String),
    SetInstall(Install),
    SetAccounts(Vec<Account>),
    SetLanguages(Languages),
    Check,
    Build,
    ClearMessages,
    /// Opens the list of the message bar or folds it to one line.
    ToggleMessages,
    /// The search of the active tab was cleared: the selection comes back into view.
    SearchCleared,
    /// The selected row was scrolled into view.
    Revealed,
    /// The selection moved with the keys: it follows into view.
    Follow,
    /// Opens a page of the documentation, `name.md#heading`.
    OpenDoc(String),
    /// Shows a file in its folder in Explorer, or opens a folder of the program, made first.
    ShowInFolder(PathBuf),
    /// Opens the settings at a section (0 general, 1 policy templates, 2 files).
    OpenSettingsSection(usize),
    /// Imports the catalog file of the active tab.
    ImportOpenCatalog,
    /// Opens the editor of a theme by its id.
    EditTheme(String),
    /// Opens a theme file in its editor.
    OpenThemeFile,
    /// Makes a theme the theme of the window.
    UseTheme(String),
    Exit,
    Message(String),
}

/// What a read of this PC does with its report. A profile tab is named by its draft key, so a tab
/// moved meanwhile is found and a tab closed meanwhile is missed without harm.
enum PcJob {
    /// Every rule read into a new profile, on the profile of a tab as the base.
    NewProfile { base: Profile },
    /// The selection read into the profile of a tab.
    Selection { doc: String, base: Profile },
    /// The selected rules of the profile of a tab compared with this PC.
    Check { doc: String },
}

/// A read of this PC in a thread of its own: the features and capabilities take DISM a few
/// seconds, and the window goes on drawing meanwhile.
struct PcRead {
    job: PcJob,
    plan: thispc::Plan,
    report: std::sync::mpsc::Receiver<Report>,
}

pub struct App {
    catalog: Catalog,
    texts: Vec<CatalogTexts>,
    /// The embedded catalog and its translations, before the imports are merged.
    base_catalog: Catalog,
    base_texts: Vec<CatalogTexts>,
    /// The imports of policy templates found next to the executable.
    imports: Vec<CatalogFile>,
    importing: Option<std::sync::mpsc::Receiver<Result<CatalogFile, String>>>,
    /// The folder of the catalog files of the imports.
    imports_dir: PathBuf,
    /// The registry the registry tab reads and changes: of this PC, or in memory for the tests.
    registry: std::sync::Arc<dyn winweaver_core::registry::Registry>,
    /// The machine This PC reads: this PC, none where Windows is not, or one in memory for the
    /// tests.
    pc: Option<std::sync::Arc<dyn thispc::Machine + Send + Sync>>,
    /// A read of this PC that runs in a thread of its own.
    pc_read: Option<PcRead>,
    /// Every catalog file of that folder, loaded or not, for the settings: listed at the start,
    /// when the settings open and after a change of the imports.
    import_list: Vec<ImportEntry>,
    docs: Vec<Doc>,
    active: usize,
    settings: Settings,
    settings_dirty: bool,
    /// The recent searches of every tab.
    search_history: Vec<String>,
    /// The searches of the bookmark tab and of the MCP tab.
    marks_search: Search,
    marks_tab: marksview::MarksTab,
    /// A question over the window and the act a yes runs.
    ask: Option<Ask>,
    mcp_search: Search,
    /// Whether the message bar shows its list or one line.
    messages_open: bool,
    status: String,
    marks: Bookmarks,
    marks_path: PathBuf,
    /// The themes of the folder and the embedded ones, read at the start, when the settings
    /// open and after a theme is saved.
    themes: Vec<themes::Theme>,
    /// The themes the window shows now: light, dark and the preference.
    applied_theme: Option<(themes::Theme, themes::Theme, egui::ThemePreference)>,
    /// The colours given to the title bar last.
    title_colors: Option<TitleColors>,
    last_title: String,
    exit: bool,
    drafts_dir: PathBuf,
    logs_dir: PathBuf,
    /// What the window draws with (`renderer::graphics_text`), empty without a window.
    graphics: String,
    mcp: mcpview::McpPanel,
    /// Actions requested by views that run without the list of the frame.
    pending_acts: Vec<Act>,
    /// The section of the settings tab: general, policy templates, files.
    settings_section: usize,
    /// The import being renamed and the name typed so far.
    renaming: Option<(String, String)>,
    /// A plan waiting for the confirmation of the user.
    confirm: Option<winweaver_core::apply::Plan>,
    /// The helper that runs a confirmed plan, and what the plan left out.
    running: Option<std::sync::mpsc::Receiver<Result<winweaver_core::apply::Report, String>>>,
    running_skipped: Vec<(String, String)>,
    last_logged: String,
    /// The file the message of the status bar names, which its button shows in its folder while
    /// the message stays.
    shown_file: Option<(String, PathBuf)>,
    /// The accounts of a users file on their way into a profile, with their master password asked.
    users_import: Option<usersview::UsersImport>,
}

/// The translation for a language code, `None` for English.
pub fn texts_for<'a>(texts: &'a [CatalogTexts], code: &str) -> Option<&'a CatalogTexts> {
    texts.iter().find(|t| t.code == code)
}

impl App {
    pub fn new(catalog: Catalog, texts: Vec<CatalogTexts>, settings: Settings) -> Self {
        // The input languages and layouts of this Windows, read at the first check of a profile.
        winweaver_core::resources::set_locale_reader(winweaver_win::locales);
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0)
            .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &catalog));
        let doc = ProfileDoc::new(profile, None, &catalog);
        let search_history = settings.search_history.clone();
        Self {
            base_catalog: catalog.clone(),
            base_texts: texts.clone(),
            imports: Vec::new(),
            importing: None,
            imports_dir: imports_dir(),
            registry: winweaver_win::this_registry(),
            pc: winweaver_win::this_pc().map(std::sync::Arc::from),
            pc_read: None,
            import_list: Vec::new(),
            catalog,
            texts,
            docs: vec![Doc::Profile(Box::new(doc))],
            active: 0,
            settings,
            settings_dirty: false,
            search_history,
            marks_search: Search::default(),
            marks_tab: marksview::MarksTab::default(),
            ask: None,
            mcp_search: Search::default(),
            messages_open: true,
            status: "Ready".to_owned(),
            marks: Bookmarks::default(),
            marks_path: exe_dir().join(bookmarks::DEFAULT_FILE),
            themes: themes::themes(&themes_dir()),
            applied_theme: None,
            title_colors: None,
            last_title: String::new(),
            exit: false,
            drafts_dir: exe_dir().join("drafts"),
            logs_dir: exe_dir().join("logs"),
            graphics: String::new(),
            confirm: None,
            pending_acts: Vec::new(),
            settings_section: 0,
            renaming: None,
            running: None,
            running_skipped: Vec::new(),
            mcp: mcpview::McpPanel::new(winweaver_mcp::config::Config::default_path(&exe_dir())),
            last_logged: String::new(),
            shown_file: None,
            users_import: None,
        }
    }

    fn active_profile(&mut self) -> Option<&mut ProfileDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => Some(doc),
            _ => None,
        }
    }

    /// The search of the active tab, `None` for a tab without one.
    fn active_search(&mut self) -> Option<&mut Search> {
        match self.docs.get_mut(self.active)? {
            Doc::Profile(d) => Some(&mut d.search),
            Doc::Diff(d) => Some(&mut d.search),
            Doc::Markdown(d) => Some(&mut d.search),
            Doc::Catalog(d) => Some(&mut d.search),
            Doc::Bookmarks => Some(&mut self.marks_search),
            Doc::Mcp => Some(&mut self.mcp_search),
            Doc::RegFile(d) => Some(&mut d.search),
            Doc::Users(d) => Some(&mut d.search),
            Doc::Settings | Doc::Theme(_) | Doc::Registry(_) | Doc::CatalogEdit(_) => None,
        }
    }

    /// After the search of a tab is cleared, the selected item stays in view: the groups above
    /// it open and its row is scrolled to at the next frame.
    fn reveal_selection(&mut self) {
        let catalog = &self.catalog;
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => {
                let mut next = match &doc.sel {
                    Sel::Rule(id) => doc
                        .focus
                        .clone()
                        .or_else(|| catalog.rule(id).map(|r| r.group.clone())),
                    Sel::Group(id) => catalog.group(id).and_then(|g| g.parent.clone()),
                    Sel::Form(_) => Some(winweaver_core::catalog::ROOT_UNATTEND.to_owned()),
                    Sel::Unknown(_) => Some(UNKNOWN_GROUP.to_owned()),
                    Sel::None => None,
                };
                while let Some(g) = next {
                    next = catalog.group(&g).and_then(|x| x.parent.clone());
                    doc.expanded.insert(g);
                }
                doc.reveal = true;
            }
            Some(Doc::Catalog(c)) => {
                let mut chain = Vec::new();
                if let Some(file) = c.catalog(&self.imports) {
                    let group = |id: &str| file.groups.iter().find(|x| x.id == id);
                    let rule = |id: &str| file.rules.iter().find(|r| r.id == id);
                    // The groups above the selected policy, or above the selected group.
                    let mut next = match (c.sel.as_deref(), c.group.as_deref()) {
                        (Some(id), _) => rule(id).map(|r| r.group.clone()),
                        (None, Some(id)) => group(id).and_then(|g| g.parent.clone()),
                        (None, None) => None,
                    };
                    while let Some(g) = next {
                        next = file
                            .groups
                            .iter()
                            .find(|x| x.id == g)
                            .and_then(|x| x.parent.clone());
                        chain.push(g);
                    }
                }
                c.expanded.extend(chain);
                c.reveal = true;
            }
            Some(Doc::Diff(d)) => d.reveal = true,
            Some(Doc::Bookmarks) => self.marks_tab.reveal = true,
            Some(Doc::Mcp) => self.mcp.journal_reveal = true,
            Some(Doc::RegFile(d)) => d.follow = true,
            Some(Doc::Users(d)) => d.follow = true,
            _ => {}
        }
    }

    /// Asks before a bookmark is removed, naming its comment when it has one.
    fn ask_remove_mark(&mut self, i: usize) {
        let Some(b) = self.marks.items.get(i) else {
            return;
        };
        let t = marksview::target(
            &self.catalog,
            texts_for(&self.texts, &i18n::current()),
            &self.imports,
            b,
        );
        let text = if b.comment.trim().is_empty() {
            trf("Remove the bookmark of {title}?", &[("title", &t.title)])
        } else {
            trf(
                "The bookmark of {title} has a comment. Remove it with the comment?",
                &[("title", &t.title)],
            )
        };
        self.ask = Some(Ask {
            title: tr("Remove the bookmark").to_owned(),
            text,
            yes: tr("Remove").to_owned(),
            act: Act::RemoveMark(i),
        });
    }

    /// The question over the window, when there is one.
    fn ask_view(&self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(ask) = &self.ask else {
            return;
        };
        egui::Window::new(ask.title.as_str())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(ask.text.as_str());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(ask.yes.as_str()).clicked() {
                        acts.push(Act::AskYes);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        acts.push(Act::AskNo);
                    }
                });
            });
    }

    fn open_settings(&mut self) {
        match self.docs.iter().position(|d| matches!(d, Doc::Settings)) {
            Some(i) => self.active = i,
            None => {
                self.docs.push(Doc::Settings);
                self.active = self.docs.len() - 1;
            }
        }
    }

    /// Opens a page of the documentation a link names, such as
    /// `docs/technical/reference/18-browsers.md#microsoft-edge`, at the heading of its anchor; a
    /// link `#heading` goes to a heading of the Markdown tab that is open.
    fn open_doc(&mut self, link: &str) {
        let (file, anchor) = match link.split_once('#') {
            Some((f, a)) => (f, Some(a.to_owned())),
            None => (link, None),
        };
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        if name.is_empty() {
            if let Some(Doc::Markdown(m)) = self.docs.get_mut(self.active) {
                m.jump = anchor.is_some();
                m.anchor = anchor;
            }
            return;
        }
        let (title, text) = if name.eq_ignore_ascii_case("guide.md") {
            (
                mdview::GUIDE_TITLE.to_owned(),
                Some(mdview::guide_text(&i18n::current())),
            )
        } else {
            (
                name.to_owned(),
                mdview::REFERENCE
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, t)| (*t).to_owned()),
            )
        };
        let Some(text) = text else {
            self.status = trf("No page {name} in the documentation", &[("name", &name)]);
            return;
        };
        let i = match self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Markdown(m) if m.path.is_none() && m.title == title))
        {
            Some(i) => i,
            None => {
                self.docs
                    .push(Doc::Markdown(Box::new(MdDoc::new(title, None, text))));
                self.docs.len() - 1
            }
        };
        self.active = i;
        if let Some(Doc::Markdown(m)) = self.docs.get_mut(i) {
            m.jump = anchor.is_some();
            m.anchor = anchor;
        }
    }

    /// Opens a catalog file in a tab of its own, without importing it.
    fn open_catalog_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Catalog(c) if c.path.as_ref() == Some(&path)))
        {
            self.active = i;
            return;
        }
        match CatalogFile::load(&path) {
            Ok(file) => {
                let doc = catalogview::CatalogDoc::from_file(file, path.clone());
                self.docs.push(Doc::Catalog(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens an ADMX file as a catalog tab without importing it: its policies with the texts of
    /// the ADML next to it; the command bar of the tab offers Import.
    fn open_admx_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Catalog(c) if c.path.as_ref() == Some(&path)))
        {
            self.active = i;
            return;
        }
        let templates = admx::read_file(&path);
        if templates.policies() == 0 {
            let why = templates
                .skipped
                .first()
                .map(|s| s.1.clone())
                .unwrap_or_else(|| tr("no policies").to_owned());
            self.status = trf("Open failed: {e}", &[("e", &why)]);
            return;
        }
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Templates".to_owned());
        let id = format!("file-{:08x}", session::fnv(&kept_name(&path)) as u32);
        let file = admx::to_catalog_file(&templates, &id, &name, "", &clock::now_utc_iso());
        let doc = catalogview::CatalogDoc::from_file(file, path.clone());
        self.docs.push(Doc::Catalog(Box::new(doc)));
        self.active = self.docs.len() - 1;
        self.status = trf("Opened {path}", &[("path", &path.display())]);
        self.remember_recent(&path);
    }

    /// Imports the catalog file of the active tab: it is saved next to the executable, merged
    /// into the catalog, and the tab shows the import from now on.
    fn import_open_catalog(&mut self) {
        let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(mut file) = c.file.take() else {
            return;
        };
        if self.imports.iter().any(|f| f.id == file.id) && !file.id.starts_with("package-") {
            file.id = format!("package-{}", file.id);
        }
        c.id = file.id.clone();
        c.path = None;
        let dir = self.imports_dir.clone();
        let path = dir.join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
        let saved = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|()| file.save(&path));
        let rules = file.rules.len();
        let id = file.id.clone();
        self.imports.retain(|f| f.id != id);
        self.imports.push(*file);
        self.imports.sort_by_key(|f| import_order(&f.id));
        self.mark_loaded(&id);
        self.refresh_import_list();
        self.rebuild_catalog();
        self.status = match saved {
            Ok(()) => trf("Imported: {rules} rules", &[("rules", &rules)]),
            Err(e) => trf("Imported, but not saved: {e}", &[("e", &e)]),
        };
    }

    /// Saves the active diff to its file, or asks for one.
    fn save_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(path) = d.path.clone() else {
            self.save_active_diff();
            return;
        };
        match d.diff.save(&path) {
            Ok(()) => {
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    fn refresh_active(&mut self) {
        let catalog = &self.catalog;
        if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
            doc.refresh(catalog);
        }
    }

    /// The current position of the active tab as a bookmark, without a comment. The path of an
    /// unsaved document is its tab title, so that the bookmark finds the tab again.
    /// The bookmark of the option selected in the active tab: a profile (with its file as the
    /// link), an imported catalog or a rule of a diff (with the file of its base).
    pub(crate) fn current_bookmark(&self) -> Option<Bookmark> {
        let (item, path) = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => {
                let item = match &doc.sel {
                    Sel::None => return None,
                    Sel::Group(id) if id == UNKNOWN_GROUP => return None,
                    Sel::Group(id) | Sel::Rule(id) | Sel::Unknown(id) => id.clone(),
                    Sel::Form(f) => f.id().to_owned(),
                };
                let path = doc
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                (item, path)
            }
            Some(Doc::Catalog(c)) if c.file.is_none() => (c.sel.clone()?, String::new()),
            Some(Doc::Diff(d)) => {
                let id = d.sel.as_deref()?.strip_prefix("rule:")?;
                (id.to_owned(), d.diff.base.path.clone())
            }
            _ => return None,
        };
        Some(Bookmark::option(&item, &path))
    }

    fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let mut acts = Vec::new();
        let active = self.active;
        let count = self.docs.len();
        let confirming = self.confirm.is_some();
        let asking = self.ask.is_some();
        let mut focus_search = false;
        ctx.input_mut(|i| {
            let sc = |m, k| KeyboardShortcut::new(m, k);
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::F)) {
                focus_search = true;
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::D)) {
                acts.push(Act::Fold);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::S)) {
                acts.push(Act::SaveAs);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::S)) {
                acts.push(Act::Save);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::O)) {
                acts.push(Act::Open);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::N)) {
                acts.push(Act::NewProfile(None));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::B)) {
                acts.push(Act::ToggleBookmark);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::W)) {
                acts.push(Act::CloseTab(active, false));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::Comma)) {
                acts.push(Act::OpenSettings);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F1)) {
                acts.push(Act::OpenGuide);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F9)) {
                acts.push(Act::Build);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F7)) {
                acts.push(Act::Check);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowLeft)) {
                acts.push(Act::Back);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowRight)) {
                acts.push(Act::Forward);
            }
            if count > 1 && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab)) {
                acts.push(Act::Activate((active + count - 1) % count));
            }
            if count > 1 && i.consume_shortcut(&sc(Modifiers::CTRL, Key::Tab)) {
                acts.push(Act::Activate((active + 1) % count));
            }
            if active > 0
                && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::PageUp))
            {
                acts.push(Act::MoveTab(active, active - 1));
            }
            if active + 1 < count
                && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::PageDown))
            {
                acts.push(Act::MoveTab(active, active + 1));
            }
            if confirming && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::CancelPlan);
            }
            if asking && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::AskNo);
            }
            for file in &i.raw.dropped_files {
                acts.push(Act::OpenPath(file.path().to_path_buf()));
            }
        });
        if focus_search && let Some(search) = self.active_search() {
            search.open = true;
            search.focus = true;
        }
        acts
    }

    fn crumbs(&self) -> Vec<(String, Sel)> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let mut crumbs = vec![(doc.profile.name.clone(), Sel::None)];
        let group = match &doc.sel {
            Sel::None | Sel::Form(_) | Sel::Unknown(_) => None,
            Sel::Group(id) if id == UNKNOWN_GROUP => None,
            Sel::Group(id) => Some(id.clone()),
            Sel::Rule(id) => doc
                .focus
                .clone()
                .or_else(|| self.catalog.rule(id).map(|r| r.group.clone())),
        };
        let in_unknown = match &doc.sel {
            Sel::Unknown(_) => true,
            Sel::Group(id) => id == UNKNOWN_GROUP,
            _ => false,
        };
        if in_unknown {
            crumbs.push((
                tr("Unknown options").to_owned(),
                Sel::Group(UNKNOWN_GROUP.to_owned()),
            ));
        }
        if let Sel::Unknown(id) = &doc.sel {
            crumbs.push((id.clone(), Sel::Unknown(id.clone())));
        }
        if let Some(group) = group {
            for id in self.catalog.path_of(&group) {
                if let Some(g) = self.catalog.group(id) {
                    crumbs.push((
                        views::gtitle(texts, g).to_owned(),
                        Sel::Group(id.to_owned()),
                    ));
                }
            }
        }
        if let Sel::Rule(id) = &doc.sel
            && let Some(rule) = self.catalog.rule(id)
        {
            crumbs.push((views::rtitle(texts, rule).to_owned(), Sel::Rule(id.clone())));
        }
        if let Sel::Form(f) = &doc.sel {
            crumbs.push((tr(f.title()).to_owned(), Sel::Form(*f)));
        }
        crumbs
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        let p = style::palette(ui);
        views::header(ui, |ui| {
            ui.label(style::strong(ui, tr("Settings"), 18.0));
        });
        ui.horizontal_top(|ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 0,
                    top: 10,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(200.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let sections = [tr("General"), tr("Policy templates"), tr("Files")];
                        for (i, title) in sections.into_iter().enumerate() {
                            let selected = self.settings_section == i;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(200.0, style::row_height(ui)),
                                egui::Sense::click(),
                            );
                            if selected {
                                ui.painter()
                                    .rect_filled(rect, egui::CornerRadius::same(6), p.sel);
                            } else if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    egui::CornerRadius::same(6),
                                    p.hover,
                                );
                            }
                            ui.painter().text(
                                egui::pos2(rect.left() + 12.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                title,
                                style::font(ui, 13.0),
                                if selected { p.sel_text } else { p.text },
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    true,
                                    selected,
                                    title,
                                )
                            });
                            if response.clicked() {
                                self.settings_section = i;
                            }
                        }
                    });
                });
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings-page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        match self.settings_section {
                            1 => {
                                let mut acts = Vec::new();
                                views::block(ui, |ui| self.imports_view(ui, &mut acts));
                                self.pending_acts.extend(acts);
                            }
                            2 => views::block(ui, |ui| self.files_settings(ui)),
                            _ => self.general_settings(ui),
                        }
                        ui.add_space(12.0);
                    });
            });
        });
        if before != self.settings {
            self.settings_dirty = true;
            if before.language != self.settings.language {
                self.apply_language();
            }
        }
    }

    /// The folders next to the executable and the recent files.
    fn files_settings(&mut self, ui: &mut egui::Ui) {
        style::section_title(ui, tr("Files"));
        ui.add_space(4.0);
        ui.label(trf(
            "Settings file: {path}",
            &[("path", &Settings::path().display())],
        ));
        let folders = [
            (
                trf(
                    "Profiles folder: {path}",
                    &[("path", &profiles_dir().display())],
                ),
                profiles_dir(),
            ),
            (
                trf(
                    "Output folder: {path}",
                    &[("path", &output_dir().display())],
                ),
                output_dir(),
            ),
            (
                trf(
                    "Policy templates folder: {path}",
                    &[("path", &self.imports_dir.display())],
                ),
                self.imports_dir.clone(),
            ),
        ];
        for (text, path) in folders {
            ui.horizontal_wrapped(|ui| {
                ui.label(text);
                if ui.small_button(tr("Open")).clicked() {
                    self.pending_acts.push(Act::ShowInFolder(path));
                }
            });
        }
        ui.add_space(8.0);
        if !self.settings.recent.is_empty() && ui.button(tr("Forget recent files")).clicked() {
            self.pending_acts.push(Act::AskForgetRecent);
        }
    }

    /// Language, theme, the fonts of the theme and the behaviour of the window, in cards.
    fn general_settings(&mut self, ui: &mut egui::Ui) {
        views::block(ui, |ui| {
            style::section_title(ui, tr("Language and theme"));
            ui.add_space(4.0);
            self.language_and_theme(ui);
        });
        views::block(ui, |ui| self.theme_fonts(ui));
        views::block(ui, |ui| {
            style::section_title(ui, tr("Window"));
            ui.add_space(4.0);
            ui.checkbox(
                &mut self.settings.desc_open,
                tr("Show the description panel (Ctrl+D)"),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Open at first in the description:"));
                let parts = &mut self.settings.desc_sections;
                ui.checkbox(&mut parts.what, tr("What it does"));
                ui.checkbox(&mut parts.risk, tr("Risk"));
                ui.checkbox(&mut parts.writes, tr("What it writes"));
                ui.checkbox(&mut parts.deps, tr("Dependencies and conflicts"));
                ui.checkbox(&mut parts.more, tr("Versions, check and rollback"));
                ui.checkbox(&mut parts.reference, tr("Reference"));
            });
            ui.checkbox(
                &mut self.settings.show_guide_at_start,
                tr("Open the guide at the next start"),
            );
            ui.checkbox(
                &mut self.settings.inline_switches,
                tr("Switches at the end of the rows of the selected group"),
            );
            ui.checkbox(
                &mut self.settings.hide_unknown,
                tr("Hide the options of the profile whose catalog is not loaded"),
            );
            ui.add_space(4.0);
            self.renderer_choice(ui);
            ui.checkbox(
                &mut self.settings.restore_tabs,
                tr("Open the tabs of the last session at the start"),
            );
        });
    }

    /// The renderer of the next start, among the ones of this executable.
    fn renderer_choice(&mut self, ui: &mut egui::Ui) {
        let current = Renderer::parse(&self.settings.renderer);
        ui.horizontal(|ui| {
            ui.label(tr("Renderer"));
            egui::ComboBox::from_id_salt("renderer")
                .selected_text(current.map_or(tr("Automatic"), Renderer::title))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.renderer,
                        String::new(),
                        tr("Automatic"),
                    );
                    for &r in Renderer::ALL {
                        ui.selectable_value(
                            &mut self.settings.renderer,
                            r.id().to_owned(),
                            r.title(),
                        );
                    }
                });
        });
        let order: Vec<&str> = Renderer::ALL.iter().map(|r| r.title()).collect();
        ui.weak(trf("A change takes effect at the next start. Automatic tries {order} in this order and takes the first that starts; OpenGL takes less memory, DirectX 12 also draws without a driver of the graphics card.", &[("order", &order.join(", "))]));
        if !self.graphics.is_empty() {
            ui.weak(trf(
                "Draws now with {graphics}",
                &[("graphics", &self.graphics)],
            ));
        }
    }

    fn language_and_theme(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(tr("Language"));
            let langs = i18n::languages(&lang_dir());
            let current = if self.settings.language.is_empty() {
                tr("Follow Windows").to_owned()
            } else {
                langs
                    .iter()
                    .find(|l| l.code == self.settings.language)
                    .map_or_else(|| self.settings.language.clone(), |l| l.name.clone())
            };
            egui::ComboBox::from_id_salt("language")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.language,
                        String::new(),
                        tr("Follow Windows"),
                    );
                    for l in &langs {
                        ui.selectable_value(&mut self.settings.language, l.code.clone(), &l.name);
                    }
                });
        });
        let shown = self.shown_theme_id(ui.visuals().dark_mode);
        ui.horizontal(|ui| {
            ui.label(tr("Theme"));
            let current = if self.settings.custom_theme.is_empty()
                && self.settings.theme == ThemeChoice::System
            {
                tr("Follow Windows").to_owned()
            } else {
                themes::find(&self.themes, &shown).name
            };
            let mut choice = if self.settings.theme == ThemeChoice::System
                && self.settings.custom_theme.is_empty()
            {
                String::new()
            } else {
                shown.clone()
            };
            let before = choice.clone();
            egui::ComboBox::from_id_salt("theme")
                .selected_text(current)
                .width(200.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, String::new(), tr("Follow Windows"))
                        .on_hover_text(tr("Light or Dark, as Windows shows its apps"));
                    for t in &self.themes {
                        ui.selectable_value(&mut choice, t.id.clone(), &t.name);
                    }
                });
            if choice != before {
                if choice.is_empty() {
                    self.settings.theme = ThemeChoice::System;
                    self.settings.custom_theme.clear();
                } else {
                    let dark = themes::find(&self.themes, &choice).dark;
                    self.settings.theme = if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    };
                    self.settings.custom_theme = choice;
                }
            }
            if ui
                .button(tr("Edit the theme"))
                .on_hover_text(tr(
                    "Opens the theme file in its editor: colours, fonts, Save as",
                ))
                .clicked()
            {
                self.pending_acts.push(Act::EditTheme(shown.clone()));
            }
            if ui.button(tr("Open a theme file...")).clicked() {
                self.pending_acts.push(Act::OpenThemeFile);
            }
        });
    }

    /// The fonts of the theme the window shows, saved in its file when one changes.
    fn theme_fonts(&mut self, ui: &mut egui::Ui) {
        let shown = self.shown_theme_id(ui.visuals().dark_mode);
        let mut theme = themes::find(&self.themes, &shown);
        style::section_title(
            ui,
            &trf("Fonts of the theme {name}", &[("name", &theme.name)]),
        );
        let families = style::installed_families();
        ui.add_space(4.0);
        if themeview::fonts_grid(ui, "settings-fonts", &mut theme, &families) {
            let path = theme.file_in(&themes_dir());
            match theme.save(&path) {
                Ok(()) => {
                    self.themes = themes::themes(&themes_dir());
                    self.status = trf("Saved {path}", &[("path", &path.display())]);
                    self.shown_file = Some((self.status.clone(), path.clone()));
                }
                Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
            }
        }
        ui.label(style::muted(
            ui,
            trf(
                "The fonts belong to the theme {name}: a change is saved in its file in the themes folder.",
                &[("name", &theme.name)],
            ),
            12.0,
        ));
    }

    /// The language the window shows: the setting, or the language of Windows when it says to
    /// follow Windows, English when there is no file for it.
    fn effective_language(&self) -> String {
        let wanted = if self.settings.language.is_empty() {
            winweaver_win::ui_language().unwrap_or_else(|| "en".to_owned())
        } else {
            self.settings.language.clone()
        };
        if i18n::languages(&lang_dir())
            .iter()
            .any(|l| l.code == wanted)
        {
            wanted
        } else {
            "en".to_owned()
        }
    }

    fn apply_language(&mut self) {
        let code = self.effective_language();
        i18n::set_language(&code, &lang_dir());
    }

    fn remember_recent(&mut self, path: &Path) {
        self.settings.remember(path);
        self.settings_dirty = true;
    }

    fn open_path(&mut self, path: PathBuf) {
        if path.extension().is_some_and(|e| e == diff::EXTENSION) {
            self.open_diff_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == bookmarks::EXTENSION) {
            self.open_marks_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == admx::CATALOG_EXTENSION)
        {
            self.open_catalog_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("admx"))
        {
            self.open_admx_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == users::EXTENSION) {
            self.open_users_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == themes::EXTENSION) {
            self.open_theme_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == winweaver_mcp::config::EXTENSION)
        {
            self.open_mcp_config(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(regfile::EXTENSION))
        {
            self.open_reg_path(path);
            return;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "xml" || ext == "json" {
            self.open_foreign(path, &ext);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            self.open_md_path(path);
            return;
        }
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Profile(p) if p.path.as_ref() == Some(&path)))
        {
            self.active = i;
            return;
        }
        match Profile::load(&path) {
            Ok(mut profile) => {
                let unknown = profile.split_unknown(&self.catalog);
                let doc = ProfileDoc::new(profile, Some(path.clone()), &self.catalog);
                self.docs.push(Doc::Profile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = if unknown.is_empty() {
                    trf("Opened {path}", &[("path", &path.display())])
                } else {
                    trf(
                        "Opened {path}: {n} options of catalogs that are not loaded are left out of the build",
                        &[("path", &path.display()), ("n", &unknown.len())],
                    )
                };
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    fn save_active(&mut self, ask: bool) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Nothing to save: the active tab is not a profile").to_owned();
            return;
        };
        let path = match (&doc.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let proposed = format!("{}.{}", file_stem(&doc.profile.name), profile::EXTENSION);
                match ask_save_path(&proposed) {
                    Some(p) => with_extension(p),
                    None => return,
                }
            }
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        if doc.profile.created.is_empty() {
            doc.profile.created = clock::now_utc_iso();
        }
        doc.profile.modified = clock::now_utc_iso();
        match doc.profile.save(&path) {
            Ok(()) => {
                session::remove_draft(&self.drafts_dir, &doc.draft_key);
                doc.draft_key = session::key_for_path(&path);
                session::remove_draft(&self.drafts_dir, &doc.draft_key);
                doc.path = Some(path.clone());
                doc.dirty = false;
                doc.changed = None;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    fn check_active(&mut self) {
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            self.status = tr("Nothing to check: the active tab is not a profile").to_owned();
            return;
        };
        doc.issues = validate::check(catalog, &doc.profile, &doc.resolution);
        self.status = validate::summary(&doc.issues);
    }

    /// Checks the profile, builds the answer file and writes it with its scripts into the
    /// output folder; errors and warnings go to the messages panel.
    fn build_active(&mut self) {
        self.check_active();
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        let errors = validate::errors(&doc.issues);
        if errors > 0 {
            self.status = trf("Fix the errors before the build: {n}", &[("n", &errors)]);
            return;
        }
        let build = match render::build(catalog, &doc.profile, winweaver_core::BUILD) {
            Ok(b) => b,
            Err(e) => {
                doc.issues.insert(
                    0,
                    Issue {
                        severity: Severity::Error,
                        target: Target::Profile,
                        text: trf("Build: {e}", &[("e", &e)]),
                    },
                );
                self.status = trf("Build failed: {e}", &[("e", &e)]);
                return;
            }
        };
        for w in &build.warnings {
            doc.issues.push(Issue {
                severity: Severity::Warning,
                target: Target::Profile,
                text: trf("Build: {w}", &[("w", &w)]),
            });
        }
        let dir = output_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.status = trf(
                "Build: cannot create {path}: {e}",
                &[("path", &dir.display()), ("e", &e)],
            );
            return;
        }
        let xml_path = dir.join("autounattend.xml");
        if let Err(e) = write_file(&xml_path, build.xml.as_bytes()) {
            self.status = trf("Build: {e}", &[("e", &e)]);
            return;
        }
        for (name, text) in &build.scripts {
            let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
            if let Err(e) = write_file(&dir.join(name), text.as_bytes()) {
                self.status = trf("Build: {e}", &[("e", &e)]);
                return;
            }
        }
        self.status = trf(
            "Built {path} ({rules} rules, {scripts} scripts, {warnings} warnings)",
            &[
                ("path", &xml_path.display()),
                ("rules", &build.rule_ids.len()),
                ("scripts", &build.scripts.len()),
                ("warnings", &build.warnings.len()),
            ],
        );
        self.shown_file = Some((self.status.clone(), xml_path.clone()));
    }

    /// The rule ids of the selection of the active profile: a rule, the rules of a group, or every
    /// rule when nothing (or a form) is selected.
    fn selection_ids(&self) -> Vec<String> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        match &doc.sel {
            Sel::Rule(id) => vec![id.clone()],
            Sel::Group(group) => self
                .catalog
                .rules_under(group)
                .iter()
                .map(|r| r.id.clone())
                .collect(),
            Sel::None | Sel::Form(_) => self.catalog.rules().iter().map(|r| r.id.clone()).collect(),
            Sel::Unknown(_) => Vec::new(),
        }
    }

    /// Reads this PC: every rule into a new profile tab, or the selection into the open profile.
    fn read_pc(&mut self, only_selection: bool) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status =
                tr("Read this PC: open a profile first; its rules and forms are the base")
                    .to_owned();
            return;
        };
        let ids = if only_selection {
            self.selection_ids()
        } else {
            self.catalog
                .rules()
                .iter()
                .filter(|r| !r.id.starts_with("admx."))
                .map(|r| r.id.clone())
                .collect()
        };
        let base = doc.profile.clone();
        let plan = thispc::plan(&self.catalog, &base, &ids);
        let job = if only_selection {
            PcJob::Selection {
                doc: doc.draft_key.clone(),
                base,
            }
        } else {
            PcJob::NewProfile { base }
        };
        self.start_pc_read(job, plan, !only_selection);
    }

    /// Compares the selected rules of the profile with this PC.
    fn check_on_pc(&mut self) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let plan = thispc::plan(&self.catalog, &doc.profile, &ids);
        let job = PcJob::Check {
            doc: doc.draft_key.clone(),
        };
        self.start_pc_read(job, plan, false);
    }

    /// Starts a read of this PC in a thread of its own, one at a time; `poll_pc_read` takes its
    /// report.
    fn start_pc_read(&mut self, job: PcJob, plan: thispc::Plan, read_system: bool) {
        if self.pc_read.is_some() {
            self.status = tr("This PC is being read; wait until the read ends").to_owned();
            return;
        }
        let Some(machine) = self.pc.clone() else {
            self.status = tr("This PC can be read on Windows only").to_owned();
            return;
        };
        let checks = plan.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name("this-pc".to_owned())
            .spawn(move || {
                let time = clock::now_utc_iso();
                let _ = tx.send(thispc::run(&*machine, &checks, read_system, &time));
            });
        if let Err(e) = started {
            self.status = trf("This PC could not be read: {e}", &[("e", &e)]);
            return;
        }
        self.pc_read = Some(PcRead {
            job,
            plan,
            report: rx,
        });
        self.status = tr("This PC is being read").to_owned();
    }

    /// Takes the report of a finished read of this PC.
    fn poll_pc_read(&mut self, ctx: &egui::Context) {
        let Some(read) = &self.pc_read else {
            return;
        };
        match read.report.try_recv() {
            Ok(report) => {
                if let Some(read) = self.pc_read.take() {
                    self.finish_pc_read(read.job, &read.plan, &report);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pc_read = None;
                self.status = tr("The read of this PC failed; the log names the error").to_owned();
            }
        }
    }

    /// Puts the report of a read of this PC where its job wants it.
    fn finish_pc_read(&mut self, job: PcJob, plan: &thispc::Plan, report: &Report) {
        let readings = thispc::judge(&self.catalog, plan, &report.results);
        match job {
            PcJob::Check { doc } => self.finish_check(&doc, plan, report, &readings),
            PcJob::NewProfile { base } => self.finish_capture(None, &base, plan, report, &readings),
            PcJob::Selection { doc, base } => {
                self.finish_capture(Some(&doc), &base, plan, report, &readings)
            }
        }
    }

    /// The capture of a read: a new profile tab, or the profile of the tab named by `target`.
    fn finish_capture(
        &mut self,
        target: Option<&str>,
        base: &Profile,
        plan: &thispc::Plan,
        report: &Report,
        readings: &[thispc::Reading],
    ) {
        let (mut profile, summary) =
            thispc::capture(&self.catalog, base, readings, report.system.as_ref());
        let mut issues = capture_issues(&summary);
        if !report.admin && plan.admin_checks() > 0 {
            issues.push(admin_issue());
        }
        if !plan.excluded.is_empty() {
            issues.push(Issue {
                severity: Severity::Info,
                target: Target::Profile,
                text: format!(
                    "This PC: {} rules act only during installation or at a first sign-in and were not read",
                    plan.excluded.len()
                ),
            });
        }
        let catalog = &self.catalog;
        if let Some(key) = target {
            let Some(doc) = profile_doc(&mut self.docs, key) else {
                self.status = tr("Read this PC: the tab of the profile was closed").to_owned();
                return;
            };
            doc.profile = profile;
            doc.refresh(catalog);
            doc.issues = issues;
            self.status = format!("Read the selection on this PC: {}", summary.line());
        } else {
            let computer = if report.computer.is_empty() {
                "This PC".to_owned()
            } else {
                report.computer.clone()
            };
            profile.name = computer;
            profile.created = clock::now_utc_iso();
            profile.modified = String::new();
            let mut doc = ProfileDoc::new(profile, None, catalog);
            doc.dirty = true;
            doc.issues = issues;
            self.docs.push(Doc::Profile(Box::new(doc)));
            self.active = self.docs.len() - 1;
            let forms = if summary.forms.is_empty() {
                String::new()
            } else {
                format!("; forms: {}", summary.forms.join(", "))
            };
            self.status = format!("Read this PC: {}{forms}", summary.line());
        }
    }

    /// The result of a check of the rules of the tab named by `key`, listed in its messages.
    fn finish_check(
        &mut self,
        key: &str,
        plan: &thispc::Plan,
        report: &Report,
        readings: &[thispc::Reading],
    ) {
        let mut issues = Vec::new();
        if !report.admin && plan.admin_checks() > 0 {
            issues.push(admin_issue());
        }
        let Some(doc) = profile_doc(&mut self.docs, key) else {
            self.status = tr("Check on this PC: the tab of the profile was closed").to_owned();
            return;
        };
        let mut differ = 0;
        for r in readings {
            let on = doc.resolution.in_effect(&r.rule);
            let agrees = match r.verdict {
                Verdict::InEffect | Verdict::AsDefault => on || r.verdict == Verdict::AsDefault,
                Verdict::Partly | Verdict::NotInEffect => !on,
                Verdict::NotReadable => true,
            };
            if !agrees {
                differ += 1;
            }
            issues.push(Issue {
                severity: if agrees {
                    Severity::Info
                } else {
                    Severity::Warning
                },
                target: Target::Rule(r.rule.clone()),
                text: format!(
                    "This PC: {} ({} in the profile)",
                    r.verdict.title(),
                    if on { "on" } else { "off" }
                ),
            });
        }
        issues.sort_by_key(|i| i.severity);
        doc.issues = issues;
        self.status = trf(
            "Checked {n} rules on this PC: {differ} differ from the profile",
            &[("n", &readings.len()), ("differ", &differ)],
        );
    }

    /// Saves a stand-alone script for the selection into a folder the user picks; an apply or
    /// return script gets the undo script next to it.
    fn save_script(&mut self, kind: ScriptKind) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Save a script: the active tab is not a profile").to_owned();
            return;
        };
        let version = winweaver_core::BUILD;
        let stem = file_stem(&doc.profile.name);
        let made = match kind {
            ScriptKind::Apply => scripts::apply_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| (t, summary_of(&p), format!("Apply-{stem}.ps1"), true)),
            ScriptKind::Revert => {
                scripts::revert_script(&self.catalog, &doc.profile, &ids, version)
                    .map(|(t, p)| (t, summary_of(&p), format!("Revert-{stem}.ps1"), true))
            }
            ScriptKind::Audit => scripts::audit_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| {
                    (
                        t,
                        trf("{n} rules checked", &[("n", &p.rules.len())]),
                        format!("Audit-{stem}.ps1"),
                        false,
                    )
                }),
        };
        let (text, summary, name, with_undo) = match made {
            Ok(m) => m,
            Err(e) => {
                self.status = trf("Script not made: {e}", &[("e", &e)]);
                return;
            }
        };
        let undo = if with_undo {
            match scripts::undo_script(&doc.profile, version) {
                Ok(u) => Some(u),
                Err(e) => {
                    self.status = trf("Script not made: {e}", &[("e", &e)]);
                    return;
                }
            }
        } else {
            None
        };
        let start = output_dir();
        let dialog = rfd::FileDialog::new().set_title(tr("Choose the folder for the script"));
        let dialog = if start.is_dir() {
            dialog.set_directory(start)
        } else {
            dialog
        };
        let Some(dir) = dialog.pick_folder() else {
            return;
        };
        let path = dir.join(&name);
        if let Err(e) = write_file(&path, &scripts::file_bytes(&text)) {
            self.status = trf("Script not saved: {e}", &[("e", &e)]);
            return;
        }
        if let Some(u) = undo
            && let Err(e) = write_file(&dir.join(scripts::UNDO_NAME), &scripts::file_bytes(&u))
        {
            self.status = trf("Undo script not saved: {e}", &[("e", &e)]);
            return;
        }
        self.status = trf(
            "Saved {path} ({summary}); run it as administrator, first on a test PC",
            &[("path", &path.display()), ("summary", &summary)],
        );
        self.shown_file = Some((self.status.clone(), path));
    }

    /// Saves the registry steps of the apply or the return of the selection as a registry file, as
    /// the scripts are saved; the steps a registry file cannot hold are comments in it.
    fn save_plan_reg(&mut self, revert: bool) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Save a registry file: the active tab is not a profile").to_owned();
            return;
        };
        let stem = file_stem(&doc.profile.name);
        let profile = &doc.profile.name;
        let (plan, title, name) = if revert {
            (
                winweaver_core::apply::plan_revert(&self.catalog, &ids, "reg"),
                format!("Return the rules of {profile} to the defaults of Windows"),
                format!("Revert-{stem}.{}", regfile::EXTENSION),
            )
        } else {
            (
                winweaver_core::apply::plan_apply(&self.catalog, &doc.profile, &ids, "reg"),
                format!("Apply the profile {profile}"),
                format!("Apply-{stem}.{}", regfile::EXTENSION),
            )
        };
        let file = RegFile::from_plan(&plan, &title);
        let values = file
            .lines
            .iter()
            .filter(|l| matches!(l, Line::Value { .. }))
            .count();
        if values == 0 {
            let text = tr("Nothing for a registry file: the selection writes no registry value");
            self.status = text.to_owned();
            return;
        }
        let start = output_dir();
        let dialog = rfd::FileDialog::new()
            .set_title(tr("Save the registry file"))
            .add_filter(tr("Registry files"), &[regfile::EXTENSION])
            .set_file_name(name);
        let dialog = if start.is_dir() {
            dialog.set_directory(start)
        } else {
            dialog
        };
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = reg_extension(path);
        if let Err(e) = file.save(&path) {
            self.status = trf("Registry file not saved: {e}", &[("e", &e)]);
            return;
        }
        self.status = trf(
            "Saved {path} ({n} values); import it as administrator, first on a test PC",
            &[("path", &path.display()), ("n", &values)],
        );
        self.shown_file = Some((self.status.clone(), path));
    }

    /// Opens a registry file in a tab of its own, or brings its tab to the front.
    fn open_reg_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::RegFile(r) if r.path == path))
        {
            self.active = i;
            return;
        }
        match RegFile::load(&path) {
            Ok(file) => {
                let doc = regfileview::RegFileDoc::new(file, path.clone());
                self.docs.push(Doc::RegFile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Unlocks the passwords of the users file in front, sets or changes their master password
    /// (`set`), or sets the password of the selected account (`password`) once unlocked.
    fn users_unlock(&mut self, set: bool, password: bool) {
        let Some(d) = self.active_users() else {
            return;
        };
        if password && d.unlocked() {
            d.start_password();
        } else if d.file.passwords.is_none() || (set && d.unlocked()) {
            d.ask.open(usersview::Purpose::Set);
        } else {
            d.ask.open(usersview::Purpose::Unlock);
        }
    }

    fn active_users(&mut self) -> Option<&mut usersview::UsersDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Users(d)) => Some(d),
            _ => None,
        }
    }

    /// A new users file in a tab of its own, empty or of the accounts of this PC.
    fn new_users_doc(&mut self, read: Vec<users::User>) {
        let mut file = UsersFile::new(tr("Users"));
        file.users = read;
        let mut doc = usersview::UsersDoc::new(file, None);
        doc.dirty = !doc.file.users.is_empty();
        doc.check();
        self.docs.push(Doc::Users(Box::new(doc)));
        self.active = self.docs.len() - 1;
    }

    /// Opens a users file in a tab of its own, or brings its tab to the front.
    fn open_users_path(&mut self, path: PathBuf) {
        let same = |d: &Doc| matches!(d, Doc::Users(u) if u.path.as_ref() == Some(&path));
        if let Some(i) = self.docs.iter().position(same) {
            self.active = i;
            return;
        }
        match UsersFile::load(&path) {
            Ok(file) => {
                let mut doc = usersview::UsersDoc::new(file, Some(path.clone()));
                doc.check();
                self.docs.push(Doc::Users(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Saves the users file in front, with its passwords encrypted again, at its place or at
    /// one the user picks.
    fn save_users_doc(&mut self, ask: bool) {
        let Some(Doc::Users(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = match (&d.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = format!("{}.{}", file_stem(&d.file.name), users::EXTENSION);
                let dialog = file_dialog(tr("Users files"), users::EXTENSION)
                    .set_title(tr("Save the users file as"))
                    .set_file_name(name);
                match dialog.save_file() {
                    Some(p) if p.extension().is_some() => p,
                    Some(p) => p.with_extension(users::EXTENSION),
                    None => return,
                }
            }
        };
        let Some(Doc::Users(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        if let Err(e) = d.seal() {
            self.status = trf("Save failed: {e}", &[("e", &e)]);
            return;
        }
        match d.file.save(&path) {
            Ok(()) => {
                d.path = Some(path.clone());
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Asks before an operation of the users file in front on this PC, once its plan is sound.
    fn ask_users_run(&mut self, operation: users::Operation, all: bool) {
        if !winweaver_win::elevated() {
            self.status =
                tr("Run WinWeaver as administrator to change the accounts of this PC").to_owned();
            return;
        }
        let Some(d) = self.active_users() else {
            return;
        };
        let Some(passwords) = &d.passwords else {
            self.status = tr("Unlock the passwords first").to_owned();
            return;
        };
        let names = d.chosen(all);
        match users::Plan::new(operation, &d.file.users, &names, passwords) {
            Ok(plan) => {
                if plan.users.is_empty() {
                    self.status = tr("The file holds no accounts").to_owned();
                    return;
                }
                let text = d.question(operation, &names);
                self.ask = Some(Ask {
                    title: tr("Change this PC").to_owned(),
                    text,
                    yes: tr("Change this PC").to_owned(),
                    act: Act::UsersRun(operation, names),
                });
            }
            Err(issues) => {
                d.issues = issues;
                self.status = tr("The operation was not started: see the messages").to_owned();
            }
        }
    }

    /// Runs a confirmed operation on this PC and shows its outcomes in the tab.
    fn run_users(&mut self, operation: users::Operation, names: &[String]) {
        let Some(d) = self.active_users() else {
            return;
        };
        let Some(passwords) = &d.passwords else {
            return;
        };
        let plan = match users::Plan::new(operation, &d.file.users, names, passwords) {
            Ok(plan) => plan,
            Err(issues) => {
                d.issues = issues;
                return;
            }
        };
        let outcomes = winweaver_win::run_users_plan(&plan, passwords);
        let failed = outcomes.iter().filter(|o| o.status.failed()).count();
        let n = outcomes.len();
        d.outcomes = outcomes;
        self.status = trf(
            "This PC changed: {n} accounts, {failed} failed",
            &[("n", &n), ("failed", &failed)],
        );
    }

    /// Saves the stand-alone script of the users file in front next to the file it reads.
    fn save_users_script(&mut self) {
        let Some(Doc::Users(d)) = self.docs.get(self.active) else {
            return;
        };
        let Some(path) = d.path.clone() else {
            self.status =
                tr("Save the users file first: the script reads it next to itself").to_owned();
            return;
        };
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = match users::script(&file_name, winweaver_core::BUILD) {
            Ok(text) => text.replace("\r\n", "\n").replace('\n', "\r\n"),
            Err(e) => {
                self.status = trf("Script not made: {e}", &[("e", &e)]);
                return;
            }
        };
        let stem = file_stem(&d.file.name);
        let mut dialog = rfd::FileDialog::new()
            .set_title(tr("Save the script as"))
            .add_filter("PowerShell", &["ps1"])
            .set_file_name(format!("Create-{stem}.ps1"));
        if let Some(dir) = path.parent().filter(|p| p.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(target) = dialog.save_file() else {
            return;
        };
        match write_file(&target, text.as_bytes()) {
            Ok(()) => {
                self.status = trf(
                    "Saved {path}; keep {file} next to it and run it as administrator",
                    &[("path", &target.display()), ("file", &file_name)],
                );
                self.shown_file = Some((self.status.clone(), target));
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Loads the accounts of a users file into the profile in front, asking the master password
    /// for their passwords when the file holds some.
    fn users_into_profile(&mut self) {
        if self.active_profile().is_none() {
            return;
        }
        let dialog = file_dialog(tr("Users files"), users::EXTENSION)
            .set_title(tr("Accounts from a users file"));
        let Some(path) = dialog.pick_file() else {
            return;
        };
        match UsersFile::load(&path) {
            Ok(file) if file.passwords.is_some() => {
                let mut ask = usersview::MasterAsk::default();
                ask.open(usersview::Purpose::Unlock);
                self.users_import = Some(usersview::UsersImport { file, ask });
            }
            Ok(file) => {
                let n = file.users.len();
                self.apply(vec![Act::SetAccounts(file.accounts(None))]);
                self.status = trf("{n} accounts loaded without passwords", &[("n", &n)]);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// The window of the master password of a users file loaded into a profile.
    fn users_import_view(&mut self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(import) = &mut self.users_import else {
            return;
        };
        match usersview::import_window(ctx, import) {
            usersview::ImportDone::Open => {}
            usersview::ImportDone::Cancel => self.users_import = None,
            usersview::ImportDone::Without => {
                acts.push(Act::SetAccounts(import.file.accounts(None)));
                self.users_import = None;
            }
            usersview::ImportDone::With(passwords) => {
                acts.push(Act::SetAccounts(import.file.accounts(Some(&passwords))));
                self.users_import = None;
            }
        }
    }

    /// Saves the registry file in front at its place, or at one the user picks.
    fn save_reg_doc(&mut self, ask: bool) {
        let Some(Doc::RegFile(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = if ask {
            let name = d.title();
            let dialog = rfd::FileDialog::new()
                .set_title(tr("Save the registry file as"))
                .add_filter(tr("Registry files"), &[regfile::EXTENSION])
                .set_file_name(name);
            let dialog = match d.path.parent() {
                Some(dir) if dir.is_dir() => dialog.set_directory(dir),
                _ => dialog,
            };
            match dialog.save_file() {
                Some(p) => reg_extension(p),
                None => return,
            }
        } else {
            d.path.clone()
        };
        let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.file.save(&path) {
            Ok(()) => {
                d.path = path.clone();
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens the report of an audit script made on another PC and reads it into a new profile.
    fn open_report(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Open an audit report"))
            .add_filter(tr("Audit report"), &["json"])
            .pick_file()
        else {
            return;
        };
        match Report::load(&path) {
            Ok(report) => self.open_report_from(report, &path),
            Err(e) => self.status = trf("Report not read: {e}", &[("e", &e)]),
        }
    }

    /// Reads an audit report into a new profile, on the base of the open profile.
    fn open_report_from(&mut self, report: Report, path: &Path) {
        let base = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => doc.profile.clone(),
            _ => Profile::parse(PRESETS[0].1, PRESETS[0].0)
                .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &self.catalog)),
        };
        let mut ids: Vec<String> = Vec::new();
        for r in &report.results {
            if !ids.contains(&r.rule) {
                ids.push(r.rule.clone());
            }
        }
        let plan = thispc::plan(&self.catalog, &base, &ids);
        let readings = thispc::judge(&self.catalog, &plan, &report.results);
        let (mut profile, summary) =
            thispc::capture(&self.catalog, &base, &readings, report.system.as_ref());
        profile.name = if report.computer.is_empty() {
            "Audit report".to_owned()
        } else {
            report.computer.clone()
        };
        let mut doc = ProfileDoc::new(profile, None, &self.catalog);
        doc.dirty = true;
        doc.issues = capture_issues(&summary);
        self.docs.push(Doc::Profile(Box::new(doc)));
        self.active = self.docs.len() - 1;
        self.status = format!("Read the report {}: {}", path.display(), summary.line());
    }

    /// Prepares the native apply or return of the selection and asks for the confirmation.
    fn plan_native(&mut self, revert: bool) {
        if self.running.is_some() {
            self.status = tr("A change of this PC is running; wait for its report").to_owned();
            return;
        }
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Open a profile first").to_owned();
            return;
        };
        let nonce = match winweaver_mcp::new_token() {
            Ok(n) => n,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let plan = if revert {
            winweaver_core::apply::plan_revert(&self.catalog, &ids, &nonce)
        } else {
            winweaver_core::apply::plan_apply(&self.catalog, &doc.profile, &ids, &nonce)
        };
        if plan.steps.is_empty() {
            self.status = trf(
                "Nothing to change natively; {n} steps are left to the stand-alone script",
                &[("n", &plan.skipped.len())],
            );
            return;
        }
        self.confirm = Some(plan);
    }

    /// Prepares the undo of a backup file the user picks.
    fn plan_undo(&mut self) {
        let start = output_dir();
        let dialog = rfd::FileDialog::new()
            .set_title(tr("Choose the backup to restore"))
            .add_filter(tr("Backups"), &["json"]);
        let dialog = if start.is_dir() {
            dialog.set_directory(start)
        } else {
            dialog
        };
        let Some(path) = dialog.pick_file() else {
            return;
        };
        let entries = match winweaver_core::apply::read_backup(&path) {
            Ok(e) => e,
            Err(e) => {
                self.status = trf("Backup not read: {e}", &[("e", &e)]);
                return;
            }
        };
        let nonce = match winweaver_mcp::new_token() {
            Ok(n) => n,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let plan = winweaver_core::apply::plan_undo(&entries, &nonce);
        if plan.steps.is_empty() {
            self.status = tr("The backup holds nothing to restore").to_owned();
            return;
        }
        self.confirm = Some(plan);
    }

    /// Writes the confirmed plan into the output folder and runs the elevated helper on it in a
    /// thread; the report comes back through a channel.
    fn run_plan(&mut self) {
        let Some(plan) = self.confirm.take() else {
            return;
        };
        let dir = output_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.status = trf("The plan was not written: {e}", &[("e", &e)]);
            return;
        }
        let stamp = clock::now_utc_iso()
            .replace([':', '-'], "")
            .replace('T', "-")
            .trim_end_matches('Z')
            .to_owned();
        let path = dir.join(format!("plan-{stamp}.json"));
        let text = plan.to_json();
        let digest = winweaver_core::apply::digest(text.as_bytes());
        if let Err(e) = write_file(&path, text.as_bytes()) {
            self.status = trf("The plan was not written: {e}", &[("e", &e)]);
            return;
        }
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        self.running_skipped = plan.skipped.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let args = vec![
                "--elevated-apply".to_owned(),
                path.display().to_string(),
                digest,
            ];
            let result = winweaver_win::run_elevated(&exe, &args).and_then(|code| {
                let report_path = path.with_file_name(format!("plan-{stamp}-report.json"));
                let read = std::fs::read_to_string(&report_path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| {
                        serde_json::from_str::<winweaver_core::apply::Report>(&t)
                            .map_err(|e| e.to_string())
                    });
                let _ = std::fs::remove_file(&report_path);
                match (code, read) {
                    (0, Ok(report)) => Ok(report),
                    (0, Err(e)) => Err(e),
                    (code, _) => Err(format!("the helper ended with the code {code}")),
                }
            });
            let _ = std::fs::remove_file(&path);
            let _ = tx.send(result);
        });
        self.running = Some(rx);
        self.status = tr("Windows asks for the permission; the change of this PC runs").to_owned();
    }

    /// Takes the report of a finished helper.
    fn poll_running(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.running else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.running = None;
                let skipped = std::mem::take(&mut self.running_skipped);
                match result {
                    Ok(report) => {
                        let mut issues: Vec<Issue> = report
                            .results
                            .iter()
                            .filter(|r| r.status != "ok")
                            .map(|r| Issue {
                                severity: if r.status == "failed" {
                                    Severity::Warning
                                } else {
                                    Severity::Info
                                },
                                target: Target::Rule(r.rule.clone()),
                                text: format!("{}: {} {}", r.status, r.what, r.note),
                            })
                            .collect();
                        issues.extend(skipped.into_iter().map(|(rule, why)| Issue {
                            severity: Severity::Info,
                            target: Target::Rule(rule),
                            text: trf("Left to the stand-alone script: {why}", &[("why", &why)]),
                        }));
                        if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
                            doc.issues = issues;
                        }
                        self.status = trf(
                            "This PC changed: {ok} done, {failed} failed, {skipped} skipped; backup {backup}",
                            &[
                                ("ok", &report.count("ok")),
                                ("failed", &report.count("failed")),
                                ("skipped", &report.count("skipped")),
                                (
                                    "backup",
                                    &if report.backup.is_empty() {
                                        "-".to_owned()
                                    } else {
                                        report.backup.clone()
                                    },
                                ),
                            ],
                        );
                    }
                    Err(e) => self.status = trf("This PC was not changed: {e}", &[("e", &e)]),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.running = None,
        }
    }

    /// The confirmation of a plan, over the window.
    fn confirm_view(&self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(plan) = &self.confirm else {
            return;
        };
        let title = match plan.purpose.as_str() {
            "revert" => tr("Return to Windows defaults"),
            "undo" => tr("Undo a change"),
            _ => tr("Apply to this PC"),
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(trf(
                    "{n} changes of this PC run as administrator; Windows asks for the permission first.",
                    &[("n", &plan.changes())],
                ));
                if !plan.skipped.is_empty() {
                    ui.label(trf(
                        "{n} steps are left to the stand-alone script (scripts, the profiles of other users, values without a known Windows default).",
                        &[("n", &plan.skipped.len())],
                    ));
                }
                if plan.uses_default_user() {
                    ui.label(tr("The default user profile changes too: accounts created from now on get its settings."));
                }
                if plan.removals() > 0 {
                    ui.label(trf(
                        "{n} steps remove apps or capabilities of Windows; Undo does not bring them back.",
                        &[("n", &plan.removals())],
                    ));
                }
                if plan.irreversible() > 0 {
                    ui.label(trf(
                        "{n} steps run programs, change accounts or delete copies of the answer file; Undo does not take them back.",
                        &[("n", &plan.irreversible())],
                    ));
                }
                ui.label(tr("Every registry value is saved to a backup in the output folder before its change; Undo restores it."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("Change this PC")).clicked() {
                        acts.push(Act::ConfirmPlan);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        acts.push(Act::CancelPlan);
                    }
                });
            });
    }

    /// Reads the catalog files of the imports the settings load, and no other file; the
    /// templates of this Windows first, so that they win a policy two imports hold. Settings of
    /// an earlier version load every import but the hidden ones, and name them from now on.
    fn load_imports(&mut self) {
        let ids = loaded_import_ids(&self.settings, &self.imports_dir);
        if self.settings.loaded_imports.is_none() {
            self.settings.loaded_imports = Some(ids.clone());
            self.settings.hidden_imports.clear();
            self.settings_dirty = true;
        }
        let (files, broken) = read_imports(&self.imports_dir, &ids);
        self.imports = files;
        if !broken.is_empty() {
            self.status = trf(
                "{n} imports of policy templates could not be read",
                &[("n", &broken.len())],
            );
        }
        self.refresh_import_list();
        self.rebuild_catalog();
    }

    /// Lists the catalog files of the imports folder for the settings; of a file that is not
    /// loaded only the header is read.
    fn refresh_import_list(&mut self) {
        self.import_list = import_entries(&self.imports_dir, &self.imports);
    }

    /// Reads the catalog file of an import, unless it is loaded, and marks it in the settings.
    fn load_import(&mut self, id: &str) -> Result<(), String> {
        if !self.imports.iter().any(|f| f.id == id) {
            let path = self
                .imports_dir
                .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
            let mut file = CatalogFile::load(&path)?;
            file.id = id.to_owned();
            self.imports.push(file);
            self.imports.sort_by_key(|f| import_order(&f.id));
        }
        self.mark_loaded(id);
        Ok(())
    }

    /// Names an import among the loaded ones of the settings.
    fn mark_loaded(&mut self, id: &str) {
        let list = self.settings.loaded_imports.get_or_insert_with(Vec::new);
        if !list.iter().any(|x| x == id) {
            list.push(id.to_owned());
            self.settings_dirty = true;
        }
    }

    /// Forgets an import: out of the settings, out of memory, and its tab closes.
    fn unload_import(&mut self, id: &str) {
        self.imports.retain(|f| f.id != id);
        if let Some(list) = &mut self.settings.loaded_imports {
            list.retain(|x| x != id);
        }
        self.settings_dirty = true;
        let active = self.active;
        let mut index = 0;
        let mut removed_before = 0;
        self.docs.retain(|d| {
            let keep = !matches!(d, Doc::Catalog(c) if c.file.is_none() && c.id == id);
            if !keep && index < active {
                removed_before += 1;
            }
            index += 1;
            keep
        });
        self.active = active
            .saturating_sub(removed_before)
            .min(self.docs.len().saturating_sub(1));
    }

    /// Looks for the options of the active profile whose catalog is not loaded in the catalog
    /// files of the imports that are not loaded, reading only the ids of their rules, and loads
    /// the files that have some of them: the templates of this Windows first, no file more than
    /// needed.
    fn load_catalogs_for_unknown(&mut self) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let mut wanted: HashSet<String> = doc.profile.unknown.keys().cloned().collect();
        if wanted.is_empty() {
            return;
        }
        let mut found = Vec::new();
        for (id, path) in import_files(&self.imports_dir) {
            if wanted.is_empty() {
                break;
            }
            if self.imports.iter().any(|f| f.id == id) {
                continue;
            }
            let Ok(ids) = admx::rule_ids(&path) else {
                continue;
            };
            let before = wanted.len();
            wanted.retain(|w| !ids.contains(w));
            if wanted.len() < before {
                found.push(id);
            }
        }
        if found.is_empty() {
            self.status = tr("No imported catalog has these options: import their policy templates in the settings").to_owned();
            return;
        }
        let mut failed = Vec::new();
        for id in &found {
            if let Err(e) = self.load_import(id) {
                failed.push(e);
            }
        }
        self.refresh_import_list();
        self.rebuild_catalog();
        self.check_active();
        let left = match self.docs.get(self.active) {
            Some(Doc::Profile(d)) => d.profile.unknown.len(),
            _ => 0,
        };
        self.status = match failed.first() {
            Some(e) => trf("A catalog was not loaded: {e}", &[("e", e)]),
            None => trf(
                "Loaded {n} catalogs; {left} options are still unknown",
                &[("n", &(found.len() - failed.len())), ("left", &left)],
            ),
        };
    }

    /// Merges the loaded imports into the catalog and brings the choices of the open profiles in
    /// line with it.
    fn rebuild_catalog(&mut self) {
        match admx::merge(&self.base_catalog, &self.imports) {
            Ok((catalog, notes)) => {
                self.catalog = catalog;
                if !notes.is_empty() {
                    self.status = notes.join("; ");
                }
            }
            Err(e) => {
                self.status = trf("The imports were not merged: {e}", &[("e", &e)]);
                self.catalog = self.base_catalog.clone();
            }
        }
        let mut texts = self.base_texts.clone();
        admx::merge_texts(&mut texts, &self.imports);
        self.texts = texts;
        let catalog = &self.catalog;
        for doc in &mut self.docs {
            if let Doc::Profile(d) = doc {
                d.profile.take_known(catalog);
                d.profile.split_unknown(catalog);
                d.resolution = resolver::resolve(catalog, &d.profile);
                // The selection follows an option between the catalog and the unknown ones.
                d.sel = match std::mem::replace(&mut d.sel, Sel::None) {
                    Sel::Unknown(id) if catalog.rule(&id).is_some() => Sel::Rule(id),
                    Sel::Rule(id) if catalog.rule(&id).is_none() => {
                        if d.profile.unknown.contains_key(&id) {
                            Sel::Unknown(id)
                        } else {
                            Sel::None
                        }
                    }
                    other => other,
                };
            }
        }
    }

    /// Starts an import in a thread: the templates of this Windows, a folder or a catalog file.
    fn start_import(&mut self, source: ImportSource) {
        if self.importing.is_some() {
            return;
        }
        let path = match source {
            ImportSource::ThisWindows => None,
            ImportSource::Folder => match rfd::FileDialog::new()
                .set_title(tr("Choose the folder of the ADMX files"))
                .pick_folder()
            {
                Some(p) => Some(p),
                None => return,
            },
            ImportSource::CatalogFile => match rfd::FileDialog::new()
                .set_title(tr("Open a catalog file"))
                .add_filter(tr("Catalog files"), &[admx::CATALOG_EXTENSION])
                .pick_file()
            {
                Some(p) => Some(p),
                None => return,
            },
            ImportSource::Archive => match rfd::FileDialog::new()
                .set_title(tr("Open an archive of templates or of a catalog file"))
                .add_filter(tr("Zip archives"), &[admx::ARCHIVE_EXTENSION])
                .pick_file()
            {
                Some(p) => Some(p),
                None => return,
            },
        };
        let build = winweaver_win::this_pc()
            .map(|m| m.system_info().build)
            .unwrap_or_default();
        let taken: Vec<String> = self.imports.iter().map(|f| f.id.clone()).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let now = clock::now_utc_iso();
            let result = match (source, path) {
                (ImportSource::ThisWindows, _) => {
                    let t =
                        admx::read_folder(std::path::Path::new("C:\\Windows\\PolicyDefinitions"));
                    let name = if build.is_empty() {
                        "Policy templates of this Windows".to_owned()
                    } else {
                        format!("Policy templates of this Windows ({build})")
                    };
                    Ok(admx::to_catalog_file(&t, "system", &name, &build, &now))
                }
                (ImportSource::Folder, Some(dir)) => {
                    let t = admx::read_folder(&dir);
                    let name = dir
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Templates".to_owned());
                    let id = format!("folder-{:08x}", session::fnv(&kept_name(&dir)) as u32);
                    Ok(admx::to_catalog_file(&t, &id, &name, "", &now))
                }
                (ImportSource::CatalogFile, Some(file)) => CatalogFile::load(&file).map(|mut f| {
                    if taken.contains(&f.id) && !f.id.starts_with("package-") {
                        f.id = format!("package-{}", f.id);
                    }
                    f
                }),
                (ImportSource::Archive, Some(file)) => match admx::read_archive(&file) {
                    Ok(admx::Archive::Templates(t)) => {
                        let name = file
                            .file_stem()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Templates".to_owned());
                        let id = format!("archive-{:08x}", session::fnv(&kept_name(&file)) as u32);
                        Ok(admx::to_catalog_file(&t, &id, &name, "", &now))
                    }
                    Ok(admx::Archive::Catalog(f)) => {
                        let mut f = *f;
                        if taken.contains(&f.id) && !f.id.starts_with("package-") {
                            f.id = format!("package-{}", f.id);
                        }
                        Ok(f)
                    }
                    Err(e) => Err(e),
                },
                _ => Err("nothing to import".to_owned()),
            };
            let _ = tx.send(result);
        });
        self.importing = Some(rx);
        self.status = tr("The policy templates are being read").to_owned();
    }

    /// Takes the result of a finished import: saved next to the executable, merged.
    fn poll_import(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.importing else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.importing = None;
                match result {
                    Ok(file) => {
                        let dir = self.imports_dir.clone();
                        let path = dir.join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
                        let saved = std::fs::create_dir_all(&dir)
                            .map_err(|e| e.to_string())
                            .and_then(|()| file.save(&path));
                        let rules = file.rules.len();
                        let skipped = file.skipped.len();
                        let id = file.id.clone();
                        self.imports.retain(|f| f.id != id);
                        self.imports.push(file);
                        self.imports.sort_by_key(|f| import_order(&f.id));
                        self.mark_loaded(&id);
                        self.refresh_import_list();
                        self.rebuild_catalog();
                        self.status = match saved {
                            Ok(()) => trf(
                                "Imported: {rules} rules, {skipped} policies skipped",
                                &[("rules", &rules), ("skipped", &skipped)],
                            ),
                            Err(e) => trf("Imported, but not saved: {e}", &[("e", &e)]),
                        };
                    }
                    Err(e) => self.status = trf("The import failed: {e}", &[("e", &e)]),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(250))
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.importing = None,
        }
    }

    /// Opens the tab of an import, or brings it to the front.
    fn open_catalog(&mut self, id: &str) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Catalog(c) if c.id == id))
        {
            self.active = i;
            return;
        }
        let Some(file) = self.imports.iter().find(|f| f.id == id) else {
            return;
        };
        self.docs
            .push(Doc::Catalog(Box::new(catalogview::CatalogDoc::new(file))));
        self.active = self.docs.len() - 1;
    }

    /// Shows a policy of an import in the tree of a profile: an import that is not loaded is, the
    /// first profile tab comes to the front, the groups above the policy open and it is selected.
    fn show_in_profile(&mut self, import: &str, rule: &str) {
        if !import.is_empty() && !self.imports.iter().any(|f| f.id == import) {
            if let Err(e) = self.load_import(import) {
                self.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                return;
            }
            self.refresh_import_list();
            self.rebuild_catalog();
        }
        let Some(index) = self.docs.iter().position(|d| matches!(d, Doc::Profile(_))) else {
            self.status = tr("Open a profile first").to_owned();
            return;
        };
        let mut groups = Vec::new();
        let mut group = self.catalog.rule(rule).map(|r| r.group.clone());
        while let Some(g) = group {
            group = self.catalog.group(&g).and_then(|x| x.parent.clone());
            groups.push(g);
        }
        self.active = index;
        if let Some(Doc::Profile(doc)) = self.docs.get_mut(index) {
            doc.expanded.extend(groups);
            doc.select(Sel::Rule(rule.to_owned()));
            doc.focus = None;
            doc.reveal = true;
        }
    }

    /// Gives an import another name, in the settings and as the title of its tree.
    fn rename_import(&mut self, id: &str, name: &str) {
        self.renaming = None;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
            self.status =
                tr("A name of 1 to 120 characters without control characters is needed").to_owned();
            return;
        }
        let Some(file) = self.imports.iter_mut().find(|f| f.id == id) else {
            return;
        };
        file.name = name.to_owned();
        let root = Some(winweaver_core::catalog::ROOT_POLICIES);
        for group in file
            .groups
            .iter_mut()
            .filter(|g| g.parent.as_deref() == root)
        {
            group.title = name.to_owned();
        }
        let path = self
            .imports_dir
            .join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
        let saved = file.save(&path);
        for doc in &mut self.docs {
            if let Doc::Catalog(c) = doc
                && c.id == id
            {
                c.name = name.to_owned();
            }
        }
        self.refresh_import_list();
        self.rebuild_catalog();
        self.status = match saved {
            Ok(()) => trf("Renamed to {name}", &[("name", &name)]),
            Err(e) => trf("Renamed, but not saved: {e}", &[("e", &e)]),
        };
    }

    fn export_import(&mut self, id: &str) {
        let Some(file) = self.imports.iter().find(|f| f.id == id) else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Export the catalog file"))
            .add_filter(tr("Catalog files"), &[admx::CATALOG_EXTENSION])
            .set_file_name(format!("{}.{}", file.id, admx::CATALOG_EXTENSION))
            .save_file()
        else {
            return;
        };
        match file.save(&path) {
            Ok(()) => {
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path));
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// The section of the settings that lists the imports of policy templates.
    fn imports_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        style::section_title(ui, tr("Policy templates"));
        ui.weak(tr("Imported ADMX templates become rules under Standard policies. Only the checked catalogs are read and loaded; when two of them hold a policy, the templates of this Windows win."));
        for entry in &self.import_list {
            let file = self.imports.iter().find(|f| f.id == entry.id);
            ui.horizontal(|ui| {
                let mut loaded = file.is_some();
                if ui.checkbox(&mut loaded, &entry.name).changed() {
                    acts.push(Act::ToggleImport(entry.id.clone()));
                }
                match file {
                    Some(f) => {
                        ui.weak(trf(
                            "{rules} rules, {skipped} skipped",
                            &[("rules", &f.rules.len()), ("skipped", &f.skipped.len())],
                        ));
                        if ui.small_button(tr("Export...")).clicked() {
                            acts.push(Act::ExportImport(f.id.clone()));
                        }
                        if ui.small_button(tr("Open")).clicked() {
                            acts.push(Act::OpenCatalog(f.id.clone()));
                        }
                        if ui.small_button(tr("Rename")).clicked() {
                            acts.push(Act::StartRename(f.id.clone(), f.name.clone()));
                        }
                    }
                    None => {
                        ui.weak(trf(
                            "not loaded, {size}",
                            &[("size", &size_text(entry.bytes))],
                        ));
                    }
                }
                if ui.small_button(tr("Remove")).clicked() {
                    acts.push(Act::AskRemoveImport(entry.id.clone()));
                }
            });
            if let Some((id, name)) = &mut self.renaming
                && id == &entry.id
            {
                ui.horizontal(|ui| {
                    let edit = ui.text_edit_singleline(name);
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.small_button(tr("OK")).clicked() || enter {
                        acts.push(Act::RenameImport(id.clone(), name.clone()));
                    }
                    if ui.small_button(tr("Cancel")).clicked() {
                        acts.push(Act::CancelRename);
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            let busy = self.importing.is_some();
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(tr("Import the templates of this Windows")),
                )
                .clicked()
            {
                acts.push(Act::ImportTemplates(ImportSource::ThisWindows));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import a folder...")))
                .clicked()
            {
                acts.push(Act::ImportTemplates(ImportSource::Folder));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import a catalog file...")))
                .clicked()
            {
                acts.push(Act::ImportTemplates(ImportSource::CatalogFile));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import an archive...")))
                .on_hover_text(tr(
                    "A zip archive of ADMX templates with their ADML, or of one catalog file; it is read without unpacking",
                ))
                .clicked()
            {
                acts.push(Act::ImportTemplates(ImportSource::Archive));
            }
        });
    }

    fn apply(&mut self, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Select(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = None;
                    }
                }
                Act::Reveal(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = None;
                    }
                    self.reveal_selection();
                }
                Act::SelectIn(sel, group) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = group;
                    }
                }
                Act::ShowIn(rule, group) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(Sel::Rule(rule));
                        doc.focus = Some(group);
                    }
                    self.reveal_selection();
                }
                Act::OpenCatalogAt(import, rule) => {
                    self.open_catalog(&import);
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active)
                        && c.id == import
                    {
                        c.group = None;
                        c.sel = Some(rule);
                    }
                    self.reveal_selection();
                }
                Act::Toggle(id, on) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.set_on(&id, on);
                    }
                    self.refresh_active();
                }
                Act::SetGroup(group, on) => {
                    let ids: Vec<String> = self
                        .catalog
                        .rules_under(&group)
                        .iter()
                        .filter(|r| !on || !r.id.starts_with("admx."))
                        .map(|r| r.id.clone())
                        .collect();
                    if let Some(doc) = self.active_profile() {
                        for id in &ids {
                            doc.profile.set_on(id, on);
                        }
                    }
                    self.refresh_active();
                }
                Act::SetParam(id, name, value) => {
                    let default = self
                        .catalog
                        .rule(&id)
                        .and_then(|r| r.params.get(&name))
                        .map(|p| p.default_value());
                    if let Some(doc) = self.active_profile() {
                        let choice = doc.profile.rules.entry(id).or_default();
                        if default
                            .as_ref()
                            .is_some_and(|d| check::same_value(d, &value))
                        {
                            choice.params.shift_remove(&name);
                        } else {
                            choice.params.insert(name, value);
                        }
                    }
                    self.refresh_active();
                }
                Act::Expand(group, open) => {
                    if let Some(doc) = self.active_profile() {
                        if open {
                            doc.expanded.insert(group);
                        } else {
                            doc.expanded.remove(&group);
                        }
                    }
                }
                Act::Activate(i) => {
                    if i < self.docs.len() {
                        self.active = i;
                    }
                }
                Act::MoveTab(from, to) => self.move_tab(from, to),
                Act::CloseTab(i, force) => {
                    if i < self.docs.len() {
                        if self.docs[i].dirty() && !force {
                            self.status = tr("The tab has unsaved changes: save it (Ctrl+S) or close it from the menu discarding them").to_owned();
                        } else {
                            if let Doc::Profile(d) = &self.docs[i] {
                                session::remove_draft(&self.drafts_dir, &d.draft_key);
                            }
                            self.docs.remove(i);
                            if self.active >= self.docs.len() {
                                self.active = self.docs.len().saturating_sub(1);
                            }
                        }
                    }
                }
                Act::NewProfile(preset) => {
                    let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                        Some((name, text)) => Profile::parse(text, name)
                            .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog)),
                        None => Profile::new("New profile", self.catalog.version()),
                    };
                    let doc = ProfileDoc::new(profile, None, &self.catalog);
                    self.docs.push(Doc::Profile(Box::new(doc)));
                    self.active = self.docs.len() - 1;
                }
                Act::Open => {
                    if let Some(path) = pick_any() {
                        self.open_path(path);
                    }
                }
                Act::OpenPath(path) => self.open_path(path),
                Act::OpenSettings => {
                    self.themes = themes::themes(&themes_dir());
                    self.refresh_import_list();
                    self.open_settings();
                }
                Act::ImportOpenCatalog => self.import_open_catalog(),
                Act::OpenSettingsSection(section) => {
                    self.settings_section = section;
                    self.open_settings();
                }
                Act::Save => match self.docs.get(self.active) {
                    Some(Doc::Mcp) => self.persist_mcp(),
                    Some(Doc::Diff(_)) => self.save_diff(),
                    Some(Doc::Theme(_)) => self.save_theme(false),
                    Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                    Some(Doc::RegFile(_)) => self.save_reg_doc(false),
                    Some(Doc::Users(_)) => self.save_users_doc(false),
                    _ => self.save_active(false),
                },
                Act::SaveAs => match self.docs.get(self.active) {
                    Some(Doc::Mcp) => self.save_mcp_as(),
                    Some(Doc::Diff(_)) => self.save_active_diff(),
                    Some(Doc::Theme(_)) => self.save_theme(true),
                    Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                    Some(Doc::RegFile(_)) => self.save_reg_doc(true),
                    Some(Doc::Users(_)) => self.save_users_doc(true),
                    _ => self.save_active(true),
                },
                Act::EditTheme(id) => {
                    let theme = themes::find(&self.themes, &id);
                    self.edit_theme(theme);
                }
                Act::OpenThemeFile => {
                    let dir = themes_dir();
                    let dialog = rfd::FileDialog::new()
                        .add_filter(tr("WinWeaver theme"), &[themes::EXTENSION, "json"])
                        .set_title(tr("Open a theme file"));
                    let dialog = if dir.is_dir() {
                        dialog.set_directory(dir)
                    } else {
                        dialog
                    };
                    if let Some(path) = dialog.pick_file() {
                        self.open_theme_path(path);
                    }
                }
                Act::UseTheme(id) => {
                    let dark = themes::find(&self.themes, &id).dark;
                    self.settings.theme = if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    };
                    self.settings.custom_theme = id;
                    self.settings_dirty = true;
                }
                Act::Back => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(-1);
                    }
                }
                Act::Forward => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(1);
                    }
                }
                Act::ToggleBookmark => {
                    if let Some(mut mark) = self.current_bookmark() {
                        if let Some(j) = self.marks.find(&mark)
                            && !self.marks.items[j].comment.trim().is_empty()
                        {
                            self.ask_remove_mark(j);
                            continue;
                        }
                        mark.created = clock::now_utc_iso();
                        let added = self.marks.toggle(mark);
                        if self.save_marks() {
                            let text = if added {
                                tr("Bookmark added ({n} bookmarks in {path})")
                            } else {
                                tr("Bookmark removed ({n} bookmarks in {path})")
                            };
                            self.status = text
                                .replace("{n}", &self.marks.items.len().to_string())
                                .replace("{path}", &self.marks_path.display().to_string());
                        }
                    }
                }
                Act::OpenMcp => match self.docs.iter().position(|d| matches!(d, Doc::Mcp)) {
                    Some(i) => self.active = i,
                    None => {
                        self.docs.push(Doc::Mcp);
                        self.active = self.docs.len() - 1;
                    }
                },
                Act::McpStart => self.start_mcp(),
                Act::McpStop => {
                    self.mcp.run = None;
                    self.status = tr("The MCP server stopped").to_owned();
                }
                Act::McpRenewToken => match winweaver_mcp::new_token() {
                    Ok(t) => self.mcp.token = t,
                    Err(e) => self.mcp.error = e,
                },
                Act::McpOpen(name) => {
                    let profile = self
                        .mcp
                        .run
                        .as_ref()
                        .and_then(|r| r.server.workspace_profile(&name));
                    if let Some(profile) = profile {
                        let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                        doc.dirty = true;
                        self.docs.push(Doc::Profile(Box::new(doc)));
                        self.active = self.docs.len() - 1;
                    }
                }
                Act::OpenMarks => {
                    match self.docs.iter().position(|d| matches!(d, Doc::Bookmarks)) {
                        Some(i) => self.active = i,
                        None => {
                            self.docs.push(Doc::Bookmarks);
                            self.active = self.docs.len() - 1;
                        }
                    }
                }
                Act::GoToMark(i) => self.go_to_mark(i),
                Act::SetMarkComment(i, text) => {
                    if let Some(b) = self.marks.items.get_mut(i) {
                        b.comment = text;
                        self.save_marks();
                    }
                }
                Act::RemoveMark(i) => {
                    if i < self.marks.items.len() {
                        self.marks.items.remove(i);
                        self.marks_tab.sel = match self.marks_tab.sel {
                            Some(s) if s == i => None,
                            Some(s) if s > i => Some(s - 1),
                            other => other,
                        };
                        self.save_marks();
                    }
                }
                Act::RemoveMarkAsk(i) => self.ask_remove_mark(i),
                Act::MarkSelect(i) => self.marks_tab.sel = Some(i),
                Act::ShowInFolder(path) => {
                    // A folder of the program that is not there yet is made first.
                    if !path.exists() && path.extension().is_none() {
                        let _ = std::fs::create_dir_all(&path);
                    }
                    if let Err(e) = winweaver_win::show_in_folder(&path) {
                        self.status = trf("The folder was not opened: {e}", &[("e", &e)]);
                    }
                }
                Act::AskYes => {
                    if let Some(ask) = self.ask.take() {
                        self.apply(vec![ask.act]);
                    }
                }
                Act::AskNo => self.ask = None,
                Act::Fold => {
                    self.settings.desc_open = !self.settings.desc_open;
                    self.settings_dirty = true;
                }
                Act::Check => {
                    self.check_active();
                    self.messages_open = true;
                }
                Act::ReadPc => self.read_pc(false),
                Act::ReadSelection => self.read_pc(true),
                Act::CheckOnPc => self.check_on_pc(),
                Act::SaveScript(kind) => self.save_script(kind),
                Act::SaveRegFile(revert) => self.save_plan_reg(revert),
                Act::RegFileEdit => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active)
                        && let Some(line) = d.target()
                    {
                        d.start_edit(line);
                    }
                }
                Act::RegFileNew(kind) => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) {
                        d.start_new(kind);
                    }
                }
                Act::AskRegFileRemove => {
                    if let Some(Doc::RegFile(d)) = self.docs.get(self.active)
                        && let Some((lines, text)) = d.removal()
                    {
                        self.ask = Some(Ask {
                            title: tr("Remove from the file").to_owned(),
                            text,
                            yes: tr("Remove").to_owned(),
                            act: Act::RegFileRemove(lines),
                        });
                    }
                }
                Act::RegFileRemove(lines) => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) {
                        d.remove_lines(&lines);
                    }
                }
                Act::OpenReport => self.open_report(),
                Act::NewUsersFile => self.new_users_doc(Vec::new()),
                Act::UsersOfThisPc => match winweaver_win::users_of_this_pc() {
                    Ok(read) => self.new_users_doc(read),
                    Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
                },
                Act::UsersAdd => {
                    if let Some(d) = self.active_users() {
                        let n = d.file.users.len() + 1;
                        d.add(users::User {
                            name: format!("User{n}"),
                            ..users::User::default()
                        });
                    }
                }
                Act::UsersDuplicate => {
                    if let Some(d) = self.active_users() {
                        d.duplicate();
                    }
                }
                Act::AskUsersRemove => {
                    let found = self
                        .active_users()
                        .and_then(|d| d.selected.zip(d.selected_user().map(|u| u.name.clone())));
                    if let Some((i, name)) = found {
                        self.ask = Some(Ask {
                            title: tr("Remove the account from the file").to_owned(),
                            text: trf(
                                "Remove {name} from the file? An account of a PC stays.",
                                &[("name", &name)],
                            ),
                            yes: tr("Remove").to_owned(),
                            act: Act::UsersRemove(i),
                        });
                    }
                }
                Act::UsersRemove(i) => {
                    if let Some(d) = self.active_users() {
                        d.remove(i);
                    }
                }
                Act::UsersMove(down) => {
                    if let Some(d) = self.active_users() {
                        d.move_selected(down);
                    }
                }
                Act::UsersUnlock => self.users_unlock(false, false),
                Act::UsersSetMaster => self.users_unlock(true, false),
                Act::UsersSetPassword => self.users_unlock(false, true),
                Act::UsersLock => {
                    if let Some(d) = self.active_users() {
                        d.lock();
                    }
                }
                Act::UsersCheck => {
                    if let Some(d) = self.active_users() {
                        d.check();
                        let errors = d.issues.iter().filter(|i| i.error).count();
                        self.status = trf(
                            "Checked {n} accounts: {errors} errors",
                            &[("n", &d.file.users.len()), ("errors", &errors)],
                        );
                    }
                }
                Act::UsersReadPc => match winweaver_win::users_of_this_pc() {
                    Ok(read) => {
                        if let Some(d) = self.active_users() {
                            let n = d.merge(read);
                            self.status = trf("{n} accounts of this PC added", &[("n", &n)]);
                        }
                    }
                    Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
                },
                Act::AskUsersRun(operation, all) => self.ask_users_run(operation, all),
                Act::UsersRun(operation, names) => self.run_users(operation, &names),
                Act::UsersScript => self.save_users_script(),
                Act::UsersIntoProfile => self.users_into_profile(),
                Act::ImportTemplates(source) => self.start_import(source),
                Act::ToggleImport(id) => {
                    if self.imports.iter().any(|f| f.id == id) {
                        self.unload_import(&id);
                    } else if let Err(e) = self.load_import(&id) {
                        self.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                    }
                    self.refresh_import_list();
                    self.rebuild_catalog();
                }
                Act::RemoveImport(id) => {
                    let _ = std::fs::remove_file(
                        self.imports_dir
                            .join(format!("{id}.{}", admx::CATALOG_EXTENSION)),
                    );
                    self.unload_import(&id);
                    self.refresh_import_list();
                    self.rebuild_catalog();
                }
                Act::LoadCatalogs => self.load_catalogs_for_unknown(),
                Act::AskRemoveImport(id) => {
                    let name = self
                        .import_list
                        .iter()
                        .find(|e| e.id == id)
                        .map_or_else(|| id.clone(), |e| e.name.clone());
                    self.ask = Some(Ask {
                        title: tr("Remove the import").to_owned(),
                        text: trf(
                            "Remove {name}? Its catalog file is deleted from the imports folder, and the options of profiles from it become unknown.",
                            &[("name", &name)],
                        ),
                        yes: tr("Remove").to_owned(),
                        act: Act::RemoveImport(id),
                    });
                }
                Act::AskCloseDiscard(i) => match self.docs.get(i) {
                    Some(d) if d.dirty() => {
                        self.ask = Some(Ask {
                            title: tr("Discard the changes").to_owned(),
                            text: trf(
                                "Close {title} and discard its unsaved changes?",
                                &[("title", &d.title())],
                            ),
                            yes: tr("Discard").to_owned(),
                            act: Act::CloseTab(i, true),
                        });
                    }
                    Some(_) => self.apply(vec![Act::CloseTab(i, true)]),
                    None => {}
                },
                Act::AskForgetRecent => {
                    self.ask = Some(Ask {
                        title: tr("Forget the recent files").to_owned(),
                        text: tr(
                            "Forget the list of the recent files? The files stay where they are.",
                        )
                        .to_owned(),
                        yes: tr("Forget").to_owned(),
                        act: Act::ForgetRecent,
                    });
                }
                Act::ForgetRecent => {
                    self.settings.recent.clear();
                    self.settings_dirty = true;
                }
                Act::AskRenewToken => {
                    self.ask = Some(Ask {
                        title: tr("Renew the token").to_owned(),
                        text: tr("Renew the token of the MCP server? The clients set up with the old one lose their access until they get the new one.")
                            .to_owned(),
                        yes: tr("Renew").to_owned(),
                        act: Act::McpRenewToken,
                    });
                }
                Act::CompareCatalog(other) => self.compare_catalog(other),
                Act::OpenRegistry(path, value) => self.open_registry(&path, value.as_deref()),
                Act::RegistryGo(path, value) => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.go(&*reg, &path, value.as_deref());
                    }
                }
                Act::RegistryExpand(path, open) => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.expand(&*reg, &path, open);
                    }
                }
                Act::RegistryRefresh => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.refresh(&*reg);
                    }
                }
                Act::RegistryEdit => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.start_edit(regview::EditKind::Value, "");
                    }
                }
                Act::RegistryNew(kind, type_name) => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.start_edit(kind, type_name);
                    }
                }
                Act::RegistrySave => self.registry_save(),
                Act::RegistrySearch => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.search.open = !d.search.open;
                        d.search.focus = d.search.open;
                    }
                }
                Act::RegistryFind => self.registry_find(),
                Act::RegistryFindStop => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active)
                        && let Some(running) = &d.search.running
                    {
                        running
                            .stop
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                Act::AskRegistryDelete => self.ask_registry_delete(),
                Act::RegistryRights => self.registry_rights(),
                Act::RegistryRightsAdd => self.registry_rights_add(),
                Act::AskRegistryRights => {
                    if let Some(Doc::Registry(d)) = self.docs.get(self.active)
                        && let Some(edit) = &d.rights
                    {
                        self.ask = Some(Ask {
                            title: tr("Change the rights").to_owned(),
                            text: trf(
                                "Change the rights of {path}? A wrong access list can shut Windows or a program out of the key.",
                                &[("path", &edit.path)],
                            ),
                            yes: tr("Change").to_owned(),
                            act: Act::RegistryRightsSave,
                        });
                    }
                }
                Act::RegistryRightsSave => self.registry_rights_save(),
                Act::RegistryDelete(path, value) => self.registry_delete(&path, value.as_deref()),
                Act::BookmarkRegistry => self.bookmark_registry(),
                Act::EditCatalog(which) => self.edit_catalog(which),
                Act::NewCatalog => self.new_catalog(),
                Act::AskCatalogEditRemove(id) => {
                    self.ask = Some(Ask {
                        title: tr("Remove the policy").to_owned(),
                        text: trf(
                            "Remove the policy {id} from the catalog? It leaves the file at the next save.",
                            &[("id", &id)],
                        ),
                        yes: tr("Remove").to_owned(),
                        act: Act::CatalogEditRemove(id),
                    });
                }
                Act::CatalogEditRemove(id) => {
                    if let Some(Doc::CatalogEdit(d)) = self.docs.get_mut(self.active) {
                        d.remove(&id);
                    }
                }
                Act::ExportImport(id) => self.export_import(&id),
                Act::StartRename(id, name) => self.renaming = Some((id, name)),
                Act::OpenCatalog(id) => self.open_catalog(&id),
                Act::CatalogSelect(key) => {
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) {
                        if let Some(group) = key.strip_prefix("g:") {
                            c.group = Some(group.to_owned());
                            c.sel = None;
                        } else if key.is_empty() {
                            c.group = None;
                            c.sel = None;
                        } else {
                            c.group = None;
                            c.sel = Some(key);
                        }
                    }
                }
                Act::CatalogReveal(key) => {
                    self.apply(vec![Act::CatalogSelect(key)]);
                    self.reveal_selection();
                }
                Act::CatalogExpand(gid, open) => {
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) {
                        if open {
                            c.expanded.insert(gid);
                        } else {
                            c.expanded.remove(&gid);
                        }
                    }
                }
                Act::ShowInProfile(import, rule) => self.show_in_profile(&import, &rule),
                Act::ShowGroupInProfile(group) => {
                    match self.docs.iter().position(|d| matches!(d, Doc::Profile(_))) {
                        Some(index) => {
                            self.active = index;
                            self.apply(vec![Act::Reveal(Sel::Group(group))]);
                        }
                        None => self.status = tr("Open a profile first").to_owned(),
                    }
                }
                Act::CancelRename => self.renaming = None,
                Act::RenameImport(id, name) => self.rename_import(&id, &name),
                Act::PlanNative(revert) => self.plan_native(revert),
                Act::PlanUndo => self.plan_undo(),
                Act::ConfirmPlan => self.run_plan(),
                Act::CancelPlan => self.confirm = None,
                Act::Build => self.build_active(),
                Act::OpenGuide => self.open_guide(),
                Act::OpenLicense => self.open_license(),
                Act::Compare => {
                    if let Some(path) = pick_profile() {
                        match Profile::load(&path) {
                            Ok(other) => self.compare_active(other, Some(&path)),
                            Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                        }
                    }
                }
                Act::CompareWith(name) => {
                    if let Some((name, text)) = PRESETS.iter().find(|p| p.0 == name) {
                        match Profile::parse(text, name) {
                            Ok(other) => self.compare_active(other, None),
                            Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                        }
                    }
                }
                Act::ApplyDiff => self.apply_active_diff(),
                Act::SaveDiffAs => self.save_active_diff(),
                Act::DiffSelect(key) => {
                    if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                        d.sel = Some(key);
                    }
                }
                Act::DiffComment(key, text) => {
                    if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                        if text.trim().is_empty() {
                            d.diff.comments.shift_remove(&key);
                        } else {
                            d.diff.comments.insert(key, text);
                        }
                        d.dirty = true;
                    }
                }
                Act::DiffGoTo(key, other) => self.go_to_diff_item(&key, other),
                Act::DiffApplyItem(key) => self.apply_diff_item(&key),
                Act::SetInstall(install) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.install = install;
                    }
                    self.refresh_active();
                }
                Act::SetAccounts(accounts) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.accounts = accounts;
                    }
                    self.refresh_active();
                }
                Act::SetLanguages(languages) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.languages = languages;
                    }
                    self.refresh_active();
                }
                Act::ClearMessages => {
                    if let Some(doc) = self.active_profile() {
                        doc.issues.clear();
                    }
                }
                Act::ToggleMessages => self.messages_open = !self.messages_open,
                Act::SearchCleared => self.reveal_selection(),
                Act::Revealed => match self.docs.get_mut(self.active) {
                    Some(Doc::Profile(doc)) => {
                        doc.reveal = false;
                        doc.follow = false;
                    }
                    Some(Doc::Catalog(c)) => {
                        c.reveal = false;
                        c.follow = false;
                    }
                    Some(Doc::Diff(d)) => {
                        d.reveal = false;
                        d.follow = false;
                    }
                    Some(Doc::Bookmarks) => {
                        self.marks_tab.reveal = false;
                        self.marks_tab.follow = false;
                    }
                    _ => {}
                },
                Act::Follow => match self.docs.get_mut(self.active) {
                    Some(Doc::Profile(doc)) => doc.follow = true,
                    Some(Doc::Catalog(c)) => c.follow = true,
                    Some(Doc::Diff(d)) => d.follow = true,
                    Some(Doc::Bookmarks) => self.marks_tab.follow = true,
                    _ => {}
                },
                Act::OpenDoc(link) => self.open_doc(&link),
                Act::Exit => self.exit = true,
                Act::Message(text) => self.status = text,
            }
        }
    }

    /// Writes the bookmark file; `false` (and a status message) when it cannot be written.
    fn save_marks(&mut self) -> bool {
        if let Some(parent) = self.marks_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match self.marks.save(&self.marks_path) {
            Ok(()) => true,
            Err(e) => {
                self.status = trf("Bookmarks not saved: {e}", &[("e", &e)]);
                false
            }
        }
    }

    fn open_marks_path(&mut self, path: PathBuf) {
        match Bookmarks::load(&path) {
            Ok(marks) => {
                self.marks = marks;
                self.marks_path = path.clone();
                self.status = trf(
                    "Opened {path}: the star now writes there",
                    &[("path", &path.display())],
                );
                self.remember_recent(&path);
                self.apply(vec![Act::OpenMarks]);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens the document of a bookmark (a file, or the tab of an unsaved document by its
    /// title) and selects its item.
    /// Opens the profile of a bookmark (loading its file when no tab has it) and selects the
    /// option; without a profile file the option is shown in the profile tab in front, else the first one.
    fn go_to_mark(&mut self, i: usize) {
        let Some(mark) = self.marks.items.get(i).cloned() else {
            return;
        };
        self.marks_tab.sel = Some(i);
        if mark.is_registry() {
            let value = (!mark.item.is_empty()).then_some(mark.item.as_str());
            self.open_registry(&mark.path, value);
            return;
        }
        let path = PathBuf::from(&mark.path);
        if !mark.path.is_empty() && path.is_file() {
            self.open_path(path);
        } else if let Some(j) = (!mark.path.is_empty())
            .then(|| self.docs.iter().position(|d| d.title() == mark.path))
            .flatten()
        {
            self.active = j;
        } else if mark.is_option() {
            let front = match self.docs.get(self.active) {
                Some(Doc::Profile(_)) => Some(self.active),
                _ => self.docs.iter().position(|d| matches!(d, Doc::Profile(_))),
            };
            match front {
                Some(j) => self.active = j,
                None => {
                    self.status =
                        tr("Open a profile to show the option of the bookmark in").to_owned();
                    return;
                }
            }
        } else {
            self.status = trf(
                "The document of the bookmark is not open: {path}",
                &[("path", &mark.path)],
            );
            return;
        }
        if mark.is_option() {
            let mut sel = sel_from_item(&self.catalog, &mark.item);
            if let Some(doc) = self.active_profile() {
                if sel == Sel::None && doc.profile.unknown.contains_key(&mark.item) {
                    sel = Sel::Unknown(mark.item.clone());
                }
                doc.select(sel);
            }
            self.reveal_selection();
        }
    }

    /// Opens the editor of a loaded import or of the file of a catalog tab; an editor of the same
    /// file comes to the front.
    fn edit_catalog(&mut self, which: CatalogRef) {
        let import_path =
            |dir: &Path, id: &str| dir.join(format!("{id}.{}", admx::CATALOG_EXTENSION));
        let (file, path, import) = match &which {
            CatalogRef::Import(id) => match self.imports.iter().find(|f| &f.id == id) {
                Some(f) => (f.clone(), import_path(&self.imports_dir, &f.id), true),
                None => return,
            },
            CatalogRef::Tab(i) => match self.docs.get(*i) {
                Some(Doc::Catalog(c)) => match (&c.file, &c.path) {
                    (Some(f), Some(p)) => ((**f).clone(), p.clone(), false),
                    (None, _) => match c.catalog(&self.imports) {
                        Some(f) => (f.clone(), import_path(&self.imports_dir, &f.id), true),
                        None => return,
                    },
                    _ => return,
                },
                _ => return,
            },
        };
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::CatalogEdit(e) if e.path == path))
        {
            self.active = i;
            return;
        }
        let doc = cataledit::EditorDoc::new(file, path, import);
        self.docs.push(Doc::CatalogEdit(Box::new(doc)));
        self.active = self.docs.len() - 1;
    }

    /// An empty catalog file among the imports, loaded, in its editor.
    fn new_catalog(&mut self) {
        let mut n = 1;
        let id = loop {
            let id = format!("custom-{n}");
            let path = self
                .imports_dir
                .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
            if !path.exists() && !self.imports.iter().any(|f| f.id == id) {
                break id;
            }
            n += 1;
        };
        let name = trf("My policies {n}", &[("n", &n)]);
        let file = admx::to_catalog_file(
            &admx::Templates::default(),
            &id,
            &name,
            "",
            &clock::now_utc_iso(),
        );
        let path = self
            .imports_dir
            .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
        let _ = std::fs::create_dir_all(&self.imports_dir);
        if let Err(e) = file.save(&path) {
            self.status = trf("Save failed: {e}", &[("e", &e)]);
            return;
        }
        self.imports.push(file.clone());
        self.imports.sort_by_key(|f| import_order(&f.id));
        self.mark_loaded(&id);
        self.refresh_import_list();
        self.rebuild_catalog();
        let doc = cataledit::EditorDoc::new(file, path, true);
        self.docs.push(Doc::CatalogEdit(Box::new(doc)));
        self.active = self.docs.len() - 1;
    }

    /// Saves the file of the editor in front after a check; a loaded import is read again.
    fn save_catalog_edit(&mut self) {
        let Some(Doc::CatalogEdit(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        if let Some(parent) = d.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let result = d.file.check().and_then(|()| d.file.save(&d.path));
        if let Err(e) = result {
            self.status = trf("Save failed: {e}", &[("e", &e)]);
            return;
        }
        d.dirty = false;
        let path = d.path.clone();
        let reload = d.import.then(|| d.file.clone());
        self.status = trf("Saved {path}", &[("path", &path.display())]);
        self.shown_file = Some((self.status.clone(), path));
        if let Some(file) = reload {
            if let Some(slot) = self.imports.iter_mut().find(|f| f.id == file.id) {
                *slot = file;
            }
            self.refresh_import_list();
            self.rebuild_catalog();
        }
    }

    /// Opens the window of the rights of the selected key of the registry tab in front: its owner
    /// and its access list with the names of the accounts.
    fn registry_rights(&mut self) {
        use winweaver_core::security::{self, Rights};
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let path = d.path.clone();
        let rights = match reg.security(&path).and_then(|s| Rights::parse(&s)) {
            Ok(rights) => rights,
            Err(e) => {
                d.error = Some(e);
                return;
            }
        };
        let mut names = std::collections::HashMap::new();
        let owner = std::iter::once(&rights.owner);
        for sid in owner.chain(rights.entries.iter().map(|e| &e.sid)) {
            if sid.is_empty() || names.contains_key(sid) {
                continue;
            }
            let name = reg
                .account_name(sid)
                .or_else(|| security::alias_title(sid).map(str::to_owned))
                .unwrap_or_else(|| sid.clone());
            names.insert(sid.clone(), name);
        }
        d.rights = Some(regview::RightsEdit {
            path,
            rights,
            names,
            add: String::new(),
            error: None,
            changed: false,
        });
    }

    /// Adds an entry that lets the account typed in the window of the rights read the key.
    fn registry_rights_add(&mut self) {
        use winweaver_core::security::{Entry, Scope};
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &mut d.rights else {
            return;
        };
        let name = edit.add.trim().to_owned();
        if name.is_empty() {
            return;
        }
        match reg.account_sid(&name) {
            Some(sid) => {
                let shown = reg.account_name(&sid).unwrap_or(name);
                edit.names.insert(sid.clone(), shown);
                let entry = Entry::new(&sid, false, false, Scope::KeyAndSubkeys);
                edit.rights.entries.push(entry);
                edit.add.clear();
                edit.error = None;
                edit.changed = true;
            }
            None => {
                let text = trf("No account of this PC: {name}", &[("name", &name)]);
                edit.error = Some(text);
            }
        }
    }

    /// Writes the access list of the window of the rights to its key.
    fn registry_rights_save(&mut self) {
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &mut d.rights else {
            return;
        };
        let dacl = edit.rights.dacl();
        match reg.set_security(&edit.path, &dacl, edit.rights.protected) {
            Ok(()) => {
                self.status = trf("The rights of {path} changed", &[("path", &edit.path)]);
                d.rights = None;
            }
            Err(e) => edit.error = Some(e),
        }
    }

    /// Starts the search of the registry tab in front under its selected key, in a thread of its
    /// own; `poll_registry_search` takes the result.
    fn registry_find(&mut self) {
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let query = d.search.query();
        if d.search.running.is_some() {
            return;
        }
        if query.text.is_empty() || !(query.keys || query.values || query.data) {
            self.status =
                tr("Search the registry: type a text and choose what to look at").to_owned();
            return;
        }
        let root = d.path.clone();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let read = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let (thread_stop, thread_read, thread_root) = (stop.clone(), read.clone(), root.clone());
        let started = std::thread::Builder::new()
            .name("registry-search".to_owned())
            .spawn(move || {
                let limit = regview::SEARCH_LIMIT;
                let found = winweaver_core::registry::search(
                    &*reg,
                    &thread_root,
                    &query,
                    limit,
                    usize::MAX,
                    &thread_stop,
                    &thread_read,
                );
                let _ = tx.send(found);
            });
        if let Err(e) = started {
            self.status = trf("The search did not start: {e}", &[("e", &e)]);
            return;
        }
        d.search.root = root;
        d.search.found = None;
        d.search.running = Some(regview::Running {
            stop,
            read,
            result: rx,
        });
    }

    /// Takes the result of a finished search of a registry tab.
    fn poll_registry_search(&mut self, ctx: &egui::Context) {
        for doc in &mut self.docs {
            let Doc::Registry(d) = doc else {
                continue;
            };
            let Some(running) = &d.search.running else {
                continue;
            };
            match running.result.try_recv() {
                Ok(found) => {
                    d.search.running = None;
                    d.search.found = Some(found);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    d.search.running = None;
                    self.status =
                        tr("The search of the registry failed; the log names the error").to_owned();
                }
            }
        }
    }

    /// Opens the registry tab, or brings it to the front, at a key and a value of it.
    fn open_registry(&mut self, path: &str, value: Option<&str>) {
        let reg = self.registry.clone();
        match self.docs.iter().position(|d| matches!(d, Doc::Registry(_))) {
            Some(i) => {
                self.active = i;
                if let Some(Doc::Registry(d)) = self.docs.get_mut(i) {
                    d.go(&*reg, path, value);
                }
            }
            None => {
                let doc = regview::RegistryDoc::new(&*reg, path, value);
                self.docs.push(Doc::Registry(Box::new(doc)));
                self.active = self.docs.len() - 1;
            }
        }
    }

    /// Writes the value or makes the key of the editor of the registry tab; an error stays in
    /// the editor.
    fn registry_save(&mut self) {
        use regview::EditKind;
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &d.edit else {
            return;
        };
        let kind = edit.kind;
        let name = if kind == EditKind::Value {
            edit.name.clone()
        } else {
            edit.name.trim().to_owned()
        };
        let (type_name, text) = (edit.type_name.clone(), edit.text.clone());
        let taken = d
            .key
            .as_ref()
            .is_some_and(|k| k.values.iter().any(|v| v.name.eq_ignore_ascii_case(&name)));
        let result: Result<Option<String>, String> = match kind {
            EditKind::NewKey => {
                if name.is_empty() || name.contains('\\') {
                    Err(tr("A name without backslashes is needed").to_owned())
                } else {
                    let path = format!("{}\\{name}", d.path);
                    reg.create_key(&path).map(|()| Some(path))
                }
            }
            EditKind::NewValue if taken => {
                Err(tr("A value of this name is there already").to_owned())
            }
            _ => winweaver_core::registry::Data::parse(&type_name, &text)
                .and_then(|data| reg.set(&d.path, &name, &data).map(|()| None)),
        };
        match result {
            Ok(key) => {
                d.edit = None;
                match key {
                    Some(path) => d.go(&*reg, &path, None),
                    None => {
                        d.value = Some(name);
                        d.refresh(&*reg);
                    }
                }
            }
            Err(e) => {
                if let Some(edit) = &mut d.edit {
                    edit.error = Some(e);
                }
            }
        }
    }

    /// Asks before the selected value of the registry tab, or its key without one, is deleted.
    fn ask_registry_delete(&mut self) {
        let Some(Doc::Registry(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = d.path.clone();
        let (text, act) = match d.selected() {
            Some(v) => (
                trf(
                    "Delete the value {name} of {key}?",
                    &[("name", &v.name), ("key", &path)],
                ),
                Act::RegistryDelete(path.clone(), Some(v.name.clone())),
            ),
            None if winweaver_core::registry::parent(&path).is_none() => {
                self.status = tr("A hive is not deleted").to_owned();
                return;
            }
            None => (
                trf(
                    "Delete the key {key} with all its subkeys and values?",
                    &[("key", &path)],
                ),
                Act::RegistryDelete(path.clone(), None),
            ),
        };
        self.ask = Some(Ask {
            title: tr("Delete from the registry").to_owned(),
            text,
            yes: tr("Delete").to_owned(),
            act,
        });
    }

    fn registry_delete(&mut self, path: &str, value: Option<&str>) {
        let reg = self.registry.clone();
        let result = match value {
            Some(v) => reg.delete_value(path, v),
            None => reg.delete_key(path),
        };
        if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
            if value.is_none() && result.is_ok() {
                let up = winweaver_core::registry::parent(path)
                    .unwrap_or(path)
                    .to_owned();
                d.go(&*reg, &up, None);
            } else {
                d.refresh(&*reg);
            }
        }
        self.status = match result {
            Ok(()) => tr("Deleted").to_owned(),
            Err(e) => e,
        };
    }

    /// Bookmarks the key of the registry tab and its selected value.
    fn bookmark_registry(&mut self) {
        let Some(Doc::Registry(d)) = self.docs.get(self.active) else {
            return;
        };
        let mut mark = Bookmark::registry(&d.path, d.value.as_deref().unwrap_or(""));
        if self.marks.find(&mark).is_some() {
            self.status = tr("This is bookmarked already").to_owned();
            return;
        }
        mark.created = clock::now_utc_iso();
        self.marks.items.push(mark);
        self.save_marks();
        self.status = tr("Bookmarked").to_owned();
    }

    /// The catalog of the active tab and the other one, side by side in a Markdown page.
    fn compare_catalog(&mut self, other: CatalogRef) {
        let Some(Doc::Catalog(c)) = self.docs.get(self.active) else {
            return;
        };
        let Some(first) = c.catalog(&self.imports) else {
            return;
        };
        let second = match &other {
            CatalogRef::Import(id) => self.imports.iter().find(|f| &f.id == id),
            CatalogRef::Tab(i) => match self.docs.get(*i) {
                Some(Doc::Catalog(o)) => o.catalog(&self.imports),
                _ => None,
            },
        };
        let Some(second) = second else {
            self.status = tr("The other catalog is not open any more").to_owned();
            return;
        };
        let title = trf(
            "{first} and {second}",
            &[("first", &first.name), ("second", &second.name)],
        );
        let page = comparison_page(first, second);
        self.docs
            .push(Doc::Markdown(Box::new(MdDoc::new(title, None, page))));
        self.active = self.docs.len() - 1;
    }

    fn open_license(&mut self) {
        match self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Markdown(m) if m.is_license()))
        {
            Some(i) => self.active = i,
            None => {
                self.docs.push(Doc::Markdown(Box::new(MdDoc::license())));
                self.active = self.docs.len() - 1;
            }
        }
    }

    fn open_guide(&mut self) {
        match self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Markdown(m) if m.is_guide()))
        {
            Some(i) => self.active = i,
            None => {
                self.docs.push(Doc::Markdown(Box::new(MdDoc::guide())));
                self.active = self.docs.len() - 1;
            }
        }
    }

    fn open_md_path(&mut self, path: PathBuf) {
        match winweaver_core::jsonfile::read_text(&path, MARKDOWN_MAX_BYTES) {
            Ok(text) => {
                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "page.md".to_owned());
                self.docs.push(Doc::Markdown(Box::new(MdDoc::new(
                    title,
                    Some(path.clone()),
                    text,
                ))));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens an answer file (its embedded profile), a WinKickOff profile or an audit report as a
    /// new, unsaved profile.
    fn open_foreign(&mut self, path: PathBuf, ext: &str) {
        let name = jsonfile::display_name(&path);
        let text = match jsonfile::read_text_bom(&path, jsonfile::MAX_BYTES) {
            Ok(t) => t,
            Err(e) => {
                self.status = trf("Open failed: {e}", &[("e", &e)]);
                return;
            }
        };
        if ext == "json"
            && let Ok(report) = Report::parse(&text, &name)
        {
            self.open_report_from(report, &path);
            return;
        }
        let opened = if ext == "xml" {
            import::profile_from_answer_file(&text, &name)
        } else {
            import::profile_from_json(&text, &name)
        };
        match opened {
            Ok((mut profile, notes)) => {
                profile.split_unknown(&self.catalog);
                let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                doc.dirty = true;
                doc.issues.extend(notes.into_iter().map(|n| Issue {
                    severity: Severity::Info,
                    target: Target::Profile,
                    text: trf("Open: {n}", &[("n", &n)]),
                }));
                self.docs.push(Doc::Profile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf(
                    "Opened the profile of {path}; save it as a .wwprofile file",
                    &[("path", &path.display())],
                );
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    fn open_diff_path(&mut self, path: PathBuf) {
        match Diff::load(&path) {
            Ok(diff) => {
                self.docs
                    .push(Doc::Diff(Box::new(DiffDoc::new(diff, Some(path.clone())))));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    fn compare_active(&mut self, other: Profile, other_path: Option<&Path>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Compare: the active tab is not a profile").to_owned();
            return;
        };
        let base_side = diff::Side::of(&doc.profile, doc.path.as_deref());
        let other_side = diff::Side::of(&other, other_path);
        let diff = Diff::between(&doc.profile, &other, base_side, other_side);
        self.status = trf("{n} differences", &[("n", &diff.count())]);
        self.docs
            .push(Doc::Diff(Box::new(DiffDoc::new(diff, None))));
        self.active = self.docs.len() - 1;
    }

    /// The profile tab a diff belongs to: the one with the path of its base, else the one with
    /// its name, else the first profile tab.
    fn diff_target(&self, diff: &Diff) -> Option<usize> {
        let by_path = |d: &Doc| {
            matches!(d, Doc::Profile(p) if !diff.base.path.is_empty()
                && p.path.as_ref().is_some_and(|x| x.display().to_string() == diff.base.path))
        };
        let by_name = |d: &Doc| matches!(d, Doc::Profile(p) if p.profile.name == diff.base.name);
        self.docs
            .iter()
            .position(by_path)
            .or_else(|| self.docs.iter().position(by_name))
            .or_else(|| self.docs.iter().position(|d| matches!(d, Doc::Profile(_))))
    }

    fn apply_active_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let Some(i) = self.diff_target(&diff) else {
            self.status = tr("Apply: open the profile to change first").to_owned();
            return;
        };
        let catalog = &self.catalog;
        if let Some(Doc::Profile(p)) = self.docs.get_mut(i) {
            diff.apply_to(&mut p.profile);
            p.refresh(catalog);
            self.status = trf(
                "Applied {n} differences to {profile}",
                &[("n", &diff.count()), ("profile", &p.title())],
            );
        }
        self.active = i;
    }

    /// Shows an item of the active diff in the profile tab of a side; the profile is opened from
    /// its file, or from the preset of its name, when no tab has it.
    fn go_to_diff_item(&mut self, key: &str, other: bool) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let base_tab = self.diff_target(&diff);
        let tab = if other {
            let side = &diff.other;
            let by_path = |(i, d): (usize, &Doc)| {
                Some(i) != base_tab
                    && matches!(d, Doc::Profile(p) if !side.path.is_empty()
                        && p.path.as_ref().is_some_and(|x| x.display().to_string() == side.path))
            };
            let by_name = |(i, d): (usize, &Doc)| {
                Some(i) != base_tab && matches!(d, Doc::Profile(p) if p.profile.name == side.name)
            };
            self.docs
                .iter()
                .enumerate()
                .position(by_path)
                .or_else(|| self.docs.iter().enumerate().position(by_name))
        } else {
            base_tab
        };
        let side = if other { &diff.other } else { &diff.base };
        match tab {
            Some(i) => self.active = i,
            None if !side.path.is_empty() && Path::new(&side.path).is_file() => {
                self.open_path(PathBuf::from(&side.path));
            }
            None => match PRESETS.iter().find(|p| p.0 == side.name) {
                Some(&(name, text)) => {
                    let profile = Profile::parse(text, name)
                        .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog));
                    let doc = ProfileDoc::new(profile, None, &self.catalog);
                    self.docs.push(Doc::Profile(Box::new(doc)));
                    self.active = self.docs.len() - 1;
                }
                None => {
                    self.status = trf(
                        "The profile {name} is not open and has no file to open",
                        &[("name", &side.name)],
                    );
                    return;
                }
            },
        }
        let sel = sel_from_diff_key(&self.catalog, key);
        if let Some(doc) = self.active_profile() {
            doc.select(sel);
        }
        self.reveal_selection();
    }

    /// Sets one item of the active diff in the profile of its base and stays on the diff.
    fn apply_diff_item(&mut self, key: &str) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let Some(i) = self.diff_target(&diff) else {
            self.status = tr("Apply: open the profile to change first").to_owned();
            return;
        };
        let catalog = &self.catalog;
        if let Some(Doc::Profile(p)) = self.docs.get_mut(i) {
            diff.apply_item(key, &mut p.profile);
            p.refresh(catalog);
            self.status = trf("Applied one difference to {name}", &[("name", &p.title())]);
        }
    }

    fn save_active_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            self.status = tr("Save as a diff file: the active tab is not a diff").to_owned();
            return;
        };
        let proposed = format!(
            "{}-vs-{}.{}",
            file_stem(&d.diff.base.name),
            file_stem(&d.diff.other.name),
            diff::EXTENSION
        );
        let Some(path) = file_dialog("WinWeaver diff", diff::EXTENSION)
            .set_title(tr("Save the diff as"))
            .set_file_name(proposed)
            .save_file()
        else {
            return;
        };
        let path = if path.extension().is_some() {
            path
        } else {
            path.with_extension(diff::EXTENSION)
        };
        let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.diff.save(&path) {
            Ok(()) => {
                d.path = Some(path.clone());
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Writes the drafts of the modified profiles: two seconds after the last change, and all of
    /// them when the window closes.
    fn write_drafts(&mut self, all: bool) {
        let mut failed = None;
        for doc in &mut self.docs {
            let Doc::Profile(d) = doc else {
                continue;
            };
            if !d.dirty {
                continue;
            }
            let due = match d.changed {
                Some(t) => all || t.elapsed() >= std::time::Duration::from_secs(2),
                None => false,
            };
            if !due {
                continue;
            }
            match session::write_draft(
                &self.drafts_dir,
                &d.draft_key,
                d.path.as_deref(),
                &d.profile,
            ) {
                Ok(()) => d.changed = None,
                Err(e) => {
                    d.changed = None;
                    failed = Some(e);
                }
            }
        }
        if let Some(e) = failed {
            self.status = trf("Draft not saved: {e}", &[("e", &e)]);
        }
    }

    /// Remembers the files of the open tabs for the next start.
    fn remember_tabs(&mut self) {
        let mut tabs = Vec::new();
        let mut active = 0;
        for (i, doc) in self.docs.iter().enumerate() {
            let path = match doc {
                Doc::Profile(d) => d.path.clone(),
                Doc::Diff(d) => d.path.clone(),
                Doc::Markdown(d) => d.path.clone(),
                Doc::Catalog(c) => c.path.clone(),
                Doc::Theme(t) => t.theme.path.clone(),
                Doc::Bookmarks
                | Doc::Mcp
                | Doc::Settings
                | Doc::Registry(_)
                | Doc::CatalogEdit(_) => None,
                Doc::RegFile(d) => Some(d.path.clone()),
                Doc::Users(d) => d.path.clone(),
            };
            if let Some(p) = path {
                if i == self.active {
                    active = tabs.len();
                }
                tabs.push(p);
            }
        }
        if tabs != self.settings.open_tabs || active != self.settings.active_tab {
            self.settings.open_tabs = tabs;
            self.settings.active_tab = active;
            self.settings_dirty = true;
        }
    }

    /// Opens the tabs of the last close and the drafts of unsaved changes.
    fn restore_session(&mut self) {
        let start_tab_untouched =
            matches!(self.docs.first(), Some(Doc::Profile(d)) if !d.dirty && d.path.is_none());
        let before = self.docs.len();
        let mut missing = 0;
        if self.settings.restore_tabs {
            for path in self.settings.open_tabs.clone() {
                if path.exists() {
                    self.open_path(path);
                } else {
                    missing += 1;
                }
            }
        }
        let restored_tabs = self.docs.len() - before;
        let (drafts, broken) = session::read_drafts(&self.drafts_dir);
        let mut restored = 0;
        for (key, draft) in drafts {
            let path = (!draft.path.is_empty()).then(|| PathBuf::from(&draft.path));
            let open = path.as_ref().and_then(|p| {
                self.docs
                    .iter()
                    .position(|d| matches!(d, Doc::Profile(x) if x.path.as_ref() == Some(p)))
            });
            let catalog = &self.catalog;
            let mut doc = match open {
                Some(i) => match self.docs.remove(i) {
                    Doc::Profile(mut d) => {
                        d.profile = draft.profile;
                        d
                    }
                    _ => continue,
                },
                None => Box::new(ProfileDoc::new(
                    draft.profile,
                    path.filter(|p| p.exists()),
                    catalog,
                )),
            };
            doc.refresh(catalog);
            doc.changed = None;
            doc.draft_key = key;
            doc.issues.push(Issue {
                severity: Severity::Info,
                target: Target::Profile,
                text: tr("The unsaved changes of the last session were restored from a draft")
                    .to_owned(),
            });
            self.docs.push(Doc::Profile(doc));
            restored += 1;
        }
        if start_tab_untouched && (restored_tabs > 0 || restored > 0) {
            self.docs.remove(0);
        }
        if restored_tabs > 0 {
            self.active = self
                .settings
                .active_tab
                .min(self.docs.len().saturating_sub(1));
        }
        if restored > 0 {
            self.active = self.docs.len() - 1;
        }
        let mut notes = Vec::new();
        if restored > 0 {
            notes.push(trf("{n} drafts restored", &[("n", &restored)]));
        }
        if missing > 0 {
            notes.push(trf(
                "{n} files of the last session are missing",
                &[("n", &missing)],
            ));
        }
        if !broken.is_empty() {
            notes.push(trf("{n} drafts could not be read", &[("n", &broken.len())]));
        }
        if !notes.is_empty() {
            self.status = notes.join("; ");
        }
    }

    /// Writes every new status line to the log.
    fn log_status(&mut self) {
        if self.status != self.last_logged {
            session::log(&self.logs_dir, "INFO", &self.status);
            self.last_logged = self.status.clone();
        }
    }

    /// Reads the configuration of the MCP tab at the start: `mcp/default.wwmcp`, made from the port
    /// and the token an earlier version kept in the settings when it does not exist yet.
    fn load_mcp_config(&mut self) {
        let path = self.mcp.path.clone();
        if path.exists() {
            match winweaver_mcp::config::Config::load(&path) {
                Ok(config) => {
                    self.mcp.take(&config);
                    self.mcp.saved = Some(config);
                }
                Err(e) => {
                    self.mcp.error = trf("The MCP configuration was not read: {e}", &[("e", &e)]);
                    // A file that cannot be read is not written over.
                    self.mcp.saved = Some(self.mcp.config());
                }
            }
            return;
        }
        let mut config = winweaver_mcp::config::Config::default();
        if self.settings.mcp_port >= 1024 {
            config.port = self.settings.mcp_port;
        }
        config.token = self.settings.mcp_token.clone();
        if config.check().is_err() {
            config.token.clear();
        }
        self.mcp.take(&config);
        // Written at the next frame by `persist_mcp`.
        self.mcp.saved = None;
    }

    /// Writes the configuration of the MCP tab once it changed; a failure is shown once.
    fn persist_mcp(&mut self) {
        let config = self.mcp.config();
        if self.mcp.saved.as_ref() == Some(&config) {
            return;
        }
        if let Err(e) = config.save(&self.mcp.path) {
            self.mcp.error = trf("The MCP configuration was not saved: {e}", &[("e", &e)]);
        }
        self.mcp.saved = Some(config);
    }

    /// Opens a configuration file in the MCP tab; a running server keeps the one it has.
    fn open_mcp_config(&mut self, path: PathBuf) {
        if self.mcp.run.is_some() {
            self.status =
                tr("Stop the MCP server before you open another configuration").to_owned();
            return;
        }
        match winweaver_mcp::config::Config::load(&path) {
            Ok(config) => {
                self.mcp.take(&config);
                self.mcp.path = path;
                self.mcp.saved = Some(config);
                self.mcp.error.clear();
                self.apply(vec![Act::OpenMcp]);
            }
            Err(e) => self.status = trf("Not opened: {e}", &[("e", &e)]),
        }
    }

    /// Saves the configuration of the MCP tab into another file, which the tab edits from then on.
    fn save_mcp_as(&mut self) {
        let dir = winweaver_mcp::config::Config::folder(&exe_dir());
        let _ = std::fs::create_dir_all(&dir);
        let ext = winweaver_mcp::config::EXTENSION;
        let Some(mut path) = rfd::FileDialog::new()
            .set_title(tr("Save the MCP configuration as"))
            .add_filter(tr("MCP configurations"), &[ext])
            .set_directory(&dir)
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        let config = self.mcp.config();
        match config.save(&path) {
            Ok(()) => {
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.mcp.path = path;
                self.mcp.saved = Some(config);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Starts the HTTP server of the window with the configuration of the MCP tab.
    fn start_mcp(&mut self) {
        if self.mcp.token.is_empty() {
            match winweaver_mcp::new_token() {
                Ok(t) => self.mcp.token = t,
                Err(e) => {
                    self.mcp.error = e;
                    return;
                }
            }
        }
        let server = std::sync::Arc::new(winweaver_mcp::Server::new(
            self.catalog.clone(),
            &PRESETS,
            winweaver_mcp::Options {
                mode: self.mcp.mode,
                registry: self.mcp.registry,
                edit_library: self.mcp.edit_library,
                marks_path: self.marks_path.clone(),
                imports_dir: self.imports_dir.clone(),
                read_pc: self.mcp.read_pc,
                profiles_dir: profiles_dir(),
                output_dir: output_dir(),
                detailed_log: self.mcp.detailed_log,
            },
        ));
        if !self.mcp.profile.is_empty()
            && let Err(e) = server.select(&self.mcp.profile)
        {
            self.mcp.error = trf(
                "The profile {name}: {e}",
                &[("name", &self.mcp.profile), ("e", &e)],
            );
            return;
        }
        match winweaver_mcp::http::HttpServer::start(
            server.clone(),
            self.mcp.port,
            self.mcp.token.clone(),
        ) {
            Ok(http) => {
                self.status = trf("The MCP server runs on {url}", &[("url", &http.url())]);
                self.mcp.error.clear();
                self.mcp.run = Some(mcpview::McpRun { server, http });
            }
            Err(e) => {
                self.mcp.error = trf("The server did not start: {e}", &[("e", &e)]);
            }
        }
    }

    /// The theme of the settings: a colour theme over its light or dark base, or the plain theme.
    /// The id of the theme the settings choose for a side when they follow Windows.
    fn shown_theme_id(&self, dark_mode: bool) -> String {
        if !self.settings.custom_theme.is_empty() {
            return self.settings.custom_theme.clone();
        }
        let dark = match self.settings.theme {
            ThemeChoice::System => dark_mode,
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
        };
        if dark { "dark" } else { "light" }.to_owned()
    }

    /// The light and dark themes of the window and the preference: the theme in the editor in
    /// front, else the one of the settings, else Light and Dark as Windows shows its apps.
    fn wanted_theme(&self) -> (themes::Theme, themes::Theme, egui::ThemePreference) {
        let light = themes::find(&self.themes, "light");
        let dark = themes::find(&self.themes, "dark");
        let chosen = match self.docs.get(self.active) {
            Some(Doc::Theme(d)) => Some(d.theme.clone()),
            _ => (!self.settings.custom_theme.is_empty())
                .then(|| themes::find(&self.themes, &self.settings.custom_theme)),
        };
        match chosen {
            Some(t) if t.dark => (light, t, egui::ThemePreference::Dark),
            Some(t) => (t, dark, egui::ThemePreference::Light),
            None => (light, dark, self.settings.theme.preference()),
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let wanted = self.wanted_theme();
        if self.applied_theme.as_ref() == Some(&wanted) {
            return;
        }
        let (light, dark, preference) = &wanted;
        style::apply(
            ctx,
            (light.palette(), light.fonts()),
            (dark.palette(), dark.fonts()),
        );
        ctx.set_theme(*preference);
        self.applied_theme = Some(wanted);
    }

    /// Moves a tab to another place of the row; the tab in front stays in front.
    fn move_tab(&mut self, from: usize, to: usize) {
        let n = self.docs.len();
        if from >= n || to >= n || from == to {
            return;
        }
        let doc = self.docs.remove(from);
        self.docs.insert(to, doc);
        self.active = if self.active == from {
            to
        } else if from < self.active && self.active <= to {
            self.active - 1
        } else if to <= self.active && self.active < from {
            self.active + 1
        } else {
            self.active
        };
    }

    /// Opens the editor of a theme, or brings its tab to the front.
    fn edit_theme(&mut self, theme: themes::Theme) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Theme(t) if t.theme.id == theme.id))
        {
            self.active = i;
            return;
        }
        self.docs
            .push(Doc::Theme(Box::new(themeview::ThemeDoc::new(theme))));
        self.active = self.docs.len() - 1;
    }

    fn open_theme_path(&mut self, path: PathBuf) {
        match themes::Theme::load(&path) {
            Ok(theme) => {
                self.edit_theme(theme);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Writes the theme of the active tab to its file, or to one the user names.
    fn save_theme(&mut self, ask: bool) {
        let Some(Doc::Theme(d)) = self.docs.get(self.active) else {
            return;
        };
        let dir = themes_dir();
        let path = if ask {
            let proposed = format!("{}.{}", file_stem(&d.theme.name), themes::EXTENSION);
            let dialog = rfd::FileDialog::new()
                .add_filter(tr("WinWeaver theme"), &[themes::EXTENSION])
                .set_title(tr("Save the theme as"))
                .set_file_name(proposed);
            let dialog = if std::fs::create_dir_all(&dir).is_ok() {
                dialog.set_directory(&dir)
            } else {
                dialog
            };
            let Some(path) = dialog.save_file() else {
                return;
            };
            if path.extension().is_some() {
                path
            } else {
                path.with_extension(themes::EXTENSION)
            }
        } else {
            d.theme.file_in(&dir)
        };
        let Some(Doc::Theme(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.theme.save(&path) {
            Ok(()) => {
                d.theme.id = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                d.theme.path = Some(path.clone());
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.themes = themes::themes(&dir);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// The title bar in the colours of the theme (Windows 11; the dark frame on Windows 10), given
    /// again whenever the colours change, also when the theme follows Windows.
    fn color_title_bar(&mut self, ui: &egui::Ui, frame: &eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let v = ui.visuals();
        let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
        let colors = (
            v.dark_mode,
            rgb(v.panel_fill),
            rgb(v.text_color()),
            rgb(v.window_stroke.color),
        );
        if self.title_colors == Some(colors) {
            return;
        }
        if let Ok(handle) = frame.window_handle()
            && let RawWindowHandle::Win32(w) = handle.as_raw()
        {
            winweaver_win::color_title_bar(w.hwnd.get(), colors.0, colors.1, colors.2, colors.3);
            self.title_colors = Some(colors);
        }
    }

    fn persist_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        if let Err(e) = self.settings.save_to(&Settings::path()) {
            self.status = trf("Settings not saved: {e}", &[("e", &e)]);
        }
    }

    fn window_title(&self) -> String {
        match self.docs.get(self.active) {
            Some(doc) => format!("{} - WinWeaver", doc.title()),
            None => "WinWeaver".to_owned(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.exit {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.apply_theme(ui.ctx());
        self.color_title_bar(ui, frame);
        let title = self.window_title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        let mut acts = self.shortcuts(ui.ctx());
        let (tabs_frame, tool_frame, nav_frame, status_frame) = Self::chrome_frames(ui);
        egui::Panel::top("tabs")
            .frame(tabs_frame)
            .show_separator_line(false)
            .show(ui, |ui| self.tabs_row(ui, &mut acts));
        let compact = ui.available_width() < COMPACT_WIDTH;
        if !compact && self.has_tools() {
            let search_here = !self.has_nav() && self.has_search();
            let hint = self.search_hint_now();
            egui::Panel::top("toolbar")
                .frame(tool_frame)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.toolbar(ui, &mut acts, false);
                        if search_here {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    self.search_field(ui, &mut acts, hint, 0.0);
                                },
                            );
                        }
                    });
                });
        }
        if self.has_nav() || (compact && self.has_tools()) {
            egui::Panel::top("nav")
                .frame(nav_frame)
                .show(ui, |ui| self.nav_row(ui, &mut acts, compact));
        }
        self.messages_panel(ui, &mut acts);
        egui::Panel::bottom("status")
            .frame(status_frame)
            .show(ui, |ui| self.status_bar(ui, &mut acts));
        let kind = match self.docs.get(self.active) {
            Some(Doc::Profile(_)) => 0,
            Some(Doc::Settings) => 1,
            Some(Doc::Diff(_)) => 3,
            Some(Doc::Markdown(_)) => 4,
            Some(Doc::Bookmarks) => 5,
            Some(Doc::Mcp) => 6,
            Some(Doc::Catalog(_)) => 7,
            Some(Doc::Theme(_)) => 8,
            Some(Doc::Registry(_)) => 9,
            Some(Doc::CatalogEdit(_)) => 10,
            Some(Doc::RegFile(_)) => 11,
            Some(Doc::Users(_)) => 12,
            None => 2,
        };
        if kind == 0 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            let settings = &self.settings;
            if let Some(Doc::Profile(doc)) = self.docs.get(self.active) {
                let doc: &ProfileDoc = doc;
                let p = style::palette(ui);
                egui::Panel::left("tree")
                    .resizable(true)
                    .default_size(380.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.card)
                            .inner_margin(egui::Margin::symmetric(0, 4)),
                    )
                    .show(ui, |ui| {
                        views::tree_view(
                            ui,
                            catalog,
                            texts,
                            doc,
                            &doc.search,
                            settings.inline_switches,
                            !settings.hide_unknown,
                            &mut acts,
                        );
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.bg))
                    .show(ui, |ui| {
                        views::detail_view(
                            ui,
                            catalog,
                            texts,
                            &self.imports,
                            doc,
                            settings,
                            &mut acts,
                        );
                    });
            }
        } else if kind == 3 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            let settings = &self.settings;
            let marked = self
                .current_bookmark()
                .is_some_and(|b| self.marks.find(&b).is_some());
            if let Some(Doc::Diff(doc)) = self.docs.get(self.active) {
                let doc: &DiffDoc = doc;
                let p = style::palette(ui);
                let (default, range) = list_split(ui);
                egui::Panel::left("diff-list")
                    .resizable(true)
                    .default_size(default)
                    .size_range(range)
                    .frame(egui::Frame::new().fill(p.card))
                    .show(ui, |ui| {
                        diffview::diff_view(ui, catalog, texts, doc, &mut acts);
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.bg))
                    .show(ui, |ui| {
                        diffview::diff_detail(ui, catalog, texts, doc, settings, marked, &mut acts);
                    });
            }
        } else if kind == 4 {
            if let Some(Doc::Markdown(doc)) = self.docs.get_mut(self.active) {
                egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc));
            }
        } else if kind == 6 {
            let panel = &mut self.mcp;
            let search = &self.mcp_search;
            let imports: Vec<String> = self.imports.iter().map(|f| f.id.clone()).collect();
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            let p = style::palette(ui);
            let settings_frame = egui::Frame::new().fill(p.bg);
            if panel.run.is_some() {
                let max = (ui.available_height() - 120.0).max(160.0);
                egui::Panel::top("mcp-settings")
                    .resizable(true)
                    .default_size(420.0)
                    .size_range(120.0..=max)
                    .frame(settings_frame)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| mcpview::mcp_view(ui, panel, &mut acts));
                    });
                if panel.detail_open {
                    egui::Panel::right("mcp-detail")
                        .resizable(true)
                        .default_size(380.0)
                        .frame(
                            egui::Frame::new()
                                .fill(p.bg)
                                .inner_margin(egui::Margin::same(12)),
                        )
                        .show(ui, |ui| {
                            mcpview::detail_view(ui, panel, catalog, texts, &imports, &mut acts)
                        });
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.card))
                    .show(ui, |ui| mcpview::journal_view(ui, panel, search));
            } else {
                egui::CentralPanel::default()
                    .frame(settings_frame)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| mcpview::mcp_view(ui, panel, &mut acts));
                    });
            }
        } else if kind == 5 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            let imports = &self.imports;
            let settings = &self.settings;
            let marks = &self.marks;
            let path = &self.marks_path;
            let search = &self.marks_search;
            let tab = &self.marks_tab;
            let p = style::palette(ui);
            let (default, range) = list_split(ui);
            egui::Panel::left("marks-list")
                .resizable(true)
                .default_size(default)
                .size_range(range)
                .frame(egui::Frame::new().fill(p.card))
                .show(ui, |ui| {
                    marksview::marks_view(
                        ui, catalog, texts, imports, marks, path, search, tab, &mut acts,
                    );
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.bg))
                .show(ui, |ui| {
                    marksview::marks_detail(
                        ui, catalog, texts, imports, marks, tab, settings, &mut acts,
                    );
                });
        } else if kind == 7 {
            let shown = match self.docs.get(self.active) {
                Some(Doc::Catalog(doc)) => doc.catalog(&self.imports).map(|f| (f, doc)),
                _ => None,
            };
            match shown {
                Some((file, doc)) => catalogview::catalog_view(
                    ui,
                    file,
                    doc,
                    &self.catalog,
                    &self.imports,
                    &self.settings,
                    &mut acts,
                ),
                None => {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui.label(tr("This import is no longer there."));
                    });
                }
            }
        } else if kind == 8 {
            let families = style::installed_families();
            let p = style::palette(ui);
            if let Some(Doc::Theme(doc)) = self.docs.get_mut(self.active) {
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.bg))
                    .show(ui, |ui| {
                        themeview::theme_view(ui, doc, &families, &mut acts);
                    });
            }
        } else if kind == 10 {
            if let Some(Doc::CatalogEdit(doc)) = self.docs.get_mut(self.active) {
                cataledit::editor_view(ui, doc, &mut acts);
            }
        } else if kind == 11 {
            let texts = texts_for(&self.texts, &i18n::current());
            if let Some(Doc::RegFile(doc)) = self.docs.get_mut(self.active) {
                regfileview::regfile_view(ui, doc, &self.catalog, texts, &self.imports, &mut acts);
            }
        } else if kind == 12 {
            if let Some(Doc::Users(doc)) = self.docs.get_mut(self.active) {
                usersview::users_view(ui, doc, &mut acts);
            }
        } else if kind == 9 {
            let texts = texts_for(&self.texts, &i18n::current());
            if let Some(Doc::Registry(doc)) = self.docs.get_mut(self.active) {
                regview::registry_view(ui, doc, &self.catalog, texts, &mut acts);
            }
        } else if kind == 1 {
            let p = style::palette(ui);
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.bg))
                .show(ui, |ui| self.settings_view(ui));
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(tr("No document is open. Use Menu, New or Ctrl+N."));
            });
        }
        self.confirm_view(ui.ctx(), &mut acts);
        self.ask_view(ui.ctx(), &mut acts);
        self.users_import_view(ui.ctx(), &mut acts);
        let links: Vec<String> = ui.ctx().output_mut(|o| {
            let mut found = Vec::new();
            o.commands.retain(|c| match c {
                egui::OutputCommand::OpenUrl(u) if is_doc_link(&u.url) => {
                    found.push(u.url.clone());
                    false
                }
                _ => true,
            });
            found
        });
        acts.extend(links.into_iter().map(Act::OpenDoc));
        acts.append(&mut self.pending_acts);
        self.apply(acts);
        self.poll_running(ui.ctx());
        self.poll_import(ui.ctx());
        self.poll_pc_read(ui.ctx());
        self.poll_registry_search(ui.ctx());
        let closing = self.exit || ui.ctx().input(|i| i.viewport().close_requested());
        self.write_drafts(closing);
        if closing {
            self.remember_tabs();
        }
        self.persist_settings();
        self.persist_mcp();
        self.log_status();
    }
}

/// Whether a link points into the documentation of the program (a Markdown page or a heading)
/// rather than to the web.
fn is_doc_link(url: &str) -> bool {
    !url.contains("://")
        && (url.starts_with('#')
            || url.to_ascii_lowercase().ends_with(".md")
            || url.to_ascii_lowercase().contains(".md#"))
}

/// The selection a bookmark item names: a form id, a rule id or a group id.
/// The first width of the list of a diff or of the bookmarks and the widths its divider may take:
/// the detail on the right keeps at least 300 points.
fn list_split(ui: &egui::Ui) -> (f32, std::ops::RangeInclusive<f32>) {
    let width = ui.available_width();
    let max = (width - 300.0).max(320.0);
    ((width * 0.55).clamp(320.0, max), 320.0..=max)
}

/// The profile tab with this draft key.
fn profile_doc<'a>(docs: &'a mut [Doc], key: &str) -> Option<&'a mut ProfileDoc> {
    docs.iter_mut().find_map(|d| match d {
        Doc::Profile(p) if p.draft_key == key => Some(&mut **p),
        _ => None,
    })
}

/// The note of a read of this PC without the rights that the features and capabilities need.
fn admin_issue() -> Issue {
    Issue {
        severity: Severity::Info,
        target: Target::Profile,
        text: tr("This PC: the features and capabilities of Windows are read only when WinWeaver runs as administrator")
            .to_owned(),
    }
}

/// The messages of a read of this PC or of a report: its notes, and every rule partly in effect
/// with the values that differ, so a value Windows or a program changed after the installation
/// shows where it is.
fn capture_issues(summary: &thispc::CaptureSummary) -> Vec<Issue> {
    let mut issues: Vec<Issue> = summary
        .notes
        .iter()
        .map(|n| Issue {
            severity: Severity::Info,
            target: Target::Profile,
            text: trf("This PC: {n}", &[("n", &n)]),
        })
        .collect();
    for (rule, differing) in &summary.partly {
        let mut what = differing
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        if differing.len() > 3 {
            what.push_str(&format!("; +{}", differing.len() - 3));
        }
        issues.push(Issue {
            severity: Severity::Warning,
            target: Target::Rule(rule.clone()),
            text: trf(
                "This PC: partly in effect, so read as off: {what}",
                &[("what", &what)],
            ),
        });
    }
    issues
}

/// What a profile tab selects for an item of a diff ([`Diff::keys`]).
fn sel_from_diff_key(catalog: &Catalog, key: &str) -> Sel {
    if let Some(id) = key.strip_prefix("rule:") {
        sel_from_item(catalog, id)
    } else if key.starts_with("install:") {
        Sel::Form(Form::Install)
    } else if key.starts_with("languages:") {
        Sel::Form(Form::Languages)
    } else if key == "accounts" {
        Sel::Form(Form::Accounts)
    } else {
        Sel::None
    }
}

fn sel_from_item(catalog: &Catalog, item: &str) -> Sel {
    if item.is_empty() {
        Sel::None
    } else if let Some(f) = Form::ALL.iter().find(|f| f.id() == item) {
        Sel::Form(*f)
    } else if catalog.rule(item).is_some() {
        Sel::Rule(item.to_owned())
    } else if catalog.group(item).is_some() {
        Sel::Group(item.to_owned())
    } else {
        Sel::None
    }
}

/// What differs in a policy, as the comparison page names it.
fn difference_text(what: &str) -> &'static str {
    match what {
        "title" => tr("title"),
        "description" => tr("description"),
        "values" => tr("values"),
        "values when off" => tr("values when off"),
        "parameters" => tr("parameters"),
        "category" => tr("category"),
        _ => tr("other"),
    }
}

/// A title in a line of Markdown: its marks as plain characters.
fn md_plain(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\`*_[]<>#|".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Two catalog files compared policy by policy, as a Markdown page: the counts, then the policies
/// of each alone and those that differ, at most 1000 lines a section.
fn comparison_page(first: &CatalogFile, second: &CatalogFile) -> String {
    const LINES: usize = 1000;
    let c = admx::compare(first, second);
    let mut page = format!(
        "# {}\n\n{}\n",
        md_plain(&trf(
            "{first} and {second}",
            &[("first", &first.name), ("second", &second.name)],
        )),
        trf(
            "{same} policies are the same, {changed} differ, {first_only} are only in {first} and {second_only} only in {second}.",
            &[
                ("same", &c.same),
                ("changed", &c.changed.len()),
                ("first_only", &c.only_first.len()),
                ("second_only", &c.only_second.len()),
                ("first", &md_plain(&first.name)),
                ("second", &md_plain(&second.name)),
            ],
        )
    );
    let mut section = |title: String, lines: Vec<String>| {
        if lines.is_empty() {
            return;
        }
        page.push_str(&format!("\n## {} ({})\n\n", md_plain(&title), lines.len()));
        for line in lines.iter().take(LINES) {
            page.push_str(line);
            page.push('\n');
        }
        if lines.len() > LINES {
            page.push_str(&format!(
                "\n{}\n",
                trf("and {n} more", &[("n", &(lines.len() - LINES))])
            ));
        }
    };
    let line = |id: &str, title: &str| format!("- **{}** `{id}`", md_plain(title));
    section(
        trf("Only in {name}", &[("name", &first.name)]),
        c.only_first.iter().map(|(id, t)| line(id, t)).collect(),
    );
    section(
        trf("Only in {name}", &[("name", &second.name)]),
        c.only_second.iter().map(|(id, t)| line(id, t)).collect(),
    );
    section(
        tr("Different").to_owned(),
        c.changed
            .iter()
            .map(|d| {
                let what: Vec<&str> = d.what.iter().map(|w| difference_text(w)).collect();
                format!("{}: {}", line(&d.id, &d.title), what.join(", "))
            })
            .collect(),
    );
    page
}

/// The folder of the imports of policy templates next to the executable.
pub fn imports_dir() -> PathBuf {
    exe_dir().join("imports")
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
fn import_order(id: &str) -> (bool, String) {
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
fn import_entries(dir: &Path, loaded: &[CatalogFile]) -> Vec<ImportEntry> {
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
fn size_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

/// The warning of a profile with options of catalogs that are not loaded.
fn unknown_issues(profile: &Profile) -> Vec<Issue> {
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

/// The folder of the colour themes next to the executable.
pub fn themes_dir() -> PathBuf {
    exe_dir().join("themes")
}

/// The folder of the translations next to the executable.
pub fn lang_dir() -> PathBuf {
    exe_dir().join("lang")
}

/// The folder of the built files next to the executable.
pub fn output_dir() -> PathBuf {
    exe_dir().join("output")
}

/// Writes a file through a temporary neighbour that replaces the target.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The folder of the user's profiles next to the executable.
pub fn profiles_dir() -> PathBuf {
    exe_dir().join("profiles")
}

/// What a stand-alone script does with the selection, in the language of the interface.
fn summary_of(plan: &scripts::ScriptPlan) -> String {
    let counts = [
        (plan.applied.len(), tr("{n} rules applied")),
        (plan.reverted.len(), tr("{n} returned to the defaults")),
        (plan.added.len(), tr("{n} added as dependencies")),
        (plan.excluded.len(), tr("{n} left out")),
        (plan.partial.len(), tr("{n} only partly revertible")),
    ];
    let parts: Vec<String> = counts
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, text)| text.replace("{n}", &n.to_string()))
        .collect();
    if parts.is_empty() {
        tr("nothing to do").to_owned()
    } else {
        parts.join(", ")
    }
}

/// A path as the files of the program keep it, in lower case: the name of an import made of a
/// file or a folder inside the program folder stays when the folder moves.
fn kept_name(path: &Path) -> String {
    let kept = winweaver_core::paths::stored(path, &exe_dir());
    kept.display().to_string().to_lowercase()
}

/// A file name stem made of a profile name.
pub fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim().to_owned();
    if stem.is_empty() {
        "profile".to_owned()
    } else {
        stem
    }
}

/// The path with the profile extension when it has none.
/// A path the save dialog gave, with the extension of a registry file when it has none.
fn reg_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(regfile::EXTENSION)
    }
}

fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(profile::EXTENSION)
    }
}

fn file_dialog(kind: &str, extension: &str) -> rfd::FileDialog {
    let dir = profiles_dir();
    let dialog = rfd::FileDialog::new().add_filter(kind, &[extension]);
    if dir.is_dir() {
        dialog.set_directory(dir)
    } else {
        dialog
    }
}

fn profile_dialog() -> rfd::FileDialog {
    file_dialog("WinWeaver profile", profile::EXTENSION)
}

fn pick_profile() -> Option<PathBuf> {
    profile_dialog().set_title(tr("Open a profile")).pick_file()
}

/// The open dialog for every kind of file the program opens.
fn pick_any() -> Option<PathBuf> {
    let dir = profiles_dir();
    let dialog = rfd::FileDialog::new()
        .set_title(tr("Open"))
        .add_filter(
            tr("WinWeaver files"),
            &[
                profile::EXTENSION,
                diff::EXTENSION,
                bookmarks::EXTENSION,
                admx::CATALOG_EXTENSION,
                themes::EXTENSION,
                winweaver_mcp::config::EXTENSION,
                users::EXTENSION,
            ],
        )
        .add_filter(tr("Registry files"), &[regfile::EXTENSION])
        .add_filter(tr("Policy templates (opened without importing)"), &["admx"])
        .add_filter(tr("Answer files (the profile they embed)"), &["xml"])
        .add_filter(tr("WinKickOff profiles and audit reports"), &["json"])
        .add_filter(tr("Markdown pages"), &["md"]);
    let dialog = if dir.is_dir() {
        dialog.set_directory(dir)
    } else {
        dialog
    };
    dialog.pick_file()
}

fn ask_save_path(proposed: &str) -> Option<PathBuf> {
    let _ = std::fs::create_dir_all(profiles_dir());
    profile_dialog()
        .set_title(tr("Save the profile as"))
        .set_file_name(proposed)
        .save_file()
}

/// The renderer of `--renderer` (an error for a name this executable lacks), else of the settings
/// (Automatic for a name it lacks); `None` is Automatic.
fn chosen_renderer(cli: Option<&str>, settings: &Settings) -> Result<Option<Renderer>, String> {
    match cli.map(str::trim) {
        Some("auto") => Ok(None),
        Some(name) => Renderer::parse(name).map(Some).ok_or_else(|| {
            let names: Vec<&str> = Renderer::ALL.iter().map(|r| r.id()).collect();
            format!(
                "unknown renderer {name}: this executable has auto, {}",
                names.join(", ")
            )
        }),
        None => Ok(Renderer::parse(&settings.renderer)),
    }
}

/// Opens a small window with the renderers of `--renderer` or the settings, draws two frames and
/// closes it: what it drew with and the renderers that did not start before. For CI
/// (`--smoke-window`).
pub fn smoke_window(cli_renderer: Option<&str>) -> Result<String, String> {
    struct Smoke {
        frames: u32,
    }
    impl eframe::App for Smoke {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ui, |ui| ui.label("WinWeaver"));
            self.frames += 1;
            if self.frames >= 2 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
    let settings = Settings::load_from(&Settings::path()).unwrap_or_default();
    let order = renderer::order(chosen_renderer(cli_renderer, &settings)?);
    let info = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let viewport = egui::ViewportBuilder::default()
        .with_title("WinWeaver smoke test")
        .with_inner_size([400.0, 300.0]);
    let (_, failures) = renderer::start("WinWeaver smoke test", &viewport, &order, |_, _| {
        let seen = info.clone();
        Box::new(move |cc| {
            *seen.lock().unwrap_or_else(|e| e.into_inner()) = renderer::graphics_text(cc);
            Ok(Box::new(Smoke { frames: 0 }))
        })
    })?;
    let mut text = info.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for failure in failures {
        text.push_str("; ");
        text.push_str(&failure);
    }
    Ok(text)
}

/// Opens the window with the renderer of `--renderer` or of the settings first; returns the
/// error text when the catalog or the window cannot start.
pub fn run(cli_renderer: Option<&str>) -> Result<(), String> {
    let catalog = Catalog::embedded().map_err(|e| format!("catalog: {e}"))?;
    let texts = CatalogTexts::embedded().map_err(|e| format!("translations: {e}"))?;
    let (settings, warning) = match Settings::load_from(&Settings::path()) {
        Ok(s) => (s, None),
        Err(e) => (Settings::default(), Some(e)),
    };
    let order = renderer::order(chosen_renderer(cli_renderer, &settings)?);
    let viewport = egui::ViewportBuilder::default()
        .with_title("WinWeaver")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([900.0, 560.0]);
    renderer::start("WinWeaver", &viewport, &order, |drawn, failures| {
        let catalog = catalog.clone();
        let texts = texts.clone();
        let settings = settings.clone();
        let warning = warning.clone();
        let failures = failures.to_vec();
        Box::new(move |cc| {
            style::install_fonts(&cc.egui_ctx);
            let mut app = App::new(catalog, texts, settings);
            app.graphics = renderer::graphics_text(cc);
            session::log(
                &app.logs_dir,
                "INFO",
                &format!(
                    "WinWeaver {} started; graphics: {}",
                    winweaver_core::BUILD,
                    app.graphics
                ),
            );
            app.load_mcp_config();
            app.apply_language();
            if let Some(w) = warning {
                app.status = trf(
                    "Settings not read, the defaults are used: {w}",
                    &[("w", &w)],
                );
            }
            match Bookmarks::load_or_default(&app.marks_path) {
                Ok(marks) => app.marks = marks,
                Err(e) => app.status = trf("Bookmarks not read: {e}", &[("e", &e)]),
            }
            app.load_imports();
            app.restore_session();
            if app.settings.show_guide_at_start {
                app.open_guide();
                app.settings.show_guide_at_start = false;
                app.settings_dirty = true;
            }
            if !failures.is_empty() {
                app.status = trf(
                    "{failed}; the window draws with {renderer}",
                    &[
                        ("failed", &failures.join("; ")),
                        ("renderer", &drawn.title()),
                    ],
                );
            }
            Ok(Box::new(app))
        })
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems_are_safe() {
        assert_eq!(file_stem("Office 2026"), "Office 2026");
        assert_eq!(file_stem("a/b:c"), "a_b_c");
        assert_eq!(file_stem("   "), "profile");
        assert_eq!(
            with_extension(PathBuf::from("x")),
            PathBuf::from("x.wwprofile")
        );
        assert_eq!(
            with_extension(PathBuf::from("x.json")),
            PathBuf::from("x.json")
        );
    }

    #[test]
    fn app_starts_with_the_office_preset() {
        let catalog = Catalog::embedded().unwrap();
        let app = App::new(
            catalog,
            CatalogTexts::embedded().unwrap(),
            Settings::default(),
        );
        assert_eq!(app.docs.len(), 1);
        assert_eq!(app.docs[0].title(), "Office.wwprofile");
        assert!(!app.docs[0].dirty());
        assert_eq!(app.window_title(), "Office.wwprofile - WinWeaver");
    }

    #[test]
    fn the_guide_opens_once_and_markdown_files_open() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::OpenGuide, Act::OpenGuide]);
        assert_eq!(app.docs.len(), 2, "the guide tab exists once");
        assert_eq!(app.docs[1].title(), "Guide.md");
        let dir = std::env::temp_dir().join(format!("winweaver-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("notes.md");
        std::fs::write(&path, "# Notes\n\nA line.\n").unwrap();
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), 3);
        assert!(matches!(&app.docs[2], Doc::Markdown(m) if m.text.starts_with("# Notes")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Draws frames of the whole window without a screen (egui_kittest): every kind of tab is
    /// drawn and shows its texts, and a click on a rule of the tree selects it.
    #[test]
    fn every_kind_of_tab_draws_without_a_screen() {
        use egui_kittest::kittest::Queryable;
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(
            catalog,
            CatalogTexts::embedded().unwrap(),
            Settings::default(),
        );
        let empty = admx::to_catalog_file(
            &admx::Templates::default(),
            "empty",
            "Empty templates",
            "",
            "now",
        );
        app.imports.push(empty);
        app.rebuild_catalog();
        let mut harness = egui_kittest::Harness::new_eframe(|_| app);
        harness.step();
        let has = |h: &egui_kittest::Harness<'_, App>, label: &str| {
            h.query_all_by_label(label).next().is_some()
        };
        assert!(has(&harness, "Menu"));
        for (act, label) in [
            (Act::OpenSettings, Some("General")),
            (
                Act::OpenCatalog("empty".to_owned()),
                Some("Select a policy in the tree."),
            ),
            (Act::OpenMarks, None),
            (Act::OpenMcp, None),
            (Act::OpenGuide, None),
            (Act::EditTheme("matrix".to_owned()), Some("Use this theme")),
            (Act::Activate(0), None),
        ] {
            harness.state_mut().apply(vec![act]);
            harness.step();
            harness.step();
            if let Some(label) = label {
                assert!(has(&harness, label), "{label} is not drawn");
            }
        }
        assert_eq!(harness.state().docs.len(), 7);
        assert_eq!(
            harness.state().docs[harness.state().active].title(),
            "Office.wwprofile"
        );
        harness.state_mut().apply(vec![
            Act::Expand("unattend".to_owned(), true),
            Act::Expand("install".to_owned(), true),
        ]);
        harness.step();
        harness.step();
        let title = "Bypass the TPM 2.0 check";
        assert!(has(&harness, title), "the rules of an open group are drawn");
        harness.query_all_by_label(title).next().unwrap().click();
        harness.step();
        harness.step();
        assert!(
            matches!(harness.state().docs[0], Doc::Profile(ref d) if d.sel == Sel::Rule("install.bypass-tpm".into())),
            "a click on a rule selects it"
        );
        harness.state_mut().apply(vec![Act::CompareWith("Strict")]);
        harness.step();
        harness.step();
        assert!(has(&harness, "Select a difference to see it here."));
        let key = first_rule_key(harness.state()).expect("Strict differs in a rule");
        harness.state_mut().apply(vec![Act::DiffSelect(key)]);
        harness.step();
        harness.step();
        assert!(
            has(&harness, "Apply this difference"),
            "the item is shown on the right"
        );
    }

    /// The key of the first rule of the diff in the last tab.
    fn first_rule_key(app: &App) -> Option<String> {
        match app.docs.last() {
            Some(Doc::Diff(d)) => d.diff.keys().into_iter().find(|k| k.starts_with("rule:")),
            _ => None,
        }
    }

    /// The divider between the differences and the item on the right stays where it is whatever
    /// item is selected: the item never widens the list or pushes into it.
    #[test]
    fn the_divider_of_a_diff_stays_put_whatever_item_is_shown() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(
            catalog,
            CatalogTexts::embedded().unwrap(),
            Settings::default(),
        );
        app.apply(vec![Act::CompareWith("Strict")]);
        let keys = match app.docs.last() {
            Some(Doc::Diff(d)) => d.diff.keys(),
            _ => Vec::new(),
        };
        assert!(keys.len() > 3, "{keys:?}");
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .build_eframe(|_| app);
        let list = egui::Id::new(("diff-list", "Office", "Strict"));
        let width =
            |h: &egui_kittest::Harness<'_, App>| h.ctx.read_response(list).map(|r| r.rect.width());
        harness.step();
        harness.step();
        let first = width(&harness).expect("the list is drawn");
        for key in keys {
            harness
                .state_mut()
                .apply(vec![Act::DiffSelect(key.clone())]);
            harness.step();
            harness.step();
            assert_eq!(width(&harness), Some(first), "the divider moved for {key}");
        }
    }

    /// Pictures of the window for the review of its design. Only when `WINWEAVER_SCREENSHOTS` names
    /// a folder (CI sets it on the Windows runner): the main tabs are drawn with wgpu without a
    /// screen and saved there as PNG files; a renderer that cannot start writes `error.txt`.
    #[test]
    fn screenshots_of_the_window() {
        let Some(dir) = std::env::var_os("WINWEAVER_SCREENSHOTS") else {
            return;
        };
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let catalog = Catalog::embedded().unwrap();
        let app = App::new(
            catalog,
            CatalogTexts::embedded().unwrap(),
            Settings::default(),
        );
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .with_pixels_per_point(1.0)
            .wgpu()
            .build_eframe(|cc| {
                style::install_fonts(&cc.egui_ctx);
                app
            });
        let shot = |h: &mut egui_kittest::Harness<'_, App>, name: &str| {
            for _ in 0..4 {
                h.step();
            }
            match h.render() {
                Ok(image) => {
                    if let Err(e) = image.save(dir.join(format!("{name}.png"))) {
                        let _ = std::fs::write(dir.join("error.txt"), format!("{name}: {e}"));
                    }
                }
                Err(e) => {
                    let _ = std::fs::write(dir.join("error.txt"), format!("{name}: {e}"));
                }
            }
        };
        shot(&mut harness, "01-profile");
        harness.state_mut().apply(vec![
            Act::Expand("policies".to_owned(), true),
            Act::Expand("defender".to_owned(), true),
            Act::Select(Sel::Rule("defender.pua".to_owned())),
        ]);
        shot(&mut harness, "02-rule");
        harness.state_mut().apply(vec![
            Act::ToggleBookmark,
            Act::SetMarkComment(0, "Ask the customer about the Audit mode".into()),
            Act::Check,
        ]);
        shot(&mut harness, "03-checked");
        harness.state_mut().apply(vec![Act::CompareWith("Strict")]);
        if let Some(key) = first_rule_key(harness.state()) {
            harness.state_mut().apply(vec![Act::DiffSelect(key)]);
        }
        shot(&mut harness, "04-diff");
        for (act, name) in [
            (Act::OpenSettings, "05-settings"),
            (Act::OpenMcp, "06-mcp"),
            (Act::OpenMarks, "07-bookmarks"),
            (Act::OpenGuide, "08-guide"),
        ] {
            harness.state_mut().apply(vec![act]);
            if name == "07-bookmarks" {
                harness.state_mut().apply(vec![Act::MarkSelect(0)]);
            }
            shot(&mut harness, name);
        }
        harness
            .state_mut()
            .apply(vec![Act::EditTheme("latte".to_owned())]);
        shot(&mut harness, "10-theme-editor");
        harness
            .state_mut()
            .apply(vec![Act::UseTheme("matrix".to_owned()), Act::Activate(0)]);
        shot(&mut harness, "11-matrix");
        harness
            .state_mut()
            .apply(vec![Act::UseTheme("light".to_owned())]);
        // A registry file of the plan of the first preset, at its first key.
        let catalog = Catalog::embedded().unwrap();
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0).unwrap();
        let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
        let plan = winweaver_core::apply::plan_apply(&catalog, &profile, &ids, "shot");
        let reg = dir.join("Apply-Office.reg");
        let file = RegFile::from_plan(&plan, "Apply the profile Office");
        file.save(&reg).unwrap();
        harness.state_mut().apply(vec![Act::OpenPath(reg)]);
        if let Some(Doc::RegFile(d)) = harness.state_mut().docs.last_mut() {
            let key = |l: &Line| matches!(l, Line::Key { .. });
            d.at = d.file.lines.iter().position(key);
        }
        shot(&mut harness, "12-regfile");
        harness.set_size(egui::vec2(1000.0, 700.0));
        shot(&mut harness, "09-compact");
    }

    #[test]
    fn a_theme_tab_previews_its_theme_and_a_theme_can_be_chosen() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        let (light, _, preference) = app.wanted_theme();
        assert_eq!(light.id, "light");
        assert_eq!(preference, egui::ThemePreference::System);
        app.apply(vec![Act::EditTheme("matrix".into())]);
        assert!(matches!(&app.docs[1], Doc::Theme(t) if t.theme.id == "matrix"));
        assert_eq!(app.docs[1].title(), "matrix.wwtheme");
        if let Some(Doc::Theme(t)) = app.docs.get_mut(1) {
            t.theme.font_size = 16.0;
        }
        let (_, dark, preference) = app.wanted_theme();
        assert_eq!(
            dark.id, "matrix",
            "the edited theme shows while its tab is in front"
        );
        assert_eq!(dark.font_size, 16.0);
        assert_eq!(preference, egui::ThemePreference::Dark);
        app.apply(vec![Act::EditTheme("matrix".into())]);
        assert_eq!(app.docs.len(), 2, "one tab per theme");
        app.apply(vec![Act::Activate(0)]);
        assert_eq!(app.wanted_theme().0.id, "light", "the settings theme again");
        app.apply(vec![Act::UseTheme("latte".into())]);
        assert_eq!(app.settings.custom_theme, "latte");
        assert_eq!(app.settings.theme, ThemeChoice::Light);
        let (light, _, preference) = app.wanted_theme();
        assert_eq!(light.name, "Latte");
        assert_eq!(preference, egui::ThemePreference::Light);
        assert_eq!(app.shown_theme_id(true), "latte");
    }

    #[test]
    fn bookmarks_toggle_and_lead_back() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        let dir = std::env::temp_dir().join(format!("winweaver-marks-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        app.marks_path = dir.join(bookmarks::DEFAULT_FILE);
        app.apply(vec![
            Act::Select(Sel::Rule("install.bypass-tpm".into())),
            Act::ToggleBookmark,
        ]);
        assert_eq!(app.marks.items.len(), 1, "{}", app.status);
        assert_eq!(app.marks.items[0].item, "install.bypass-tpm");
        assert_eq!(
            app.marks.items[0].path, "",
            "an unsaved profile has no file to link"
        );
        assert_eq!(
            app.marks.items[0].title, "",
            "the title comes from the catalog"
        );
        assert!(
            app.marks_path.exists(),
            "the default file is created at the first bookmark"
        );
        app.apply(vec![
            Act::Select(Sel::None),
            Act::OpenMarks,
            Act::GoToMark(0),
        ]);
        assert_eq!(app.active, 0, "{}", app.status);
        assert_eq!(
            app.active_profile().unwrap().sel,
            Sel::Rule("install.bypass-tpm".into())
        );
        app.apply(vec![Act::ToggleBookmark]);
        assert!(app.marks.items.is_empty());
        app.apply(vec![
            Act::ToggleBookmark,
            Act::SetMarkComment(0, "Ask the customer\nabout the TPM".into()),
            Act::ToggleBookmark,
        ]);
        assert_eq!(
            app.marks.items.len(),
            1,
            "a bookmark with a comment asks first"
        );
        assert!(app.ask.is_some());
        app.apply(vec![Act::AskNo]);
        assert_eq!(app.marks.items.len(), 1);
        app.apply(vec![
            Act::OpenMarks,
            Act::MarkSelect(0),
            Act::RemoveMarkAsk(0),
        ]);
        assert!(app.ask.is_some());
        app.apply(vec![Act::AskYes]);
        assert!(app.marks.items.is_empty());
        assert_eq!(app.marks_tab.sel, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn drafts_come_back_after_a_restart() {
        let catalog = Catalog::embedded().unwrap();
        let dir = std::env::temp_dir().join(format!("winweaver-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
        app.drafts_dir = dir.join("drafts");
        app.logs_dir = dir.join("logs");
        app.apply(vec![Act::Toggle("install.bypass-tpm".into(), false)]);
        app.write_drafts(true);
        let (drafts, _) = session::read_drafts(&app.drafts_dir);
        assert_eq!(drafts.len(), 1);
        let mut next = App::new(catalog, Vec::new(), Settings::default());
        next.drafts_dir = app.drafts_dir.clone();
        next.logs_dir = app.logs_dir.clone();
        next.restore_session();
        assert_eq!(
            next.docs.len(),
            1,
            "the untouched start tab gives way to the draft"
        );
        let Doc::Profile(doc) = &next.docs[0] else {
            panic!("a profile");
        };
        assert!(doc.dirty && !doc.profile.is_on("install.bypass-tpm"));
        next.apply(vec![Act::CloseTab(0, true)]);
        assert!(
            session::read_drafts(&next.drafts_dir).0.is_empty(),
            "discarding deletes the draft"
        );
        next.log_status();
        assert!(dir.join("logs").is_dir());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn history_moves_back_and_forward() {
        let catalog = Catalog::embedded().unwrap();
        let mut doc = ProfileDoc::new(Profile::from_defaults("x", &catalog), None, &catalog);
        doc.select(Sel::Group("policies".into()));
        doc.select(Sel::Rule("install.bypass-tpm".into()));
        assert!(doc.can_go(-1) && !doc.can_go(1));
        doc.go(-1);
        assert_eq!(doc.sel, Sel::Group("policies".into()));
        doc.go(1);
        assert_eq!(doc.sel, Sel::Rule("install.bypass-tpm".into()));
    }

    #[test]
    fn comparing_with_a_preset_opens_a_diff_tab_that_applies() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::CompareWith("Strict")]);
        assert_eq!(app.docs.len(), 2);
        assert!(matches!(&app.docs[1], Doc::Diff(d) if !d.diff.is_empty()));
        assert_eq!(app.docs[1].title(), "Office vs Strict.wwdiff");
        app.apply(vec![Act::ApplyDiff]);
        assert_eq!(app.active, 0);
        let Doc::Profile(office) = &app.docs[0] else {
            panic!("the first tab is the profile");
        };
        assert!(office.dirty);
        let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
        assert_eq!(office.profile.rules, strict.rules);
    }

    #[test]
    fn an_item_of_a_diff_takes_a_comment_applies_alone_and_leads_to_both_profiles() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::CompareWith("Strict")]);
        let Doc::Diff(d) = &app.docs[1] else {
            panic!("the second tab is the diff");
        };
        let key = d
            .diff
            .keys()
            .into_iter()
            .find(|k| k.starts_with("rule:"))
            .expect("Strict differs in a rule");
        let id = key.trim_start_matches("rule:").to_owned();
        app.apply(vec![
            Act::DiffSelect(key.clone()),
            Act::DiffComment(
                key.clone(),
                "Checked with the lab
and kept"
                    .into(),
            ),
        ]);
        let Doc::Diff(d) = &app.docs[1] else {
            panic!("the second tab is the diff");
        };
        assert_eq!(d.sel.as_deref(), Some(key.as_str()));
        assert_eq!(
            d.diff.comments[&key],
            "Checked with the lab
and kept"
        );
        assert!(d.dirty);
        app.apply(vec![Act::DiffComment(key.clone(), "  ".into())]);
        assert!(matches!(&app.docs[1], Doc::Diff(d) if d.diff.comments.is_empty()));
        app.apply(vec![Act::DiffApplyItem(key.clone())]);
        assert_eq!(app.active, 1, "the diff stays in front");
        let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
        let Doc::Profile(office) = &app.docs[0] else {
            panic!("the first tab is the profile");
        };
        assert_eq!(office.profile.rules.get(&id), strict.rules.get(&id));
        app.apply(vec![Act::DiffGoTo(key.clone(), true)]);
        assert_eq!(app.docs.len(), 3, "the preset Strict opens in a new tab");
        assert_eq!(app.active, 2);
        assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id.clone()));
        app.active = 1;
        app.apply(vec![Act::DiffGoTo(key, false)]);
        assert_eq!(app.active, 0);
        assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id));
    }

    #[test]
    fn tabs_move_and_the_tab_in_front_stays_in_front() {
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::OpenGuide, Act::OpenMarks, Act::OpenSettings]);
        let titles = |app: &App| app.docs.iter().map(Doc::title).collect::<Vec<_>>();
        let start = titles(&app);
        assert_eq!(start.len(), 4);
        assert_eq!(app.active, 3);
        app.apply(vec![Act::MoveTab(3, 0)]);
        assert_eq!(app.active, 0, "the moved tab stays in front");
        assert_eq!(titles(&app)[0], start[3]);
        assert_eq!(titles(&app)[1..], start[..3]);
        app.apply(vec![Act::Activate(2), Act::MoveTab(0, 3)]);
        assert_eq!(
            app.active, 1,
            "the tab in front shifts left when a tab before it moves after it"
        );
        assert_eq!(titles(&app), start);
        app.apply(vec![Act::MoveTab(1, 1), Act::MoveTab(0, 9)]);
        assert_eq!(titles(&app), start, "nothing moves out of the row");
    }

    #[test]
    fn opening_a_file_adds_a_tab_and_remembers_it() {
        let catalog = Catalog::embedded().unwrap();
        let dir = std::env::temp_dir().join(format!("winweaver-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("strict.wwprofile");
        std::fs::write(&path, PRESETS[1].1).unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.active, 1);
        assert_eq!(app.settings.recent, vec![path.clone()]);
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), 2, "the same file activates its tab");
        app.open_path(dir.join("missing.wwprofile"));
        assert!(app.status.starts_with("Open failed"), "{}", app.status);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_registry_file_opens_in_its_tab_and_keeps_its_comments() {
        let catalog = Catalog::embedded().unwrap();
        let dir = std::env::temp_dir().join(format!("winweaver-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.reg");
        let text = [
            regfile::HEADER,
            "",
            "; kept",
            "[HKEY_CURRENT_USER\\Software\\WinWeaver]",
            "\"A\"=dword:00000001",
        ]
        .join("\r\n");
        std::fs::write(&path, text).unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::OpenPath(path.clone())]);
        let Some(Doc::RegFile(d)) = app.docs.get_mut(app.active) else {
            panic!("the file has no tab: {}", app.status);
        };
        // A new value goes after the last value of the selected key.
        d.at = Some(2);
        d.start_new(regfileview::EditKind::Value);
        let edit = d.edit.as_mut().unwrap();
        edit.name = "B".to_owned();
        edit.text = "text".to_owned();
        assert!(d.apply_edit());
        assert!(app.docs[app.active].dirty());
        app.apply(vec![Act::Save]);
        assert!(!app.docs[app.active].dirty(), "{}", app.status);
        let saved = RegFile::load(&path).unwrap().to_text();
        assert!(saved.contains("; kept"), "{saved}");
        assert!(saved.contains("\"B\"=\"text\""), "{saved}");
        // Removing the key asks first; its comment stays.
        if let Some(Doc::RegFile(d)) = app.docs.get_mut(app.active) {
            d.line = None;
        }
        app.apply(vec![Act::AskRegFileRemove]);
        assert!(app.ask.is_some());
        app.apply(vec![Act::AskYes]);
        let Some(Doc::RegFile(d)) = app.docs.get(app.active) else {
            panic!("the tab of the file is gone");
        };
        let text = d.file.to_text();
        assert!(text.contains("; kept"), "{text}");
        assert!(!text.contains("WinWeaver]"), "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_link_selects_and_opens_the_tree_down_to_it() {
        let catalog = Catalog::embedded().unwrap();
        let group = catalog
            .groups()
            .iter()
            .find(|g| g.parent.is_some())
            .map(|g| g.id.clone())
            .unwrap();
        let parent = catalog
            .group(&group)
            .and_then(|g| g.parent.clone())
            .unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        if let Some(Doc::Profile(doc)) = app.docs.get_mut(0) {
            doc.expanded.clear();
        }
        app.apply(vec![Act::Reveal(Sel::Group(group.clone()))]);
        let Some(Doc::Profile(doc)) = app.docs.first() else {
            panic!("a profile");
        };
        assert_eq!(doc.sel, Sel::Group(group));
        assert!(doc.expanded.contains(&parent), "the tree opens to it");
        assert!(doc.reveal);
    }

    #[test]
    fn the_rights_of_a_key_change_after_a_question() {
        use winweaver_core::registry::{MemoryRegistry, Registry};
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        let reg = std::sync::Arc::new(MemoryRegistry::default());
        let key = "HKEY_CURRENT_USER\\Software\\WinWeaverRights";
        reg.create_key(key).unwrap();
        app.registry = reg.clone();
        app.apply(vec![Act::OpenRegistry(key.to_owned(), None)]);
        app.apply(vec![Act::RegistryRights]);
        let open = |app: &App| match app.docs.get(app.active) {
            Some(Doc::Registry(d)) => d.rights.is_some(),
            _ => false,
        };
        let Some(Doc::Registry(d)) = app.docs.get_mut(app.active) else {
            panic!("the registry tab");
        };
        let rights = d.rights.as_mut().expect("the window of the rights");
        assert_eq!(rights.rights.entries.len(), 2, "what the key inherits");
        let owner = rights.names.get("BA").map(String::as_str);
        assert_eq!(owner, Some("Administrators"));
        rights.add = "Users".to_owned();
        app.apply(vec![Act::RegistryRightsAdd, Act::AskRegistryRights]);
        assert!(app.ask.is_some(), "a change of the rights asks first");
        assert!(open(&app));
        app.apply(vec![Act::AskYes]);
        let text = reg.security(key).unwrap();
        assert!(text.contains("(A;CI;KR;;;BU)"), "{text}");
        assert!(!open(&app), "the window closes");
    }

    #[test]
    fn a_group_shows_in_the_profile_from_another_tab() {
        let catalog = Catalog::embedded().unwrap();
        let group = catalog
            .groups()
            .iter()
            .find(|g| g.parent.is_some())
            .map(|g| g.id.clone())
            .unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.apply(vec![Act::OpenMarks, Act::ShowGroupInProfile(group.clone())]);
        assert_eq!(app.active, 0, "the profile tab comes to the front");
        let Some(Doc::Profile(doc)) = app.docs.first() else {
            panic!("a profile");
        };
        assert_eq!(doc.sel, Sel::Group(group));
        assert!(doc.reveal);
    }

    const SAMPLE_ADMX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions" revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target prefix="sample" namespace="Test.Policies.Sample" />
    <using prefix="windows" namespace="Microsoft.Policies.Windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="Sample" displayName="$(string.Sample)">
      <parentCategory ref="windows:WindowsComponents" />
    </category>
  </categories>
  <policies>
    <policy name="TurnOff" class="Machine" displayName="$(string.TurnOff)" key="Software\Policies\Test" valueName="Off">
      <parentCategory ref="Sample" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
  </policies>
</policyDefinitions>"#;

    const SAMPLE_ADML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <resources>
    <stringTable>
      <string id="Sample">Sample</string>
      <string id="TurnOff">Turn the sample off</string>
    </stringTable>
  </resources>
</policyDefinitionResources>"#;

    /// Writes the sample templates as the catalog file `test` into `<dir>/imports`.
    fn sample_import(dir: &Path) {
        let admx = dir.join("admx");
        std::fs::create_dir_all(admx.join("en-US")).unwrap();
        std::fs::write(admx.join("sample.admx"), SAMPLE_ADMX).unwrap();
        std::fs::write(admx.join("en-US").join("sample.adml"), SAMPLE_ADML).unwrap();
        let templates = admx::read_folder(&admx);
        let file = admx::to_catalog_file(&templates, "test", "Test templates", "", "now");
        assert!(
            file.rules.iter().any(|r| r.id == "admx.sample.turnoff"),
            "{:?}",
            file.skipped
        );
        let imports = dir.join("imports");
        std::fs::create_dir_all(&imports).unwrap();
        file.save(&imports.join("test.wwcatalog")).unwrap();
    }

    /// Only the imports the settings name are read; Load catalogs finds the one an unknown option
    /// of the profile belongs to, marks it and loads it; unchecking it forgets it again.
    #[test]
    fn only_the_checked_imports_load_and_unknown_options_find_their_catalog() {
        let dir = std::env::temp_dir().join(format!("winweaver-unknown-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        sample_import(&dir);
        let catalog = Catalog::embedded().unwrap();
        let earlier = Settings {
            loaded_imports: None,
            hidden_imports: vec!["test".to_owned()],
            ..Settings::default()
        };
        let mut app = App::new(catalog.clone(), Vec::new(), earlier);
        app.imports_dir = dir.join("imports");
        app.load_imports();
        assert!(app.imports.is_empty(), "a hidden import stays unread");
        assert_eq!(app.settings.loaded_imports, Some(Vec::new()));
        assert_eq!(app.import_list.len(), 1);
        assert_eq!(
            app.import_list[0].name, "Test templates",
            "the name comes from the header of the file"
        );
        let id = "admx.sample.turnoff";
        let mut profile = Profile::new("P", catalog.version());
        profile.set_on(id, true);
        profile.split_unknown(&app.catalog);
        app.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
            profile,
            None,
            &app.catalog,
        )))];
        app.active = 0;
        let state = |app: &App| match &app.docs[0] {
            Doc::Profile(d) => (
                d.profile.unknown.len(),
                d.profile.is_on(id),
                d.issues.iter().any(|i| i.severity == Severity::Warning),
                d.sel.clone(),
            ),
            _ => panic!("a profile"),
        };
        assert_eq!(state(&app).0, 1);
        assert!(state(&app).2, "the open warns of the option");
        app.apply(vec![Act::Select(Sel::Unknown(id.to_owned()))]);
        app.apply(vec![Act::LoadCatalogs]);
        assert_eq!(app.imports.len(), 1, "{}", app.status);
        assert_eq!(app.settings.loaded_imports, Some(vec!["test".to_owned()]));
        let (unknown, on, _, sel) = state(&app);
        assert_eq!((unknown, on), (0, true), "the option is known and on again");
        assert_eq!(sel, Sel::Rule(id.to_owned()), "the selection follows it");
        app.apply(vec![Act::ToggleImport("test".to_owned())]);
        assert!(
            app.imports.is_empty(),
            "unchecked, the import leaves memory"
        );
        assert_eq!(app.settings.loaded_imports, Some(Vec::new()));
        let (unknown, _, _, sel) = state(&app);
        assert_eq!((unknown, sel), (1, Sel::Unknown(id.to_owned())));
        let mut next = App::new(
            catalog,
            Vec::new(),
            Settings {
                loaded_imports: None,
                ..Settings::default()
            },
        );
        next.imports_dir = dir.join("imports");
        next.load_imports();
        assert_eq!(
            next.imports.len(),
            1,
            "settings of an earlier version load what was not hidden"
        );
        assert_eq!(next.settings.loaded_imports, Some(vec!["test".to_owned()]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The options of catalogs that are not loaded have their group at the end of the tree,
    /// unless the settings hide them.
    #[test]
    fn the_unknown_options_have_their_group_in_the_tree() {
        use egui_kittest::kittest::Queryable;
        let catalog = Catalog::embedded().unwrap();
        let mut profile = Profile::new("P", catalog.version());
        profile.set_on("admx.other.policy", true);
        profile.split_unknown(&catalog);
        let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
        app.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
            profile, None, &catalog,
        )))];
        app.active = 0;
        app.apply(vec![Act::Expand(UNKNOWN_GROUP.to_owned(), true)]);
        let mut harness = egui_kittest::Harness::new_eframe(|_| app);
        harness.step();
        let has = |h: &egui_kittest::Harness<'_, App>, label: &str| {
            h.query_all_by_label(label).next().is_some()
        };
        assert!(has(&harness, "Unknown options"));
        assert!(has(&harness, "admx.other.policy"));
        harness.state_mut().apply(vec![Act::Select(Sel::Unknown(
            "admx.other.policy".to_owned(),
        ))]);
        harness.step();
        assert!(has(&harness, "Load catalogs"));
        harness.state_mut().apply(vec![Act::Select(Sel::None)]);
        harness.state_mut().settings.hide_unknown = true;
        harness.step();
        harness.step();
        assert!(!has(&harness, "Unknown options"), "hidden by the settings");
    }

    /// A new catalog gets a policy in its editor; a key of another root is refused and changes
    /// nothing; the save loads the policy into the catalog; a removal asks first.
    #[test]
    fn a_new_catalog_is_edited_checked_and_saved() {
        let dir = std::env::temp_dir().join(format!("winweaver-editor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let catalog = Catalog::embedded().unwrap();
        let settings = Settings {
            loaded_imports: Some(Vec::new()),
            ..Settings::default()
        };
        let mut app = App::new(catalog, Vec::new(), settings);
        app.imports_dir = dir.join("imports");
        app.apply(vec![Act::NewCatalog]);
        let Some(Doc::CatalogEdit(d)) = app.docs.get_mut(app.active) else {
            panic!("the editor: {}", app.status);
        };
        d.new_name = "Mute".to_owned();
        d.new_policy();
        let id = d.form.as_ref().map(|f| f.id.clone()).unwrap();
        assert_eq!(id, "admx.custom-1.mute");
        if let Some(form) = d.form.as_mut() {
            form.actions.push(cataledit::ActionRow {
                remove: false,
                key: "HKU\\X".to_owned(),
                name: "Mute".to_owned(),
                kind: winweaver_core::catalog::RegKind::DWord,
                value: "1".to_owned(),
                keep: None,
            });
        }
        d.apply();
        assert!(d.error.is_some(), "a key of another root is refused");
        let rule = d.file.rules.iter().find(|r| r.id == id).unwrap();
        assert!(rule.actions.is_empty(), "and changes nothing");
        if let Some(form) = d.form.as_mut() {
            form.actions[0].key = "HKEY_LOCAL_MACHINE\\Software\\Policies\\Zoom".to_owned();
        }
        d.apply();
        assert!(d.error.is_none(), "{:?}", d.error);
        app.apply(vec![Act::Save]);
        assert!(app.catalog.rule(&id).is_some(), "{}", app.status);
        app.apply(vec![Act::AskCatalogEditRemove(id.clone())]);
        assert!(app.ask.is_some(), "a removal asks");
        app.apply(vec![Act::AskYes, Act::Save]);
        assert!(app.catalog.rule(&id).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The registry tab writes a value of a type through its editor, asks before it deletes and
    /// bookmarks a value; here on a registry in memory.
    #[test]
    fn the_registry_tab_edits_asks_and_bookmarks() {
        use winweaver_core::registry::{Data, MemoryRegistry, Registry};
        let dir = std::env::temp_dir().join(format!("winweaver-reg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.marks_path = dir.join("marks.wwmarks");
        let reg = std::sync::Arc::new(MemoryRegistry::default());
        let key = "HKEY_CURRENT_USER\\Software\\WinWeaverTest";
        reg.create_key(key).unwrap();
        app.registry = reg.clone();
        app.apply(vec![Act::OpenRegistry(
            "HKCU:\\Software\\WinWeaverTest".to_owned(),
            None,
        )]);
        let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
            panic!("the registry tab");
        };
        assert_eq!(d.path, key);
        app.apply(vec![Act::RegistryNew(
            regview::EditKind::NewValue,
            "REG_DWORD",
        )]);
        if let Some(Doc::Registry(d)) = app.docs.get_mut(app.active)
            && let Some(edit) = &mut d.edit
        {
            edit.name = "N".to_owned();
            edit.text = "0x10".to_owned();
        }
        app.apply(vec![Act::RegistrySave]);
        assert_eq!(reg.read(key).unwrap().values[0].data, Data::Dword(16));
        app.apply(vec![Act::AskRegistryDelete]);
        assert!(app.ask.is_some(), "a deletion asks");
        app.apply(vec![Act::AskNo, Act::BookmarkRegistry]);
        assert_eq!(reg.read(key).unwrap().values.len(), 1);
        assert!(
            app.marks
                .items
                .iter()
                .any(|b| b.is_registry() && b.path == key && b.item == "N")
        );
        app.apply(vec![Act::AskRegistryDelete, Act::AskYes]);
        assert!(reg.read(key).unwrap().values.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A link opens the catalog tab of an import at the policy, and the same policy in the tree
    /// of another import is selected there, not in the first tree.
    #[test]
    fn links_open_the_catalog_at_a_policy_and_select_it_in_another_tree() {
        let dir = std::env::temp_dir().join(format!("winweaver-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        sample_import(&dir);
        let again = admx::to_catalog_file(
            &admx::read_folder(&dir.join("admx")),
            "again",
            "Again",
            "",
            "now",
        );
        again
            .save(&dir.join("imports").join("again.wwcatalog"))
            .unwrap();
        let catalog = Catalog::embedded().unwrap();
        let settings = Settings {
            loaded_imports: Some(vec!["test".to_owned(), "again".to_owned()]),
            ..Settings::default()
        };
        let mut app = App::new(catalog, Vec::new(), settings);
        app.imports_dir = dir.join("imports");
        app.load_imports();
        let id = "admx.sample.turnoff";
        let alias: Vec<String> = app.catalog.alias_groups(id).map(str::to_owned).collect();
        assert_eq!(alias.len(), 1, "the second import shows the policy too");
        app.apply(vec![Act::ShowIn(id.to_owned(), alias[0].clone())]);
        let Doc::Profile(doc) = &app.docs[0] else {
            panic!("a profile");
        };
        assert_eq!(doc.sel, Sel::Rule(id.to_owned()));
        assert_eq!(doc.focus.as_ref(), Some(&alias[0]));
        assert!(doc.expanded.contains(&alias[0]), "its tree opens to it");
        app.apply(vec![Act::OpenCatalogAt("again".to_owned(), id.to_owned())]);
        let Some(Doc::Catalog(c)) = app.docs.get(app.active) else {
            panic!("the catalog tab");
        };
        assert_eq!((c.id.as_str(), c.sel.as_deref()), ("again", Some(id)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Removing an import, discarding changes, forgetting the recent files and renewing the
    /// token ask first; two catalogs compare in a page.
    #[test]
    fn removals_ask_first_and_two_catalogs_compare() {
        let dir = std::env::temp_dir().join(format!("winweaver-ask-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        sample_import(&dir);
        let mut copy = admx::to_catalog_file(
            &admx::read_folder(&dir.join("admx")),
            "copy",
            "Copy",
            "",
            "now",
        );
        copy.rules.clear();
        copy.save(&dir.join("imports").join("copy.wwcatalog"))
            .unwrap();
        let catalog = Catalog::embedded().unwrap();
        let settings = Settings {
            loaded_imports: Some(vec!["test".to_owned()]),
            recent: vec![PathBuf::from("a.wwprofile")],
            ..Settings::default()
        };
        let mut app = App::new(catalog, Vec::new(), settings);
        app.imports_dir = dir.join("imports");
        app.load_imports();
        app.apply(vec![Act::AskRemoveImport("test".to_owned())]);
        assert!(app.ask.is_some());
        app.apply(vec![Act::AskNo]);
        assert!(dir.join("imports").join("test.wwcatalog").exists());
        assert_eq!(app.imports.len(), 1);
        app.apply(vec![Act::Toggle("install.bypass-tpm".to_owned(), false)]);
        app.apply(vec![Act::AskCloseDiscard(0)]);
        assert!(app.ask.is_some(), "a tab with changes asks");
        app.apply(vec![Act::AskNo]);
        assert_eq!(app.docs.len(), 1);
        app.apply(vec![Act::AskForgetRecent, Act::AskYes]);
        assert!(app.settings.recent.is_empty());
        app.apply(vec![Act::AskRenewToken]);
        assert!(app.ask.is_some());
        app.apply(vec![Act::AskNo]);
        app.apply(vec![
            Act::ToggleImport("copy".to_owned()),
            Act::OpenCatalog("test".to_owned()),
            Act::CompareCatalog(CatalogRef::Import("copy".to_owned())),
        ]);
        let Some(Doc::Markdown(page)) = app.docs.get(app.active) else {
            panic!("a page of the comparison: {}", app.status);
        };
        assert!(
            page.text.contains("Only in Test templates"),
            "{}",
            page.text
        );
        assert!(page.text.contains("`admx.sample.turnoff`"), "{}", page.text);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Waits for a read of this PC as the frames do.
    fn wait_pc_read(app: &mut App) {
        let ctx = egui::Context::default();
        let started = std::time::Instant::now();
        while app.pc_read.is_some() {
            assert!(started.elapsed() < std::time::Duration::from_secs(60));
            std::thread::sleep(std::time::Duration::from_millis(10));
            app.poll_pc_read(&ctx);
        }
    }

    /// A read of this PC runs in a thread of its own, one at a time, and its result lands in the
    /// tab it started from even when another tab came to the front meanwhile.
    #[test]
    fn this_pc_is_read_in_the_background() {
        let catalog = Catalog::embedded().unwrap();
        let profile = Profile::from_defaults("Office", &catalog);
        let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
        app.docs = vec![
            Doc::Profile(Box::new(ProfileDoc::new(profile.clone(), None, &catalog))),
            Doc::Profile(Box::new(ProfileDoc::new(profile, None, &catalog))),
        ];
        app.active = 0;
        app.pc = Some(std::sync::Arc::new(thispc::MemoryMachine::default()));
        app.apply(vec![Act::CheckOnPc]);
        assert!(app.pc_read.is_some());
        app.apply(vec![Act::ReadPc]);
        assert_eq!(
            app.status,
            tr("This PC is being read; wait until the read ends")
        );
        app.active = 1;
        wait_pc_read(&mut app);
        let issues = |app: &App, i: usize| match app.docs.get(i) {
            Some(Doc::Profile(d)) => d.issues.clone(),
            _ => panic!("profile tab {i}"),
        };
        // The machine in memory is no administrator: the features and capabilities are not read.
        let first = issues(&app, 0);
        assert!(
            first.iter().any(|i| i.text == admin_issue().text),
            "{first:?}"
        );
        assert!(issues(&app, 1).is_empty());

        app.apply(vec![Act::ReadPc]);
        wait_pc_read(&mut app);
        assert_eq!(app.docs.len(), 3);
        assert_eq!(app.active, 2);
        let Some(Doc::Profile(read)) = app.docs.get(2) else {
            panic!("the profile read from this PC");
        };
        assert_eq!(read.profile.name, "MEMORY");

        // A tab closed during the read is missed without harm.
        app.active = 0;
        app.apply(vec![Act::CheckOnPc]);
        app.docs.remove(0);
        app.active = 0;
        wait_pc_read(&mut app);
        assert_eq!(
            app.status,
            tr("Check on this PC: the tab of the profile was closed")
        );
        assert_eq!(app.docs.len(), 2);
    }

    /// The search of the registry tab runs in a thread of its own under the selected key and
    /// lists what it finds; a click on a place shows it.
    #[test]
    fn the_registry_tab_searches_under_its_key() {
        use winweaver_core::registry::{Data, MemoryRegistry, Registry};
        fn search(app: &mut App) -> &mut regview::Search {
            match app.docs.get_mut(app.active) {
                Some(Doc::Registry(d)) => &mut d.search,
                _ => panic!("the registry tab"),
            }
        }
        let catalog = Catalog::embedded().unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        let reg = std::sync::Arc::new(MemoryRegistry::default());
        let key = "HKEY_CURRENT_USER\\Software\\WinWeaverSearch";
        let inner = format!("{key}\\Inner");
        let blue = Data::Sz("dark blue".to_owned());
        reg.create_key(&inner).unwrap();
        reg.set(&inner, "Colour", &blue).unwrap();
        app.registry = reg;
        app.apply(vec![Act::OpenRegistry(key.to_owned(), None)]);
        app.apply(vec![Act::RegistrySearch]);
        assert!(search(&mut app).open);
        // An empty text starts nothing.
        app.apply(vec![Act::RegistryFind]);
        assert!(search(&mut app).running.is_none());
        search(&mut app).text = "BLUE".to_owned();
        app.apply(vec![Act::RegistryFind]);
        assert!(search(&mut app).running.is_some());
        let ctx = egui::Context::default();
        let started = std::time::Instant::now();
        while search(&mut app).running.is_some() {
            assert!(started.elapsed() < std::time::Duration::from_secs(30));
            std::thread::sleep(std::time::Duration::from_millis(10));
            app.poll_registry_search(&ctx);
        }
        let found = search(&mut app).found.clone().expect("a result");
        assert!(found.complete);
        assert_eq!(found.hits.len(), 1);
        assert_eq!(found.hits[0].path, inner);
        assert_eq!(found.hits[0].value.as_deref(), Some("Colour"));
        let hit = found.hits[0].clone();
        app.apply(vec![Act::RegistryGo(hit.path, hit.value)]);
        let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
            panic!("the registry tab");
        };
        assert_eq!(d.value.as_deref(), Some("Colour"));
    }
}
