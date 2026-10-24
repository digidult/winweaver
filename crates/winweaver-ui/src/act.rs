//! The acts of the window: a change requested while a frame was drawn, applied after it by
//! the subsystem it belongs to (`WinWeaver::apply` hands each one to its `apply_*` method); the
//! acts of the window itself (the tabs, the files, the settings, the themes, a question) are
//! `ShellAct` of WW in the variant `Shell`.

use serde_json::Value;
use winweaver_core::profile::{Account, Install, Languages};
use winweaver_core::users;
use ww::ShellAct;

use crate::{CatalogRef, ImportSource, ScriptKind, Sel, TreeKey, WinWeaver, regfileview, regview};

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    Profile(ProfileAct),
    File(FileAct),
    Settings(SettingsAct),
    Pc(PcAct),
    Users(UsersAct),
    Diff(DiffAct),
    Marks(MarksAct),
    Mcp(McpAct),
    Catalog(CatalogAct),
    Import(ImportAct),
    Registry(RegistryAct),
    RegFile(RegFileAct),
    Pol(PolAct),
    /// An act of the window itself, run by the shell.
    Shell(ShellAct<WinWeaver>),
}

impl From<ShellAct<WinWeaver>> for Act {
    fn from(act: ShellAct<WinWeaver>) -> Self {
        Act::Shell(act)
    }
}

impl From<PolAct> for Act {
    fn from(act: PolAct) -> Self {
        Act::Pol(act)
    }
}

impl From<ProfileAct> for Act {
    fn from(act: ProfileAct) -> Self {
        Act::Profile(act)
    }
}

impl From<FileAct> for Act {
    fn from(act: FileAct) -> Self {
        Act::File(act)
    }
}

impl From<SettingsAct> for Act {
    fn from(act: SettingsAct) -> Self {
        Act::Settings(act)
    }
}

impl From<PcAct> for Act {
    fn from(act: PcAct) -> Self {
        Act::Pc(act)
    }
}

impl From<UsersAct> for Act {
    fn from(act: UsersAct) -> Self {
        Act::Users(act)
    }
}

impl From<DiffAct> for Act {
    fn from(act: DiffAct) -> Self {
        Act::Diff(act)
    }
}

impl From<MarksAct> for Act {
    fn from(act: MarksAct) -> Self {
        Act::Marks(act)
    }
}

impl From<McpAct> for Act {
    fn from(act: McpAct) -> Self {
        Act::Mcp(act)
    }
}

impl From<CatalogAct> for Act {
    fn from(act: CatalogAct) -> Self {
        Act::Catalog(act)
    }
}

impl From<ImportAct> for Act {
    fn from(act: ImportAct) -> Self {
        Act::Import(act)
    }
}

impl From<RegistryAct> for Act {
    fn from(act: RegistryAct) -> Self {
        Act::Registry(act)
    }
}

impl From<RegFileAct> for Act {
    fn from(act: RegFileAct) -> Self {
        Act::RegFile(act)
    }
}

/// The acts of the profile tab.
pub enum ProfileAct {
    Select(Sel),
    /// Selects an item of the profile in front and opens its tree down to it: a link of a view.
    Reveal(Sel),
    Toggle(String, bool),
    SetGroup(String, bool),
    SetParam(String, String, Value),
    Expand(String, bool),
    Back,
    Forward,
    /// Selects an item of the tree of a profile in the group it is drawn under.
    SelectIn(Sel, Option<String>),
    /// Selects a rule of a profile in the tree of another import (the group) and shows it.
    ShowIn(String, String),
    /// Shows a policy of an import (the import, the rule) in the tree of a profile.
    ShowInProfile(String, String),
    /// Shows a group in the tree of the first profile tab.
    ShowGroupInProfile(String),
    SetInstall(Box<Install>),
    SetAccounts(Vec<Account>),
    SetLanguages(Languages),
    Check,
    Build,
    ClearMessages,
    /// Shows or hides the description panel.
    Fold,
}

/// The acts of the files of WinWeaver: a new profile, the pages of the program.
pub enum FileAct {
    NewProfile(Option<&'static str>),
    OpenReport,
    OpenGuide,
    /// Opens the root page of the documentation built into the executable, the reference of the
    /// parameters, from which its other pages are linked.
    OpenDocs,
    /// Opens the license, built into the executable, in a Markdown tab.
    OpenLicense,
}

/// The acts of the settings tab.
pub enum SettingsAct {
    /// Starts to rename an import: its id and its name now.
    StartRename(String, String),
    CancelRename,
}

/// The acts of This PC: the read, the check, the scripts, the native apply.
pub enum PcAct {
    /// Read every rule of this PC into a new profile tab.
    Read,
    /// Read the selected rules of this PC into the open profile.
    ReadSelection,
    /// Compare the selected rules with this PC in the messages panel.
    Check,
    SaveScript(ScriptKind),
    /// A stand-alone script of the whole profile: every rule, the ones off return to the defaults.
    SaveScriptAll(ScriptKind),
    /// Saves the registry steps of the apply (false) or the return (true) of the selection as a
    /// registry file.
    SaveRegFile(bool),
    /// The registry file of the apply (false) or the return (true) of the whole profile.
    SaveRegFileAll(bool),
    /// Prepare the native apply (false) or the return to the defaults (true) of the selection.
    PlanNative(bool),
    PlanUndo,
    Confirm,
    Cancel,
}

/// The acts of a users file.
pub enum UsersAct {
    /// A new users file, empty or of the accounts of this PC.
    New,
    OfThisPc,
    Add,
    Duplicate,
    Remove(usize),
    /// Moves the selected account down (`true`) or up.
    Move(bool),
    Unlock,
    Lock,
    SetMaster,
    SetPassword,
    Check,
    /// Adds the accounts of this PC the file lacks.
    ReadPc,
    /// Asks before an operation on this PC: of every account (`true`) or the selected one.
    AskRun(users::Operation, bool),
    Run(users::Operation, Vec<String>),
    Script,
    /// The accounts of a users file into the profile in front.
    IntoProfile,
    /// A users file of the accounts of the profile in front.
    FromProfile,
}

/// The acts of the comparison of profiles and of the diff tab.
pub enum DiffAct {
    /// The differences of the profile in front from its file or its preset, as a diff tab.
    Unsaved,
    Compare,
    CompareWith(&'static str),
    Apply,
    SaveAs,
    /// Selects an item of the active diff by its key.
    Select(String),
    /// Sets the comment on an item of the active diff (empty removes it).
    Comment(String, String),
    /// Shows an item of the active diff in the profile of its base (false) or other side (true).
    GoTo(String, bool),
    /// Sets one item of the active diff in the profile of its base.
    ApplyItem(String),
    /// Saves the stand-alone script that applies the other side of the active diff to a PC.
    SaveScript(ScriptKind),
    /// Saves the registry file of the apply (false) or the return (true) of the rules of the
    /// active diff.
    SaveReg(bool),
}

/// The acts of the bookmarks.
pub enum MarksAct {
    Toggle,
    Open,
    GoTo(usize),
    /// Selects a bookmark in the bookmark tab.
    Select(usize),
    SetComment(usize, String),
    Remove(usize),
}

/// The acts of the MCP server and its tab.
pub enum McpAct {
    Open,
    Start,
    Stop,
    RenewToken,
    OpenProfile(String),
}

/// The acts of the tab of an import or of a catalog file, and of its editor.
pub enum CatalogAct {
    /// Opens the tab of a loaded import, or brings it to the front, at a rule.
    OpenAt(String, String),
    /// Opens the editor of a loaded import or of the file of a catalog tab.
    Edit(CatalogRef),
    /// Makes an empty catalog file among the imports and opens its editor.
    New,
    EditRemove(String),
    /// Compares the catalog of the active tab with another one in a page.
    Compare(CatalogRef),
    /// Opens the tab of an import.
    Open(String),
    /// A group, a policy, or (`None`) the catalog itself.
    Select(Option<TreeKey>),
    /// Selects a group (`g:<id>`) or a policy of the catalog tab in front and opens its tree down
    /// to it: a link of a view.
    Reveal(TreeKey),
    Expand(String, bool),
    /// Imports the catalog file of the active tab.
    ImportOpen,
}

/// The acts of the imports of policy templates.
pub enum ImportAct {
    /// Import the templates of this Windows (None), of a folder or a catalog file.
    Templates(ImportSource),
    /// Loads an import that is not loaded, or forgets one that is.
    Toggle(String),
    Remove(String),
    /// Looks for the catalogs of the unknown options of the active profile among the imports
    /// that are not loaded, and loads the ones that have them.
    LoadCatalogs,
    Export(String),
    Rename(String, String),
}

/// The acts of the registry tab.
pub enum RegistryAct {
    /// Opens the registry tab, or brings it to the front, at a key and a value of it.
    Open(String, Option<String>),
    /// Shows a key in the registry tab, and a value of it.
    Go(String, Option<String>),
    Expand(String, bool),
    Refresh,
    /// Edits the selected value of the registry tab.
    Edit,
    /// Starts a new key or a new value of a type in the registry tab.
    New(regview::EditKind, &'static str),
    /// Writes what the editor of the registry tab holds.
    Save,
    /// Renames the selected value of the registry tab, or its key without one.
    Rename,
    /// Asks before the selected value, or the key without one, is deleted.
    AskDelete,
    Delete(String, Option<String>),
    /// Bookmarks the key and the value of the registry tab.
    Bookmark,
    /// Shows or hides the search of the registry tab.
    Search,
    /// Starts the search of the registry tab under its selected key.
    Find,
    /// Stops the search of the registry tab.
    FindStop,
    /// Opens the window of the rights of the selected key of the registry tab.
    Rights,
    /// Adds an entry for the account typed in the window of the rights.
    RightsAdd,
    RightsSave,
}

/// The acts of the Group Policy files (`.pol`).
pub enum PolAct {
    /// Opens the file of the computer or of the users of this Windows, at a key and a value.
    OpenSystem(winweaver_core::pol::Scope, Option<String>, Option<String>),
    /// Opens a file chosen in a dialog.
    OpenFile,
    /// Reads the file of the tab in front again.
    Refresh,
}

/// The acts of the tab of a registry file.
pub enum RegFileAct {
    /// Edits the selected line of the registry file in front.
    Edit,
    /// Starts a new key or a new value in the registry file in front.
    New(regfileview::EditKind),
    Remove(Vec<usize>),
    /// Shows the outline as a tree of the keys (true) or as a list in the order of the file.
    Outline(bool),
    /// Asks before the selected value, the selected key or the whole file is applied to this PC.
    AskApply(regfileview::ApplyScope),
    /// Applies the selected value, key or the whole file to the registry of this PC.
    Apply(regfileview::ApplyScope),
}
