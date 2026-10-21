//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`), built on eframe.
//!
//! This is the skeleton of variant A (the command bar): the tab row with the menu button, the
//! command bar, the navigation row (back, forward, breadcrumb, bookmark, search), the profile
//! tree with the detail panel, the Settings tab and the status bar. Every change of the profile
//! is collected as an [`Act`] while the frame is drawn and applied afterwards.

mod act;
mod app;
mod cataledit;
mod catalogview;
mod chrome;
mod diffview;
mod doc;
mod forms;
mod i18n;
mod list;
mod marksview;
mod mcpview;
mod mdview;
mod question;
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
use winweaver_core::admx::{self, CatalogFile};
use winweaver_core::bookmarks::{self, Bookmark, Bookmarks};
use winweaver_core::catalog::{Catalog, TREE_ROOTS, check};
use winweaver_core::clock;
use winweaver_core::diff::{self, Diff};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::import;
use winweaver_core::jsonfile;
use winweaver_core::profile::{self, Profile};
use winweaver_core::regfile::{self, Line, RegFile};
use winweaver_core::render;
use winweaver_core::resolver::{self, Resolution};
use winweaver_core::scripts;
use winweaver_core::thispc::{self, Report, Verdict};
use winweaver_core::users::{self, UsersFile};
use winweaver_core::validate::{self, Issue, Severity, Target};

pub use act::{
    Act, CatalogAct, DiffAct, FileAct, ImportAct, MarksAct, McpAct, NavAct, PcAct, ProfileAct,
    RegFileAct, RegistryAct, SettingsAct, ThemeAct, UsersAct,
};
pub use diffview::DiffDoc;
pub use doc::{Doc, DocKind, SettingsDoc, SettingsSection, Tab};
pub use forms::Form;
pub use i18n::{tr, trf};
pub use mdview::MdDoc;
pub use question::Question;
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
    /// A question over the window and the act a yes runs.
    question: Option<Question>,
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
            question: None,
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
            running: None,
            running_skipped: Vec::new(),
            mcp: mcpview::McpPanel::new(winweaver_mcp::config::Config::default_path(&exe_dir())),
            last_logged: String::new(),
            shown_file: None,
            users_import: None,
        }
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
mod tests;
