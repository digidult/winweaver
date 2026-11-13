//! The files of the browser edition with the feature `full` (winweaverweb): every kind of file of
//! WinWeaver opened from the bytes the page hands over. A tab has the name of its file as its
//! path; saved, it is a download of that name (`ww_base::files`). A catalog file or an archive of
//! policy templates is imported at once, as WinWeaver loads the catalog files next to it: kept in
//! the storage of the page, its options in the profiles. A file asked for another purpose (the
//! other side of a comparison, the entries added to a Group Policy file, the accounts of a users
//! file) comes by its bytes to a later frame.

use std::cell::RefCell;
use std::rc::Rc;

use super::pol::Added;
use crate::*;

/// What a file the browser hands over is for.
#[derive(Clone, Copy)]
pub(crate) enum Purpose {
    /// Opened in a tab of its kind, as the menu Open does.
    Open,
    /// The other side of a comparison with the profile in front.
    Compare,
    /// Its entries go into the Group Policy file in front.
    AddToPol,
    /// Its accounts go into the profile in front.
    UsersIntoProfile,
}

/// The files that came from the dialog of the page: the purpose, the name and the bytes of each.
type Picked = Rc<RefCell<Vec<(Purpose, String, Vec<u8>)>>>;

/// The files that came from the dialog of the page, with their purposes, and the context that
/// wakes the frame they come to.
#[derive(Default)]
pub(crate) struct WebState {
    picked: Picked,
    ctx: Option<egui::Context>,
}

impl WinWeaver {
    /// Asks the browser for a file with its dialog, filtered by kinds (a name and its extensions
    /// each); it comes by its bytes to a later frame (`take_picked`).
    pub(crate) fn pick_for(
        &self,
        purpose: Purpose,
        title: &str,
        filters: Vec<(String, Vec<String>)>,
    ) {
        let slot = self.full.web.picked.clone();
        let ctx = self.full.web.ctx.clone();
        let title = title.to_owned();
        wasm_bindgen_futures::spawn_local(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(title);
            for (name, extensions) in &filters {
                dialog = dialog.add_filter(name, extensions.as_slice());
            }
            let Some(file) = dialog.pick_file().await else {
                return;
            };
            let bytes = file.read().await;
            slot.borrow_mut().push((purpose, file.file_name(), bytes));
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Runs the files that came since the last frame, each for its purpose.
    pub(crate) fn take_picked(&mut self, shell: &mut Shell<WinWeaver>, ctx: &egui::Context) {
        if self.full.web.ctx.is_none() {
            self.full.web.ctx = Some(ctx.clone());
        }
        let picked = std::mem::take(&mut *self.full.web.picked.borrow_mut());
        for (purpose, name, bytes) in picked {
            match purpose {
                Purpose::Open => self.open_full_bytes(shell, &name, bytes),
                Purpose::Compare => self.compare_with_bytes(shell, &name, &bytes),
                Purpose::AddToPol => {
                    let path = PathBuf::from(&name);
                    let reg = path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case(regfile::EXTENSION));
                    let added = if reg {
                        text_of(&name, &bytes).and_then(|t| RegFile::parse(&t).map(Added::Reg))
                    } else {
                        winweaver_core::pol::PolFile::parse(&bytes, &path).map(Added::Pol)
                    };
                    self.add_entries_to_pol(shell, added);
                }
                Purpose::UsersIntoProfile => {
                    let file = text_of(&name, &bytes)
                        .and_then(|t| UsersFile::parse(&t, &name).map_err(|e| e.to_string()));
                    self.users_file_into_profile(shell, file);
                }
            }
        }
    }

    /// The profile of a file the browser handed over, compared with the profile in front.
    fn compare_with_bytes(&mut self, shell: &mut Shell<WinWeaver>, name: &str, bytes: &[u8]) {
        match text_of(name, bytes).and_then(|t| Profile::parse(&t, name).map_err(|e| e.to_string()))
        {
            Ok(other) => self.compare_active(shell, other, Some(Path::new(name))),
            Err(e) => shell.status = trf("Compare: {e}", &[("e", &e)]),
        }
    }

    /// A file the browser hands over by its bytes, by its extension: a diff, bookmarks, a catalog
    /// file or an archive of policy templates (imported at once), a users file, a Group Policy
    /// file, a registry file, a Markdown page, an audit report; else a profile or an answer file.
    pub(crate) fn open_full_bytes(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        name: &str,
        bytes: Vec<u8>,
    ) {
        let path = PathBuf::from(name);
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let text = || text_of(name, &bytes);
        let opened: Result<Doc, String> = match ext.as_str() {
            e if e == diff::EXTENSION => text().and_then(|t| {
                Diff::parse(&t, name)
                    .map(|d| Doc::Diff(Box::new(DiffDoc::new(d, None))))
                    .map_err(|e| e.to_string())
            }),
            e if e == bookmarks::EXTENSION => {
                match text().and_then(|t| Bookmarks::parse(&t, name).map_err(|e| e.to_string())) {
                    Ok(marks) => {
                        self.full.marks = marks;
                        shell.status = trf("Opened {path}", &[("path", &name)]);
                        self.apply(shell, Act::Marks(MarksAct::Open));
                    }
                    Err(e) => shell.open_failed(e),
                }
                return;
            }
            e if e == admx::CATALOG_EXTENSION => {
                match text().and_then(|t| CatalogFile::parse(&t, name)) {
                    Ok(file) => self.import_catalog_file(shell, file, false),
                    Err(e) => shell.open_failed(e),
                }
                return;
            }
            e if e == admx::ARCHIVE_EXTENSION => {
                let file = admx::read_archive_bytes(&bytes, name).map(|a| match a {
                    admx::Archive::Catalog(file) => *file,
                    admx::Archive::Templates(t) => {
                        let stem = file_stem(name);
                        let id = package_id(&stem.to_lowercase(), &loaded_ids(&self.imports));
                        admx::to_catalog_file(&t, &id, &stem, "", &clock::now_utc_iso())
                    }
                });
                match file {
                    Ok(file) => self.import_catalog_file(shell, file, false),
                    Err(e) => shell.open_failed(e),
                }
                return;
            }
            e if e == users::EXTENSION => text().and_then(|t| {
                UsersFile::parse(&t, name)
                    .map(|f| {
                        let mut doc = usersview::UsersDoc::new(f, Some(path));
                        doc.check();
                        Doc::Users(Box::new(doc))
                    })
                    .map_err(|e| e.to_string())
            }),
            e if e == winweaver_core::pol::EXTENSION => {
                winweaver_core::pol::PolFile::parse(&bytes, &path)
                    .map(|p| Doc::Pol(Box::new(polview::PolDoc::new(p))))
            }
            e if e == regfile::EXTENSION => text().and_then(|t| {
                RegFile::parse(&t)
                    .map(|f| Doc::RegFile(Box::new(regfileview::RegFileDoc::new(f, path))))
            }),
            "md" => text().map(|t| Doc::Markdown(Box::new(MdDoc::new(name.to_owned(), None, t)))),
            "json" => {
                // An audit report of a script run on a PC becomes a profile, as natively.
                if let Ok(report) = text().and_then(|t| Report::parse(&t, name)) {
                    self.open_report_from(shell, report, Path::new(name));
                } else {
                    self.open_any_bytes(shell, name, bytes);
                }
                return;
            }
            _ => {
                self.open_any_bytes(shell, name, bytes);
                return;
            }
        };
        match opened {
            Ok(doc) => {
                shell.open_tab(doc);
                shell.status = trf("Opened {path}", &[("path", &name)]);
            }
            Err(e) => shell.open_failed(e),
        }
    }
}

/// The text of the bytes of a file, without a byte order mark.
fn text_of(name: &str, bytes: &[u8]) -> Result<String, String> {
    jsonfile::decode_bom(bytes).ok_or_else(|| format!("{name}: not a text"))
}

/// The ids of the imports loaded, for the id of a new one.
fn loaded_ids(imports: &[CatalogFile]) -> Vec<String> {
    imports.iter().map(|f| f.id.clone()).collect()
}

/// The kinds of files the dialog of the page offers: every file of WinWeaver.
pub(crate) fn open_filters_full() -> Vec<(String, Vec<String>)> {
    vec![
        kind(
            tr("WinWeaver files"),
            &[
                profile::EXTENSION,
                diff::EXTENSION,
                bookmarks::EXTENSION,
                admx::CATALOG_EXTENSION,
                users::EXTENSION,
            ],
        ),
        kind(
            tr("Archives of policy templates"),
            &[admx::ARCHIVE_EXTENSION],
        ),
        kind(tr("Registry files"), &[regfile::EXTENSION]),
        kind(tr("Group Policy files"), &[winweaver_core::pol::EXTENSION]),
        kind(tr("Answer files (the profile they embed)"), &["xml"]),
        kind(tr("Older profiles and audit reports"), &["json"]),
        kind(tr("Markdown pages"), &["md"]),
    ]
}

/// A kind of files of the dialog of the page: its name and its extensions.
pub(crate) fn kind(name: &str, extensions: &[&str]) -> (String, Vec<String>) {
    (
        name.to_owned(),
        extensions.iter().map(|e| (*e).to_owned()).collect(),
    )
}
