//! Opening and saving documents of every kind, the recent files, the pages.

use crate::*;

impl App {
    /// Opens a page of the documentation a link names, such as
    /// `docs/technical/reference/18-browsers.md#microsoft-edge`, at the heading of its anchor; a
    /// link `#heading` goes to a heading of the Markdown tab that is open.
    pub(crate) fn open_doc(&mut self, link: &str) {
        let (file, anchor) = match link.split_once('#') {
            Some((f, a)) => (f, Some(a.to_owned())),
            None => (link, None),
        };
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        if name.is_empty() {
            if let Some(Doc::Markdown(m)) = self.docs.get_mut(self.active) {
                m.jump = anchor.is_some();
                m.anchor = anchor;
            }
            return;
        }
        let (title, text) = if name.eq_ignore_ascii_case("guide.md") {
            (
                mdview::GUIDE_TITLE.to_owned(),
                Some(mdview::guide_text(&i18n::current())),
            )
        } else {
            (
                name.to_owned(),
                mdview::REFERENCE
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, t)| (*t).to_owned()),
            )
        };
        let Some(text) = text else {
            self.status = trf("No page {name} in the documentation", &[("name", &name)]);
            return;
        };
        self.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.path.is_none() && m.title == title),
            || Doc::Markdown(Box::new(MdDoc::new(title.clone(), None, text))),
        );
        if let Some(Doc::Markdown(m)) = self.docs.get_mut(i) {
            m.jump = anchor.is_some();
            m.anchor = anchor;
        }
    }

    pub(crate) fn remember_recent(&mut self, path: &Path) {
        self.settings.remember(path);
        self.settings_dirty = true;
    }

    pub(crate) fn open_path(&mut self, path: PathBuf) {
        if path.extension().is_some_and(|e| e == diff::EXTENSION) {
            self.open_diff_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == bookmarks::EXTENSION) {
            self.open_marks_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == admx::CATALOG_EXTENSION)
        {
            self.open_catalog_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("admx"))
        {
            self.open_admx_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == users::EXTENSION) {
            self.open_users_path(path);
            return;
        }
        if path.extension().is_some_and(|e| e == themes::EXTENSION) {
            self.open_theme_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == winweaver_mcp::config::EXTENSION)
        {
            self.open_mcp_config(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(regfile::EXTENSION))
        {
            self.open_reg_path(path);
            return;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "xml" || ext == "json" {
            self.open_foreign(path, &ext);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            self.open_md_path(path);
            return;
        }
        if self.focus_tab(|d| matches!(d, Doc::Profile(p) if p.path.as_ref() == Some(&path))) {
            return;
        }
        match Profile::load(&path) {
            Ok(mut profile) => {
                let unknown = profile.split_unknown(&self.catalog);
                let doc = ProfileDoc::new(profile, Some(path.clone()), &self.catalog);
                self.open_tab(Doc::Profile(Box::new(doc)));
                self.status = if unknown.is_empty() {
                    trf("Opened {path}", &[("path", &path.display())])
                } else {
                    trf(
                        "Opened {path}: {n} options of catalogs that are not loaded are left out of the build",
                        &[("path", &path.display()), ("n", &unknown.len())],
                    )
                };
                self.remember_recent(&path);
            }
            Err(e) => self.open_failed(e),
        }
    }

    pub(crate) fn save_active(&mut self, ask: bool) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Nothing to save: the active tab is not a profile").to_owned();
            return;
        };
        let path = match (&doc.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let proposed = format!("{}.{}", file_stem(&doc.profile.name), profile::EXTENSION);
                match ask_save_path(&proposed) {
                    Some(p) => with_extension(p),
                    None => return,
                }
            }
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        if doc.profile.created.is_empty() {
            doc.profile.created = clock::now_utc_iso();
        }
        doc.profile.modified = clock::now_utc_iso();
        match doc.profile.save(&path) {
            Ok(()) => {
                session::remove_draft(&self.drafts_dir, &doc.draft_key);
                doc.draft_key = session::key_for_path(&path);
                session::remove_draft(&self.drafts_dir, &doc.draft_key);
                doc.path = Some(path.clone());
                doc.dirty = false;
                doc.changed = None;
                self.saved(&path);
                self.remember_recent(&path);
            }
            Err(e) => self.save_failed(e),
        }
    }

    pub(crate) fn open_license(&mut self) {
        self.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.is_license()),
            || Doc::Markdown(Box::new(MdDoc::license())),
        );
    }

    pub(crate) fn open_guide(&mut self) {
        self.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.is_guide()),
            || Doc::Markdown(Box::new(MdDoc::guide())),
        );
    }

    pub(crate) fn open_md_path(&mut self, path: PathBuf) {
        match winweaver_core::jsonfile::read_text(&path, MARKDOWN_MAX_BYTES) {
            Ok(text) => {
                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "page.md".to_owned());
                self.docs.push(Doc::Markdown(Box::new(MdDoc::new(
                    title,
                    Some(path.clone()),
                    text,
                ))));
                self.active = self.docs.len() - 1;
                self.opened(&path);
            }
            Err(e) => self.open_failed(e),
        }
    }

    /// Opens an answer file (its embedded profile), a WinKickOff profile or an audit report as a
    /// new, unsaved profile.
    pub(crate) fn open_foreign(&mut self, path: PathBuf, ext: &str) {
        let name = jsonfile::display_name(&path);
        let text = match jsonfile::read_text_bom(&path, jsonfile::MAX_BYTES) {
            Ok(t) => t,
            Err(e) => {
                self.open_failed(e);
                return;
            }
        };
        if ext == "json"
            && let Ok(report) = Report::parse(&text, &name)
        {
            self.open_report_from(report, &path);
            return;
        }
        let opened = if ext == "xml" {
            import::profile_from_answer_file(&text, &name)
        } else {
            import::profile_from_json(&text, &name)
        };
        match opened {
            Ok((mut profile, notes)) => {
                profile.split_unknown(&self.catalog);
                let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                doc.dirty = true;
                doc.issues.extend(notes.into_iter().map(|n| Issue {
                    severity: Severity::Info,
                    target: Target::Profile,
                    text: trf("Open: {n}", &[("n", &n)]),
                }));
                self.open_tab(Doc::Profile(Box::new(doc)));
                self.status = trf(
                    "Opened the profile of {path}; save it as a .wwprofile file",
                    &[("path", &path.display())],
                );
                self.remember_recent(&path);
            }
            Err(e) => self.open_failed(e),
        }
    }

    /// Runs an act of the acts of the files: new, open, save, the pages of the program.
    pub(crate) fn apply_file(&mut self, act: FileAct) {
        match act {
            FileAct::NewProfile(preset) => {
                let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                    Some((name, text)) => Profile::parse(text, name)
                        .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog)),
                    None => Profile::new("New profile", self.catalog.version()),
                };
                let doc = ProfileDoc::new(profile, None, &self.catalog);
                self.open_tab(Doc::Profile(Box::new(doc)));
            }
            FileAct::Open => {
                if let Some(path) = pick_any() {
                    self.open_path(path);
                }
            }
            FileAct::OpenPath(path) => self.open_path(path),
            FileAct::Save => match self.docs.get(self.active) {
                Some(Doc::Mcp(_)) => self.persist_mcp(),
                Some(Doc::Diff(_)) => self.save_diff(),
                Some(Doc::Theme(_)) => self.save_theme(false),
                Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                Some(Doc::RegFile(_)) => self.save_reg_doc(false),
                Some(Doc::Users(_)) => self.save_users_doc(false),
                _ => self.save_active(false),
            },
            FileAct::SaveAs => match self.docs.get(self.active) {
                Some(Doc::Mcp(_)) => self.save_mcp_as(),
                Some(Doc::Diff(_)) => self.save_active_diff(),
                Some(Doc::Theme(_)) => self.save_theme(true),
                Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                Some(Doc::RegFile(_)) => self.save_reg_doc(true),
                Some(Doc::Users(_)) => self.save_users_doc(true),
                _ => self.save_active(true),
            },
            FileAct::OpenReport => self.open_report(),
            FileAct::ForgetRecent => {
                self.settings.recent.clear();
                self.settings_dirty = true;
            }
            FileAct::OpenGuide => self.open_guide(),
            FileAct::OpenLicense => self.open_license(),
        }
    }
}
