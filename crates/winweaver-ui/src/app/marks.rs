//! The bookmarks: their file, their tab, the way to a bookmark.

use crate::*;

impl App {
    /// The bookmark tab, when it is open.
    pub(crate) fn marks_doc(&mut self) -> Option<&mut marksview::MarksDoc> {
        self.docs.iter_mut().find_map(|d| match d {
            Doc::Bookmarks(m) => Some(&mut **m),
            _ => None,
        })
    }

    /// Brings the bookmark tab to the front, opening it when there is none.
    pub(crate) fn open_marks(&mut self) {
        self.show_single(DocKind::Bookmarks, || Doc::Bookmarks(Box::default()));
    }

    /// Asks before a bookmark is removed, naming its comment when it has one.
    pub(crate) fn ask_remove_mark(&mut self, i: usize) {
        let Some(b) = self.marks.items.get(i) else {
            return;
        };
        let t = marksview::target(
            &self.catalog,
            texts_for(&self.texts, &i18n::current()),
            &self.imports,
            b,
        );
        let text = if b.comment.trim().is_empty() {
            trf("Remove the bookmark of {title}?", &[("title", &t.title)])
        } else {
            trf(
                "The bookmark of {title} has a comment. Remove it with the comment?",
                &[("title", &t.title)],
            )
        };
        self.ask = Some(Ask {
            title: tr("Remove the bookmark").to_owned(),
            text,
            yes: tr("Remove").to_owned(),
            act: Act::RemoveMark(i),
        });
    }

    /// The current position of the active tab as a bookmark, without a comment. The path of an
    /// unsaved document is its tab title, so that the bookmark finds the tab again.
    /// The bookmark of the option selected in the active tab: a profile (with its file as the
    /// link), an imported catalog or a rule of a diff (with the file of its base).
    pub(crate) fn current_bookmark(&self) -> Option<Bookmark> {
        let (item, path) = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => {
                let item = match &doc.sel {
                    Sel::None => return None,
                    Sel::Group(id) if id == UNKNOWN_GROUP => return None,
                    Sel::Group(id) | Sel::Rule(id) | Sel::Unknown(id) => id.clone(),
                    Sel::Form(f) => f.id().to_owned(),
                };
                let path = doc
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                (item, path)
            }
            Some(Doc::Catalog(c)) if c.file.is_none() => (c.sel.clone()?, String::new()),
            Some(Doc::Diff(d)) => {
                let id = d.sel.as_deref()?.strip_prefix("rule:")?;
                (id.to_owned(), d.diff.base.path.clone())
            }
            _ => return None,
        };
        Some(Bookmark::option(&item, &path))
    }

    /// Writes the bookmark file; `false` (and a status message) when it cannot be written.
    pub(crate) fn save_marks(&mut self) -> bool {
        if let Some(parent) = self.marks_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match self.marks.save(&self.marks_path) {
            Ok(()) => true,
            Err(e) => {
                self.status = trf("Bookmarks not saved: {e}", &[("e", &e)]);
                false
            }
        }
    }

    pub(crate) fn open_marks_path(&mut self, path: PathBuf) {
        match Bookmarks::load(&path) {
            Ok(marks) => {
                self.marks = marks;
                self.marks_path = path.clone();
                self.status = trf(
                    "Opened {path}: the star now writes there",
                    &[("path", &path.display())],
                );
                self.remember_recent(&path);
                self.apply(vec![Act::OpenMarks]);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens the document of a bookmark (a file, or the tab of an unsaved document by its
    /// title) and selects its item.
    /// Opens the profile of a bookmark (loading its file when no tab has it) and selects the
    /// option; without a profile file the option is shown in the profile tab in front, else the first one.
    pub(crate) fn go_to_mark(&mut self, i: usize) {
        let Some(mark) = self.marks.items.get(i).cloned() else {
            return;
        };
        if let Some(m) = self.marks_doc() {
            m.sel = Some(i);
        }
        if mark.is_registry() {
            let value = (!mark.item.is_empty()).then_some(mark.item.as_str());
            self.open_registry(&mark.path, value);
            return;
        }
        let path = PathBuf::from(&mark.path);
        if !mark.path.is_empty() && path.is_file() {
            self.open_path(path);
        } else if let Some(j) = (!mark.path.is_empty())
            .then(|| self.docs.iter().position(|d| d.title() == mark.path))
            .flatten()
        {
            self.active = j;
        } else if mark.is_option() {
            let front = match self.docs.get(self.active) {
                Some(Doc::Profile(_)) => Some(self.active),
                _ => self.docs.iter().position(|d| matches!(d, Doc::Profile(_))),
            };
            match front {
                Some(j) => self.active = j,
                None => {
                    self.status =
                        tr("Open a profile to show the option of the bookmark in").to_owned();
                    return;
                }
            }
        } else {
            self.status = trf(
                "The document of the bookmark is not open: {path}",
                &[("path", &mark.path)],
            );
            return;
        }
        if mark.is_option() {
            let mut sel = sel_from_item(&self.catalog, &mark.item);
            if let Some(doc) = self.active_profile() {
                if sel == Sel::None && doc.profile.unknown.contains_key(&mark.item) {
                    sel = Sel::Unknown(mark.item.clone());
                }
                doc.select(sel);
            }
            self.reveal_selection();
        }
    }
}
