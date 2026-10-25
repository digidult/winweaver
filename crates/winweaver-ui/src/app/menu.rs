//! The menu of the tab row and the menus of the command bar (This PC, Compare, Catalogs).

use ww::shell::{menu_entry, menu_item, sub_menu};

use crate::ShellAct;
use crate::commands::{Menu, keys};
use crate::*;

impl WinWeaver {
    /// The items of the menu button of the tab row.
    pub(crate) fn menu_items(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        sub_menu(ui, icons::NEW, tr("New"), |ui| {
            menu_item(
                ui,
                icons::NEW,
                tr("Empty profile"),
                keys::NEW.text,
                Act::File(FileAct::NewProfile(None)),
                acts,
            );
            ui.separator();
            for (name, _) in PRESETS {
                menu_item(
                    ui,
                    icons::PAGE,
                    &trf("Profile from the preset {name}", &[("name", &name)]),
                    "",
                    Act::File(FileAct::NewProfile(Some(name))),
                    acts,
                );
            }
            if lite() {
                return;
            }
            ui.separator();
            menu_item(
                ui,
                icons::PEOPLE,
                tr("Users file"),
                "",
                Act::Users(UsersAct::New),
                acts,
            );
            menu_item(
                ui,
                icons::PEOPLE,
                tr("Users file of the accounts of this PC"),
                "",
                Act::Users(UsersAct::OfThisPc),
                acts,
            );
        });
        menu_item(
            ui,
            icons::OPEN,
            tr("Open..."),
            keys::OPEN.text,
            Act::Shell(ShellAct::Open),
            acts,
        );
        ui.add_enabled_ui(!shell.settings.recent.is_empty(), |ui| {
            sub_menu(ui, icons::RECENT, tr("Open recent"), |ui| {
                for path in &shell.settings.recent {
                    menu_item(
                        ui,
                        icons::PAGE,
                        &path.display().to_string(),
                        "",
                        Act::Shell(ShellAct::OpenPath(path.clone())),
                        acts,
                    );
                }
            });
        });
        menu_item(
            ui,
            icons::SAVE,
            tr("Save"),
            keys::SAVE.text,
            Act::Shell(ShellAct::Save),
            acts,
        );
        menu_item(
            ui,
            icons::SAVE,
            tr("Save as..."),
            keys::SAVE_AS.text,
            Act::Shell(ShellAct::SaveAs),
            acts,
        );
        menu_item(
            ui,
            icons::CLOSE,
            tr("Close tab"),
            keys::CLOSE.text,
            Act::Shell(ShellAct::CloseTab(shell.active, false)),
            acts,
        );
        ui.separator();
        menu_item(
            ui,
            icons::BOOK,
            tr("Guide"),
            keys::GUIDE.text,
            Act::File(FileAct::OpenGuide),
            acts,
        );
        if lite() {
            menu_item(
                ui,
                icons::LICENSE,
                tr("License"),
                "",
                Act::File(FileAct::OpenLicense),
                acts,
            );
            menu_item(
                ui,
                icons::GEAR,
                tr("Settings"),
                keys::SETTINGS.text,
                Act::Shell(ShellAct::OpenSettings),
                acts,
            );
            ui.separator();
            menu_item(
                ui,
                icons::INFO,
                tr("About"),
                "",
                Act::Shell(ShellAct::About),
                acts,
            );
            menu_item(
                ui,
                icons::EXIT,
                tr("Exit"),
                "",
                Act::Shell(ShellAct::Exit),
                acts,
            );
            return;
        }
        menu_item(
            ui,
            icons::BOOKS,
            tr("Documentation"),
            "",
            Act::File(FileAct::OpenDocs),
            acts,
        );
        menu_item(
            ui,
            icons::LICENSE,
            tr("License"),
            "",
            Act::File(FileAct::OpenLicense),
            acts,
        );
        menu_item(
            ui,
            icons::STAR,
            tr("Bookmarks"),
            "",
            Act::Marks(MarksAct::Open),
            acts,
        );
        menu_item(
            ui,
            icons::REGISTRY,
            tr("Registry"),
            "",
            Act::Registry(RegistryAct::Open("HKEY_LOCAL_MACHINE".to_owned(), None)),
            acts,
        );
        menu_item(
            ui,
            icons::SCRIPT,
            tr("Group Policy of the computer"),
            "",
            Act::Pol(PolAct::OpenSystem(
                winweaver_core::pol::Scope::Computer,
                None,
                None,
            )),
            acts,
        );
        menu_item(
            ui,
            icons::LINK,
            tr("MCP server"),
            "",
            Act::Mcp(McpAct::Open),
            acts,
        );
        menu_item(
            ui,
            icons::GEAR,
            tr("Settings"),
            keys::SETTINGS.text,
            Act::Shell(ShellAct::OpenSettings),
            acts,
        );
        ui.separator();
        menu_item(
            ui,
            icons::INFO,
            tr("About"),
            "",
            Act::Shell(ShellAct::About),
            acts,
        );
        menu_item(
            ui,
            icons::EXIT,
            tr("Exit"),
            "",
            Act::Shell(ShellAct::Exit),
            acts,
        );
    }

    /// The menu a button of the command bar opens.
    pub(crate) fn bar_menu(
        &self,
        shell: &Shell<WinWeaver>,
        menu: Menu,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        match menu {
            Menu::ThisPc => {
                let ids = self.selection_ids(shell);
                let applicable = winweaver_core::export::applicable(&self.catalog, &ids);
                this_pc_menu(ui, acts, applicable);
            }
            Menu::CompareProfile => self.compare_profile_menu(shell, ui, acts),
            Menu::Catalogs => self.catalogs_menu(ui, acts),
            Menu::CompareCatalog => self.compare_catalog_menu(shell, ui, acts),
            Menu::RegFileApply => regfile_apply_menu(shell, ui, acts),
            Menu::Pol => pol_menu(ui, acts),
        }
    }

    /// The menu Compare of a profile tab: a file, a preset, the unsaved changes.
    fn compare_profile_menu(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let file = tr("Compare with a file...");
        if menu_entry(ui, icons::OPEN, file, "", true) {
            acts.push(Act::Diff(DiffAct::Compare));
        }
        ui.separator();
        for (name, _) in PRESETS {
            let text = trf("With the preset {name}", &[("name", &name)]);
            if menu_entry(ui, icons::PAGE, &text, "", true) {
                acts.push(Act::Diff(DiffAct::CompareWith(name)));
            }
        }
        ui.separator();
        let dirty = shell.active_doc().is_some_and(Tab::dirty);
        if menu_entry(ui, icons::EDIT, tr("Unsaved changes"), "", dirty) {
            acts.push(Act::Diff(DiffAct::Unsaved));
        }
    }

    /// The menu Compare of a catalog tab: the loaded imports and the tabs of catalog files to
    /// compare the catalog of the tab with.
    fn compare_catalog_menu(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let own = match shell.active_doc() {
            Some(Doc::Catalog(c)) if c.file.is_none() => Some(c.id.clone()),
            _ => None,
        };
        let mut others: Vec<(String, CatalogRef)> = self
            .imports
            .iter()
            .filter(|f| Some(&f.id) != own.as_ref())
            .map(|f| (f.name.clone(), CatalogRef::Import(f.id.clone())))
            .collect();
        for (i, d) in shell.docs.iter().enumerate() {
            if i != shell.active
                && let Doc::Catalog(c) = d
                && let Some(f) = &c.file
            {
                others.push((f.name.clone(), CatalogRef::Tab(i)));
            }
        }
        if others.is_empty() {
            ui.weak(tr(
                "Load or open another catalog to compare it with this one",
            ));
        }
        for (name, other) in others {
            if menu_entry(ui, icons::PACKAGE, &name, "", true) {
                acts.push(Act::Catalog(CatalogAct::Compare(other)));
            }
        }
    }

    /// The menu of the policy templates: the imports to open in a tab and the import commands.
    fn catalogs_menu(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if self.imports.is_empty() {
            ui.weak(tr("No policy templates imported yet."));
        }
        for f in &self.imports {
            if menu_entry(ui, icons::PACKAGE, &f.name, "", true) {
                acts.push(Act::Catalog(CatalogAct::Open(f.id.clone())));
            }
        }
        ui.separator();
        for (icon, text, source) in [
            (
                icons::THIS_PC,
                tr("Import the templates of this Windows"),
                ImportSource::ThisWindows,
            ),
            (
                icons::FOLDER,
                tr("Import a folder..."),
                ImportSource::Folder,
            ),
            (
                icons::IMPORT,
                tr("Import a catalog file..."),
                ImportSource::CatalogFile,
            ),
            (
                icons::ARCHIVE,
                tr("Import an archive..."),
                ImportSource::Archive,
            ),
        ] {
            if menu_entry(ui, icon, text, "", true) {
                acts.push(Act::Import(ImportAct::Templates(source)));
            }
        }
        if menu_entry(ui, icons::NEW, tr("New catalog"), "", true) {
            acts.push(Act::Catalog(CatalogAct::New));
        }
        ui.separator();
        let settings = tr("Settings of the policy templates");
        if menu_entry(ui, icons::GEAR, settings, "", true) {
            acts.push(Act::Shell(ShellAct::OpenSection(
                SettingsSection::Templates.index(),
            )));
        }
    }
}

/// The Group Policy files: the computer's and the users' of this Windows, or any file.
fn pol_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    use winweaver_core::pol::Scope;
    for (icon, text, act) in [
        (
            icons::THIS_PC,
            tr("Group Policy of the computer"),
            PolAct::OpenSystem(Scope::Computer, None, None),
        ),
        (
            icons::PEOPLE,
            tr("Group Policy of the users"),
            PolAct::OpenSystem(Scope::User, None, None),
        ),
        (
            icons::OPEN,
            tr("Open a Group Policy file..."),
            PolAct::OpenFile,
        ),
    ] {
        if menu_entry(ui, icon, text, "", true) {
            acts.push(Act::Pol(act));
        }
    }
}

/// What of the registry file in front goes to this PC: the selected value, the selected key with
/// its values, the whole file; each asks first.
fn regfile_apply_menu(shell: &Shell<WinWeaver>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    let Some(Doc::RegFile(d)) = shell.docs.get(shell.active) else {
        return;
    };
    use regfileview::ApplyScope;
    for (icon, text, scope) in [
        (
            icons::BULLET,
            tr("The selected value..."),
            ApplyScope::Value,
        ),
        (
            icons::REGISTRY,
            tr("The selected key with its values..."),
            ApplyScope::Key,
        ),
        (icons::PAGE, tr("The whole file..."), ApplyScope::File),
    ] {
        let enabled = d.changes(scope).is_some();
        if menu_entry(ui, icon, text, "", enabled) {
            acts.push(Act::RegFile(RegFileAct::AskApply(scope)));
        }
    }
}

/// The commands of This PC for the selection (offered only when a rule of it can be applied on a
/// PC) and for the whole profile.
fn this_pc_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>, applicable: bool) {
    let mut item = |ui: &mut egui::Ui, icon: &str, text: &str, act: Act| {
        let selection = matches!(
            act,
            Act::Pc(
                PcAct::ReadSelection
                    | PcAct::Check
                    | PcAct::SaveScript(_)
                    | PcAct::SaveRegFile(_)
                    | PcAct::PlanNative(_)
            )
        );
        if menu_entry(ui, icon, text, "", applicable || !selection) {
            acts.push(act);
        }
    };
    item(
        ui,
        icons::THIS_PC,
        tr("Read this PC into a new profile"),
        Act::Pc(PcAct::Read),
    );
    item(
        ui,
        icons::REFRESH,
        tr("Read the selection into this profile"),
        Act::Pc(PcAct::ReadSelection),
    );
    item(
        ui,
        icons::CHECK,
        tr("Check the selection on this PC"),
        Act::Pc(PcAct::Check),
    );
    ui.separator();
    item(
        ui,
        icons::SCRIPT,
        tr("Save an apply script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Apply)),
    );
    item(
        ui,
        icons::SCRIPT,
        tr("Save a return-to-defaults script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Revert)),
    );
    item(
        ui,
        icons::REGISTRY,
        tr("Save a registry file for the selection..."),
        Act::Pc(PcAct::SaveRegFile(false)),
    );
    item(
        ui,
        icons::REGISTRY,
        tr("Save a return-to-defaults registry file for the selection..."),
        Act::Pc(PcAct::SaveRegFile(true)),
    );
    item(
        ui,
        icons::FIND,
        tr("Save an audit script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Audit)),
    );
    ui.separator();
    item(
        ui,
        icons::SCRIPT,
        tr("Save an apply script for the whole profile..."),
        Act::Pc(PcAct::SaveScriptAll(ScriptKind::Apply)),
    );
    item(
        ui,
        icons::SCRIPT,
        tr("Save a return-to-defaults script for the whole profile..."),
        Act::Pc(PcAct::SaveScriptAll(ScriptKind::Revert)),
    );
    item(
        ui,
        icons::REGISTRY,
        tr("Save a registry file for the whole profile..."),
        Act::Pc(PcAct::SaveRegFileAll(false)),
    );
    item(
        ui,
        icons::REGISTRY,
        tr("Save a return-to-defaults registry file for the whole profile..."),
        Act::Pc(PcAct::SaveRegFileAll(true)),
    );
    ui.separator();
    item(
        ui,
        icons::START,
        tr("Apply the selection to this PC..."),
        Act::Pc(PcAct::PlanNative(false)),
    );
    item(
        ui,
        icons::UNDO,
        tr("Return the selection to Windows defaults..."),
        Act::Pc(PcAct::PlanNative(true)),
    );
    item(
        ui,
        icons::UNDO,
        tr("Undo a change from its backup..."),
        Act::Pc(PcAct::PlanUndo),
    );
    ui.separator();
    item(
        ui,
        icons::OPEN,
        tr("Open an audit report..."),
        Act::File(FileAct::OpenReport),
    );
}
