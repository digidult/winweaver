//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`), built on eframe.
//!
//! This is the skeleton of variant A (the command bar): the tab row with the menu button, the
//! command bar, the navigation row (back, forward, breadcrumb, bookmark, search), the profile
//! tree with the detail panel, the Settings tab and the status bar. Every change of the profile
//! is collected as an [`Act`] while the frame is drawn and applied afterwards.

mod catalogview;
mod diffview;
mod forms;
mod i18n;
mod marksview;
mod mcpview;
mod mdview;
mod session;
mod themes;
mod settings;
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
use winweaver_core::render;
use winweaver_core::resolver::{self, Resolution};
use winweaver_core::scripts;
use winweaver_core::thispc::{self, Report, Verdict};
use winweaver_core::validate::{self, Issue, Severity, Target};

pub use diffview::DiffDoc;
pub use forms::Form;
pub use i18n::{tr, trf};
pub use mdview::MdDoc;
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

/// The selected item of a profile document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sel {
    None,
    Group(String),
    Rule(String),
    Form(Form),
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
}

impl ProfileDoc {
    pub fn new(profile: Profile, path: Option<PathBuf>, catalog: &Catalog) -> Self {
        let resolution = resolver::resolve(catalog, &profile);
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
            issues: Vec::new(),
            draft_key,
            changed: None,
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
        }
    }

    pub fn dirty(&self) -> bool {
        matches!(self, Doc::Profile(d) if d.dirty) || matches!(self, Doc::Diff(d) if d.dirty)
    }
}

/// The search of the navigation row.
#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub text: String,
    pub history: Vec<String>,
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

    /// Puts the current text first in the history; `true` when the history changed.
    fn remember(&mut self) -> bool {
        let text = self.text.trim().to_owned();
        if text.is_empty() || self.history.first() == Some(&text) {
            return false;
        }
        self.history.retain(|h| h != &text);
        self.history.insert(0, text);
        self.history.truncate(SEARCH_HISTORY);
        true
    }
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
}

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    Select(Sel),
    Toggle(String, bool),
    SetGroup(String, bool),
    SetParam(String, String, Value),
    Expand(String, bool),
    Activate(usize),
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
    /// Import the templates of this Windows (None), of a folder or a catalog file.
    ImportTemplates(ImportSource),
    ToggleImport(String),
    RemoveImport(String),
    ExportImport(String),
    /// Starts to rename an import: its id and its name now.
    StartRename(String, String),
    /// Opens the tab of an import.
    OpenCatalog(String),
    CatalogSelect(String),
    CatalogExpand(String, bool),
    /// Shows a policy of an import (the import, the rule) in the tree of a profile.
    ShowInProfile(String, String),
    CancelRename,
    RenameImport(String, String),
    /// Prepare the native apply (false) or the return to the defaults (true) of the selection.
    PlanNative(bool),
    PlanUndo,
    ConfirmPlan,
    CancelPlan,
    OpenReport,
    OpenGuide,
    OpenMarks,
    OpenMcp,
    McpStart,
    McpStop,
    McpRenewToken,
    McpOpen(String),
    GoToMark(usize),
    SetMarkComment(usize, String),
    RemoveMark(usize),
    Compare,
    CompareWith(&'static str),
    ApplyDiff,
    SaveDiffAs,
    SetInstall(Install),
    SetAccounts(Vec<Account>),
    SetLanguages(Languages),
    Check,
    Build,
    ClearMessages,
    Exit,
    Message(String),
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
    docs: Vec<Doc>,
    active: usize,
    settings: Settings,
    settings_dirty: bool,
    search: Search,
    status: String,
    marks: Bookmarks,
    marks_path: PathBuf,
    theme_applied: bool,
    /// The colours given to the title bar last: dark, caption, text, border.
    title_colors: Option<(bool, [u8; 3], [u8; 3], [u8; 3])>,
    last_title: String,
    exit: bool,
    drafts_dir: PathBuf,
    logs_dir: PathBuf,
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
}

/// The translation for a language code, `None` for English.
pub fn texts_for<'a>(texts: &'a [CatalogTexts], code: &str) -> Option<&'a CatalogTexts> {
    texts.iter().find(|t| t.code == code)
}

impl App {
    pub fn new(catalog: Catalog, texts: Vec<CatalogTexts>, settings: Settings) -> Self {
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0)
            .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &catalog));
        let doc = ProfileDoc::new(profile, None, &catalog);
        let search = Search {
            history: settings.search_history.clone(),
            ..Search::default()
        };
        Self {
            base_catalog: catalog.clone(),
            base_texts: texts.clone(),
            imports: Vec::new(),
            importing: None,
            catalog,
            texts,
            docs: vec![Doc::Profile(Box::new(doc))],
            active: 0,
            settings,
            settings_dirty: false,
            search,
            status: "Ready".to_owned(),
            marks: Bookmarks::default(),
            marks_path: exe_dir().join(bookmarks::DEFAULT_FILE),
            theme_applied: false,
            title_colors: None,
            last_title: String::new(),
            exit: false,
            drafts_dir: exe_dir().join("drafts"),
            logs_dir: exe_dir().join("logs"),
            confirm: None,
            pending_acts: Vec::new(),
            settings_section: 0,
            renaming: None,
            running: None,
            running_skipped: Vec::new(),
            mcp: mcpview::McpPanel {
                port: 8765,
                token: String::new(),
                mode: winweaver_mcp::Mode::Read,
                read_pc: false,
                run: None,
                error: String::new(),
            },
            last_logged: String::new(),
        }
    }

    fn active_profile(&mut self) -> Option<&mut ProfileDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => Some(doc),
            _ => None,
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
    fn current_bookmark(&self) -> Option<Bookmark> {
        let (document, path, item, title) = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => {
                let item = match &doc.sel {
                    Sel::None => String::new(),
                    Sel::Group(id) | Sel::Rule(id) => id.clone(),
                    Sel::Form(f) => f.id().to_owned(),
                };
                let title = self
                    .crumbs()
                    .last()
                    .map(|c| c.0.clone())
                    .unwrap_or_else(|| doc.profile.name.clone());
                let path = doc
                    .path
                    .as_ref()
                    .map_or_else(|| doc.title(), |p| p.display().to_string());
                ("profile", path, item, title)
            }
            Some(Doc::Markdown(doc)) => {
                let path = doc
                    .path
                    .as_ref()
                    .map_or_else(|| doc.title.clone(), |p| p.display().to_string());
                ("markdown", path, String::new(), doc.title.clone())
            }
            Some(Doc::Diff(doc)) => {
                let path = doc
                    .path
                    .as_ref()
                    .map_or_else(|| doc.title(), |p| p.display().to_string());
                ("diff", path, String::new(), doc.title())
            }
            _ => return None,
        };
        Some(Bookmark {
            document: document.to_owned(),
            path,
            item,
            title,
            ..Bookmark::default()
        })
    }

    fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let mut acts = Vec::new();
        let active = self.active;
        let count = self.docs.len();
        let confirming = self.confirm.is_some();
        ctx.input_mut(|i| {
            let sc = |m, k| KeyboardShortcut::new(m, k);
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::F)) {
                self.search.open = true;
                self.search.focus = true;
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
            if confirming && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::CancelPlan);
            }
            for file in &i.raw.dropped_files {
                acts.push(Act::OpenPath(file.path().to_path_buf()));
            }
        });
        acts
    }

    fn tabs_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        ui.horizontal(|ui| {
            ui.menu_button(tr("Menu"), |ui| {
                ui.menu_button(tr("New"), |ui| {
                    if ui.button(tr("Empty profile")).clicked() {
                        acts.push(Act::NewProfile(None));
                        ui.close();
                    }
                    ui.separator();
                    for (name, _) in PRESETS {
                        if ui
                            .button(trf("Profile from the preset {name}", &[("name", &name)]))
                            .clicked()
                        {
                            acts.push(Act::NewProfile(Some(name)));
                            ui.close();
                        }
                    }
                });
                if ui
                    .add(egui::Button::new(tr("Open...")).shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    acts.push(Act::Open);
                    ui.close();
                }
                ui.add_enabled_ui(!self.settings.recent.is_empty(), |ui| {
                    ui.menu_button(tr("Open recent"), |ui| {
                        for path in &self.settings.recent {
                            if ui.button(path.display().to_string()).clicked() {
                                acts.push(Act::OpenPath(path.clone()));
                                ui.close();
                            }
                        }
                    });
                });
                if ui
                    .add(egui::Button::new(tr("Save")).shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    acts.push(Act::Save);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr("Save as...")).shortcut_text("Ctrl+Shift+S"))
                    .clicked()
                {
                    acts.push(Act::SaveAs);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr("Close tab")).shortcut_text("Ctrl+W"))
                    .clicked()
                {
                    acts.push(Act::CloseTab(self.active, false));
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Compare with a file...")).clicked() {
                    acts.push(Act::Compare);
                    ui.close();
                }
                ui.menu_button(tr("Compare with a preset"), |ui| {
                    for (name, _) in PRESETS {
                        if ui.button(name).clicked() {
                            acts.push(Act::CompareWith(name));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if ui.button(tr("Close tab, discard changes")).clicked() {
                    acts.push(Act::CloseTab(self.active, true));
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(tr("Guide")).shortcut_text("F1"))
                    .clicked()
                {
                    acts.push(Act::OpenGuide);
                    ui.close();
                }
                if ui.button(tr("MCP server")).clicked() {
                    acts.push(Act::OpenMcp);
                    ui.close();
                }
                if ui.button(tr("Bookmarks")).clicked() {
                    acts.push(Act::OpenMarks);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr("Settings")).shortcut_text("Ctrl+,"))
                    .clicked()
                {
                    acts.push(Act::OpenSettings);
                    ui.close();
                }
                if ui.button(tr("About")).clicked() {
                    acts.push(Act::Message(format!(
                        "WinWeaver {}, catalog {}, settings {}",
                        winweaver_core::APP_VERSION,
                        self.catalog.version(),
                        Settings::path().display()
                    )));
                    ui.close();
                }
                if ui.button(tr("Exit")).clicked() {
                    acts.push(Act::Exit);
                    ui.close();
                }
            });
            ui.separator();
            for (i, doc) in self.docs.iter().enumerate() {
                let mark = if doc.dirty() { "* " } else { "" };
                let label = format!("{mark}{}", doc.title());
                let response = ui.selectable_label(i == self.active, label);
                if response.clicked() {
                    acts.push(Act::Activate(i));
                }
                if response.middle_clicked() {
                    acts.push(Act::CloseTab(i, false));
                }
                if let Doc::Profile(d) = doc
                    && let Some(p) = &d.path
                {
                    response.on_hover_text(p.display().to_string());
                }
                if ui
                    .small_button(tr("x"))
                    .on_hover_text(tr("Close (Ctrl+W)"))
                    .clicked()
                {
                    acts.push(Act::CloseTab(i, false));
                }
                ui.add_space(4.0);
            }
            if ui
                .button(tr("+"))
                .on_hover_text(tr("New profile (Ctrl+N)"))
                .clicked()
            {
                acts.push(Act::NewProfile(None));
            }
        });
    }

    /// The commands of the active document: icons with labels, icons only when `compact`.
    fn toolbar_items(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let label = |icon: &str, text: &'static str| {
            let text = tr(text);
            if compact {
                icon.to_owned()
            } else {
                format!("{icon} {text}")
            }
        };
        let button = |ui: &mut egui::Ui, icon: &str, text: &'static str, keys: &str| {
            let hint = if keys.is_empty() {
                tr(text).to_owned()
            } else {
                format!("{} ({keys})", tr(text))
            };
            ui.button(label(icon, text)).on_hover_text(hint).clicked()
        };
        if button(ui, "\u{1F4C4}", "New", "Ctrl+N") {
            acts.push(Act::NewProfile(None));
        }
        if button(ui, "\u{1F4C2}", "Open", "Ctrl+O") {
            acts.push(Act::Open);
        }
        if button(ui, "\u{1F4BE}", "Save", "Ctrl+S") {
            acts.push(Act::Save);
        }
        ui.separator();
        if button(ui, "\u{2714}", "Check", "F7") {
            acts.push(Act::Check);
        }
        if button(ui, "\u{1F528}", "Build", "F9") {
            acts.push(Act::Build);
        }
        ui.separator();
        ui.menu_button(label("\u{1F5A5}", "This PC"), |ui| {
            if ui.button(tr("Read this PC into a new profile")).clicked() {
                acts.push(Act::ReadPc);
                ui.close();
            }
            if ui
                .button(tr("Read the selection into this profile"))
                .clicked()
            {
                acts.push(Act::ReadSelection);
                ui.close();
            }
            if ui.button(tr("Check the selection on this PC")).clicked() {
                acts.push(Act::CheckOnPc);
                ui.close();
            }
            ui.separator();
            if ui
                .button(tr("Save an apply script for the selection..."))
                .clicked()
            {
                acts.push(Act::SaveScript(ScriptKind::Apply));
                ui.close();
            }
            if ui
                .button(tr("Save a return-to-defaults script for the selection..."))
                .clicked()
            {
                acts.push(Act::SaveScript(ScriptKind::Revert));
                ui.close();
            }
            if ui
                .button(tr("Save an audit script for the selection..."))
                .clicked()
            {
                acts.push(Act::SaveScript(ScriptKind::Audit));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Apply the selection to this PC...")).clicked() {
                acts.push(Act::PlanNative(false));
                ui.close();
            }
            if ui
                .button(tr("Return the selection to Windows defaults..."))
                .clicked()
            {
                acts.push(Act::PlanNative(true));
                ui.close();
            }
            if ui.button(tr("Undo a change from its backup...")).clicked() {
                acts.push(Act::PlanUndo);
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Open an audit report...")).clicked() {
                acts.push(Act::OpenReport);
                ui.close();
            }
        });
        if button(ui, "\u{1F500}", "Compare", "") {
            acts.push(Act::Compare);
        }
    }

    fn toolbar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        ui.horizontal(|ui| self.toolbar_items(ui, acts, false));
    }

    fn crumbs(&self) -> Vec<(String, Sel)> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let mut crumbs = vec![(doc.profile.name.clone(), Sel::None)];
        let group = match &doc.sel {
            Sel::None | Sel::Form(_) => None,
            Sel::Group(id) => Some(id.clone()),
            Sel::Rule(id) => self.catalog.rule(id).map(|r| r.group.clone()),
        };
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

    fn nav_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let (can_back, can_forward) = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => (doc.can_go(-1), doc.can_go(1)),
            _ => (false, false),
        };
        let crumbs = self.crumbs();
        let marked = self
            .current_bookmark()
            .is_some_and(|b| self.marks.find(&b).is_some());
        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_back, egui::Button::new(tr("<")).frame(false))
                .on_hover_text(tr("Back (Alt+Left)"))
                .clicked()
            {
                acts.push(Act::Back);
            }
            if ui
                .add_enabled(can_forward, egui::Button::new(tr(">")).frame(false))
                .on_hover_text(tr("Forward (Alt+Right)"))
                .clicked()
            {
                acts.push(Act::Forward);
            }
            ui.separator();
            for (i, (label, sel)) in crumbs.iter().enumerate() {
                if i > 0 {
                    ui.weak(tr(">"));
                }
                let text = if i + 1 == crumbs.len() {
                    egui::RichText::new(label.as_str()).strong()
                } else {
                    egui::RichText::new(label.as_str())
                };
                if ui.add(egui::Button::new(text).frame(false)).clicked() {
                    acts.push(Act::Select(sel.clone()));
                }
            }
            if !crumbs.is_empty() {
                let mut m = marked;
                if ui
                    .toggle_value(&mut m, tr("Bookmark"))
                    .on_hover_text(tr("Ctrl+B"))
                    .changed()
                {
                    acts.push(Act::ToggleBookmark);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if compact {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.separator();
                        self.toolbar_items(ui, acts, true);
                    });
                }
                if self.search.open {
                    if ui
                        .button(tr("x"))
                        .on_hover_text(tr("Clear and close (Esc)"))
                        .clicked()
                    {
                        self.search.text.clear();
                        self.search.open = false;
                    }
                    let id = egui::Id::new("search-field");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search.text)
                            .id(id)
                            .hint_text(tr("Search rules (Ctrl+F)"))
                            .desired_width((ui.available_width() - 60.0).clamp(220.0, 640.0)),
                    );
                    if self.search.focus {
                        ui.memory_mut(|m| m.request_focus(id));
                        self.search.focus = false;
                    }
                    if response.lost_focus() {
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.search.text.clear();
                            self.search.open = false;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && self.search.remember()
                        {
                            self.settings.search_history = self.search.history.clone();
                            self.settings_dirty = true;
                        }
                    }
                    if !self.search.history.is_empty() {
                        ui.menu_button(tr("v"), |ui| {
                            for h in self.search.history.clone() {
                                if ui.button(&h).clicked() {
                                    self.search.text = h;
                                    ui.close();
                                }
                            }
                        });
                    }
                } else if ui
                    .button(tr("Search"))
                    .on_hover_text(tr("Ctrl+F"))
                    .clicked()
                {
                    self.search.open = true;
                    self.search.focus = true;
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let state = match self.docs.get(self.active) {
                Some(Doc::Profile(d)) if d.dirty => tr("Modified"),
                Some(Doc::Profile(d)) if d.path.is_some() => tr("Saved"),
                Some(Doc::Profile(_)) => tr("Not saved yet"),
                _ => "",
            };
            ui.weak(state);
            ui.weak(trf(
                "Catalog {version}, {n} rules",
                &[
                    ("version", &self.catalog.version()),
                    ("n", &self.catalog.rules().len()),
                ],
            ));
            if let Some(run) = &self.mcp.run {
                ui.weak(trf(
                    "MCP: {url}, mode {mode}",
                    &[
                        ("url", &run.http.url()),
                        ("mode", &run.server.options().mode.name()),
                    ],
                ));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(&self.status);
            });
        });
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        ui.heading(tr("Settings"));
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(170.0);
                let sections = [tr("General"), tr("Policy templates"), tr("Files")];
                for (i, title) in sections.into_iter().enumerate() {
                    if ui
                        .selectable_label(self.settings_section == i, title)
                        .clicked()
                    {
                        self.settings_section = i;
                    }
                }
            });
            ui.separator();
            ui.vertical(|ui| match self.settings_section {
                1 => {
                    let mut acts = Vec::new();
                    self.imports_view(ui, &mut acts);
                    self.pending_acts.extend(acts);
                }
                2 => self.files_settings(ui),
                _ => self.general_settings(ui),
            });
        });
        if before != self.settings {
            self.settings_dirty = true;
            if before.theme != self.settings.theme || before.custom_theme != self.settings.custom_theme {
                self.theme_applied = false;
            }
            if before.language != self.settings.language {
                self.apply_language();
            }
        }
    }

    /// The folders next to the executable and the recent files.
    fn files_settings(&mut self, ui: &mut egui::Ui) {
        ui.label(trf("Settings file: {path}", &[("path", &Settings::path().display())]));
        ui.label(trf("Profiles folder: {path}", &[("path", &profiles_dir().display())]));
        ui.label(trf("Output folder: {path}", &[("path", &output_dir().display())]));
        ui.label(trf("Policy templates folder: {path}", &[("path", &imports_dir().display())]));
        ui.add_space(8.0);
        if !self.settings.recent.is_empty() && ui.button(tr("Forget recent files")).clicked() {
            self.settings.recent.clear();
        }
    }

    /// Language, theme and the behaviour of the window.
    fn general_settings(&mut self, ui: &mut egui::Ui) {
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
        ui.horizontal(|ui| {
            ui.label(tr("Theme"));
            ui.radio_value(
                &mut self.settings.theme,
                ThemeChoice::System,
                tr("Follow Windows"),
            );
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Light, tr("Light"));
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Dark, tr("Dark"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Colours"));
            let list = themes::themes(&themes_dir());
            let current = list
                .iter()
                .find(|t| t.id == self.settings.custom_theme)
                .map_or_else(|| tr("Those of the theme").to_owned(), |t| t.name.clone());
            egui::ComboBox::from_id_salt("colours")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.custom_theme, String::new(), tr("Those of the theme"));
                    for t in &list {
                        ui.selectable_value(&mut self.settings.custom_theme, t.id.clone(), &t.name);
                    }
                });
        });
        ui.checkbox(
            &mut self.settings.desc_open,
            tr("Show the description panel (Ctrl+D)"),
        );
        ui.checkbox(
            &mut self.settings.show_guide_at_start,
            tr("Open the guide at the next start"),
        );
        ui.checkbox(
            &mut self.settings.inline_switches,
            tr("Switches at the end of the rows of the selected group"),
        );
        ui.checkbox(
            &mut self.settings.restore_tabs,
            tr("Open the tabs of the last session at the start"),
        );
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
                    format!("Opened {}", path.display())
                } else {
                    format!(
                        "Opened {} ({} choices of rules this catalog does not have are kept aside)",
                        path.display(),
                        unknown.len()
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
        if validate::errors(&doc.issues) > 0 {
            self.status = format!(
                "{}: fix the errors before the build",
                validate::summary(&doc.issues)
            );
            return;
        }
        let build = match render::build(catalog, &doc.profile, winweaver_core::APP_VERSION) {
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
            self.status = format!("Build: cannot create {}: {e}", dir.display());
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
        self.status = format!(
            "Built {} ({} rules, {} scripts, {} warnings)",
            xml_path.display(),
            build.rule_ids.len(),
            build.scripts.len(),
            build.warnings.len()
        );
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
        }
    }

    /// Reads this PC: every rule into a new profile tab, or the selection into the open profile.
    fn read_pc(&mut self, only_selection: bool) {
        let Some(machine) = winweaver_win::this_pc() else {
            self.status = tr("This PC can be read on Windows only").to_owned();
            return;
        };
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
        let report = thispc::run(
            machine.as_ref(),
            &plan,
            !only_selection,
            &clock::now_utc_iso(),
        );
        let readings = thispc::judge(&self.catalog, &plan, &report.results);
        let (mut profile, summary) =
            thispc::capture(&self.catalog, &base, &readings, report.system.as_ref());
        let mut issues: Vec<Issue> = summary
            .notes
            .iter()
            .map(|n| Issue {
                severity: Severity::Info,
                target: Target::Profile,
                text: trf("This PC: {n}", &[("n", &n)]),
            })
            .collect();
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
        if only_selection {
            if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
                doc.profile = profile;
                doc.refresh(catalog);
                doc.issues = issues;
            }
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

    /// Compares the selected rules of the profile with this PC and lists the result.
    fn check_on_pc(&mut self) {
        let Some(machine) = winweaver_win::this_pc() else {
            self.status = tr("This PC can be read on Windows only").to_owned();
            return;
        };
        let ids = self.selection_ids();
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        let plan = thispc::plan(catalog, &doc.profile, &ids);
        let report = thispc::run(machine.as_ref(), &plan, false, &clock::now_utc_iso());
        let readings = thispc::judge(catalog, &plan, &report.results);
        let mut issues = Vec::new();
        let mut differ = 0;
        for r in &readings {
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
        self.status = format!(
            "Checked {} rules on this PC: {differ} differ from the profile",
            readings.len()
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
        let version = winweaver_core::APP_VERSION;
        let stem = file_stem(&doc.profile.name);
        let made = match kind {
            ScriptKind::Apply => scripts::apply_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| (t, p.summary(), format!("Apply-{stem}.ps1"), true)),
            ScriptKind::Revert => {
                scripts::revert_script(&self.catalog, &doc.profile, &ids, version)
                    .map(|(t, p)| (t, p.summary(), format!("Revert-{stem}.ps1"), true))
            }
            ScriptKind::Audit => scripts::audit_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| {
                    (
                        t,
                        format!("{} rules checked", p.rules.len()),
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
        self.status = format!(
            "Saved {} ({summary}); run it as administrator, first on a test PC",
            path.display()
        );
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
        let dialog = if start.is_dir() { dialog.set_directory(start) } else { dialog };
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
        let stamp = clock::now_utc_iso().replace([':', '-'], "").replace('T', "-").trim_end_matches('Z').to_owned();
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
            let args = vec!["--elevated-apply".to_owned(), path.display().to_string(), digest];
            let result = winweaver_win::run_elevated(&exe, &args).and_then(|code| {
                let report_path = path.with_file_name(format!("plan-{stamp}-report.json"));
                let read = std::fs::read_to_string(&report_path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| serde_json::from_str::<winweaver_core::apply::Report>(&t).map_err(|e| e.to_string()));
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
                                severity: if r.status == "failed" { Severity::Warning } else { Severity::Info },
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
                                ("backup", &if report.backup.is_empty() { "-".to_owned() } else { report.backup.clone() }),
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
                        "{n} steps are left to the stand-alone script (scripts, app removals, values without a known Windows default).",
                        &[("n", &plan.skipped.len())],
                    ));
                }
                if plan.uses_default_user() {
                    ui.label(tr("The default user profile changes too: accounts created from now on get its settings."));
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

    /// Reads the catalog files of the imports next to the executable; the templates of this
    /// Windows first, so that they win a policy two imports hold.
    fn load_imports(&mut self) {
        let dir = imports_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        let mut files: Vec<std::path::PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == admx::CATALOG_EXTENSION))
            .collect();
        files.sort_by_key(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            (!name.starts_with("system"), name)
        });
        let mut broken = 0;
        for path in files {
            match CatalogFile::load(&path) {
                Ok(f) => self.imports.push(f),
                Err(_) => broken += 1,
            }
        }
        if broken > 0 {
            self.status = trf("{n} imports of policy templates could not be read", &[("n", &broken)]);
        }
        self.rebuild_catalog();
    }

    /// Merges the visible imports into the catalog and brings the choices of the open profiles
    /// in line with it.
    fn rebuild_catalog(&mut self) {
        let visible: Vec<CatalogFile> = self
            .imports
            .iter()
            .filter(|f| !self.settings.hidden_imports.contains(&f.id))
            .cloned()
            .collect();
        match admx::merge(&self.base_catalog, &visible) {
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
        admx::merge_texts(&mut texts, &visible);
        self.texts = texts;
        let catalog = &self.catalog;
        for doc in &mut self.docs {
            if let Doc::Profile(d) = doc {
                d.profile.take_known(catalog);
                d.profile.split_unknown(catalog);
                d.resolution = resolver::resolve(catalog, &d.profile);
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
            ImportSource::Folder => match rfd::FileDialog::new().set_title(tr("Choose the folder of the ADMX files")).pick_folder() {
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
        };
        let build = winweaver_win::this_pc().map(|m| m.system_info().build).unwrap_or_default();
        let taken: Vec<String> = self.imports.iter().map(|f| f.id.clone()).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let now = clock::now_utc_iso();
            let result = match (source, path) {
                (ImportSource::ThisWindows, _) => {
                    let t = admx::read_folder(std::path::Path::new("C:\\Windows\\PolicyDefinitions"));
                    let name = if build.is_empty() { "Policy templates of this Windows".to_owned() } else { format!("Policy templates of this Windows ({build})") };
                    Ok(admx::to_catalog_file(&t, "system", &name, &build, &now))
                }
                (ImportSource::Folder, Some(dir)) => {
                    let t = admx::read_folder(&dir);
                    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Templates".to_owned());
                    let id = format!("folder-{:08x}", session::fnv(&dir.display().to_string().to_lowercase()) as u32);
                    Ok(admx::to_catalog_file(&t, &id, &name, "", &now))
                }
                (ImportSource::CatalogFile, Some(file)) => CatalogFile::load(&file).map(|mut f| {
                    if taken.contains(&f.id) && !f.id.starts_with("package-") {
                        f.id = format!("package-{}", f.id);
                    }
                    f
                }),
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
                        let dir = imports_dir();
                        let path = dir.join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
                        let saved = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|()| file.save(&path));
                        let rules = file.rules.len();
                        let skipped = file.skipped.len();
                        self.imports.retain(|f| f.id != file.id);
                        self.imports.push(file);
                        self.rebuild_catalog();
                        self.status = match saved {
                            Ok(()) => trf("Imported: {rules} rules, {skipped} policies skipped", &[("rules", &rules), ("skipped", &skipped)]),
                            Err(e) => trf("Imported, but not saved: {e}", &[("e", &e)]),
                        };
                    }
                    Err(e) => self.status = trf("The import failed: {e}", &[("e", &e)]),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
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

    /// Shows a policy of an import in the tree of a profile: a hidden import is shown again, the
    /// first profile tab comes to the front, the groups above the policy open and it is selected.
    fn show_in_profile(&mut self, import: &str, rule: &str) {
        if let Some(i) = self.settings.hidden_imports.iter().position(|h| h == import) {
            self.settings.hidden_imports.remove(i);
            self.settings_dirty = true;
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
        for group in file.groups.iter_mut().filter(|g| g.parent.as_deref() == root) {
            group.title = name.to_owned();
        }
        let path = imports_dir().join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
        let saved = file.save(&path);
        for doc in &mut self.docs {
            if let Doc::Catalog(c) = doc
                && c.id == id
            {
                c.name = name.to_owned();
            }
        }
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
        self.status = match file.save(&path) {
            Ok(()) => trf("Saved {path}", &[("path", &path.display())]),
            Err(e) => trf("Save failed: {e}", &[("e", &e)]),
        };
    }

    /// The section of the settings that lists the imports of policy templates.
    fn imports_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        ui.strong(tr("Policy templates"));
        ui.weak(tr("Imported ADMX templates become rules under Standard policies. When two imports hold a policy, the templates of this Windows win."));
        for f in &self.imports {
            ui.horizontal(|ui| {
                let mut shown = !self.settings.hidden_imports.contains(&f.id);
                if ui.checkbox(&mut shown, &f.name).changed() {
                    acts.push(Act::ToggleImport(f.id.clone()));
                }
                ui.weak(trf("{rules} rules, {skipped} skipped", &[("rules", &f.rules.len()), ("skipped", &f.skipped.len())]));
                if ui.small_button(tr("Export...")).clicked() {
                    acts.push(Act::ExportImport(f.id.clone()));
                }
                if ui.small_button(tr("Open")).clicked() {
                    acts.push(Act::OpenCatalog(f.id.clone()));
                }
                if ui.small_button(tr("Rename")).clicked() {
                    acts.push(Act::StartRename(f.id.clone(), f.name.clone()));
                }
                if ui.small_button(tr("Remove")).clicked() {
                    acts.push(Act::RemoveImport(f.id.clone()));
                }
            });
            if let Some((id, name)) = &mut self.renaming
                && id == &f.id
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
            if ui.add_enabled(!busy, egui::Button::new(tr("Import the templates of this Windows"))).clicked() {
                acts.push(Act::ImportTemplates(ImportSource::ThisWindows));
            }
            if ui.add_enabled(!busy, egui::Button::new(tr("Import a folder..."))).clicked() {
                acts.push(Act::ImportTemplates(ImportSource::Folder));
            }
            if ui.add_enabled(!busy, egui::Button::new(tr("Import a catalog file..."))).clicked() {
                acts.push(Act::ImportTemplates(ImportSource::CatalogFile));
            }
        });
    }

    fn messages_panel(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        if doc.issues.is_empty() {
            return;
        }
        let texts = texts_for(&self.texts, &i18n::current());
        egui::Panel::top("messages").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(validate::summary(&doc.issues));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(tr("x"))
                        .on_hover_text(tr("Hide the messages"))
                        .clicked()
                    {
                        acts.push(Act::ClearMessages);
                    }
                });
            });
            egui::ScrollArea::vertical()
                .id_salt("messages")
                .max_height(140.0)
                .show(ui, |ui| {
                    for issue in &doc.issues {
                        ui.horizontal(|ui| {
                            let tag = match issue.severity {
                                Severity::Error => tr("Error"),
                                Severity::Warning => tr("Warning"),
                                Severity::Info => tr("Info"),
                            };
                            ui.weak(tag);
                            let (label, sel) = match &issue.target {
                                Target::Rule(id) => (
                                    self.catalog.rule(id).map_or_else(
                                        || id.clone(),
                                        |r| views::rtitle(texts, r).to_owned(),
                                    ),
                                    Sel::Rule(id.clone()),
                                ),
                                Target::Install => {
                                    ("Installation".to_owned(), Sel::Form(Form::Install))
                                }
                                Target::Accounts => {
                                    ("Accounts".to_owned(), Sel::Form(Form::Accounts))
                                }
                                Target::Languages => {
                                    ("Languages".to_owned(), Sel::Form(Form::Languages))
                                }
                                Target::Profile => (doc.profile.name.clone(), Sel::None),
                            };
                            if ui.link(label).clicked() {
                                acts.push(Act::Select(sel));
                            }
                            ui.label(&issue.text);
                        });
                    }
                });
        });
    }

    fn apply(&mut self, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Select(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                    }
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
                    match self.docs.iter().position(|d| matches!(d, Doc::Settings)) {
                        Some(i) => self.active = i,
                        None => {
                            self.docs.push(Doc::Settings);
                            self.active = self.docs.len() - 1;
                        }
                    }
                }
                Act::Save => self.save_active(false),
                Act::SaveAs => self.save_active(true),
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
                        mark.created = clock::now_utc_iso();
                        let added = self.marks.toggle(mark);
                        if self.save_marks() {
                            self.status = format!(
                                "{} ({} bookmarks in {})",
                                if added {
                                    "Bookmark added"
                                } else {
                                    "Bookmark removed"
                                },
                                self.marks.items.len(),
                                self.marks_path.display()
                            );
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
                    Ok(t) => {
                        self.mcp.token = t.clone();
                        self.settings.mcp_token = t;
                        self.settings_dirty = true;
                    }
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
                        self.save_marks();
                    }
                }
                Act::Fold => {
                    self.settings.desc_open = !self.settings.desc_open;
                    self.settings_dirty = true;
                }
                Act::Check => self.check_active(),
                Act::ReadPc => self.read_pc(false),
                Act::ReadSelection => self.read_pc(true),
                Act::CheckOnPc => self.check_on_pc(),
                Act::SaveScript(kind) => self.save_script(kind),
                Act::OpenReport => self.open_report(),
                Act::ImportTemplates(source) => self.start_import(source),
                Act::ToggleImport(id) => {
                    if let Some(i) = self.settings.hidden_imports.iter().position(|h| h == &id) {
                        self.settings.hidden_imports.remove(i);
                    } else {
                        self.settings.hidden_imports.push(id);
                    }
                    self.settings_dirty = true;
                    self.rebuild_catalog();
                }
                Act::RemoveImport(id) => {
                    let _ = std::fs::remove_file(imports_dir().join(format!("{id}.{}", admx::CATALOG_EXTENSION)));
                    self.imports.retain(|f| f.id != id);
                    self.rebuild_catalog();
                }
                Act::ExportImport(id) => self.export_import(&id),
                Act::StartRename(id, name) => self.renaming = Some((id, name)),
                Act::OpenCatalog(id) => self.open_catalog(&id),
                Act::CatalogSelect(id) => {
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) {
                        c.sel = Some(id);
                    }
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
                Act::CancelRename => self.renaming = None,
                Act::RenameImport(id, name) => self.rename_import(&id, &name),
                Act::PlanNative(revert) => self.plan_native(revert),
                Act::PlanUndo => self.plan_undo(),
                Act::ConfirmPlan => self.run_plan(),
                Act::CancelPlan => self.confirm = None,
                Act::Build => self.build_active(),
                Act::OpenGuide => self.open_guide(),
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
                self.status = format!("Opened {}: the star now writes there", path.display());
                self.remember_recent(&path);
                self.apply(vec![Act::OpenMarks]);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens the document of a bookmark (a file, or the tab of an unsaved document by its
    /// title) and selects its item.
    fn go_to_mark(&mut self, i: usize) {
        let Some(mark) = self.marks.items.get(i).cloned() else {
            return;
        };
        let path = PathBuf::from(&mark.path);
        if path.is_file() {
            self.open_path(path);
        } else {
            match self.docs.iter().position(|d| d.title() == mark.path) {
                Some(j) => self.active = j,
                None => {
                    self.status =
                        format!("The document of the bookmark is not open: {}", mark.path);
                    return;
                }
            }
        }
        if mark.document == "profile" && !mark.item.is_empty() {
            let sel = sel_from_item(&self.catalog, &mark.item);
            if let Some(doc) = self.active_profile() {
                doc.select(sel);
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
                self.status = format!("Opened {}", path.display());
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
                let unknown = profile.split_unknown(&self.catalog);
                let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                doc.dirty = true;
                doc.issues = notes
                    .into_iter()
                    .map(|n| Issue {
                        severity: Severity::Info,
                        target: Target::Profile,
                        text: trf("Open: {n}", &[("n", &n)]),
                    })
                    .collect();
                if !unknown.is_empty() {
                    doc.issues.push(Issue {
                        severity: Severity::Info,
                        target: Target::Profile,
                        text: format!(
                            "Open: {} choices of rules this catalog does not have are kept aside",
                            unknown.len()
                        ),
                    });
                }
                self.docs.push(Doc::Profile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = format!(
                    "Opened the profile of {}; save it as a .wwprofile file",
                    path.display()
                );
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    fn open_diff_path(&mut self, path: PathBuf) {
        match Diff::load(&path) {
            Ok(diff) => {
                self.docs.push(Doc::Diff(Box::new(DiffDoc {
                    diff,
                    path: Some(path.clone()),
                    dirty: false,
                })));
                self.active = self.docs.len() - 1;
                self.status = format!("Opened {}", path.display());
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
        self.status = format!("{} differences", diff.count());
        self.docs.push(Doc::Diff(Box::new(DiffDoc {
            diff,
            path: None,
            dirty: false,
        })));
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
            self.status = format!("Applied {} differences to {}", diff.count(), p.title());
        }
        self.active = i;
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
                self.status = format!("Saved {}", path.display());
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
                Doc::Bookmarks | Doc::Mcp | Doc::Settings | Doc::Catalog(_) => None,
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

    /// Starts the HTTP server of the window with the port, mode and token of the MCP tab.
    fn start_mcp(&mut self) {
        if self.mcp.token.is_empty() {
            match winweaver_mcp::new_token() {
                Ok(t) => {
                    self.mcp.token = t.clone();
                    self.settings.mcp_token = t;
                }
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
                read_pc: self.mcp.read_pc,
                profiles_dir: profiles_dir(),
                output_dir: output_dir(),
            },
        ));
        match winweaver_mcp::http::HttpServer::start(
            server.clone(),
            self.mcp.port,
            self.mcp.token.clone(),
        ) {
            Ok(http) => {
                self.status = trf("The MCP server runs on {url}", &[("url", &http.url())]);
                self.mcp.error.clear();
                self.mcp.run = Some(mcpview::McpRun { server, http });
                self.settings.mcp_port = self.mcp.port;
                self.settings_dirty = true;
            }
            Err(e) => {
                self.mcp.error = trf("The server did not start: {e}", &[("e", &e)]);
            }
        }
    }

    /// The theme of the settings: a colour theme over its light or dark base, or the plain theme.
    fn apply_theme(&self, ctx: &egui::Context) {
        let custom = if self.settings.custom_theme.is_empty() {
            None
        } else {
            themes::themes(&themes_dir())
                .into_iter()
                .find(|t| t.id == self.settings.custom_theme)
        };
        match custom {
            Some(t) => {
                let base = if t.dark { egui::Theme::Dark } else { egui::Theme::Light };
                ctx.set_visuals_of(base, themes::visuals(&t));
                ctx.set_theme(base);
            }
            None => {
                ctx.set_visuals_of(egui::Theme::Light, egui::Visuals::light());
                ctx.set_visuals_of(egui::Theme::Dark, egui::Visuals::dark());
                ctx.set_theme(self.settings.theme.preference());
            }
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
        if !self.theme_applied {
            self.apply_theme(ui.ctx());
            self.theme_applied = true;
        }
        self.color_title_bar(ui, frame);
        let title = self.window_title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        let mut acts = self.shortcuts(ui.ctx());
        egui::Panel::top("tabs").show(ui, |ui| self.tabs_row(ui, &mut acts));
        let compact = ui.available_width() < COMPACT_WIDTH;
        if !compact {
            egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui, &mut acts));
        }
        egui::Panel::top("nav").show(ui, |ui| self.nav_row(ui, &mut acts, compact));
        self.messages_panel(ui, &mut acts);
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        let kind = match self.docs.get(self.active) {
            Some(Doc::Profile(_)) => 0,
            Some(Doc::Settings) => 1,
            Some(Doc::Diff(_)) => 3,
            Some(Doc::Markdown(_)) => 4,
            Some(Doc::Bookmarks) => 5,
            Some(Doc::Mcp) => 6,
            Some(Doc::Catalog(_)) => 7,
            None => 2,
        };
        if kind == 0 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            let search = &self.search;
            let settings = &self.settings;
            if let Some(Doc::Profile(doc)) = self.docs.get(self.active) {
                let doc: &ProfileDoc = doc;
                egui::Panel::left("tree")
                    .resizable(true)
                    .default_size(380.0)
                    .show(ui, |ui| {
                        views::tree_view(
                            ui,
                            catalog,
                            texts,
                            doc,
                            search,
                            settings.inline_switches,
                            &mut acts,
                        );
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    views::detail_view(ui, catalog, texts, doc, settings, &mut acts);
                });
            }
        } else if kind == 3 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &i18n::current());
            if let Some(Doc::Diff(doc)) = self.docs.get(self.active) {
                let doc: &DiffDoc = doc;
                egui::CentralPanel::default().show(ui, |ui| {
                    diffview::diff_view(ui, catalog, texts, doc, &mut acts);
                });
            }
        } else if kind == 4 {
            if let Some(Doc::Markdown(doc)) = self.docs.get_mut(self.active) {
                egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc));
            }
        } else if kind == 6 {
            let panel = &mut self.mcp;
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        mcpview::mcp_view(ui, panel, &mut acts);
                    });
            });
        } else if kind == 5 {
            let marks = &self.marks;
            let path = &self.marks_path;
            egui::CentralPanel::default().show(ui, |ui| {
                marksview::marks_view(ui, marks, path, &mut acts);
            });
        } else if kind == 7 {
            let shown = match self.docs.get(self.active) {
                Some(Doc::Catalog(doc)) => self
                    .imports
                    .iter()
                    .find(|f| f.id == doc.id)
                    .map(|f| (f, doc)),
                _ => None,
            };
            match shown {
                Some((file, doc)) => catalogview::catalog_view(ui, file, doc, &mut acts),
                None => {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui.label(tr("This import is no longer there."));
                    });
                }
            }
        } else if kind == 1 {
            egui::CentralPanel::default().show(ui, |ui| self.settings_view(ui));
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(tr("No document is open. Use Menu, New or Ctrl+N."));
            });
        }
        self.confirm_view(ui.ctx(), &mut acts);
        acts.append(&mut self.pending_acts);
        self.apply(acts);
        self.poll_running(ui.ctx());
        self.poll_import(ui.ctx());
        let closing = self.exit || ui.ctx().input(|i| i.viewport().close_requested());
        self.write_drafts(closing);
        if closing {
            self.remember_tabs();
        }
        self.persist_settings();
        self.log_status();
    }
}

/// The selection a bookmark item names: a form id, a rule id or a group id.
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

/// The folder of the imports of policy templates next to the executable.
pub fn imports_dir() -> PathBuf {
    exe_dir().join("imports")
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
            &[profile::EXTENSION, diff::EXTENSION, bookmarks::EXTENSION],
        )
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

/// Opens the window; returns the error text when the catalog or the window cannot start.
pub fn run() -> Result<(), String> {
    let catalog = Catalog::embedded().map_err(|e| format!("catalog: {e}"))?;
    let texts = CatalogTexts::embedded().map_err(|e| format!("translations: {e}"))?;
    let (settings, warning) = match Settings::load_from(&Settings::path()) {
        Ok(s) => (s, None),
        Err(e) => (Settings::default(), Some(e)),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WinWeaver")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "WinWeaver",
        options,
        Box::new(move |_cc| {
            let mut app = App::new(catalog, texts, settings);
            app.mcp.port = app.settings.mcp_port;
            app.mcp.token = app.settings.mcp_token.clone();
            app.apply_language();
            if let Some(w) = warning {
                app.status = format!("Settings not read, the defaults are used: {w}");
            }
            match Bookmarks::load_or_default(&app.marks_path) {
                Ok(marks) => app.marks = marks,
                Err(e) => app.status = format!("Bookmarks not read: {e}"),
            }
            app.load_imports();
            app.restore_session();
            if app.settings.show_guide_at_start {
                app.open_guide();
                app.settings.show_guide_at_start = false;
                app.settings_dirty = true;
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())
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
        assert_eq!(app.marks.items[0].path, "Office.wwprofile");
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
}
