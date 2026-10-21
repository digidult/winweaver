//! The tabs of the window: the enum of every document, what each answers to the window
//! (`ww::Document`, named `Tab` here), the kinds the chrome knows and the sections of the
//! settings.

use std::path::Path;

use ww::{Draft, Scrolled, ShellDoc};

use crate::cataledit::EditorDoc;
use crate::catalogview::CatalogDoc;
use crate::diffview::DiffDoc;
use crate::i18n::tr;
use crate::marksview::MarksDoc;
use crate::mcpview::McpDoc;
use crate::regfileview::RegFileDoc;
use crate::regview::RegistryDoc;
use crate::usersview::UsersDoc;
use crate::{MdDoc, ProfileDoc, Search, Tab, icons};
use winweaver_core::{diff, profile};

/// A draft is written this long after the last change.
const DRAFT_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

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

/// A section of the settings tab after General, the one of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    /// The policy templates: the imports.
    Templates,
    /// The folders of the program and the recent files.
    Files,
}

impl SettingsSection {
    /// The sections in their order after General.
    pub const ALL: [SettingsSection; 2] = [Self::Templates, Self::Files];

    /// The title of the section.
    pub fn title(self) -> &'static str {
        match self {
            Self::Templates => tr("Policy templates"),
            Self::Files => tr("Files"),
        }
    }

    /// The number of the section in the settings tab, where General is 0.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).map_or(0, |i| i + 1)
    }
}

/// A tab of the window.
pub enum Doc {
    Profile(Box<ProfileDoc>),
    Diff(Box<DiffDoc>),
    Markdown(Box<MdDoc>),
    Bookmarks(Box<MarksDoc>),
    Mcp(Box<McpDoc>),
    /// The tree of an import of policy templates.
    Catalog(Box<CatalogDoc>),
    /// The registry of this PC.
    Registry(Box<RegistryDoc>),
    /// The editor of a catalog file.
    CatalogEdit(Box<EditorDoc>),
    /// A registry file (`.reg`).
    RegFile(Box<RegFileDoc>),
    /// A users file (`.wwusers`).
    Users(Box<UsersDoc>),
    /// A document of the window itself: the settings, the editor of a theme.
    Shell(ShellDoc),
}

impl Doc {
    pub fn kind(&self) -> DocKind {
        match self {
            Doc::Profile(_) => DocKind::Profile,
            Doc::Diff(_) => DocKind::Diff,
            Doc::Markdown(_) => DocKind::Markdown,
            Doc::Bookmarks(_) => DocKind::Bookmarks,
            Doc::Mcp(_) => DocKind::Mcp,
            Doc::Catalog(_) => DocKind::Catalog,
            Doc::Registry(_) => DocKind::Registry,
            Doc::CatalogEdit(_) => DocKind::CatalogEdit,
            Doc::RegFile(_) => DocKind::RegFile,
            Doc::Users(_) => DocKind::Users,
            Doc::Shell(ShellDoc::Settings(_)) => DocKind::Settings,
            Doc::Shell(ShellDoc::Theme(_)) => DocKind::Theme,
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
            Doc::Catalog(d) => &**d,
            Doc::Registry(d) => &**d,
            Doc::CatalogEdit(d) => &**d,
            Doc::RegFile(d) => &**d,
            Doc::Users(d) => &**d,
            Doc::Shell(d) => d,
        }
    }

    pub fn tab_mut(&mut self) -> &mut dyn Tab {
        match self {
            Doc::Profile(d) => &mut **d,
            Doc::Diff(d) => &mut **d,
            Doc::Markdown(d) => &mut **d,
            Doc::Bookmarks(d) => &mut **d,
            Doc::Mcp(d) => &mut **d,
            Doc::Catalog(d) => &mut **d,
            Doc::Registry(d) => &mut **d,
            Doc::CatalogEdit(d) => &mut **d,
            Doc::RegFile(d) => &mut **d,
            Doc::Users(d) => &mut **d,
            Doc::Shell(d) => d,
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

    fn icon(&self) -> &'static str {
        self.kind().icon()
    }

    fn search_hint(&self) -> &'static str {
        self.kind().search_hint()
    }

    fn has_tools(&self) -> bool {
        self.kind().has_tools()
    }

    fn has_nav(&self) -> bool {
        self.kind().has_nav()
    }

    /// The tabs with a tree or a list: a move with the keys makes the selection follow into
    /// view, and a row scrolled to stops asking.
    fn scrolled(&mut self, how: Scrolled) {
        let follow = how == Scrolled::Follow;
        let (reveal, follow_flag) = match self {
            Doc::Profile(d) => (&mut d.reveal, &mut d.follow),
            Doc::Catalog(c) => (&mut c.reveal, &mut c.follow),
            Doc::Diff(d) => (&mut d.reveal, &mut d.follow),
            Doc::Bookmarks(d) => (&mut d.reveal, &mut d.follow),
            _ => return,
        };
        *follow_flag = follow;
        if !follow {
            *reveal = false;
        }
    }

    fn draft_key(&self) -> Option<String> {
        match self {
            Doc::Profile(d) => Some(d.draft_key.clone()),
            _ => None,
        }
    }

    /// The draft of a modified profile: two seconds after the last change, or at once when the
    /// window closes.
    fn draft_due(&mut self, all: bool) -> Option<Draft> {
        let Doc::Profile(d) = self else {
            return None;
        };
        if !d.dirty {
            return None;
        }
        let due = match d.changed {
            Some(t) => all || t.elapsed() >= DRAFT_DELAY,
            None => false,
        };
        if !due {
            return None;
        }
        d.changed = None;
        Some(Draft {
            key: d.draft_key.clone(),
            path: d.path.clone(),
            document: serde_json::to_value(&d.profile).unwrap_or_default(),
        })
    }

    fn shell_doc(&self) -> Option<&ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            _ => None,
        }
    }

    fn shell_doc_mut(&mut self) -> Option<&mut ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            _ => None,
        }
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
            self.diff.base.name,
            self.diff.other.name,
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
