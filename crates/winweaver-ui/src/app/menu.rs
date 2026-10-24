//! The menu of the tab row and the menus of the command bar (This PC, Compare, Catalogs).

use ww::shell::menu_item;

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
        ui.menu_button(tr("New"), |ui| {
            menu_item(
                ui,
                tr("Empty profile"),
                keys::NEW.text,
                Act::File(FileAct::NewProfile(None)),
                acts,
            );
            ui.separator();
            for (name, _) in PRESETS {
                menu_item(
                    ui,
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
            menu_item(ui, tr("Users file"), "", Act::Users(UsersAct::New), acts);
            menu_item(
                ui,
                tr("Users file of the accounts of this PC"),
                "",
                Act::Users(UsersAct::OfThisPc),
                acts,
            );
        });
        menu_item(
            ui,
            tr("Open..."),
            keys::OPEN.text,
            Act::Shell(ShellAct::Open),
            acts,
        );
        ui.add_enabled_ui(!shell.settings.recent.is_empty(), |ui| {
            ui.menu_button(tr("Open recent"), |ui| {
                for path in &shell.settings.recent {
                    menu_item(
                        ui,
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
            tr("Save"),
            keys::SAVE.text,
            Act::Shell(ShellAct::Save),
            acts,
        );
        menu_item(
            ui,
            tr("Save as..."),
            keys::SAVE_AS.text,
            Act::Shell(ShellAct::SaveAs),
            acts,
        );
        menu_item(
            ui,
            tr("Close tab"),
            keys::CLOSE.text,
            Act::Shell(ShellAct::CloseTab(shell.active, false)),
            acts,
        );
        ui.separator();
        menu_item(
            ui,
            tr("Guide"),
            keys::GUIDE.text,
            Act::File(FileAct::OpenGuide),
            acts,
        );
        if lite() {
            menu_item(ui, tr("License"), "", Act::File(FileAct::OpenLicense), acts);
            menu_item(
                ui,
                tr("Settings"),
                keys::SETTINGS.text,
                Act::Shell(ShellAct::OpenSettings),
                acts,
            );
            ui.separator();
            menu_item(ui, tr("About"), "", Act::Shell(ShellAct::About), acts);
            menu_item(ui, tr("Exit"), "", Act::Shell(ShellAct::Exit), acts);
            return;
        }
        menu_item(
            ui,
            tr("Documentation"),
            "",
            Act::File(FileAct::OpenDocs),
            acts,
        );
        menu_item(ui, tr("License"), "", Act::File(FileAct::OpenLicense), acts);
        menu_item(ui, tr("Bookmarks"), "", Act::Marks(MarksAct::Open), acts);
        menu_item(
            ui,
            tr("Registry"),
            "",
            Act::Registry(RegistryAct::Open("HKEY_LOCAL_MACHINE".to_owned(), None)),
            acts,
        );
        menu_item(
            ui,
            tr("Group Policy of the computer"),
            "",
            Act::Pol(PolAct::OpenSystem(
                winweaver_core::pol::Scope::Computer,
                None,
                None,
            )),
            acts,
        );
        menu_item(ui, tr("MCP server"), "", Act::Mcp(McpAct::Open), acts);
        menu_item(
            ui,
            tr("Settings"),
            keys::SETTINGS.text,
            Act::Shell(ShellAct::OpenSettings),
            acts,
        );
        ui.separator();
        menu_item(ui, tr("About"), "", Act::Shell(ShellAct::About), acts);
        menu_item(ui, tr("Exit"), "", Act::Shell(ShellAct::Exit), acts);
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
        if ui.button(tr("Compare with a file...")).clicked() {
            acts.push(Act::Diff(DiffAct::Compare));
            ui.close();
        }
        ui.separator();
        for (name, _) in PRESETS {
            if ui
                .button(trf("With the preset {name}", &[("name", &name)]))
                .clicked()
            {
                acts.push(Act::Diff(DiffAct::CompareWith(name)));
                ui.close();
            }
        }
        ui.separator();
        let dirty = shell.active_doc().is_some_and(Tab::dirty);
        let unsaved = egui::Button::new(tr("Unsaved changes"));
        if ui.add_enabled(dirty, unsaved).clicked() {
            acts.push(Act::Diff(DiffAct::Unsaved));
            ui.close();
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
            if ui.button(name).clicked() {
                acts.push(Act::Catalog(CatalogAct::Compare(other)));
                ui.close();
            }
        }
    }

    /// The menu of the policy templates: the imports to open in a tab and the import commands.
    fn catalogs_menu(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if self.imports.is_empty() {
            ui.weak(tr("No policy templates imported yet."));
        }
        for f in &self.imports {
            if ui.button(&f.name).clicked() {
                acts.push(Act::Catalog(CatalogAct::Open(f.id.clone())));
                ui.close();
            }
        }
        ui.separator();
        for (text, source) in [
            (
                tr("Import the templates of this Windows"),
                ImportSource::ThisWindows,
            ),
            (tr("Import a folder..."), ImportSource::Folder),
            (tr("Import a catalog file..."), ImportSource::CatalogFile),
            (tr("Import an archive..."), ImportSource::Archive),
        ] {
            if ui.button(text).clicked() {
                acts.push(Act::Import(ImportAct::Templates(source)));
                ui.close();
            }
        }
        if ui.button(tr("New catalog")).clicked() {
            acts.push(Act::Catalog(CatalogAct::New));
            ui.close();
        }
        ui.separator();
        if ui.button(tr("Settings of the policy templates")).clicked() {
            acts.push(Act::Shell(ShellAct::OpenSection(
                SettingsSection::Templates.index(),
            )));
            ui.close();
        }
    }
}

/// The Group Policy files: the computer's and the users' of this Windows, or any file.
fn pol_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    use winweaver_core::pol::Scope;
    for (text, act) in [
        (
            tr("Group Policy of the computer"),
            PolAct::OpenSystem(Scope::Computer, None, None),
        ),
        (
            tr("Group Policy of the users"),
            PolAct::OpenSystem(Scope::User, None, None),
        ),
        (tr("Open a Group Policy file..."), PolAct::OpenFile),
    ] {
        if ui.button(text).clicked() {
            acts.push(Act::Pol(act));
            ui.close();
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
    for (text, scope) in [
        (tr("The selected value..."), ApplyScope::Value),
        (tr("The selected key with its values..."), ApplyScope::Key),
        (tr("The whole file..."), ApplyScope::File),
    ] {
        let enabled = d.changes(scope).is_some();
        if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
            acts.push(Act::RegFile(RegFileAct::AskApply(scope)));
            ui.close();
        }
    }
}

/// The commands of This PC for the selection (offered only when a rule of it can be applied on a
/// PC) and for the whole profile.
fn this_pc_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>, applicable: bool) {
    let mut add = |ui: &mut egui::Ui, text: &str, enabled: bool, act: Act| {
        if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
            acts.push(act);
            ui.close();
        }
    };
    let mut item = |ui: &mut egui::Ui, text: &str, act: Act| {
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
        add(ui, text, applicable || !selection, act);
    };
    item(
        ui,
        tr("Read this PC into a new profile"),
        Act::Pc(PcAct::Read),
    );
    item(
        ui,
        tr("Read the selection into this profile"),
        Act::Pc(PcAct::ReadSelection),
    );
    item(
        ui,
        tr("Check the selection on this PC"),
        Act::Pc(PcAct::Check),
    );
    ui.separator();
    item(
        ui,
        tr("Save an apply script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Apply)),
    );
    item(
        ui,
        tr("Save a return-to-defaults script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Revert)),
    );
    item(
        ui,
        tr("Save a registry file for the selection..."),
        Act::Pc(PcAct::SaveRegFile(false)),
    );
    item(
        ui,
        tr("Save a return-to-defaults registry file for the selection..."),
        Act::Pc(PcAct::SaveRegFile(true)),
    );
    item(
        ui,
        tr("Save an audit script for the selection..."),
        Act::Pc(PcAct::SaveScript(ScriptKind::Audit)),
    );
    ui.separator();
    item(
        ui,
        tr("Save an apply script for the whole profile..."),
        Act::Pc(PcAct::SaveScriptAll(ScriptKind::Apply)),
    );
    item(
        ui,
        tr("Save a return-to-defaults script for the whole profile..."),
        Act::Pc(PcAct::SaveScriptAll(ScriptKind::Revert)),
    );
    item(
        ui,
        tr("Save a registry file for the whole profile..."),
        Act::Pc(PcAct::SaveRegFileAll(false)),
    );
    item(
        ui,
        tr("Save a return-to-defaults registry file for the whole profile..."),
        Act::Pc(PcAct::SaveRegFileAll(true)),
    );
    ui.separator();
    item(
        ui,
        tr("Apply the selection to this PC..."),
        Act::Pc(PcAct::PlanNative(false)),
    );
    item(
        ui,
        tr("Return the selection to Windows defaults..."),
        Act::Pc(PcAct::PlanNative(true)),
    );
    item(
        ui,
        tr("Undo a change from its backup..."),
        Act::Pc(PcAct::PlanUndo),
    );
    ui.separator();
    item(
        ui,
        tr("Open an audit report..."),
        Act::File(FileAct::OpenReport),
    );
}
