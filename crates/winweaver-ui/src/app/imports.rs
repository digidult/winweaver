//! The policy templates: the imports, their catalog files, the editor of a catalog.

use crate::*;

impl WinWeaver {
    /// Opens a catalog file in a tab of its own, without importing it.
    pub(crate) fn open_catalog_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Catalog(c) if c.path.as_ref() == Some(&path)),
            CatalogFile::load,
            |file, path| Doc::Catalog(Box::new(catalogview::CatalogDoc::from_file(file, path))),
        );
    }

    /// Opens an ADMX file as a catalog tab without importing it: its policies with the texts of
    /// the ADML next to it; the command bar of the tab offers Import.
    pub(crate) fn open_admx_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        if shell.focus_tab(|d| matches!(d, Doc::Catalog(c) if c.path.as_ref() == Some(&path))) {
            return;
        }
        let file = match admx_file_catalog(&path) {
            Ok(f) => f,
            Err(why) => {
                shell.open_failed(why);
                return;
            }
        };
        let doc = catalogview::CatalogDoc::from_file(file, path.clone());
        shell.open_tab(Doc::Catalog(Box::new(doc)));
        shell.opened(&path);
    }

    /// Imports the catalog file of the active tab: it is saved next to the executable, merged
    /// into the catalog, and the tab shows the import from now on.
    pub(crate) fn import_open_catalog(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Catalog(c)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        let Some(mut file) = c.file.take() else {
            return;
        };
        if self.imports.iter().any(|f| f.id == file.id) && !file.id.starts_with("package-") {
            file.id = format!("package-{}", file.id);
        }
        c.id = file.id.clone();
        c.path = None;
        let saved = save_import(&self.full.imports_dir, &file);
        let rules = file.rules.len();
        let id = file.id.clone();
        self.imports.retain(|f| f.id != id);
        self.imports.push(*file);
        self.imports.sort_by_key(|f| import_order(&f.id));
        self.mark_loaded(shell, &id);
        self.refresh_import_list();
        self.rebuild_catalog(shell);
        shell.status = match saved {
            Ok(_) => trf("Imported: {rules} rules", &[("rules", &rules)]),
            Err(e) => trf("Imported, but not saved: {e}", &[("e", &e)]),
        };
    }

    /// Reads the catalog files of the imports the settings load, and no other file; the
    /// templates of this Windows first, so that they win a policy two imports hold. Settings of
    /// an earlier version load every import but the hidden ones, and name them from now on.
    pub(crate) fn load_imports(&mut self, shell: &mut Shell<WinWeaver>) {
        let ids = loaded_import_ids(&shell.settings, &self.full.imports_dir);
        if shell.settings.extra.loaded_imports.is_none() {
            shell.settings.extra.loaded_imports = Some(ids.clone());
            shell.settings.extra.hidden_imports.clear();
            shell.settings_dirty = true;
        }
        let (files, broken) = read_imports(&self.full.imports_dir, &ids);
        self.imports = files;
        if !broken.is_empty() {
            shell.status = trf(
                "{n} imports of policy templates could not be read",
                &[("n", &broken.len())],
            );
        }
        self.refresh_import_list();
        self.rebuild_catalog(shell);
    }

    /// Lists the catalog files of the imports folder for the settings; of a file that is not
    /// loaded only the header is read.
    pub(crate) fn refresh_import_list(&mut self) {
        self.full.import_list = import_entries(&self.full.imports_dir, &self.imports);
    }

    /// Reads the catalog file of an import, unless it is loaded, and marks it in the settings.
    pub(crate) fn load_import(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        id: &str,
    ) -> Result<(), String> {
        if !self.imports.iter().any(|f| f.id == id) {
            let path = self
                .full
                .imports_dir
                .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
            let mut file = CatalogFile::load(&path)?;
            file.id = id.to_owned();
            self.imports.push(file);
            self.imports.sort_by_key(|f| import_order(&f.id));
        }
        self.mark_loaded(shell, id);
        Ok(())
    }

    /// Names an import among the loaded ones of the settings.
    pub(crate) fn mark_loaded(&mut self, shell: &mut Shell<WinWeaver>, id: &str) {
        let list = shell
            .settings
            .extra
            .loaded_imports
            .get_or_insert_with(Vec::new);
        if !list.iter().any(|x| x == id) {
            list.push(id.to_owned());
            shell.settings_dirty = true;
        }
    }

    /// Forgets an import: out of the settings, out of memory, and its tab closes.
    pub(crate) fn unload_import(&mut self, shell: &mut Shell<WinWeaver>, id: &str) {
        self.imports.retain(|f| f.id != id);
        if let Some(list) = &mut shell.settings.extra.loaded_imports {
            list.retain(|x| x != id);
        }
        shell.settings_dirty = true;
        let active = shell.active;
        let mut index = 0;
        let mut removed_before = 0;
        shell.docs.retain(|d| {
            let keep = !matches!(d, Doc::Catalog(c) if c.file.is_none() && c.id == id);
            if !keep && index < active {
                removed_before += 1;
            }
            index += 1;
            keep
        });
        shell.active = active
            .saturating_sub(removed_before)
            .min(shell.docs.len().saturating_sub(1));
    }

    /// Looks for the options of the active profile whose catalog is not loaded in the catalog
    /// files of the imports that are not loaded, reading only the ids of their rules, and loads
    /// the files that have some of them: the templates of this Windows first, no file more than
    /// needed.
    pub(crate) fn load_catalogs_for_unknown(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let mut wanted: HashSet<String> = doc.profile.unknown.keys().cloned().collect();
        if wanted.is_empty() {
            return;
        }
        let mut found = Vec::new();
        for (id, path) in import_files(&self.full.imports_dir) {
            if wanted.is_empty() {
                break;
            }
            if self.imports.iter().any(|f| f.id == id) {
                continue;
            }
            let Ok(ids) = admx::rule_ids(&path) else {
                continue;
            };
            let before = wanted.len();
            wanted.retain(|w| !ids.contains(w));
            if wanted.len() < before {
                found.push(id);
            }
        }
        if found.is_empty() {
            shell.status = tr("No imported catalog has these options: import their policy templates in the settings").to_owned();
            return;
        }
        let mut failed = Vec::new();
        for id in &found {
            if let Err(e) = self.load_import(shell, id) {
                failed.push(e);
            }
        }
        self.refresh_import_list();
        self.rebuild_catalog(shell);
        self.check_active(shell);
        let left = match shell.docs.get(shell.active) {
            Some(Doc::Profile(d)) => d.profile.unknown.len(),
            _ => 0,
        };
        shell.status = match failed.first() {
            Some(e) => trf("A catalog was not loaded: {e}", &[("e", e)]),
            None => trf(
                "Loaded {n} catalogs; {left} options are still unknown",
                &[("n", &(found.len() - failed.len())), ("left", &left)],
            ),
        };
    }

    /// Merges the loaded imports into the catalog and brings the choices of the open profiles in
    /// line with it.
    pub(crate) fn rebuild_catalog(&mut self, shell: &mut Shell<WinWeaver>) {
        match admx::merge(&self.full.base_catalog, &self.imports) {
            Ok((catalog, notes)) => {
                self.catalog = catalog;
                if !notes.is_empty() {
                    shell.status = notes.join("; ");
                }
            }
            Err(e) => {
                shell.status = trf("The imports were not merged: {e}", &[("e", &e)]);
                self.catalog = self.full.base_catalog.clone();
            }
        }
        let mut texts = self.full.base_texts.clone();
        admx::merge_texts(&mut texts, &self.imports);
        self.texts = texts;
        let catalog = &self.catalog;
        for doc in &mut shell.docs {
            if let Doc::Profile(d) = doc {
                d.profile.take_known(catalog);
                d.profile.split_unknown(catalog);
                d.resolution = resolver::resolve(catalog, &d.profile);
                // The selection follows an option between the catalog and the unknown ones.
                d.sel = match std::mem::replace(&mut d.sel, Sel::None) {
                    Sel::Unknown(id) if catalog.rule(&id).is_some() => Sel::Rule(id),
                    Sel::Rule(id) if catalog.rule(&id).is_none() => {
                        if d.profile.unknown.contains_key(&id) {
                            Sel::Unknown(id)
                        } else {
                            Sel::None
                        }
                    }
                    other => other,
                };
            }
        }
    }

    /// Starts an import in a thread: the templates of this Windows, a folder or a catalog file.
    pub(crate) fn start_import(&mut self, shell: &mut Shell<WinWeaver>, source: ImportSource) {
        if self.full.importing.is_some() {
            return;
        }
        let path = match source {
            ImportSource::ThisWindows => None,
            ImportSource::Folder => match rfd::FileDialog::new()
                .set_title(tr("Choose the folder of the ADMX files"))
                .pick_folder()
            {
                Some(p) => Some(p),
                None => return,
            },
            ImportSource::CatalogFile => match rfd::FileDialog::new()
                .set_title(tr("Open a catalog file"))
                .add_filter(tr("Catalog files"), &[admx::CATALOG_EXTENSION])
                .pick_file()
            {
                Some(p) => Some(p),
                None => return,
            },
            ImportSource::Archive => match rfd::FileDialog::new()
                .set_title(tr("Open an archive of templates or of a catalog file"))
                .add_filter(tr("Zip archives"), &[admx::ARCHIVE_EXTENSION])
                .pick_file()
            {
                Some(p) => Some(p),
                None => return,
            },
        };
        let build = windows_build();
        let taken: Vec<String> = self.imports.iter().map(|f| f.id.clone()).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(import_source(source, path.as_deref(), &build, &taken));
        });
        self.full.importing = Some(rx);
        shell.status = tr("The policy templates are being read").to_owned();
    }

    /// Takes the result of a finished import: saved next to the executable, merged.
    pub(crate) fn poll_import(&mut self, shell: &mut Shell<WinWeaver>, ctx: &egui::Context) {
        let Some(rx) = &self.full.importing else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.full.importing = None;
                match result {
                    Ok(file) => {
                        let saved = save_import(&self.full.imports_dir, &file);
                        let rules = file.rules.len();
                        let skipped = file.skipped.len();
                        let id = file.id.clone();
                        self.imports.retain(|f| f.id != id);
                        self.imports.push(file);
                        self.imports.sort_by_key(|f| import_order(&f.id));
                        self.mark_loaded(shell, &id);
                        self.refresh_import_list();
                        self.rebuild_catalog(shell);
                        shell.status = match saved {
                            Ok(_) => trf(
                                "Imported: {rules} rules, {skipped} policies skipped",
                                &[("rules", &rules), ("skipped", &skipped)],
                            ),
                            Err(e) => trf("Imported, but not saved: {e}", &[("e", &e)]),
                        };
                    }
                    Err(e) => shell.status = trf("The import failed: {e}", &[("e", &e)]),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(250))
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.full.importing = None,
        }
    }

    /// Opens the tab of an import, or brings it to the front.
    pub(crate) fn open_catalog(&mut self, shell: &mut Shell<WinWeaver>, id: &str) {
        if shell.focus_tab(|d| matches!(d, Doc::Catalog(c) if c.id == id)) {
            return;
        }
        let Some(file) = self.imports.iter().find(|f| f.id == id) else {
            return;
        };
        shell
            .docs
            .push(Doc::Catalog(Box::new(catalogview::CatalogDoc::new(file))));
        shell.active = shell.docs.len() - 1;
    }

    /// Gives an import another name, in the settings and as the title of its tree.
    pub(crate) fn rename_import(&mut self, shell: &mut Shell<WinWeaver>, id: &str, name: &str) {
        self.full.renaming = None;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
            shell.status =
                tr("A name of 1 to 120 characters without control characters is needed").to_owned();
            return;
        }
        let Some(file) = self.imports.iter_mut().find(|f| f.id == id) else {
            return;
        };
        file.name = name.to_owned();
        let root = Some(winweaver_core::catalog::ROOT_POLICIES);
        for group in file
            .groups
            .iter_mut()
            .filter(|g| g.parent.as_deref() == root)
        {
            group.title = name.to_owned();
        }
        let path = self
            .full
            .imports_dir
            .join(format!("{}.{}", file.id, admx::CATALOG_EXTENSION));
        let saved = file.save(&path);
        for doc in &mut shell.docs {
            if let Doc::Catalog(c) = doc
                && c.id == id
            {
                c.name = name.to_owned();
            }
        }
        self.refresh_import_list();
        self.rebuild_catalog(shell);
        shell.status = match saved {
            Ok(()) => trf("Renamed to {name}", &[("name", &name)]),
            Err(e) => trf("Renamed, but not saved: {e}", &[("e", &e)]),
        };
    }

    pub(crate) fn export_import(&mut self, shell: &mut Shell<WinWeaver>, id: &str) {
        let Some(file) = self.imports.iter().find(|f| f.id == id) else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Export the catalog file"))
            .add_filter(tr("Catalog files"), &[admx::CATALOG_EXTENSION])
            .set_file_name(format!("{}.{}", file.id, admx::CATALOG_EXTENSION))
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

    /// The section of the settings that lists the imports of policy templates.
    pub(crate) fn imports_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        style::section_title(ui, tr("Policy templates"));
        ui.weak(tr("Imported ADMX templates become rules under Policies. Only the checked catalogs are read and loaded; when two of them hold a policy, the templates of this Windows win."));
        let mut renaming = self.full.renaming.take();
        for entry in &self.full.import_list {
            let file = self.imports.iter().find(|f| f.id == entry.id);
            ui.horizontal(|ui| {
                let mut loaded = file.is_some();
                if ui.checkbox(&mut loaded, &entry.name).changed() {
                    acts.push(Act::Import(ImportAct::Toggle(entry.id.clone())));
                }
                match file {
                    Some(f) => {
                        ui.weak(trf(
                            "{rules} rules, {skipped} skipped",
                            &[("rules", &f.rules.len()), ("skipped", &f.skipped.len())],
                        ));
                        if ui.small_button(tr("Export...")).clicked() {
                            acts.push(Act::Import(ImportAct::Export(f.id.clone())));
                        }
                        if ui.small_button(tr("Open")).clicked() {
                            acts.push(Act::Catalog(CatalogAct::Open(f.id.clone())));
                        }
                        if ui.small_button(tr("Rename")).clicked() {
                            acts.push(Act::Settings(SettingsAct::StartRename(
                                f.id.clone(),
                                f.name.clone(),
                            )));
                        }
                    }
                    None => {
                        ui.weak(trf(
                            "not loaded, {size}",
                            &[("size", &size_text(entry.bytes))],
                        ));
                    }
                }
                if ui.small_button(tr("Remove")).clicked() {
                    acts.push(crate::question::remove_import(&entry.name, entry.id.clone()).into());
                }
            });
            if let Some((id, name)) = &mut renaming
                && id == &entry.id
            {
                ui.horizontal(|ui| {
                    let edit = ui.text_edit_singleline(name);
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.small_button(tr("OK")).clicked() || enter {
                        acts.push(Act::Import(ImportAct::Rename(id.clone(), name.clone())));
                    }
                    if ui.small_button(tr("Cancel")).clicked() {
                        acts.push(Act::Settings(SettingsAct::CancelRename));
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            let busy = self.full.importing.is_some();
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(tr("Import the templates of this Windows")),
                )
                .clicked()
            {
                acts.push(Act::Import(ImportAct::Templates(ImportSource::ThisWindows)));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import a folder...")))
                .clicked()
            {
                acts.push(Act::Import(ImportAct::Templates(ImportSource::Folder)));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import a catalog file...")))
                .clicked()
            {
                acts.push(Act::Import(ImportAct::Templates(ImportSource::CatalogFile)));
            }
            if ui
                .add_enabled(!busy, egui::Button::new(tr("Import an archive...")))
                .on_hover_text(tr(
                    "A zip archive of ADMX templates with their ADML, or of one catalog file; it is read without unpacking",
                ))
                .clicked()
            {
                acts.push(Act::Import(ImportAct::Templates(ImportSource::Archive)));
            }
        });
        self.full.renaming = renaming;
    }

    /// Opens the editor of a loaded import or of the file of a catalog tab; an editor of the same
    /// file comes to the front.
    pub(crate) fn edit_catalog(&mut self, shell: &mut Shell<WinWeaver>, which: CatalogRef) {
        let import_path =
            |dir: &Path, id: &str| dir.join(format!("{id}.{}", admx::CATALOG_EXTENSION));
        let (file, path, import) = match &which {
            CatalogRef::Import(id) => match self.imports.iter().find(|f| &f.id == id) {
                Some(f) => (f.clone(), import_path(&self.full.imports_dir, &f.id), true),
                None => return,
            },
            CatalogRef::Tab(i) => match shell.docs.get(*i) {
                Some(Doc::Catalog(c)) => match (&c.file, &c.path) {
                    (Some(f), Some(p)) => ((**f).clone(), p.clone(), false),
                    (None, _) => match c.catalog(&self.imports) {
                        Some(f) => (f.clone(), import_path(&self.full.imports_dir, &f.id), true),
                        None => return,
                    },
                    _ => return,
                },
                _ => return,
            },
        };
        if shell.focus_tab(|d| matches!(d, Doc::CatalogEdit(e) if e.path == path)) {
            return;
        }
        let doc = cataledit::EditorDoc::new(file, path, import);
        shell.open_tab(Doc::CatalogEdit(Box::new(doc)));
    }

    /// An empty catalog file among the imports, loaded, in its editor.
    pub(crate) fn new_catalog(&mut self, shell: &mut Shell<WinWeaver>) {
        let mut n = 1;
        let id = loop {
            let id = format!("custom-{n}");
            let path = self
                .full
                .imports_dir
                .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
            if !path.exists() && !self.imports.iter().any(|f| f.id == id) {
                break id;
            }
            n += 1;
        };
        let name = trf("My policies {n}", &[("n", &n)]);
        let file = admx::to_catalog_file(
            &admx::Templates::default(),
            &id,
            &name,
            "",
            &clock::now_utc_iso(),
        );
        let path = self
            .full
            .imports_dir
            .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
        let _ = std::fs::create_dir_all(&self.full.imports_dir);
        if let Err(e) = file.save(&path) {
            shell.save_failed(e);
            return;
        }
        self.imports.push(file.clone());
        self.imports.sort_by_key(|f| import_order(&f.id));
        self.mark_loaded(shell, &id);
        self.refresh_import_list();
        self.rebuild_catalog(shell);
        let doc = cataledit::EditorDoc::new(file, path, true);
        shell.open_tab(Doc::CatalogEdit(Box::new(doc)));
    }

    /// Saves the file of the editor in front after a check; a loaded import is read again.
    pub(crate) fn save_catalog_edit(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::CatalogEdit(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        if let Some(parent) = d.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let result = d.file.check().and_then(|()| d.file.save(&d.path));
        if let Err(e) = result {
            shell.save_failed(e);
            return;
        }
        d.dirty = false;
        let path = d.path.clone();
        let reload = d.import.then(|| d.file.clone());
        shell.saved(&path);
        if let Some(file) = reload {
            if let Some(slot) = self.imports.iter_mut().find(|f| f.id == file.id) {
                *slot = file;
            }
            self.refresh_import_list();
            self.rebuild_catalog(shell);
        }
    }

    /// The catalog of the active tab and the other one, side by side in a tab of their
    /// differences.
    pub(crate) fn compare_catalog(&mut self, shell: &mut Shell<WinWeaver>, other: CatalogRef) {
        let Some(Doc::Catalog(c)) = shell.docs.get(shell.active) else {
            return;
        };
        let Some(first) = c.catalog(&self.imports) else {
            return;
        };
        let second = match &other {
            CatalogRef::Import(id) => self.imports.iter().find(|f| &f.id == id),
            CatalogRef::Tab(i) => match shell.docs.get(*i) {
                Some(Doc::Catalog(o)) => o.catalog(&self.imports),
                _ => None,
            },
        };
        let Some(second) = second else {
            shell.status = tr("The other catalog is not open any more").to_owned();
            return;
        };
        let doc = catalogdiff::CompareDoc::new(first, second);
        shell.docs.push(Doc::Compare(Box::new(doc)));
        shell.active = shell.docs.len() - 1;
    }

    /// Copies the selected policy or group of the catalog tab in front into the catalog of an
    /// editor tab, or into a new catalog made for it, under the group selected in the editor
    /// (its root group without one). The catalog tab stays in front, so more can follow.
    pub(crate) fn copy_to_catalog(&mut self, shell: &mut Shell<WinWeaver>, target: Option<usize>) {
        let from = shell.active;
        let Some(Doc::Catalog(c)) = shell.docs.get(from) else {
            return;
        };
        let Some(source) = c.catalog(&self.imports).cloned() else {
            return;
        };
        let (policy, group) = (c.sel.clone(), c.group.clone());
        let target = match target {
            Some(i) => i,
            None => {
                self.new_catalog(shell);
                shell.active
            }
        };
        let Some(Doc::CatalogEdit(e)) = shell.docs.get_mut(target) else {
            return;
        };
        let what = match (&policy, &group) {
            (Some(p), _) => admx::CopyWhat::Policy(p),
            (None, Some(g)) => admx::CopyWhat::Group(g),
            (None, None) => return,
        };
        let root = e
            .file
            .groups
            .iter()
            .find(|g| g.parent.as_deref() == Some(winweaver_core::catalog::ROOT_POLICIES))
            .map(|g| g.id.clone());
        let into = e
            .group
            .clone()
            .filter(|g| e.file.groups.iter().any(|x| &x.id == g))
            .or(root);
        let Some(into) = into else {
            return;
        };
        match e.file.copy_from(&source, what, &into) {
            Ok(n) => {
                e.dirty = true;
                for g in &e.file.groups {
                    e.expanded.insert(g.id.clone());
                }
                let name = e.file.name.clone();
                shell.status = trf(
                    "{n} policies copied into {name}",
                    &[("n", &n), ("name", &name)],
                );
            }
            Err(e) => shell.status = trf("Not copied: {e}", &[("e", &e)]),
        }
        shell.active = from;
    }

    /// Saves the report of the comparison in front as a Markdown file.
    pub(crate) fn save_comparison(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Compare(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let name = format!("{}.md", export::file_stem(&doc.title));
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Save the report of the comparison"))
            .add_filter(tr("Markdown pages"), &["md"])
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        let page = comparison_page(&doc.first, &doc.second);
        match jsonfile::write_atomic(&path, &page) {
            Ok(()) => {
                shell.saved(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// Runs an act of the comparison in front.
    fn apply_compare(&mut self, shell: &mut Shell<WinWeaver>, act: CatalogAct) {
        if let CatalogAct::CompareSave = act {
            self.save_comparison(shell);
            return;
        }
        if let CatalogAct::ComparePage = act {
            let Some(Doc::Compare(doc)) = shell.docs.get(shell.active) else {
                return;
            };
            let page = comparison_page(&doc.first, &doc.second);
            let title = doc.title.clone();
            shell
                .docs
                .push(Doc::Markdown(Box::new(MdDoc::new(title, None, page))));
            shell.active = shell.docs.len() - 1;
            return;
        }
        let Some(Doc::Compare(doc)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        match act {
            CatalogAct::CompareSelect(key) => {
                doc.sel = Some(key);
            }
            CatalogAct::CompareExpand(key, open) => {
                if open {
                    doc.expanded.insert(key);
                } else {
                    doc.expanded.remove(&key);
                }
            }
            CatalogAct::CompareShow(i) => {
                if let Some(on) = doc.shown.get_mut(i) {
                    *on = !*on;
                }
            }
            _ => {}
        }
    }

    /// Runs an act of the acts of the tab of an import or of a catalog file, and of its editor.
    pub(crate) fn apply_catalog(&mut self, shell: &mut Shell<WinWeaver>, act: CatalogAct) {
        match act {
            CatalogAct::OpenAt(import, rule) => {
                self.open_catalog(shell, &import);
                if let Some(Doc::Catalog(c)) = shell.docs.get_mut(shell.active)
                    && c.id == import
                {
                    c.group = None;
                    c.sel = Some(rule);
                }
                self.reveal_selection(shell);
            }
            CatalogAct::ImportOpen => self.import_open_catalog(shell),
            CatalogAct::CopyTo(target) => self.copy_to_catalog(shell, target),
            act @ (CatalogAct::CompareSelect(_)
            | CatalogAct::CompareExpand(..)
            | CatalogAct::CompareShow(_)
            | CatalogAct::CompareSave
            | CatalogAct::ComparePage) => self.apply_compare(shell, act),
            CatalogAct::Compare(other) => self.compare_catalog(shell, other),
            CatalogAct::Edit(which) => self.edit_catalog(shell, which),
            CatalogAct::New => self.new_catalog(shell),
            CatalogAct::EditRemove(id) => {
                if let Some(Doc::CatalogEdit(d)) = shell.docs.get_mut(shell.active) {
                    d.remove(&id);
                }
            }
            CatalogAct::Open(id) => self.open_catalog(shell, &id),
            CatalogAct::Select(key) => {
                if let Some(Doc::Catalog(c)) = shell.docs.get_mut(shell.active) {
                    let (group, sel) = match key {
                        Some(TreeKey::Group(g)) => (Some(g), None),
                        Some(TreeKey::Rule(r)) => (None, Some(r)),
                        None => (None, None),
                    };
                    c.group = group;
                    c.sel = sel;
                }
            }
            CatalogAct::Reveal(key) => {
                self.apply(shell, Act::Catalog(CatalogAct::Select(Some(key))));
                self.reveal_selection(shell);
            }
            CatalogAct::Expand(gid, open) => {
                if let Some(Doc::Catalog(c)) = shell.docs.get_mut(shell.active) {
                    if open {
                        c.expanded.insert(gid);
                    } else {
                        c.expanded.remove(&gid);
                    }
                }
            }
        }
    }

    /// Runs an act of the acts of the imports of policy templates.
    pub(crate) fn apply_import(&mut self, shell: &mut Shell<WinWeaver>, act: ImportAct) {
        match act {
            ImportAct::Templates(source) => self.start_import(shell, source),
            ImportAct::Toggle(id) => {
                if self.imports.iter().any(|f| f.id == id) {
                    self.unload_import(shell, &id);
                } else if let Err(e) = self.load_import(shell, &id) {
                    shell.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                }
                self.refresh_import_list();
                self.rebuild_catalog(shell);
            }
            ImportAct::Remove(id) => {
                let _ = std::fs::remove_file(
                    self.full
                        .imports_dir
                        .join(format!("{id}.{}", admx::CATALOG_EXTENSION)),
                );
                self.unload_import(shell, &id);
                self.refresh_import_list();
                self.rebuild_catalog(shell);
            }
            ImportAct::LoadCatalogs => self.load_catalogs_for_unknown(shell),
            ImportAct::Export(id) => self.export_import(shell, &id),
            ImportAct::Rename(id, name) => self.rename_import(shell, &id, &name),
        }
    }
}
