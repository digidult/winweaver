//! The registry tab: keys and values, their rights, the search, the registry files.

use crate::*;

impl App {
    /// Opens a registry file in a tab of its own, or brings its tab to the front.
    pub(crate) fn open_reg_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::RegFile(r) if r.path == path))
        {
            self.active = i;
            return;
        }
        match RegFile::load(&path) {
            Ok(file) => {
                let doc = regfileview::RegFileDoc::new(file, path.clone());
                self.docs.push(Doc::RegFile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Saves the registry file in front at its place, or at one the user picks.
    pub(crate) fn save_reg_doc(&mut self, ask: bool) {
        let Some(Doc::RegFile(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = if ask {
            let name = d.title();
            let dialog = rfd::FileDialog::new()
                .set_title(tr("Save the registry file as"))
                .add_filter(tr("Registry files"), &[regfile::EXTENSION])
                .set_file_name(name);
            let dialog = match d.path.parent() {
                Some(dir) if dir.is_dir() => dialog.set_directory(dir),
                _ => dialog,
            };
            match dialog.save_file() {
                Some(p) => reg_extension(p),
                None => return,
            }
        } else {
            d.path.clone()
        };
        let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.file.save(&path) {
            Ok(()) => {
                d.path = path.clone();
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Opens the window of the rights of the selected key of the registry tab in front: its owner
    /// and its access list with the names of the accounts.
    pub(crate) fn registry_rights(&mut self) {
        use winweaver_core::security::{self, Rights};
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let path = d.path.clone();
        let rights = match reg.security(&path).and_then(|s| Rights::parse(&s)) {
            Ok(rights) => rights,
            Err(e) => {
                d.error = Some(e);
                return;
            }
        };
        let mut names = std::collections::HashMap::new();
        let owner = std::iter::once(&rights.owner);
        for sid in owner.chain(rights.entries.iter().map(|e| &e.sid)) {
            if sid.is_empty() || names.contains_key(sid) {
                continue;
            }
            let name = reg
                .account_name(sid)
                .or_else(|| security::alias_title(sid).map(str::to_owned))
                .unwrap_or_else(|| sid.clone());
            names.insert(sid.clone(), name);
        }
        d.rights = Some(regview::RightsEdit {
            path,
            rights,
            names,
            add: String::new(),
            error: None,
            changed: false,
        });
    }

    /// Adds an entry that lets the account typed in the window of the rights read the key.
    pub(crate) fn registry_rights_add(&mut self) {
        use winweaver_core::security::{Entry, Scope};
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &mut d.rights else {
            return;
        };
        let name = edit.add.trim().to_owned();
        if name.is_empty() {
            return;
        }
        match reg.account_sid(&name) {
            Some(sid) => {
                let shown = reg.account_name(&sid).unwrap_or(name);
                edit.names.insert(sid.clone(), shown);
                let entry = Entry::new(&sid, false, false, Scope::KeyAndSubkeys);
                edit.rights.entries.push(entry);
                edit.add.clear();
                edit.error = None;
                edit.changed = true;
            }
            None => {
                let text = trf("No account of this PC: {name}", &[("name", &name)]);
                edit.error = Some(text);
            }
        }
    }

    /// Writes the access list of the window of the rights to its key.
    pub(crate) fn registry_rights_save(&mut self) {
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &mut d.rights else {
            return;
        };
        let dacl = edit.rights.dacl();
        match reg.set_security(&edit.path, &dacl, edit.rights.protected) {
            Ok(()) => {
                self.status = trf("The rights of {path} changed", &[("path", &edit.path)]);
                d.rights = None;
            }
            Err(e) => edit.error = Some(e),
        }
    }

    /// Starts the search of the registry tab in front under its selected key, in a thread of its
    /// own; `poll_registry_search` takes the result.
    pub(crate) fn registry_find(&mut self) {
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let query = d.search.query();
        if d.search.running.is_some() {
            return;
        }
        if query.text.is_empty() || !(query.keys || query.values || query.data) {
            self.status =
                tr("Search the registry: type a text and choose what to look at").to_owned();
            return;
        }
        let root = d.path.clone();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let read = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let (thread_stop, thread_read, thread_root) = (stop.clone(), read.clone(), root.clone());
        let started = std::thread::Builder::new()
            .name("registry-search".to_owned())
            .spawn(move || {
                let limit = regview::SEARCH_LIMIT;
                let found = winweaver_core::registry::search(
                    &*reg,
                    &thread_root,
                    &query,
                    limit,
                    usize::MAX,
                    &thread_stop,
                    &thread_read,
                );
                let _ = tx.send(found);
            });
        if let Err(e) = started {
            self.status = trf("The search did not start: {e}", &[("e", &e)]);
            return;
        }
        d.search.root = root;
        d.search.found = None;
        d.search.running = Some(regview::Running {
            stop,
            read,
            result: rx,
        });
    }

    /// Takes the result of a finished search of a registry tab.
    pub(crate) fn poll_registry_search(&mut self, ctx: &egui::Context) {
        for doc in &mut self.docs {
            let Doc::Registry(d) = doc else {
                continue;
            };
            let Some(running) = &d.search.running else {
                continue;
            };
            match running.result.try_recv() {
                Ok(found) => {
                    d.search.running = None;
                    d.search.found = Some(found);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    d.search.running = None;
                    self.status =
                        tr("The search of the registry failed; the log names the error").to_owned();
                }
            }
        }
    }

    /// Opens the registry tab, or brings it to the front, at a key and a value of it.
    pub(crate) fn open_registry(&mut self, path: &str, value: Option<&str>) {
        let reg = self.registry.clone();
        match self.docs.iter().position(|d| matches!(d, Doc::Registry(_))) {
            Some(i) => {
                self.active = i;
                if let Some(Doc::Registry(d)) = self.docs.get_mut(i) {
                    d.go(&*reg, path, value);
                }
            }
            None => {
                let doc = regview::RegistryDoc::new(&*reg, path, value);
                self.docs.push(Doc::Registry(Box::new(doc)));
                self.active = self.docs.len() - 1;
            }
        }
    }

    /// Writes the value or makes the key of the editor of the registry tab; an error stays in
    /// the editor.
    pub(crate) fn registry_save(&mut self) {
        use regview::EditKind;
        let reg = self.registry.clone();
        let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(edit) = &d.edit else {
            return;
        };
        let kind = edit.kind;
        let name = if kind == EditKind::Value {
            edit.name.clone()
        } else {
            edit.name.trim().to_owned()
        };
        let (type_name, text) = (edit.type_name.clone(), edit.text.clone());
        let taken = d
            .key
            .as_ref()
            .is_some_and(|k| k.values.iter().any(|v| v.name.eq_ignore_ascii_case(&name)));
        let result: Result<Option<String>, String> = match kind {
            EditKind::NewKey => {
                if name.is_empty() || name.contains('\\') {
                    Err(tr("A name without backslashes is needed").to_owned())
                } else {
                    let path = format!("{}\\{name}", d.path);
                    reg.create_key(&path).map(|()| Some(path))
                }
            }
            EditKind::NewValue if taken => {
                Err(tr("A value of this name is there already").to_owned())
            }
            _ => winweaver_core::registry::Data::parse(&type_name, &text)
                .and_then(|data| reg.set(&d.path, &name, &data).map(|()| None)),
        };
        match result {
            Ok(key) => {
                d.edit = None;
                match key {
                    Some(path) => d.go(&*reg, &path, None),
                    None => {
                        d.value = Some(name);
                        d.refresh(&*reg);
                    }
                }
            }
            Err(e) => {
                if let Some(edit) = &mut d.edit {
                    edit.error = Some(e);
                }
            }
        }
    }

    /// Asks before the selected value of the registry tab, or its key without one, is deleted.
    pub(crate) fn ask_registry_delete(&mut self) {
        let Some(Doc::Registry(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = d.path.clone();
        let (text, act) = match d.selected() {
            Some(v) => (
                trf(
                    "Delete the value {name} of {key}?",
                    &[("name", &v.name), ("key", &path)],
                ),
                Act::RegistryDelete(path.clone(), Some(v.name.clone())),
            ),
            None if winweaver_core::registry::parent(&path).is_none() => {
                self.status = tr("A hive is not deleted").to_owned();
                return;
            }
            None => (
                trf(
                    "Delete the key {key} with all its subkeys and values?",
                    &[("key", &path)],
                ),
                Act::RegistryDelete(path.clone(), None),
            ),
        };
        self.ask = Some(Ask {
            title: tr("Delete from the registry").to_owned(),
            text,
            yes: tr("Delete").to_owned(),
            act,
        });
    }

    pub(crate) fn registry_delete(&mut self, path: &str, value: Option<&str>) {
        let reg = self.registry.clone();
        let result = match value {
            Some(v) => reg.delete_value(path, v),
            None => reg.delete_key(path),
        };
        if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
            if value.is_none() && result.is_ok() {
                let up = winweaver_core::registry::parent(path)
                    .unwrap_or(path)
                    .to_owned();
                d.go(&*reg, &up, None);
            } else {
                d.refresh(&*reg);
            }
        }
        self.status = match result {
            Ok(()) => tr("Deleted").to_owned(),
            Err(e) => e,
        };
    }

    /// Bookmarks the key of the registry tab and its selected value.
    pub(crate) fn bookmark_registry(&mut self) {
        let Some(Doc::Registry(d)) = self.docs.get(self.active) else {
            return;
        };
        let mut mark = Bookmark::registry(&d.path, d.value.as_deref().unwrap_or(""));
        if self.marks.find(&mark).is_some() {
            self.status = tr("This is bookmarked already").to_owned();
            return;
        }
        mark.created = clock::now_utc_iso();
        self.marks.items.push(mark);
        self.save_marks();
        self.status = tr("Bookmarked").to_owned();
    }
}
