//! The files of the browser edition with the feature `full` (winweaverweb): every kind of file of
//! WinWeaver opened from the bytes the page hands over. A tab has the name of its file as its
//! path; saved, it is a download of that name (`ww_base::files`).

use crate::*;

impl WinWeaver {
    /// A file the browser hands over by its bytes, by its extension: a diff, bookmarks, a catalog
    /// file, an archive of policy templates (read into a catalog file to save), a users file, a
    /// Group Policy file, a registry file, a Markdown page; else a profile or an answer file.
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
        let text = || jsonfile::decode_bom(&bytes).ok_or_else(|| format!("{name}: not a text"));
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
            e if e == admx::CATALOG_EXTENSION => text().and_then(|t| {
                CatalogFile::parse(&t, name)
                    .map(|f| Doc::Catalog(Box::new(catalogview::CatalogDoc::from_file(f, path))))
            }),
            e if e == admx::ARCHIVE_EXTENSION => admx::read_archive_bytes(&bytes, name).map(|a| {
                let file = match a {
                    admx::Archive::Catalog(file) => *file,
                    admx::Archive::Templates(t) => {
                        let stem = file_stem(name);
                        let id = package_id(&stem.to_lowercase(), &loaded_ids(&self.imports));
                        admx::to_catalog_file(&t, &id, &stem, "", &clock::now_utc_iso())
                    }
                };
                let path = PathBuf::from(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
                Doc::Catalog(Box::new(catalogview::CatalogDoc::from_file(file, path)))
            }),
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

/// The ids of the imports loaded, for the id of a new one.
fn loaded_ids(imports: &[CatalogFile]) -> Vec<String> {
    imports.iter().map(|f| f.id.clone()).collect()
}

/// The kinds of files the dialog of the page offers: every file of WinWeaver.
pub(crate) fn open_filters_full() -> Vec<(String, Vec<String>)> {
    let kind = |name: &str, ext: &[&str]| {
        (
            name.to_owned(),
            ext.iter().map(|e| (*e).to_owned()).collect(),
        )
    };
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
