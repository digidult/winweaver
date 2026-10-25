//! The command bar of each tab as data: the window draws the items, the menu and the keyboard
//! name their keys, and every key text lives here once.

#[cfg(feature = "full")]
use winweaver_core::users::Operation;
#[cfg(feature = "full")]
use ww::commands::{cmd, cmd_if, when};
use ww::commands::{keyed, toggle};

use crate::i18n::tr;
use crate::{Act, Doc, FileAct, ProfileAct, Shell, ShellAct, ShellDoc, WinWeaver, icons};
#[cfg(feature = "full")]
use crate::{
    CatalogAct, CatalogRef, DiffAct, ImportAct, McpAct, PolAct, RegFileAct, RegistryAct,
    ScriptKind, UsersAct, regfileview, regview,
};

/// The keys of the commands of WinWeaver beyond the ones of WW, each named once.
pub mod keys {
    use eframe::egui::{Key, Modifiers};

    pub use ww::keys::*;

    pub const GUIDE: Keys = Keys::new("F1", Modifiers::NONE, Key::F1);
    pub const CHECK: Keys = Keys::new("F7", Modifiers::NONE, Key::F7);
    pub const BUILD: Keys = Keys::new("F9", Modifiers::NONE, Key::F9);
    #[cfg(feature = "full")]
    pub const BOOKMARK: Keys = Keys::new("Ctrl+B", Modifiers::CTRL, Key::B);
    pub const FOLD: Keys = Keys::new("Ctrl+D", Modifiers::CTRL, Key::D);
    pub const BACK: Keys = Keys::new("Alt+Left", Modifiers::ALT, Key::ArrowLeft);
    pub const FORWARD: Keys = Keys::new("Alt+Right", Modifiers::ALT, Key::ArrowRight);
    #[cfg(feature = "full")]
    pub const RENAME: Keys = Keys::new("F2", Modifiers::NONE, Key::F2);
}

/// An item of the command bar of WinWeaver.
pub type Item = ww::commands::Item<Act>;

/// The menus of the command bar.
#[cfg(feature = "full")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    ThisPc,
    CompareProfile,
    Catalogs,
    CompareCatalog,
    RegFileApply,
    Pol,
    CopyTo,
    PolRoot,
}

#[cfg(feature = "full")]
impl Menu {
    const ALL: [Menu; 8] = [
        Menu::ThisPc,
        Menu::CompareProfile,
        Menu::Catalogs,
        Menu::CompareCatalog,
        Menu::RegFileApply,
        Menu::Pol,
        Menu::CopyTo,
        Menu::PolRoot,
    ];

    /// The menu by the number of its button.
    pub fn from_id(id: usize) -> Option<Menu> {
        Self::ALL.get(id).copied()
    }

    /// The button of the command bar that opens the menu, with the least width of the menu.
    pub fn item(self) -> Item {
        let (icon, label, width) = match self {
            Menu::ThisPc => (icons::THIS_PC, tr("This PC"), 300.0),
            Menu::CompareProfile => (icons::COMPARE, tr("Compare"), 220.0),
            Menu::Catalogs => (icons::PACKAGE, tr("Catalogs"), 300.0),
            Menu::CompareCatalog => (icons::COMPARE, tr("Compare"), 300.0),
            Menu::RegFileApply => (icons::THIS_PC, tr("Apply to this PC"), 280.0),
            Menu::Pol => (icons::SCRIPT, tr("Group Policy"), 280.0),
            Menu::CopyTo => (icons::DUPLICATE, tr("Copy to"), 280.0),
            Menu::PolRoot => (icons::REGISTRY, tr("Root"), 300.0),
        };
        ww::commands::menu(icon, label, width, self as usize)
    }
}

fn new_profile() -> Item {
    keyed(icons::NEW, tr("New"), &keys::NEW, FileAct::NewProfile(None))
}

fn open() -> Item {
    keyed(icons::OPEN, tr("Open"), &keys::OPEN, ShellAct::Open)
}

fn save() -> Item {
    keyed(icons::SAVE, tr("Save"), &keys::SAVE, ShellAct::Save)
}

/// The switch of a profile tab that shows only the rules that are on.
fn only_on(shell: &Shell<WinWeaver>) -> Item {
    let on = matches!(shell.active_doc(), Some(Doc::Profile(d)) if d.only_on);
    toggle(icons::ONLY_ON, tr("Only on"), on, ProfileAct::OnlyOn)
}

#[cfg(feature = "full")]
fn save_as() -> Item {
    keyed(icons::SAVE, tr("Save as"), &keys::SAVE_AS, ShellAct::SaveAs)
}

impl WinWeaver {
    /// The command bar of the active tab in the light edition: the profile alone.
    #[cfg(not(feature = "full"))]
    pub(crate) fn tab_commands(&self, shell: &Shell<WinWeaver>) -> Vec<Item> {
        match shell.docs.get(shell.active) {
            Some(Doc::Profile(_)) => vec![
                new_profile(),
                open(),
                save(),
                Item::Separator,
                keyed(icons::CHECK, tr("Check"), &keys::CHECK, ProfileAct::Check),
                keyed(icons::HAMMER, tr("Build"), &keys::BUILD, ProfileAct::Build),
                Item::Separator,
                only_on(shell),
            ],
            Some(Doc::Shell(ShellDoc::Settings(_))) => Vec::new(),
            _ => vec![new_profile(), open()],
        }
    }

    /// Whether the diff in front changes a rule a PC can apply.
    #[cfg(feature = "full")]
    fn diff_applicable(&self, shell: &Shell<WinWeaver>) -> bool {
        match shell.docs.get(shell.active) {
            Some(Doc::Diff(d)) => {
                let ids: Vec<String> = d.diff.rules.keys().cloned().collect();
                winweaver_core::export::applicable(&self.catalog, &ids)
            }
            _ => false,
        }
    }

    /// The command bar of the active tab, in its order.
    #[cfg(feature = "full")]
    pub(crate) fn tab_commands(&self, shell: &Shell<WinWeaver>) -> Vec<Item> {
        match shell.docs.get(shell.active) {
            Some(Doc::Profile(_)) => vec![
                new_profile(),
                open(),
                save(),
                Item::Separator,
                keyed(icons::CHECK, tr("Check"), &keys::CHECK, ProfileAct::Check),
                keyed(icons::HAMMER, tr("Build"), &keys::BUILD, ProfileAct::Build),
                only_on(shell),
                Item::Separator,
                Menu::ThisPc.item(),
                Menu::CompareProfile.item(),
                Item::Separator,
                Menu::Catalogs.item(),
                Menu::Pol.item(),
            ],
            Some(Doc::Diff(_)) => vec![
                open(),
                save(),
                keyed(icons::SAVE, tr("Save as"), &keys::SAVE_AS, DiffAct::SaveAs),
                Item::Separator,
                cmd(icons::CHECK, tr("Apply"), DiffAct::Apply),
                Item::Separator,
                when(
                    icons::SCRIPT,
                    tr("Script"),
                    self.diff_applicable(shell),
                    DiffAct::SaveScript(ScriptKind::Apply),
                ),
                when(
                    icons::REGISTRY,
                    tr("Registry file"),
                    self.diff_applicable(shell),
                    DiffAct::SaveReg(false),
                ),
            ],
            Some(Doc::Markdown(_)) | Some(Doc::Bookmarks(_)) => vec![open()],
            Some(Doc::Compare(_)) => vec![
                keyed(
                    icons::SAVE,
                    tr("Save the report"),
                    &keys::SAVE,
                    CatalogAct::CompareSave,
                ),
                cmd(icons::BOOK, tr("As a page"), CatalogAct::ComparePage),
            ],
            Some(Doc::Mcp(_)) => {
                let running = self.full.mcp.run.is_some();
                vec![
                    open(),
                    save_as(),
                    Item::Separator,
                    when(icons::START, tr("Start"), !running, McpAct::Start),
                    when(icons::STOP, tr("Stop"), running, McpAct::Stop),
                ]
            }
            Some(Doc::Shell(ShellDoc::Theme(_))) => vec![
                keyed(
                    icons::OPEN,
                    tr("Open"),
                    &keys::OPEN,
                    ShellAct::OpenThemeFile,
                ),
                save(),
                save_as(),
            ],
            Some(Doc::Catalog(c)) if c.file.is_some() => vec![
                open(),
                Item::Separator,
                cmd(icons::IMPORT, tr("Import"), CatalogAct::ImportOpen),
                Menu::CompareCatalog.item(),
                cmd(
                    icons::EDIT,
                    tr("Edit"),
                    CatalogAct::Edit(CatalogRef::Tab(shell.active)),
                ),
                Menu::CopyTo.item(),
                Menu::Pol.item(),
            ],
            Some(Doc::Catalog(c)) => vec![
                open(),
                Item::Separator,
                cmd_if(
                    icons::SHOW,
                    tr("Show in the profile"),
                    c.sel
                        .clone()
                        .map(|rule| ProfileAct::ShowInProfile(c.id.clone(), rule).into()),
                ),
                cmd(icons::EXPORT, tr("Export"), ImportAct::Export(c.id.clone())),
                Menu::CompareCatalog.item(),
                cmd(
                    icons::EDIT,
                    tr("Edit"),
                    CatalogAct::Edit(CatalogRef::Import(c.id.clone())),
                ),
                Menu::CopyTo.item(),
                Menu::Pol.item(),
            ],
            Some(Doc::CatalogEdit(_)) => vec![save()],
            Some(Doc::RegFile(d)) => {
                let key = d.selected_key();
                vec![
                    open(),
                    save(),
                    save_as(),
                    Item::Separator,
                    cmd(
                        icons::FOLDER,
                        tr("New key"),
                        RegFileAct::New(regfileview::EditKind::Key),
                    ),
                    when(
                        icons::ADD,
                        tr("New value"),
                        key.is_some(),
                        RegFileAct::New(regfileview::EditKind::Value),
                    ),
                    when(
                        icons::EDIT,
                        tr("Edit"),
                        d.target().is_some(),
                        RegFileAct::Edit,
                    ),
                    cmd_if(
                        icons::DELETE,
                        tr("Delete"),
                        d.remove_question().map(Act::from),
                    ),
                    Item::Separator,
                    cmd_if(
                        icons::REGISTRY,
                        tr("Open in the registry"),
                        key.map(|(_, path)| RegistryAct::Open(path, d.selected_value()).into()),
                    ),
                    Menu::RegFileApply.item(),
                    Item::Separator,
                    if shell.settings.extra.regfile_tree {
                        cmd(icons::PAGE, tr("List"), RegFileAct::Outline(false))
                    } else {
                        cmd(icons::FOLDER, tr("Tree"), RegFileAct::Outline(true))
                    },
                ]
            }
            Some(Doc::Users(d)) => {
                let one = d.selected_user().is_some();
                let unlocked = d.unlocked();
                let create = Operation::Create {
                    update_existing: false,
                };
                let mut items = vec![
                    save(),
                    save_as(),
                    Item::Separator,
                    cmd(icons::ADD, tr("Add"), UsersAct::Add),
                    when(icons::DUPLICATE, tr("Duplicate"), one, UsersAct::Duplicate),
                    cmd_if(
                        icons::DELETE,
                        tr("Remove"),
                        d.remove_question().map(Act::from),
                    ),
                    when(icons::UP, tr("Up"), one, UsersAct::Move(false)),
                    when(icons::DOWN, tr("Down"), one, UsersAct::Move(true)),
                    Item::Separator,
                ];
                if unlocked {
                    items.extend([
                        cmd(icons::UNLOCKED, tr("Lock"), UsersAct::Lock),
                        when(icons::KEY, tr("Password"), one, UsersAct::SetPassword),
                        cmd(
                            icons::MASTER_KEY,
                            tr("Master password"),
                            UsersAct::SetMaster,
                        ),
                    ]);
                } else {
                    items.push(cmd(icons::LOCKED, tr("Unlock"), UsersAct::Unlock));
                }
                items.extend([
                    Item::Separator,
                    cmd(icons::CHECK, tr("Check"), UsersAct::Check),
                    cmd(icons::THIS_PC, tr("Read this PC"), UsersAct::ReadPc),
                    when(
                        icons::START,
                        tr("Create on this PC"),
                        unlocked,
                        UsersAct::AskRun(create, true),
                    ),
                    when(
                        icons::KEY,
                        tr("Apply passwords"),
                        unlocked,
                        UsersAct::AskRun(Operation::Passwords, true),
                    ),
                    cmd(icons::SCRIPT, tr("Script"), UsersAct::Script),
                ]);
                items
            }
            Some(Doc::Registry(d)) => vec![
                cmd(icons::REFRESH, tr("Refresh"), RegistryAct::Refresh),
                Item::Separator,
                cmd(
                    icons::FOLDER,
                    tr("New key"),
                    RegistryAct::New(regview::EditKind::NewKey, ""),
                ),
                cmd(
                    icons::ADD,
                    tr("New value"),
                    RegistryAct::New(regview::EditKind::NewValue, "REG_SZ"),
                ),
                when(
                    icons::EDIT,
                    tr("Edit"),
                    d.selected().is_some(),
                    RegistryAct::Edit,
                ),
                keyed(
                    icons::RENAME,
                    tr("Rename"),
                    &keys::RENAME,
                    RegistryAct::Rename,
                ),
                cmd(icons::DELETE, tr("Delete"), RegistryAct::AskDelete),
                Item::Separator,
                cmd(icons::STAR, tr("Bookmark"), RegistryAct::Bookmark),
                cmd(icons::FIND, tr("Find"), RegistryAct::Search),
                cmd(icons::LOCKED, tr("Permissions"), RegistryAct::Rights),
                Menu::Pol.item(),
            ],
            Some(Doc::Pol(d)) => {
                let key = d.sel.as_ref().map(|k| d.pol.full_key(k));
                let known = d.pol.scope.hive().is_some();
                vec![
                    open(),
                    save(),
                    save_as(),
                    cmd(icons::REFRESH, tr("Read again"), PolAct::Refresh),
                    Item::Separator,
                    cmd(icons::FOLDER, tr("New key"), PolAct::NewEntry(true)),
                    when(
                        icons::ADD,
                        tr("New value"),
                        d.sel.is_some(),
                        PolAct::NewEntry(false),
                    ),
                    when(icons::EDIT, tr("Edit"), d.entry.is_some(), PolAct::Edit),
                    when(
                        icons::DELETE,
                        tr("Remove"),
                        d.entry.is_some() || d.sel.is_some(),
                        PolAct::AskRemove,
                    ),
                    Item::Separator,
                    Menu::PolRoot.item(),
                    cmd(icons::IMPORT, tr("Add a file"), PolAct::AddFile),
                    when(icons::EXPORT, tr("To .reg"), known, PolAct::ExportReg),
                    cmd_if(
                        icons::REGISTRY,
                        tr("Open in the registry"),
                        key.filter(|_| known)
                            .map(|k| RegistryAct::Open(k, None).into()),
                    ),
                    Menu::Pol.item(),
                ]
            }
            Some(Doc::Shell(ShellDoc::Settings(_))) => Vec::new(),
            None => vec![new_profile(), open()],
        }
    }
}
