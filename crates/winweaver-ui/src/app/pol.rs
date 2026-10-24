//! The Group Policy files (`.pol`): the ones of this Windows and any file, in a tab.

use winweaver_core::pol::{PolFile, Scope};

use crate::polview::{self, PolDoc};
use crate::*;

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
                let dialog = rfd::FileDialog::new()
                    .set_title(tr("Open a Group Policy file"))
                    .add_filter(tr("Group Policy files"), &[winweaver_core::pol::EXTENSION]);
                if let Some(path) = dialog.pick_file() {
                    self.open_pol_path(shell, path);
                }
            }
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
