//! The Group Policy files (`.pol`): the ones of this Windows and any file, in a tab.

use winweaver_core::pol::{PolFile, Scope};

use crate::polview::{self, PolDoc};
use crate::*;

/// A path that ends with `.pol`.
fn pol_extension(mut path: PathBuf) -> PathBuf {
    let has = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(winweaver_core::pol::EXTENSION));
    if !has {
        path.set_extension(winweaver_core::pol::EXTENSION);
    }
    path
}

impl WinWeaver {
    /// Opens a Group Policy file in a tab, or brings its tab to the front.
    pub(crate) fn open_pol_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Pol(p) if p.pol.path == path),
            PolFile::load,
            |pol, _| Doc::Pol(Box::new(PolDoc::new(pol))),
        );
    }

    /// Opens the Group Policy file of the computer or of the users of this Windows, at a key and
    /// a value; the file is read again, so the tab shows it as it is now.
    pub(crate) fn open_system_pol(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        scope: Scope,
        key: Option<&str>,
        value: Option<&str>,
    ) {
        let Some((_, path)) = winweaver_core::pol::system_files()
            .into_iter()
            .find(|(s, _)| *s == scope)
        else {
            return;
        };
        if !path.is_file() {
            shell.status = trf(
                "{path} is not there: no policy of Group Policy is set for it",
                &[("path", &path.display())],
            );
            return;
        }
        let pol = match PolFile::load(&path) {
            Ok(pol) => pol,
            Err(e) => {
                shell.status = trf("Group Policy file not read: {e}", &[("e", &e)]);
                return;
            }
        };
        polview::reload_system();
        let i = shell.show_or_open(
            |d| matches!(d, Doc::Pol(p) if p.pol.path == path),
            || Doc::Pol(Box::new(PolDoc::new(pol.clone()))),
        );
        if let Some(Doc::Pol(d)) = shell.docs.get_mut(i) {
            d.pol = pol;
            if let Some(key) = key {
                d.select(key, value);
            }
        }
    }

    /// Opens the Group Policy files of this Windows that are there, behind the tab in front.
    pub(crate) fn open_system_pols_behind(&mut self, shell: &mut Shell<WinWeaver>) {
        let front = shell.active;
        for (scope, path) in winweaver_core::pol::system_files() {
            if path.is_file() {
                self.open_system_pol(shell, scope, None, None);
            }
        }
        shell.active = front.min(shell.docs.len().saturating_sub(1));
    }

    /// Saves the Group Policy file in front, or (`ask`) asks where first. A file of Group Policy
    /// of this Windows is never written: Save asks for a place for a copy.
    pub(crate) fn save_pol_doc(&mut self, shell: &mut Shell<WinWeaver>, ask: bool) {
        let Some(Doc::Pol(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let system = d.is_system();
        let path = if ask || system {
            let name = d
                .pol
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Registry.pol".to_owned());
            let dialog = crate::dialogs::FileDialog::new()
                .set_title(tr("Save the Group Policy file as"))
                .add_filter(tr("Group Policy files"), &[winweaver_core::pol::EXTENSION])
                .set_file_name(name);
            let dialog = match d.pol.path.parent().filter(|_| !system) {
                Some(dir) => crate::dialogs::in_folder(dialog, dir),
                None => crate::dialogs::in_folder(dialog, &output_dir()),
            };
            match dialog.save_file() {
                Some(p) => pol_extension(p),
                None => return,
            }
        } else {
            d.pol.path.clone()
        };
        if pol_is_system(&path) {
            shell.status = tr(
                "The files of Group Policy of this Windows are changed with its editor, not here",
            )
            .to_owned();
            return;
        }
        let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        match d.pol.save(&path) {
            Ok(()) => {
                d.pol.path = path.clone();
                d.dirty = false;
                shell.saved(&path);
                shell.remember_recent(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// Adds the entries of a Group Policy file or of a registry file chosen in a dialog to the
    /// file in front: a registry file brings the changes of the hive of the root of the file.
    fn add_to_pol(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(path) = crate::dialogs::FileDialog::new()
            .set_title(tr("Add the entries of a file"))
            .add_filter(
                tr("Group Policy and registry files"),
                &[winweaver_core::pol::EXTENSION, regfile::EXTENSION],
            )
            .pick_file()
        else {
            return;
        };
        let reg = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(regfile::EXTENSION));
        let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        if reg {
            if d.pol.scope.hive().is_none() {
                shell.status =
                    tr("Choose the root of the file first: a registry file names its hives")
                        .to_owned();
                return;
            }
            match RegFile::load(&path) {
                Ok(file) => {
                    let (taken, left) = d.pol.merge_reg(&file);
                    d.dirty |= taken > 0;
                    shell.status = trf(
                        "{taken} changes added, {left} of other hives left out",
                        &[("taken", &taken), ("left", &left)],
                    );
                }
                Err(e) => shell.status = trf("Not read: {e}", &[("e", &e)]),
            }
        } else {
            match PolFile::load(&path) {
                Ok(file) => {
                    let n = d.pol.merge(&file);
                    d.dirty |= n > 0;
                    shell.status = trf("{n} entries added", &[("n", &n)]);
                }
                Err(e) => shell.status = trf("Not read: {e}", &[("e", &e)]),
            }
        }
    }

    /// Saves the Group Policy file in front as a registry file in the hive of its root.
    fn export_pol(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Pol(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let Some(file) = d.pol.to_reg() else {
            shell.status =
                tr("Choose the root of the file first: a registry file names its hives").to_owned();
            return;
        };
        let stem = d
            .pol
            .path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Registry".to_owned());
        let Some(path) = crate::dialogs::FileDialog::new()
            .set_title(tr("Export as a registry file"))
            .add_filter(tr("Registry files"), &[regfile::EXTENSION])
            .set_file_name(format!("{stem}.{}", regfile::EXTENSION))
            .save_file()
        else {
            return;
        };
        match file.save(&path) {
            Ok(()) => {
                shell.saved(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    pub(crate) fn draw_pol(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let texts = texts_for(&self.texts, &i18n::current());
        if let Some(Doc::Pol(doc)) = shell.docs.get_mut(shell.active) {
            polview::pol_view(ui, doc, &self.catalog, texts, acts);
        }
    }

    pub(crate) fn apply_pol(&mut self, shell: &mut Shell<WinWeaver>, act: PolAct) {
        match act {
            PolAct::OpenSystem(scope, key, value) => {
                self.open_system_pol(shell, scope, key.as_deref(), value.as_deref());
            }
            PolAct::OpenFile => {
                let dialog = crate::dialogs::FileDialog::new()
                    .set_title(tr("Open a Group Policy file"))
                    .add_filter(tr("Group Policy files"), &[winweaver_core::pol::EXTENSION]);
                if let Some(path) = dialog.pick_file() {
                    self.open_pol_path(shell, path);
                }
            }
            PolAct::New => {
                let path = output_dir().join("Registry.pol");
                shell.open_tab(Doc::Pol(Box::new(PolDoc::new(PolFile::empty(&path)))));
            }
            PolAct::Edit => {
                if let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) {
                    d.start_edit();
                }
            }
            PolAct::NewEntry(key) => {
                if let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) {
                    d.start_new(key);
                }
            }
            PolAct::AskRemove => {
                let question = match shell.docs.get(shell.active) {
                    Some(Doc::Pol(d)) => d.remove_question(),
                    _ => None,
                };
                if let Some(question) = question {
                    shell.ask(question);
                }
            }
            PolAct::Remove(list) => {
                if let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) {
                    d.remove_entries(&list);
                }
            }
            PolAct::Root(scope) => {
                if let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active)
                    && !d.is_system()
                    && d.pol.scope != scope
                {
                    // The root is not in the file: a choice changes the view, not the file.
                    d.pol.scope = scope;
                }
            }
            PolAct::AddFile => self.add_to_pol(shell),
            PolAct::ExportReg => self.export_pol(shell),
            PolAct::Refresh => {
                let path = match shell.docs.get(shell.active) {
                    Some(Doc::Pol(d)) => d.pol.path.clone(),
                    _ => return,
                };
                match PolFile::load(&path) {
                    Ok(pol) => {
                        if let Some(Doc::Pol(d)) = shell.docs.get_mut(shell.active) {
                            d.pol = pol;
                        }
                        polview::reload_system();
                    }
                    Err(e) => shell.status = trf("Group Policy file not read: {e}", &[("e", &e)]),
                }
            }
        }
    }
}

/// Whether a path is one of the two files of Group Policy of this Windows.
fn pol_is_system(path: &Path) -> bool {
    let text = path.to_string_lossy();
    winweaver_core::pol::system_files()
        .iter()
        .any(|(_, p)| p.to_string_lossy().eq_ignore_ascii_case(&text))
}
