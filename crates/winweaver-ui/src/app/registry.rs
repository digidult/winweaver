//! The registry tab: keys and values, their rights, the search, the registry files.

use crate::*;

impl WinWeaver {
    /// Opens a registry file in a tab of its own, or brings its tab to the front.
    pub(crate) fn open_reg_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::RegFile(r) if r.path == path),
            RegFile::load,
            |file, path| Doc::RegFile(Box::new(regfileview::RegFileDoc::new(file, path))),
        );
    }

    /// Saves the registry file in front at its place, or at one the user picks.
    pub(crate) fn save_reg_doc(&mut self, shell: &mut Shell<WinWeaver>, ask: bool) {
        let Some(Doc::RegFile(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let path = if ask {
            let name = d.title();
            let dialog = crate::dialogs::FileDialog::new()
                .set_title(tr("Save the registry file as"))
                .add_filter(tr("Registry files"), &[regfile::EXTENSION])
                .set_file_name(name);
            let dialog = match d.path.parent() {
                Some(dir) => crate::dialogs::in_folder(dialog, dir),
                None => dialog,
            };
            match dialog.save_file() {
                Some(p) => reg_extension(p),
                None => return,
            }
        } else {
            d.path.clone()
        };
        let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        match d.file.save(&path) {
            Ok(()) => {
                d.path = path.clone();
                d.dirty = false;
                shell.saved(&path);
                shell.remember_recent(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// Opens the window of the rights of the selected key of the registry tab in front: its owner
    /// and its access list with the names of the accounts.
    pub(crate) fn registry_rights(&mut self, shell: &mut Shell<WinWeaver>) {
        use winweaver_core::security::{self, Rights};
        let reg = self.full.registry.clone();
        let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) else {
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
    pub(crate) fn registry_rights_add(&mut self, shell: &mut Shell<WinWeaver>) {
        use winweaver_core::security::{Entry, Scope};
        let reg = self.full.registry.clone();
        let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) else {
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
    pub(crate) fn registry_rights_save(&mut self, shell: &mut Shell<WinWeaver>) {
        let reg = self.full.registry.clone();
        let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        let Some(edit) = &mut d.rights else {
            return;
        };
        let dacl = edit.rights.dacl();
        match reg.set_security(&edit.path, &dacl, edit.rights.protected) {
            Ok(()) => {
                shell.status = trf("The rights of {path} changed", &[("path", &edit.path)]);
                d.rights = None;
            }
            Err(e) => edit.error = Some(e),
        }
    }

    /// Starts the search of the registry tab in front under its selected key, in a thread of its
    /// own; `poll_registry_search` takes the result.
    pub(crate) fn registry_find(&mut self, shell: &mut Shell<WinWeaver>) {
        let reg = self.full.registry.clone();
        let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        let query = d.search.query();
        if d.search.running.is_some() {
            return;
        }
        if query.text.is_empty() || !(query.keys || query.values || query.data) {
            shell.status =
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
            shell.status = trf("The search did not start: {e}", &[("e", &e)]);
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
    pub(crate) fn poll_registry_search(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ctx: &egui::Context,
    ) {
        for doc in &mut shell.docs {
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
                    shell.status =
                        tr("The search of the registry failed; the log names the error").to_owned();
                }
            }
        }
    }

    /// Opens the registry tab, or brings it to the front, at a key and a value of it.
    pub(crate) fn open_registry(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        path: &str,
        value: Option<&str>,
    ) {
        let reg = self.full.registry.clone();
        match shell
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Registry(_)))
        {
            Some(i) => {
                shell.active = i;
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(i) {
                    d.go(&*reg, path, value);
                }
            }
            None => {
                let doc = regview::RegistryDoc::new(&*reg, path, value);
                shell.open_tab(Doc::Registry(Box::new(doc)));
            }
        }
    }

    /// Writes the value or makes the key of the editor of the registry tab; an error stays in
    /// the editor.
    pub(crate) fn registry_save(&mut self, shell: &mut Shell<WinWeaver>) {
        use regview::EditKind;
        let reg = self.full.registry.clone();
        let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) else {
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
        let old = edit.old.clone();
        let result: Result<Option<String>, String> = match kind {
            EditKind::RenameValue => {
                winweaver_core::registry::rename_value(&*reg, &d.path, &old, &name).map(|()| None)
            }
            EditKind::RenameKey => reg.rename_key(&d.path, &name).map(Some),
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
                    Some(path) => {
                        d.refresh(&*reg);
                        d.go(&*reg, &path, None);
                    }
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
    pub(crate) fn ask_registry_delete(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Registry(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let path = d.path.clone();
        let (text, act) = match d.selected() {
            Some(v) => (
                trf(
                    "Delete the value {name} of {key}?",
                    &[("name", &v.name), ("key", &path)],
                ),
                Act::Registry(RegistryAct::Delete(path.clone(), Some(v.name.clone()))),
            ),
            None if winweaver_core::registry::parent(&path).is_none() => {
                shell.status = tr("A hive is not deleted").to_owned();
                return;
            }
            None => (
                trf(
                    "Delete the key {key} with all its subkeys and values?",
                    &[("key", &path)],
                ),
                Act::Registry(RegistryAct::Delete(path.clone(), None)),
            ),
        };
        shell.ask(Question::new(
            tr("Delete from the registry"),
            text,
            tr("Delete"),
            act,
        ));
    }

    pub(crate) fn registry_delete(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        path: &str,
        value: Option<&str>,
    ) {
        let reg = self.full.registry.clone();
        let result = match value {
            Some(v) => reg.delete_value(path, v),
            None => reg.delete_key(path),
        };
        if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
            if value.is_none() && result.is_ok() {
                let up = winweaver_core::registry::parent(path)
                    .unwrap_or(path)
                    .to_owned();
                d.go(&*reg, &up, None);
            } else {
                d.refresh(&*reg);
            }
        }
        shell.status = match result {
            Ok(()) => tr("Deleted").to_owned(),
            Err(e) => e,
        };
    }

    /// Bookmarks the key of the registry tab and its selected value.
    pub(crate) fn bookmark_registry(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Registry(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let mut mark = Bookmark::registry(&d.path, d.value.as_deref().unwrap_or(""));
        if self.full.marks.find(&mark).is_some() {
            shell.status = tr("This is bookmarked already").to_owned();
            return;
        }
        mark.created = clock::now_utc_iso();
        self.full.marks.items.push(mark);
        self.save_marks(shell);
        shell.status = tr("Bookmarked").to_owned();
    }

    /// Runs an act of the acts of the registry tab.
    pub(crate) fn apply_registry(&mut self, shell: &mut Shell<WinWeaver>, act: RegistryAct) {
        match act {
            RegistryAct::Open(path, value) => self.open_registry(shell, &path, value.as_deref()),
            RegistryAct::Go(path, value) => {
                let reg = self.full.registry.clone();
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.go(&*reg, &path, value.as_deref());
                }
            }
            RegistryAct::Expand(path, open) => {
                let reg = self.full.registry.clone();
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.expand(&*reg, &path, open);
                }
            }
            RegistryAct::Refresh => {
                let reg = self.full.registry.clone();
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.refresh(&*reg);
                }
            }
            RegistryAct::Edit => {
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.start_edit(regview::EditKind::Value, "");
                }
            }
            RegistryAct::Rename => {
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    let kind = if d.selected().is_some() {
                        regview::EditKind::RenameValue
                    } else {
                        regview::EditKind::RenameKey
                    };
                    d.start_edit(kind, "");
                }
            }
            RegistryAct::New(kind, type_name) => {
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.start_edit(kind, type_name);
                }
            }
            RegistryAct::Save => self.registry_save(shell),
            RegistryAct::Search => {
                if let Some(Doc::Registry(d)) = shell.docs.get_mut(shell.active) {
                    d.search.open = !d.search.open;
                    d.search.focus = d.search.open;
                }
            }
            RegistryAct::Find => self.registry_find(shell),
            RegistryAct::FindStop => {
                for doc in &mut shell.docs {
                    if let Doc::Registry(d) = doc
                        && let Some(running) = &d.search.running
                    {
                        running
                            .stop
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
            RegistryAct::AskDelete => self.ask_registry_delete(shell),
            RegistryAct::Rights => self.registry_rights(shell),
            RegistryAct::RightsAdd => self.registry_rights_add(shell),
            RegistryAct::RightsSave => self.registry_rights_save(shell),
            RegistryAct::Delete(path, value) => {
                self.registry_delete(shell, &path, value.as_deref())
            }
            RegistryAct::Bookmark => self.bookmark_registry(shell),
        }
    }

    /// Runs an act of the acts of the tab of a registry file.
    /// Applies the selected value, the selected key or the whole registry file in front to the
    /// registry of this PC, after its question; the status says what was done and the first error.
    fn apply_reg_file_to_pc(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        scope: regfileview::ApplyScope,
    ) {
        let changes = match shell.docs.get(shell.active) {
            Some(Doc::RegFile(d)) => d.changes(scope),
            _ => None,
        };
        let Some(changes) = changes else {
            return;
        };
        let (done, errors) = winweaver_core::regfile::apply_changes(&*self.full.registry, &changes);
        shell.status = match errors.first() {
            None => trf("Applied {n} changes to this PC", &[("n", &done)]),
            Some(first) => trf(
                "Applied {done} changes to this PC, {failed} failed: {first}",
                &[("done", &done), ("failed", &errors.len()), ("first", first)],
            ),
        };
        let failed = errors.len();
        shell.log(
            "INFO",
            &format!("registry file applied: {done} changes, {failed} failed"),
        );
        // Every change this PC refused, in the log and in the message bar of the tab: the status
        // names the first one only.
        for e in &errors {
            shell.log("WARN", &format!("registry file: {e}"));
        }
        if let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active) {
            d.applied = errors;
        }
    }

    pub(crate) fn apply_reg_file(&mut self, shell: &mut Shell<WinWeaver>, act: RegFileAct) {
        match act {
            RegFileAct::Edit => {
                if let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active)
                    && let Some(line) = d.target()
                {
                    d.start_edit(line);
                }
            }
            RegFileAct::New(kind) => {
                if let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active) {
                    d.start_new(kind);
                }
            }
            RegFileAct::Remove(lines) => {
                if let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active) {
                    d.remove_lines(&lines);
                }
            }
            RegFileAct::AskApply(scope) => {
                let question = match shell.docs.get(shell.active) {
                    Some(Doc::RegFile(d)) => d.apply_question(scope),
                    _ => None,
                };
                if let Some(question) = question {
                    shell.ask(question);
                }
            }
            RegFileAct::Apply(scope) => self.apply_reg_file_to_pc(shell, scope),
            RegFileAct::ClearMessages => {
                if let Some(Doc::RegFile(d)) = shell.docs.get_mut(shell.active) {
                    d.applied.clear();
                }
            }
            RegFileAct::Outline(tree) => {
                shell.settings.extra.regfile_tree = tree;
                shell.settings_dirty = true;
            }
        }
    }
}
