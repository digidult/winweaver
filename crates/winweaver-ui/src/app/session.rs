//! The session: the tabs (one of a kind, found or opened, the messages of the status bar), the
//! drafts, the tabs remembered and restored, the log, the title.

use crate::*;

impl App {
    /// Brings the tab of a kind the window holds once to the front, making it when there is
    /// none, and gives its index.
    pub(crate) fn show_single(&mut self, kind: DocKind, make: impl FnOnce() -> Doc) -> usize {
        self.show_or_open(|d| d.kind() == kind, make)
    }

    /// Brings the tab the predicate accepts to the front, making it when there is none, and
    /// gives its index.
    pub(crate) fn show_or_open(
        &mut self,
        find: impl Fn(&Doc) -> bool,
        make: impl FnOnce() -> Doc,
    ) -> usize {
        let i = match self.docs.iter().position(find) {
            Some(i) => i,
            None => {
                self.docs.push(make());
                self.docs.len() - 1
            }
        };
        self.active = i;
        i
    }

    /// Brings the first tab the predicate accepts to the front; `false` when there is none.
    pub(crate) fn focus_tab(&mut self, find: impl Fn(&Doc) -> bool) -> bool {
        match self.docs.iter().position(find) {
            Some(i) => {
                self.active = i;
                true
            }
            None => false,
        }
    }

    /// Opens a document in a new tab in front.
    pub(crate) fn open_tab(&mut self, doc: Doc) {
        self.docs.push(doc);
        self.active = self.docs.len() - 1;
    }

    /// Names a saved file in the status bar, with the button that shows it in its folder.
    pub(crate) fn saved(&mut self, path: &Path) {
        self.status = trf("Saved {path}", &[("path", &path.display())]);
        self.shown_file = Some((self.status.clone(), path.to_path_buf()));
    }

    /// Names an opened file in the status bar and in the recent files.
    pub(crate) fn opened(&mut self, path: &Path) {
        self.status = trf("Opened {path}", &[("path", &path.display())]);
        self.remember_recent(path);
    }

    pub(crate) fn save_failed(&mut self, e: impl std::fmt::Display) {
        self.save_failed(e);
    }

    pub(crate) fn open_failed(&mut self, e: impl std::fmt::Display) {
        self.open_failed(e);
    }

    /// Writes the drafts of the modified profiles: two seconds after the last change, and all of
    /// them when the window closes.
    pub(crate) fn write_drafts(&mut self, all: bool) {
        let mut failed = None;
        for doc in &mut self.docs {
            let Doc::Profile(d) = doc else {
                continue;
            };
            if !d.dirty {
                continue;
            }
            let due = match d.changed {
                Some(t) => all || t.elapsed() >= std::time::Duration::from_secs(2),
                None => false,
            };
            if !due {
                continue;
            }
            match session::write_draft(
                &self.drafts_dir,
                &d.draft_key,
                d.path.as_deref(),
                &d.profile,
            ) {
                Ok(()) => d.changed = None,
                Err(e) => {
                    d.changed = None;
                    failed = Some(e);
                }
            }
        }
        if let Some(e) = failed {
            self.status = trf("Draft not saved: {e}", &[("e", &e)]);
        }
    }

    /// Remembers the files of the open tabs for the next start.
    pub(crate) fn remember_tabs(&mut self) {
        let mut tabs = Vec::new();
        let mut active = 0;
        for (i, doc) in self.docs.iter().enumerate() {
            let path = doc.path().map(Path::to_path_buf);
            if let Some(p) = path {
                if i == self.active {
                    active = tabs.len();
                }
                tabs.push(p);
            }
        }
        if tabs != self.settings.open_tabs || active != self.settings.active_tab {
            self.settings.open_tabs = tabs;
            self.settings.active_tab = active;
            self.settings_dirty = true;
        }
    }

    /// Opens the tabs of the last close and the drafts of unsaved changes.
    pub(crate) fn restore_session(&mut self) {
        let start_tab_untouched =
            matches!(self.docs.first(), Some(Doc::Profile(d)) if !d.dirty && d.path.is_none());
        let before = self.docs.len();
        let mut missing = 0;
        if self.settings.restore_tabs {
            for path in self.settings.open_tabs.clone() {
                if path.exists() {
                    self.open_path(path);
                } else {
                    missing += 1;
                }
            }
        }
        let restored_tabs = self.docs.len() - before;
        let (drafts, broken) = session::read_drafts(&self.drafts_dir);
        let mut restored = 0;
        for (key, draft) in drafts {
            let path = (!draft.path.is_empty()).then(|| PathBuf::from(&draft.path));
            let open = path.as_ref().and_then(|p| {
                self.docs
                    .iter()
                    .position(|d| matches!(d, Doc::Profile(x) if x.path.as_ref() == Some(p)))
            });
            let catalog = &self.catalog;
            let mut doc = match open {
                Some(i) => match self.docs.remove(i) {
                    Doc::Profile(mut d) => {
                        d.profile = draft.profile;
                        d
                    }
                    _ => continue,
                },
                None => Box::new(ProfileDoc::new(
                    draft.profile,
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
            self.docs.push(Doc::Profile(doc));
            restored += 1;
        }
        if start_tab_untouched && (restored_tabs > 0 || restored > 0) {
            self.docs.remove(0);
        }
        if restored_tabs > 0 {
            self.active = self
                .settings
                .active_tab
                .min(self.docs.len().saturating_sub(1));
        }
        if restored > 0 {
            self.active = self.docs.len() - 1;
        }
        let mut notes = Vec::new();
        if restored > 0 {
            notes.push(trf("{n} drafts restored", &[("n", &restored)]));
        }
        if missing > 0 {
            notes.push(trf(
                "{n} files of the last session are missing",
                &[("n", &missing)],
            ));
        }
        if !broken.is_empty() {
            notes.push(trf("{n} drafts could not be read", &[("n", &broken.len())]));
        }
        if !notes.is_empty() {
            self.status = notes.join("; ");
        }
    }

    /// Writes every new status line to the log.
    pub(crate) fn log_status(&mut self) {
        if self.status != self.last_logged {
            session::log(&self.logs_dir, "INFO", &self.status);
            self.last_logged = self.status.clone();
        }
    }

    pub(crate) fn window_title(&self) -> String {
        match self.docs.get(self.active) {
            Some(doc) => format!("{} - WinWeaver", doc.title()),
            None => "WinWeaver".to_owned(),
        }
    }
}
