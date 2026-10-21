//! The command bar of the active tab, its menus (This PC, Compare, Catalogs) and the menu of the policy templates.

use super::*;

impl App {
    /// The command bar of the active tab; `compact` draws the icons alone.
    pub(crate) fn toolbar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        for item in self.commands() {
            match item {
                Item::Separator => {
                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);
                }
                Item::Command(c) => {
                    let hint = match c.keys {
                        Some(k) => format!("{} ({})", c.label, k.text),
                        None => c.label.to_owned(),
                    };
                    let enabled = c.act.is_some();
                    let button =
                        style::tool_button(ui, c.icon, c.label, &hint, enabled, compact, false);
                    if button.clicked()
                        && let Some(act) = c.act
                    {
                        acts.push(act);
                    }
                }
                Item::Menu(menu) => {
                    let label = menu.label();
                    let button =
                        style::tool_button(ui, menu.icon(), label, label, true, compact, true);
                    egui::Popup::menu(&button).show(|ui| {
                        ui.set_min_width(menu.width());
                        match menu {
                            Menu::ThisPc => this_pc_menu(ui, acts),
                            Menu::CompareProfile => self.compare_profile_menu(ui, acts),
                            Menu::Catalogs => self.catalogs_menu(ui, acts),
                            Menu::CompareCatalog => self.compare_catalog_menu(ui, acts),
                        }
                    });
                }
            }
        }
    }

    /// The menu Compare of a profile tab: a file, a preset, the unsaved changes.
    fn compare_profile_menu(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
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
        let dirty = self.docs.get(self.active).is_some_and(Doc::dirty);
        let unsaved = egui::Button::new(tr("Unsaved changes"));
        if ui.add_enabled(dirty, unsaved).clicked() {
            acts.push(Act::Diff(DiffAct::Unsaved));
            ui.close();
        }
    }

    /// The menu Compare of a catalog tab: the loaded imports and the tabs of catalog files to
    /// compare the catalog of the tab with.
    fn compare_catalog_menu(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let own = match self.docs.get(self.active) {
            Some(Doc::Catalog(c)) if c.file.is_none() => Some(c.id.clone()),
            _ => None,
        };
        let mut others: Vec<(String, crate::CatalogRef)> = self
            .imports
            .iter()
            .filter(|f| Some(&f.id) != own.as_ref())
            .map(|f| (f.name.clone(), crate::CatalogRef::Import(f.id.clone())))
            .collect();
        for (i, d) in self.docs.iter().enumerate() {
            if i != self.active
                && let Doc::Catalog(c) = d
                && let Some(f) = &c.file
            {
                others.push((f.name.clone(), crate::CatalogRef::Tab(i)));
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
                crate::ImportSource::ThisWindows,
            ),
            (tr("Import a folder..."), crate::ImportSource::Folder),
            (
                tr("Import a catalog file..."),
                crate::ImportSource::CatalogFile,
            ),
            (tr("Import an archive..."), crate::ImportSource::Archive),
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
            acts.push(Act::Settings(SettingsAct::OpenSection(
                SettingsSection::Templates,
            )));
            ui.close();
        }
    }
}

/// The commands of This PC for the selection.
pub(crate) fn this_pc_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    let mut item = |ui: &mut egui::Ui, text: &str, act: Act| {
        if ui.button(text).clicked() {
            acts.push(act);
            ui.close();
        }
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
