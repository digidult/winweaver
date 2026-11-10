//! The tabs of the window: the enum of every document, what each answers to the window
//! (`ww::Document`, named `Tab` here), the kinds the chrome knows and the sections of the
//! settings.

use std::path::Path;

use ww::{Draft, Scrolled, ShellDoc};

#[cfg(feature = "full")]
use crate::cataledit::EditorDoc;
#[cfg(feature = "full")]
use crate::catalogdiff::CompareDoc;
#[cfg(feature = "full")]
use crate::catalogview::CatalogDoc;
#[cfg(feature = "full")]
use crate::diffview::DiffDoc;
// The light edition in a browser names no section of the settings and no bookmarks.
#[cfg(any(feature = "full", not(target_arch = "wasm32")))]
use crate::i18n::tr;
#[cfg(feature = "full")]
use crate::marksview::MarksDoc;
#[cfg(feature = "mcp")]
use crate::mcpview::McpDoc;
#[cfg(feature = "full")]
use crate::polview::PolDoc;
#[cfg(feature = "full")]
use crate::regfileview::RegFileDoc;
#[cfg(feature = "full")]
use crate::regview::RegistryDoc;
#[cfg(feature = "full")]
use crate::usersview::UsersDoc;
use crate::{MdDoc, ProfileDoc, Search, Tab, icons};
#[cfg(feature = "full")]
use winweaver_core::diff;
use winweaver_core::profile;

/// The kinds of the drafts beside the ones of profiles (a profile is its JSON as it is).
#[cfg(feature = "full")]
pub const REGFILE_DRAFT: &str = "regfile";
#[cfg(feature = "full")]
pub const DIFF_DRAFT: &str = "diff";
#[cfg(feature = "full")]
pub const CATALOG_DRAFT: &str = "catalog-edit";
#[cfg(feature = "full")]
pub const USERS_DRAFT: &str = "users";
#[cfg(feature = "full")]
pub const POL_DRAFT: &str = "pol";

/// The bytes of a Group Policy file as hexadecimal text, for its draft.
#[cfg(feature = "full")]
pub fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes of [`hex_of`] back; `None` for a text that is not one.
#[cfg(feature = "full")]
pub fn bytes_of_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The document of a draft: its kind and its text.
#[cfg(feature = "full")]
fn draft_json(kind: &str, text: String) -> serde_json::Value {
    serde_json::json!({ "kind": kind, "text": text })
}

/// A draft is written this long after the last change.
const DRAFT_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// The kind of a tab: what the chrome shows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Profile,
    #[cfg(feature = "full")]
    Diff,
    Markdown,
    #[cfg(feature = "full")]
    Bookmarks,
    #[cfg(feature = "mcp")]
    Mcp,
    Settings,
    #[cfg(feature = "full")]
    Catalog,
    Theme,
    #[cfg(feature = "full")]
    Registry,
    #[cfg(feature = "full")]
    CatalogEdit,
    #[cfg(feature = "full")]
    RegFile,
    #[cfg(feature = "full")]
    Users,
    #[cfg(feature = "full")]
    Pol,
    #[cfg(feature = "full")]
    Compare,
}

impl DocKind {
    /// The icon of the tab.
    pub fn icon(self) -> &'static str {
        match self {
            DocKind::Profile => icons::PROFILE,
            DocKind::Settings => icons::GEAR,
            DocKind::Markdown => icons::BOOK,
            DocKind::Theme => icons::PALETTE,
            #[cfg(feature = "full")]
            DocKind::Diff | DocKind::Compare => icons::COMPARE,
            #[cfg(feature = "full")]
            DocKind::Bookmarks => icons::STAR,
            #[cfg(feature = "mcp")]
            DocKind::Mcp => icons::LINK,
            #[cfg(feature = "full")]
            DocKind::Catalog => icons::PACKAGE,
            #[cfg(feature = "full")]
            DocKind::Registry => icons::REGISTRY,
            #[cfg(feature = "full")]
            DocKind::CatalogEdit => icons::EDIT,
            #[cfg(feature = "full")]
            DocKind::RegFile => icons::PAGE,
            #[cfg(feature = "full")]
            DocKind::Users => icons::PEOPLE,
            #[cfg(feature = "full")]
            DocKind::Pol => icons::SCRIPT,
        }
    }

    /// The hint of the search field of the tab, empty for a tab without a search.
    pub fn search_hint(self) -> &'static str {
        match self {
            DocKind::Profile => "Search rules (Ctrl+F)",
            DocKind::Markdown => "Search the text (Ctrl+F)",
            DocKind::Settings | DocKind::Theme => "",
            #[cfg(feature = "full")]
            DocKind::Diff => "Search the differences (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::Bookmarks => "Search the bookmarks (Ctrl+F)",
            #[cfg(feature = "mcp")]
            DocKind::Mcp => "Filter the journal (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::Catalog | DocKind::Compare => "Search policies (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::RegFile => "Search the keys, values and comments (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::Users => "Search the accounts (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::Pol => "Search the keys, values and data (Ctrl+F)",
            #[cfg(feature = "full")]
            DocKind::Registry | DocKind::CatalogEdit => "",
        }
    }

    /// Whether the tab has commands in the command bar.
    pub fn has_tools(self) -> bool {
        self != DocKind::Settings
    }

    /// Whether the tab has a navigation row: the history, the breadcrumb and the star.
    pub fn has_nav(self) -> bool {
        #[cfg(feature = "full")]
        {
            matches!(self, DocKind::Profile | DocKind::Catalog)
        }
        #[cfg(not(feature = "full"))]
        {
            self == DocKind::Profile
        }
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
    #[cfg(feature = "full")]
    Templates,
    /// The folders of the program and the recent files; a browser has neither.
    #[cfg(not(target_arch = "wasm32"))]
    Files,
}

impl SettingsSection {
    /// The sections in their order after General.
    #[cfg(all(feature = "full", not(target_arch = "wasm32")))]
    pub const ALL: [SettingsSection; 2] = [Self::Templates, Self::Files];
    #[cfg(all(feature = "full", target_arch = "wasm32"))]
    pub const ALL: [SettingsSection; 1] = [Self::Templates];
    #[cfg(all(not(feature = "full"), not(target_arch = "wasm32")))]
    pub const ALL: [SettingsSection; 1] = [Self::Files];
    #[cfg(all(not(feature = "full"), target_arch = "wasm32"))]
    pub const ALL: [SettingsSection; 0] = [];

    /// The title of the section.
    pub fn title(self) -> &'static str {
        match self {
            #[cfg(feature = "full")]
            Self::Templates => tr("Policy templates"),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Files => tr("Files"),
        }
    }

    /// The number of the section in the settings tab, where General is 0.
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .map_or(0, |i| i + 1)
    }
}

/// A tab of the window.
pub enum Doc {
    Profile(Box<ProfileDoc>),
    #[cfg(feature = "full")]
    Diff(Box<DiffDoc>),
    Markdown(Box<MdDoc>),
    #[cfg(feature = "full")]
    Bookmarks(Box<MarksDoc>),
    #[cfg(feature = "mcp")]
    Mcp(Box<McpDoc>),
    /// The tree of an import of policy templates.
    #[cfg(feature = "full")]
    Catalog(Box<CatalogDoc>),
    /// The registry of this PC.
    #[cfg(feature = "full")]
    Registry(Box<RegistryDoc>),
    /// The editor of a catalog file.
    #[cfg(feature = "full")]
    CatalogEdit(Box<EditorDoc>),
    /// A registry file (`.reg`).
    #[cfg(feature = "full")]
    RegFile(Box<RegFileDoc>),
    /// A users file (`.wwusers`).
    #[cfg(feature = "full")]
    Users(Box<UsersDoc>),
    /// A Group Policy file (`.pol`).
    #[cfg(feature = "full")]
    Pol(Box<PolDoc>),
    /// Two catalogs compared policy by policy.
    #[cfg(feature = "full")]
    Compare(Box<CompareDoc>),
    /// A document of the window itself: the settings, the editor of a theme.
    Shell(ShellDoc),
}

impl Doc {
    pub fn kind(&self) -> DocKind {
        match self {
            Doc::Profile(_) => DocKind::Profile,
            Doc::Markdown(_) => DocKind::Markdown,
            Doc::Shell(ShellDoc::Settings(_)) => DocKind::Settings,
            Doc::Shell(ShellDoc::Theme(_)) => DocKind::Theme,
            // A page of a set of embedded files, a document of the window shown as a page.
            Doc::Shell(ShellDoc::Page(_)) => DocKind::Markdown,
            #[cfg(feature = "full")]
            Doc::Diff(_) => DocKind::Diff,
            #[cfg(feature = "full")]
            Doc::Bookmarks(_) => DocKind::Bookmarks,
            #[cfg(feature = "mcp")]
            Doc::Mcp(_) => DocKind::Mcp,
            #[cfg(feature = "full")]
            Doc::Catalog(_) => DocKind::Catalog,
            #[cfg(feature = "full")]
            Doc::Registry(_) => DocKind::Registry,
            #[cfg(feature = "full")]
            Doc::CatalogEdit(_) => DocKind::CatalogEdit,
            #[cfg(feature = "full")]
            Doc::RegFile(_) => DocKind::RegFile,
            #[cfg(feature = "full")]
            Doc::Users(_) => DocKind::Users,
            #[cfg(feature = "full")]
            Doc::Pol(_) => DocKind::Pol,
            #[cfg(feature = "full")]
            Doc::Compare(_) => DocKind::Compare,
        }
    }

    /// The document of the tab.
    pub fn tab(&self) -> &dyn Tab {
        match self {
            Doc::Profile(d) => &**d,
            Doc::Markdown(d) => &**d,
            Doc::Shell(d) => d,
            #[cfg(feature = "full")]
            Doc::Diff(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Bookmarks(d) => &**d,
            #[cfg(feature = "mcp")]
            Doc::Mcp(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Catalog(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Registry(d) => &**d,
            #[cfg(feature = "full")]
            Doc::CatalogEdit(d) => &**d,
            #[cfg(feature = "full")]
            Doc::RegFile(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Users(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Pol(d) => &**d,
            #[cfg(feature = "full")]
            Doc::Compare(d) => &**d,
        }
    }

    pub fn tab_mut(&mut self) -> &mut dyn Tab {
        match self {
            Doc::Profile(d) => &mut **d,
            Doc::Markdown(d) => &mut **d,
            Doc::Shell(d) => d,
            #[cfg(feature = "full")]
            Doc::Diff(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Bookmarks(d) => &mut **d,
            #[cfg(feature = "mcp")]
            Doc::Mcp(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Catalog(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Registry(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::CatalogEdit(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::RegFile(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Users(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Pol(d) => &mut **d,
            #[cfg(feature = "full")]
            Doc::Compare(d) => &mut **d,
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
            #[cfg(feature = "full")]
            Doc::Catalog(c) => (&mut c.reveal, &mut c.follow),
            #[cfg(feature = "full")]
            Doc::Diff(d) => (&mut d.reveal, &mut d.follow),
            #[cfg(feature = "full")]
            Doc::Bookmarks(d) => (&mut d.reveal, &mut d.follow),
            #[cfg(feature = "full")]
            Doc::Compare(d) => (&mut d.reveal, &mut d.follow),
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
            Doc::Shell(d) => d.draft_key(),
            #[cfg(feature = "full")]
            Doc::RegFile(d) => Some(d.draft.key.clone()),
            #[cfg(feature = "full")]
            Doc::Diff(d) => Some(d.draft.key.clone()),
            #[cfg(feature = "full")]
            Doc::CatalogEdit(d) => Some(d.draft.key.clone()),
            #[cfg(feature = "full")]
            Doc::Users(d) => Some(d.draft.key.clone()),
            #[cfg(feature = "full")]
            Doc::Pol(d) => Some(d.draft.key.clone()),
            _ => None,
        }
    }

    /// The draft of a document with unsaved changes: a profile two seconds after its last change,
    /// the others at most every two seconds when they changed, all at once when the window
    /// closes. A users file keeps the passwords of its last save, encrypted.
    fn draft_due(&mut self, all: bool) -> Option<Draft> {
        let other = match self {
            #[cfg(feature = "full")]
            Doc::RegFile(d) => {
                if d.draft.ready(d.dirty, all) {
                    let document = draft_json(REGFILE_DRAFT, d.file.to_text());
                    d.draft.offer(Some(d.path.clone()), document)
                } else {
                    None
                }
            }
            #[cfg(feature = "full")]
            Doc::Diff(d) => {
                if d.draft.ready(d.dirty, all) {
                    let document = draft_json(DIFF_DRAFT, d.diff.to_json());
                    d.draft.offer(d.path.clone(), document)
                } else {
                    None
                }
            }
            #[cfg(feature = "full")]
            Doc::CatalogEdit(d) => {
                if d.draft.ready(d.dirty, all) {
                    let text = serde_json::to_string(&d.file).unwrap_or_default();
                    let mut document = draft_json(CATALOG_DRAFT, text);
                    document["import"] = serde_json::Value::Bool(d.import);
                    d.draft.offer(Some(d.path.clone()), document)
                } else {
                    None
                }
            }
            #[cfg(feature = "full")]
            Doc::Users(d) => {
                if d.draft.ready(d.dirty, all) {
                    let document = draft_json(USERS_DRAFT, d.file.to_json());
                    d.draft.offer(d.path.clone(), document)
                } else {
                    None
                }
            }
            #[cfg(feature = "full")]
            Doc::Pol(d) => {
                if d.draft.ready(d.dirty, all) {
                    let mut document = draft_json(POL_DRAFT, hex_of(&d.pol.to_bytes()));
                    document["scope"] = serde_json::Value::from(d.pol.scope.name());
                    d.draft.offer(Some(d.pol.path.clone()), document)
                } else {
                    None
                }
            }
            Doc::Shell(d) => d.draft_due(all),
            _ => None,
        };
        if other.is_some() {
            return other;
        }
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
impl Tab for MarksDoc {
    fn title(&self) -> String {
        tr("Bookmarks").to_owned()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

#[cfg(feature = "mcp")]
impl Tab for McpDoc {
    fn title(&self) -> String {
        "MCP".to_owned()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
impl Tab for CompareDoc {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

#[cfg(feature = "full")]
impl Tab for RegistryDoc {
    fn title(&self) -> String {
        RegistryDoc::title(self)
    }
}

/// The editor of a catalog file names no path: at the next start its file opens as a catalog
/// tab, so the tab is not remembered.
#[cfg(feature = "full")]
impl Tab for EditorDoc {
    fn title(&self) -> String {
        EditorDoc::title(self)
    }

    fn dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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
