//! The settings of WinWeaver: its lines in the General section of the window, the section Files,
//! the rename of an import.

use crate::ShellAct;
use crate::*;

impl WinWeaver {
    /// The lines of WinWeaver in the card Window of the General section.
    pub(crate) fn general_settings(&self, shell: &mut Shell<WinWeaver>, ui: &mut egui::Ui) {
        let extra = &mut shell.settings.extra;
        ui.checkbox(
            &mut extra.desc_open,
            tr("Show the description panel (Ctrl+D)"),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Open at first in the description:"));
            let parts = &mut extra.desc_sections;
            ui.checkbox(&mut parts.what, tr("What it does"));
            ui.checkbox(&mut parts.risk, tr("Risk"));
            ui.checkbox(&mut parts.writes, tr("What it writes"));
            ui.checkbox(&mut parts.deps, tr("Dependencies and conflicts"));
            ui.checkbox(&mut parts.more, tr("Versions, check and rollback"));
            #[cfg(feature = "full")]
            ui.checkbox(&mut parts.reference, tr("Reference"));
        });
        ui.checkbox(
            &mut extra.show_guide_at_start,
            tr("Open the guide at the next start"),
        );
        ui.checkbox(
            &mut extra.inline_switches,
            tr("Switches at the end of the rows of the selected group"),
        );
        ui.checkbox(
            &mut extra.hide_unknown,
            tr("Hide the options of the profile whose catalog is not loaded"),
        );
        #[cfg(feature = "full")]
        {
            ui.checkbox(
                &mut extra.writers_open,
                tr("Show the rules that write a key of a registry file open"),
            );
            ui.checkbox(
                &mut extra.pol_with_pc,
                tr("Open the Group Policy files of this Windows when this PC is read"),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("A click on a catalog of the menu Catalogs:"));
                let opens = &mut extra.catalog_menu_opens;
                ui.radio_value(opens, false, tr("loads or unloads it"));
                ui.radio_value(opens, true, tr("opens its tab, of the loaded ones"));
            });
        }
        ui.add_space(4.0);
    }

    /// The folders next to the executable and the recent files.
    pub(crate) fn files_settings(&self, shell: &mut Shell<WinWeaver>, ui: &mut egui::Ui) {
        style::section_title(ui, tr("Files"));
        ui.add_space(4.0);
        ui.label(trf(
            "Settings file: {path}",
            &[("path", &shell.settings_path.display())],
        ));
        #[cfg_attr(not(feature = "full"), allow(unused_mut))]
        let mut folders = vec![
            (
                trf(
                    "Profiles folder: {path}",
                    &[("path", &profiles_dir().display())],
                ),
                profiles_dir(),
            ),
            (
                trf(
                    "Output folder: {path}",
                    &[("path", &output_dir().display())],
                ),
                output_dir(),
            ),
        ];
        #[cfg(feature = "full")]
        folders.push((
            trf(
                "Policy templates folder: {path}",
                &[("path", &self.full.imports_dir.display())],
            ),
            self.full.imports_dir.clone(),
        ));
        for (text, path) in folders {
            ui.horizontal_wrapped(|ui| {
                ui.label(text);
                if ui.small_button(tr("Open")).clicked() {
                    shell.pending.push(Act::Shell(ShellAct::ShowInFolder(path)));
                }
            });
        }
        ui.add_space(8.0);
        if !shell.settings.recent.is_empty() && ui.button(tr("Forget recent files")).clicked() {
            let question = shell.forget_recent_act();
            shell.pending.push(question);
        }
    }

    #[cfg(feature = "full")]
    /// Runs an act of the acts of the settings tab.
    pub(crate) fn apply_settings(&mut self, act: SettingsAct) {
        match act {
            SettingsAct::StartRename(id, name) => self.full.renaming = Some((id, name)),
            SettingsAct::CancelRename => self.full.renaming = None,
        }
    }
}
