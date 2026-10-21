//! Opening and saving documents of every kind, the recent files, the pages.

use crate::*;

impl WinWeaver {
    /// Opens a page of the documentation a link names, such as
    /// `docs/technical/reference/18-browsers.md#microsoft-edge`, at the heading of its anchor; a
    /// link `#heading` goes to a heading of the Markdown tab that is open.
    pub(crate) fn open_doc(&mut self, shell: &mut Shell<WinWeaver>, link: &str) {
        let (file, anchor) = match link.split_once('#') {
            Some((f, a)) => (f, Some(a.to_owned())),
            None => (link, None),
        };
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        if name.is_empty() {
            if let Some(Doc::Markdown(m)) = shell.docs.get_mut(shell.active) {
                m.jump = anchor.is_some();
                m.anchor = anchor;
            }
            return;
        }
        let (title, text) = if name.eq_ignore_ascii_case("guide.md") {
            (
                pages::GUIDE_TITLE.to_owned(),
                Some(pages::guide_text(&i18n::current())),
            )
        } else {
            (
                name.to_owned(),
                pages::REFERENCE
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, t)| (*t).to_owned()),
            )
        };
        let Some(text) = text else {
            shell.status = trf("No page {name} in the documentation", &[("name", &name)]);
            return;
        };
        let i = shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.path.is_none() && m.title == title),
            || Doc::Markdown(Box::new(MdDoc::new(title.clone(), None, text))),
        );
        if let Some(Doc::Markdown(m)) = shell.docs.get_mut(i) {
            m.jump = anchor.is_some();
            m.anchor = anchor;
        }
    }

    pub(crate) fn open_any(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        if path.extension().is_some_and(|e| e == diff::EXTENSION) {
            self.open_diff_path(shell, path);
            return;
        }
        if path.extension().is_some_and(|e| e == bookmarks::EXTENSION) {
            self.open_marks_path(shell, path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == admx::CATALOG_EXTENSION)
        {
            self.open_catalog_path(shell, path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("admx"))
        {
            self.open_admx_path(shell, path);
            return;
        }
        if path.extension().is_some_and(|e| e == users::EXTENSION) {
            self.open_users_path(shell, path);
            return;
        }
        if path.extension().is_some_and(|e| e == themes::EXTENSION) {
            shell.open_theme_path(path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e == winweaver_mcp::config::EXTENSION)
        {
            self.open_mcp_config(shell, path);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(regfile::EXTENSION))
        {
            self.open_reg_path(shell, path);
            return;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "xml" || ext == "json" {
            self.open_foreign(shell, path, &ext);
            return;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            self.open_md_path(shell, path);
            return;
        }
        if shell.focus_tab(|d| matches!(d, Doc::Profile(p) if p.path.as_ref() == Some(&path))) {
            return;
        }
        match Profile::load(&path) {
            Ok(mut profile) => {
                let unknown = profile.split_unknown(&self.catalog);
                let doc = ProfileDoc::new(profile, Some(path.clone()), &self.catalog);
                shell.open_tab(Doc::Profile(Box::new(doc)));
                shell.status = if unknown.is_empty() {
                    trf("Opened {path}", &[("path", &path.display())])
                } else {
                    trf(
                        "Opened {path}: {n} options of catalogs that are not loaded are left out of the build",
                        &[("path", &path.display()), ("n", &unknown.len())],
                    )
                };
                shell.remember_recent(&path);
            }
            Err(e) => shell.open_failed(e),
        }
    }

    pub(crate) fn save_active(&mut self, shell: &mut Shell<WinWeaver>, ask: bool) {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
            shell.status = tr("Nothing to save: the active tab is not a profile").to_owned();
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
        let Some(Doc::Profile(doc)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        if doc.profile.created.is_empty() {
            doc.profile.created = clock::now_utc_iso();
        }
        doc.profile.modified = clock::now_utc_iso();
        match doc.profile.save(&path) {
            Ok(()) => {
                session::remove_draft(&shell.drafts_dir, &doc.draft_key);
                doc.draft_key = session::key_for_path(&path);
                session::remove_draft(&shell.drafts_dir, &doc.draft_key);
                doc.path = Some(path.clone());
                doc.dirty = false;
                doc.changed = None;
                shell.saved(&path);
                shell.remember_recent(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    pub(crate) fn open_license(&mut self, shell: &mut Shell<WinWeaver>) {
        shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.page == Some(pages::LICENSE_ID)),
            || Doc::Markdown(Box::new(pages::license())),
        );
    }

    pub(crate) fn open_guide(&mut self, shell: &mut Shell<WinWeaver>) {
        shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.page == Some(pages::GUIDE_ID)),
            || Doc::Markdown(Box::new(pages::guide())),
        );
    }

    pub(crate) fn open_md_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "page.md".to_owned());
        shell.open_file(
            &path,
            |_| false,
            |p| winweaver_core::jsonfile::read_text(p, MARKDOWN_MAX_BYTES),
            |text, path| Doc::Markdown(Box::new(MdDoc::new(title, Some(path), text))),
        );
    }

    /// Opens an answer file (its embedded profile), a WinKickOff profile or an audit report as a
    /// new, unsaved profile.
    pub(crate) fn open_foreign(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf, ext: &str) {
        let name = jsonfile::display_name(&path);
        let text = match jsonfile::read_text_bom(&path, jsonfile::MAX_BYTES) {
            Ok(t) => t,
            Err(e) => {
                shell.open_failed(e);
                return;
            }
        };
        if ext == "json"
            && let Ok(report) = Report::parse(&text, &name)
        {
            self.open_report_from(shell, report, &path);
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
                shell.open_tab(Doc::Profile(Box::new(doc)));
                shell.status = trf(
                    "Opened the profile of {path}; save it as a .wwprofile file",
                    &[("path", &path.display())],
                );
                shell.remember_recent(&path);
            }
            Err(e) => shell.open_failed(e),
        }
    }

    /// Runs an act of the acts of the files: a new profile, the pages of the program.
    pub(crate) fn apply_file(&mut self, shell: &mut Shell<WinWeaver>, act: FileAct) {
        match act {
            FileAct::NewProfile(preset) => {
                let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                    Some((name, text)) => Profile::parse(text, name)
                        .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog)),
                    None => Profile::new("New profile", self.catalog.version()),
                };
                let doc = ProfileDoc::new(profile, None, &self.catalog);
                shell.open_tab(Doc::Profile(Box::new(doc)));
            }
            FileAct::OpenReport => self.open_report(shell),
            FileAct::OpenGuide => self.open_guide(shell),
            FileAct::OpenLicense => self.open_license(shell),
        }
    }
}
