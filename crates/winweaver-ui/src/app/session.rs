//! The session: one tab of a kind, a draft of the last session restored.

use crate::*;

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
    /// Opens a draft of the last session: the profile into the tab of its file when one is
    /// open, else into a new tab; `false` for a document that is not a profile.
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
