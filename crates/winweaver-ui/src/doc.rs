//! The tabs of the window: the enum of every document, the trait every document answers, and
//! the documents that are a state of the window alone (the bookmarks, the MCP server, the
//! settings).

use std::path::Path;

use crate::cataledit::EditorDoc;
use crate::catalogview::CatalogDoc;
use crate::diffview::DiffDoc;
use crate::i18n::tr;
use crate::marksview::MarksDoc;
use crate::mcpview::McpDoc;
use crate::mdview::MdDoc;
use crate::regfileview::RegFileDoc;
use crate::regview::RegistryDoc;
use crate::themeview::ThemeDoc;
use crate::usersview::UsersDoc;
use crate::{ProfileDoc, Search, icons, themes};
use winweaver_core::{diff, profile};

/// What every tab answers, so the chrome draws it without knowing the document.
pub trait Tab {
    /// The title of the tab: the name of its file, else `name`.
    fn title(&self) -> String {
        match self.path() {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => self.name(),
        }
    }

    /// The title of a document that has no file.
    fn name(&self) -> String {
        String::new()
    }

    /// Whether the document has changes its file does not.
    fn dirty(&self) -> bool {
        false
    }

    /// The file of the document: the hint of the tab, and opened again at the next start.
    fn path(&self) -> Option<&Path> {
        None
    }

    /// The search of the tab, for the tabs that have one.
    fn search(&mut self) -> Option<&mut Search> {
        None
    }
}

/// The kind of a tab: what the chrome shows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Profile,
    Diff,
    Markdown,
    Bookmarks,
    Mcp,
    Settings,
    Catalog,
    Theme,
    Registry,
    CatalogEdit,
    RegFile,
    Users,
}

impl DocKind {
    /// The icon of the tab.
    pub fn icon(self) -> &'static str {
        match self {
            DocKind::Profile | DocKind::Settings => icons::GEAR,
            DocKind::Diff => icons::COMPARE,
            DocKind::Markdown => icons::BOOK,
            DocKind::Bookmarks => icons::STAR,
            DocKind::Mcp => icons::LINK,
            DocKind::Catalog => icons::PACKAGE,
            DocKind::Theme => icons::PALETTE,
            DocKind::Registry => icons::REGISTRY,
            DocKind::CatalogEdit => icons::EDIT,
            DocKind::RegFile => icons::PAGE,
            DocKind::Users => icons::PEOPLE,
        }
    }

    /// The hint of the search field of the tab, empty for a tab without a search.
    pub fn search_hint(self) -> &'static str {
        match self {
            DocKind::Profile => "Search rules (Ctrl+F)",
            DocKind::Diff => "Search the differences (Ctrl+F)",
            DocKind::Markdown => "Search the text (Ctrl+F)",
            DocKind::Bookmarks => "Search the bookmarks (Ctrl+F)",
            DocKind::Mcp => "Filter the journal (Ctrl+F)",
            DocKind::Catalog => "Search policies (Ctrl+F)",
            DocKind::RegFile => "Search the keys, values and comments (Ctrl+F)",
            DocKind::Users => "Search the accounts (Ctrl+F)",
            DocKind::Settings | DocKind::Theme | DocKind::Registry | DocKind::CatalogEdit => "",
        }
    }

    /// Whether the tab has commands in the command bar.
    pub fn has_tools(self) -> bool {
        self != DocKind::Settings
    }

    /// Whether the tab has a navigation row: the history, the breadcrumb and the star.
    pub fn has_nav(self) -> bool {
        matches!(self, DocKind::Profile | DocKind::Catalog)
    }

    /// Whether the tab has something to search.
    pub fn has_search(self) -> bool {
        !self.search_hint().is_empty()
    }
}

/// A section of the settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    /// The policy templates: the imports.
    Templates,
    /// The folders of the program and the recent files.
    Files,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 3] = [Self::General, Self::Templates, Self::Files];

    /// The title of the section.
    pub fn title(self) -> &'static str {
        match self {
            Self::General => tr("General"),
            Self::Templates => tr("Policy templates"),
            Self::Files => tr("Files"),
        }
    }
}

/// The settings tab.
#[derive(Default)]
pub struct SettingsDoc {
    pub section: SettingsSection,
    /// The import being renamed and the name typed so far.
    pub renaming: Option<(String, String)>,
}

/// A tab of the window.
pub enum Doc {
    Profile(Box<ProfileDoc>),
    Diff(Box<DiffDoc>),
    Markdown(Box<MdDoc>),
    Bookmarks(Box<MarksDoc>),
    Mcp(Box<McpDoc>),
    Settings(Box<SettingsDoc>),
    /// The tree of an import of policy templates.
    Catalog(Box<CatalogDoc>),
    /// The editor of a theme.
    Theme(Box<ThemeDoc>),
    /// The registry of this PC.
    Registry(Box<RegistryDoc>),
    /// The editor of a catalog file.
    CatalogEdit(Box<EditorDoc>),
    /// A registry file (`.reg`).
    RegFile(Box<RegFileDoc>),
    /// A users file (`.wwusers`).
    Users(Box<UsersDoc>),
}

impl Doc {
    pub fn kind(&self) -> DocKind {
        match self {
            Doc::Profile(_) => DocKind::Profile,
            Doc::Diff(_) => DocKind::Diff,
            Doc::Markdown(_) => DocKind::Markdown,
            Doc::Bookmarks(_) => DocKind::Bookmarks,
            Doc::Mcp(_) => DocKind::Mcp,
            Doc::Settings(_) => DocKind::Settings,
            Doc::Catalog(_) => DocKind::Catalog,
            Doc::Theme(_) => DocKind::Theme,
            Doc::Registry(_) => DocKind::Registry,
            Doc::CatalogEdit(_) => DocKind::CatalogEdit,
            Doc::RegFile(_) => DocKind::RegFile,
            Doc::Users(_) => DocKind::Users,
        }
    }

    /// The document of the tab.
    pub fn tab(&self) -> &dyn Tab {
        match self {
            Doc::Profile(d) => &**d,
            Doc::Diff(d) => &**d,
            Doc::Markdown(d) => &**d,
            Doc::Bookmarks(d) => &**d,
            Doc::Mcp(d) => &**d,
            Doc::Settings(d) => &**d,
            Doc::Catalog(d) => &**d,
            Doc::Theme(d) => &**d,
            Doc::Registry(d) => &**d,
            Doc::CatalogEdit(d) => &**d,
            Doc::RegFile(d) => &**d,
            Doc::Users(d) => &**d,
        }
    }

    pub fn tab_mut(&mut self) -> &mut dyn Tab {
        match self {
            Doc::Profile(d) => &mut **d,
            Doc::Diff(d) => &mut **d,
            Doc::Markdown(d) => &mut **d,
            Doc::Bookmarks(d) => &mut **d,
            Doc::Mcp(d) => &mut **d,
            Doc::Settings(d) => &mut **d,
            Doc::Catalog(d) => &mut **d,
            Doc::Theme(d) => &mut **d,
            Doc::Registry(d) => &mut **d,
            Doc::CatalogEdit(d) => &mut **d,
            Doc::RegFile(d) => &mut **d,
            Doc::Users(d) => &mut **d,
        }
    }
}

impl Tab for Doc {
    fn title(&self) -> String {
        self.tab().title()
    }

    fn dirty(&self) -> bool {
        self.tab().dirty()
    }

    fn path(&self) -> Option<&Path> {
        self.tab().path()
    }

    fn search(&mut self) -> Option<&mut Search> {
        self.tab_mut().search()
    }
}

impl Tab for ProfileDoc {
    fn name(&self) -> String {
        format!("{}.{}", self.profile.name, profile::EXTENSION)
    }

    fn dirty(&self) -> bool {
        self.dirty
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for DiffDoc {
    fn name(&self) -> String {
        format!(
            "{} vs {}.{}",
            self.diff.base.name, self.diff.other.name,
            diff::EXTENSION
        )
    }

    fn dirty(&self) -> bool {
        self.dirty
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for MdDoc {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for MarksDoc {
    fn title(&self) -> String {
        tr("Bookmarks").to_owned()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for McpDoc {
    fn title(&self) -> String {
        "MCP".to_owned()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for SettingsDoc {
    fn title(&self) -> String {
        tr("Settings").to_owned()
    }
}

impl Tab for CatalogDoc {
    fn title(&self) -> String {
        self.name.clone()
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for ThemeDoc {
    fn name(&self) -> String {
        format!("{}.{}", self.theme.id, themes::EXTENSION)
    }

    fn dirty(&self) -> bool {
        self.dirty
    }

    fn path(&self) -> Option<&Path> {
        self.theme.path.as_deref()
    }
}

impl Tab for RegistryDoc {
    fn title(&self) -> String {
        RegistryDoc::title(self)
    }
}

/// The editor of a catalog file names no path: at the next start its file opens as a catalog
/// tab, so the tab is not remembered.
impl Tab for EditorDoc {
    fn title(&self) -> String {
        EditorDoc::title(self)
    }

    fn dirty(&self) -> bool {
        self.dirty
    }
}

impl Tab for RegFileDoc {
    fn dirty(&self) -> bool {
        self.dirty
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

impl Tab for UsersDoc {
    fn title(&self) -> String {
        UsersDoc::title(self)
    }

    fn dirty(&self) -> bool {
        self.dirty
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}
