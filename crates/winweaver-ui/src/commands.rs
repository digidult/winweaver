//! The commands of the command bar of each tab as data: the chrome draws them, the menu and the
//! keyboard name their keys, and every key text lives here once.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use winweaver_core::users::Operation;

use crate::i18n::tr;
use crate::{
    Act, App, CatalogAct, CatalogRef, DiffAct, Doc, FileAct, ImportAct, McpAct, ProfileAct,
    RegFileAct, RegistryAct, ThemeAct, UsersAct, icons, regfileview, regview,
};

/// A keyboard shortcut with its text.
pub struct Keys {
    pub text: &'static str,
    pub modifiers: Modifiers,
    pub key: Key,
}

impl Keys {
    pub const fn new(text: &'static str, modifiers: Modifiers, key: Key) -> Self {
        Self {
            text,
            modifiers,
            key,
        }
    }

    pub fn shortcut(&self) -> KeyboardShortcut {
        KeyboardShortcut::new(self.modifiers, self.key)
    }
}

/// The keys of the commands, each named once.
pub mod keys {
    use eframe::egui::{Key, Modifiers};

    use super::Keys;

    const CTRL_SHIFT: Modifiers = Modifiers {
        alt: false,
        ctrl: true,
        shift: true,
        mac_cmd: false,
        command: false,
    };

    pub const NEW: Keys = Keys::new("Ctrl+N", Modifiers::CTRL, Key::N);
    pub const OPEN: Keys = Keys::new("Ctrl+O", Modifiers::CTRL, Key::O);
    pub const SAVE: Keys = Keys::new("Ctrl+S", Modifiers::CTRL, Key::S);
    pub const SAVE_AS: Keys = Keys::new("Ctrl+Shift+S", CTRL_SHIFT, Key::S);
    pub const CLOSE: Keys = Keys::new("Ctrl+W", Modifiers::CTRL, Key::W);
    pub const SETTINGS: Keys = Keys::new("Ctrl+,", Modifiers::CTRL, Key::Comma);
    pub const GUIDE: Keys = Keys::new("F1", Modifiers::NONE, Key::F1);
    pub const CHECK: Keys = Keys::new("F7", Modifiers::NONE, Key::F7);
    pub const BUILD: Keys = Keys::new("F9", Modifiers::NONE, Key::F9);
    pub const BOOKMARK: Keys = Keys::new("Ctrl+B", Modifiers::CTRL, Key::B);
    pub const FOLD: Keys = Keys::new("Ctrl+D", Modifiers::CTRL, Key::D);
    pub const FIND: Keys = Keys::new("Ctrl+F", Modifiers::CTRL, Key::F);
    pub const BACK: Keys = Keys::new("Alt+Left", Modifiers::ALT, Key::ArrowLeft);
    pub const FORWARD: Keys = Keys::new("Alt+Right", Modifiers::ALT, Key::ArrowRight);
    pub const NEXT_TAB: Keys = Keys::new("Ctrl+Tab", Modifiers::CTRL, Key::Tab);
    pub const PREVIOUS_TAB: Keys = Keys::new("Ctrl+Shift+Tab", CTRL_SHIFT, Key::Tab);
    pub const TAB_LEFT: Keys = Keys::new("Ctrl+Shift+PgUp", CTRL_SHIFT, Key::PageUp);
    pub const TAB_RIGHT: Keys = Keys::new("Ctrl+Shift+PgDn", CTRL_SHIFT, Key::PageDown);
}

/// An item of the command bar.
pub enum Item {
    /// A button; without an act it is disabled.
    Command(Command),
    Separator,
    /// A button that opens a menu the chrome draws.
    Menu(Menu),
}

/// A button of the command bar.
pub struct Command {
    pub icon: &'static str,
    /// The label, translated.
    pub label: &'static str,
    pub keys: Option<&'static Keys>,
    /// What a click does; `None` disables the button.
    pub act: Option<Act>,
}

/// The menus of the command bar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    ThisPc,
    CompareProfile,
    Catalogs,
    CompareCatalog,
}

impl Menu {
    pub fn icon(self) -> &'static str {
        match self {
            Menu::ThisPc => icons::THIS_PC,
            Menu::CompareProfile | Menu::CompareCatalog => icons::COMPARE,
            Menu::Catalogs => icons::PACKAGE,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Menu::ThisPc => tr("This PC"),
            Menu::CompareProfile | Menu::CompareCatalog => tr("Compare"),
            Menu::Catalogs => tr("Catalogs"),
        }
    }

    /// The least width of the menu.
    pub fn width(self) -> f32 {
        match self {
            Menu::CompareProfile => 220.0,
            Menu::ThisPc | Menu::Catalogs | Menu::CompareCatalog => 300.0,
        }
    }
}

fn cmd(icon: &'static str, label: &'static str, act: impl Into<Act>) -> Item {
    Item::Command(Command {
        icon,
        label,
        keys: None,
        act: Some(act.into()),
    })
}

/// A command that is disabled without an act.
fn cmd_if(icon: &'static str, label: &'static str, act: Option<Act>) -> Item {
    Item::Command(Command {
        icon,
        label,
        keys: None,
        act,
    })
}

fn when(icon: &'static str, label: &'static str, enabled: bool, act: impl Into<Act>) -> Item {
    cmd_if(icon, label, enabled.then(|| act.into()))
}

fn keyed(
    icon: &'static str,
    label: &'static str,
    keys: &'static Keys,
    act: impl Into<Act>,
) -> Item {
    Item::Command(Command {
        icon,
        label,
        keys: Some(keys),
        act: Some(act.into()),
    })
}

fn new_profile() -> Item {
    keyed(icons::NEW, tr("New"), &keys::NEW, FileAct::NewProfile(None))
}

fn open() -> Item {
    keyed(icons::OPEN, tr("Open"), &keys::OPEN, FileAct::Open)
}

fn save() -> Item {
    keyed(icons::SAVE, tr("Save"), &keys::SAVE, FileAct::Save)
}

fn save_as() -> Item {
    keyed(icons::SAVE, tr("Save as"), &keys::SAVE_AS, FileAct::SaveAs)
}

impl App {
    /// The command bar of the active tab, in its order.
    pub(crate) fn commands(&self) -> Vec<Item> {
        match self.docs.get(self.active) {
            Some(Doc::Profile(_)) => vec![
                new_profile(),
                open(),
                save(),
                Item::Separator,
                keyed(icons::CHECK, tr("Check"), &keys::CHECK, ProfileAct::Check),
                keyed(icons::GEAR, tr("Build"), &keys::BUILD, ProfileAct::Build),
                Item::Separator,
                Item::Menu(Menu::ThisPc),
                Item::Menu(Menu::CompareProfile),
                Item::Separator,
                Item::Menu(Menu::Catalogs),
            ],
            Some(Doc::Diff(_)) => vec![
                open(),
                save(),
                keyed(icons::SAVE, tr("Save as"), &keys::SAVE_AS, DiffAct::SaveAs),
                Item::Separator,
                cmd(icons::CHECK, tr("Apply"), DiffAct::Apply),
            ],
            Some(Doc::Markdown(_)) | Some(Doc::Bookmarks(_)) => vec![open()],
            Some(Doc::Mcp(_)) => {
                let running = self.mcp.run.is_some();
                vec![
                    when(icons::START, tr("Start"), !running, McpAct::Start),
                    when(icons::STOP, tr("Stop"), running, McpAct::Stop),
                ]
            }
            Some(Doc::Theme(_)) => vec![
                keyed(icons::OPEN, tr("Open"), &keys::OPEN, ThemeAct::OpenFile),
                save(),
                save_as(),
            ],
            Some(Doc::Catalog(c)) if c.file.is_some() => vec![
                open(),
                Item::Separator,
                cmd(icons::IMPORT, tr("Import"), CatalogAct::ImportOpen),
                Item::Menu(Menu::CompareCatalog),
                cmd(
                    icons::EDIT,
                    tr("Edit"),
                    CatalogAct::Edit(CatalogRef::Tab(self.active)),
                ),
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
                Item::Menu(Menu::CompareCatalog),
                cmd(
                    icons::EDIT,
                    tr("Edit"),
                    CatalogAct::Edit(CatalogRef::Import(c.id.clone())),
                ),
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
                cmd(icons::DELETE, tr("Delete"), RegistryAct::AskDelete),
                Item::Separator,
                cmd(icons::STAR, tr("Bookmark"), RegistryAct::Bookmark),
                cmd(icons::FIND, tr("Find"), RegistryAct::Search),
                cmd(icons::LOCKED, tr("Permissions"), RegistryAct::Rights),
            ],
            Some(Doc::Settings(_)) => Vec::new(),
            None => vec![new_profile(), open()],
        }
    }
}
