//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`) on the shell of
//! WW (`ww`). The shell owns the tabs, the settings, the themes and the session and draws the
//! chrome; WinWeaver (`app/shell.rs`) answers with its documents, its command bars, its menus
//! and its acts. Every change is collected as an [`Act`] while the frame is drawn and applied
//! afterwards.

mod act;
mod app;
mod cataledit;
mod catalogview;
mod commands;
mod dialogs;
mod diffview;
mod doc;
mod folders;
mod forms;
mod importlist;
mod marksview;
mod mcpview;
mod pages;
mod question;
mod regedit;
mod regfileview;
mod regview;
mod settings;
mod usersview;
mod views;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
    Act, CatalogAct, DiffAct, FileAct, ImportAct, MarksAct, McpAct, PcAct, ProfileAct, RegFileAct,
    RegistryAct, SettingsAct, UsersAct,
};
pub use catalogview::TreeKey;
pub use dialogs::file_stem;
pub(crate) use dialogs::{
    ask_save_path, file_dialog, kept_name, open_dialog, pick_profile, reg_extension, with_extension,
    write_file,
};
pub use diffview::DiffDoc;
pub use doc::{Doc, DocKind, SettingsSection};
pub use folders::{imports_dir, lang_dir, output_dir, profiles_dir, themes_dir};
pub use forms::Form;
pub use importlist::{ImportEntry, import_files, loaded_import_ids, read_imports};
pub(crate) use importlist::{import_entries, import_order, size_text, unknown_issues};
pub use question::Question;
pub use settings::{DescSections, Extra, Settings, ThemeChoice};
pub use ww::doc::{Document as Tab, Search, matches_words, remember_search};
pub use ww::folders::exe_dir;
pub use ww::i18n::{tr, trf};
pub use ww::mdview::MdDoc;
pub use ww::renderer::Renderer;
pub use ww::{App, Shell, ShellAct, ShellDoc, Window};
pub use ww::{i18n, icons, list, mdview, renderer, session, style, themes, tree, widgets};

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


/// The translations of the interface embedded in the executable: the language code and the
/// file. Every text of the interface has a translation in each of them.
pub const LANGUAGES: [(&str, &str); 8] = [
    ("uk", include_str!("../../../data/lang/uk/ui.json")),
    ("ru", include_str!("../../../data/lang/ru/ui.json")),
    ("de", include_str!("../../../data/lang/de/ui.json")),
    ("es", include_str!("../../../data/lang/es/ui.json")),
    ("fr", include_str!("../../../data/lang/fr/ui.json")),
    ("id", include_str!("../../../data/lang/id/ui.json")),
    ("it", include_str!("../../../data/lang/it/ui.json")),
    ("tr", include_str!("../../../data/lang/tr/ui.json")),
];

/// The themes WinWeaver embeds besides Light and Dark of WW: the colours of the first version.
pub const THEMES: [(&str, &str); 2] = [
    ("latte", include_str!("../../../data/themes/latte.wwtheme")),
    (
        "matrix",
        include_str!("../../../data/themes/matrix.wwtheme"),
    ),
];
/// The largest Markdown file a tab shows.
pub(crate) const MARKDOWN_MAX_BYTES: u64 = 4 * 1024 * 1024;

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

pub struct WinWeaver {
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
    marks: Bookmarks,
    marks_path: PathBuf,
    mcp: mcpview::McpPanel,
    /// A plan waiting for the confirmation of the user.
    confirm: Option<winweaver_core::apply::Plan>,
    /// The helper that runs a confirmed plan, and what the plan left out.
    running: Option<std::sync::mpsc::Receiver<Result<winweaver_core::apply::Report, String>>>,
    running_skipped: Vec<(String, String)>,
    /// The accounts of a users file on their way into a profile, with their master password asked.
    users_import: Option<usersview::UsersImport>,
    /// The import being renamed in the settings and the name typed so far.
    renaming: Option<(String, String)>,
}

/// The translation for a language code, `None` for English.
pub fn texts_for<'a>(texts: &'a [CatalogTexts], code: &str) -> Option<&'a CatalogTexts> {
    texts.iter().find(|t| t.code == code)
}

impl WinWeaver {
    pub fn new(catalog: Catalog, texts: Vec<CatalogTexts>) -> Self {
        // The input languages and layouts of this Windows, read at the first check of a profile.
        winweaver_core::resources::set_locale_reader(winweaver_win::locales);
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
            marks: Bookmarks::default(),
            marks_path: exe_dir().join(bookmarks::DEFAULT_FILE),
            mcp: mcpview::McpPanel::new(winweaver_mcp::config::Config::default_path(&exe_dir())),
            confirm: None,
            running: None,
            running_skipped: Vec::new(),
            users_import: None,
            renaming: None,
        }
    }
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
    match diff::ItemKey::parse(key) {
        Some(diff::ItemKey::Rule(id)) => sel_from_item(catalog, &id),
        Some(diff::ItemKey::Install(_)) => Sel::Form(Form::Install),
        Some(diff::ItemKey::Languages(_)) => Sel::Form(Form::Languages),
        Some(diff::ItemKey::Accounts) => Sel::Form(Form::Accounts),
        None => Sel::None,
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

/// Opens the window with the renderer of `--renderer` or of the settings first; returns the
/// error text when the catalog or the window cannot start.
pub fn run(cli_renderer: Option<&str>) -> Result<(), String> {
    let catalog = Catalog::embedded().map_err(|e| format!("catalog: {e}"))?;
    let texts = CatalogTexts::embedded().map_err(|e| format!("translations: {e}"))?;
    ww::run::<WinWeaver>(cli_renderer, move |_| {
        WinWeaver::new(catalog.clone(), texts.clone())
    })
}

/// The smoke test of the renderers for CI (`--smoke-window`): a small window drawn twice.
pub fn smoke_window(cli_renderer: Option<&str>) -> Result<String, String> {
    ww::smoke_window::<WinWeaver>(cli_renderer)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod translations;
