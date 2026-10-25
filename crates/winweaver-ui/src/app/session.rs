//! The session: one tab of a kind, a draft of the last session restored.

use crate::*;

#[cfg(feature = "full")]
/// Brings the tab of a kind the window holds once to the front, making it when there is none,
/// and gives its index.
pub(crate) fn show_single(
    shell: &mut Shell<WinWeaver>,
    kind: DocKind,
    make: impl FnOnce() -> Doc,
) -> usize {
    shell.show_or_open(|d| d.kind() == kind, make)
}

impl WinWeaver {
    #[cfg(feature = "full")]
    /// Opens a draft of the last session: the profile into the tab of its file when one is
    /// open, else into a new tab; `false` for a document that is not a profile.
    /// A draft of a registry file, a diff, a catalog in its editor or a users file: into the tab
    /// of its file when it is open, else a new tab, with its changes unsaved.
    pub(crate) fn restore_other_draft(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        kind: &str,
        key: String,
        path: Option<PathBuf>,
        document: &serde_json::Value,
    ) -> bool {
        let text = document
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let draft = ww::DraftState::with_key(key);
        let same_path = |d: &Doc| path.is_some() && d.path() == path.as_deref();
        let restored = match kind {
            doc::REGFILE_DRAFT => {
                let (Ok(file), Some(path)) = (RegFile::parse(text), path.clone()) else {
                    return false;
                };
                let mut d = regfileview::RegFileDoc::new(file, path);
                d.dirty = true;
                d.draft = draft;
                Doc::RegFile(Box::new(d))
            }
            doc::DIFF_DRAFT => {
                let Ok(diff) = Diff::parse(text, "draft") else {
                    return false;
                };
                let mut d = DiffDoc::new(diff, path.clone().filter(|p| p.exists()));
                d.dirty = true;
                d.draft = draft;
                Doc::Diff(Box::new(d))
            }
            doc::CATALOG_DRAFT => {
                let Ok(file) = serde_json::from_str::<CatalogFile>(text) else {
                    return false;
                };
                if file.check().is_err() {
                    return false;
                }
                let Some(path) = path.clone() else {
                    return false;
                };
                let import = document
                    .get("import")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let mut d = cataledit::EditorDoc::new(file, path, import);
                d.dirty = true;
                d.draft = draft;
                Doc::CatalogEdit(Box::new(d))
            }
            doc::POL_DRAFT => {
                let Some(path) = path.clone() else {
                    return false;
                };
                let bytes = doc::bytes_of_hex(text);
                let Some(Ok(mut pol)) =
                    bytes.map(|b| winweaver_core::pol::PolFile::parse(&b, &path))
                else {
                    return false;
                };
                let scope = document
                    .get("scope")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                pol.scope = winweaver_core::pol::Scope::from_name(scope);
                let mut d = polview::PolDoc::new(pol);
                d.dirty = true;
                d.draft = draft;
                Doc::Pol(Box::new(d))
            }
            doc::USERS_DRAFT => {
                let Ok(file) = UsersFile::parse(text, "draft") else {
                    return false;
                };
                let mut d = usersview::UsersDoc::new(file, path.clone().filter(|p| p.exists()));
                d.dirty = true;
                d.draft = draft;
                Doc::Users(Box::new(d))
            }
            _ => return false,
        };
        match shell.docs.iter().position(same_path) {
            Some(i) => shell.docs[i] = restored,
            None => shell.docs.push(restored),
        }
        true
    }

    pub(crate) fn restore_profile_draft(
        &self,
        shell: &mut Shell<WinWeaver>,
        key: String,
        path: Option<PathBuf>,
        document: serde_json::Value,
    ) -> bool {
        let Ok(profile) = serde_json::from_value::<Profile>(document) else {
            return false;
        };
        let open = path.as_ref().and_then(|p| {
            shell
                .docs
                .iter()
                .position(|d| matches!(d, Doc::Profile(x) if x.path.as_ref() == Some(p)))
        });
        let catalog = &self.catalog;
        let mut doc = match open {
            Some(i) => match shell.docs.remove(i) {
                Doc::Profile(mut d) => {
                    d.profile = profile;
                    d
                }
                _ => return false,
            },
            None => Box::new(ProfileDoc::new(
                profile,
                path.filter(|p| p.exists()),
                catalog,
            )),
        };
        doc.refresh(catalog);
        doc.changed = None;
        doc.draft_key = key;
        doc.issues.push(Issue {
            severity: Severity::Info,
            target: Target::Profile,
            text: tr("The unsaved changes of the last session were restored from a draft")
                .to_owned(),
        });
        shell.docs.push(Doc::Profile(doc));
        true
    }
}
