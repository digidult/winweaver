//! Opening and saving documents of every kind, the recent files, the pages.

use crate::*;

impl WinWeaver {
    /// Opens a page of the documentation a link names, such as
    /// `docs/technical/reference/18-browsers.md#microsoft-edge`, at the heading of its anchor; a
    /// link `#heading` goes to a heading of the Markdown tab that is open. A page of a file links
    /// to the files next to it first; a page of the program to the pages of the program, then to
    /// the files of the program folder.
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
        let file = shell.linked_file(link).filter(|(p, _)| p.is_file());
        let from_file = shell.active_doc().and_then(ww::Document::path).is_some();
        if from_file && let Some((path, anchor)) = file.clone() {
            self.open_linked_file(shell, path, anchor);
            return;
        }
        #[cfg(feature = "full")]
        if self.open_page(shell, name, anchor) {
            return;
        }
        match file {
            Some((path, anchor)) => self.open_linked_file(shell, path, anchor),
            None => {
                shell.status = trf("No page {name} in the documentation", &[("name", &name)]);
            }
        }
    }

    #[cfg(feature = "full")]
    /// Opens a page built into the executable by its file name (the guide in the language of the
    /// interface, or a page of the reference) at a heading; false when there is no such page.
    pub(crate) fn open_page(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        name: &str,
        anchor: Option<String>,
    ) -> bool {
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
            return false;
        };
        let i = shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.path.is_none() && m.title == title),
            || Doc::Markdown(Box::new(MdDoc::new(title.clone(), None, text))),
        );
        if let Some(Doc::Markdown(m)) = shell.docs.get_mut(i) {
            m.jump = anchor.is_some();
            m.anchor = anchor;
        }
        true
    }

    pub(crate) fn open_any(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        #[cfg(not(feature = "full"))]
        {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if ext != profile::EXTENSION && ext != "xml" && ext != "json" {
                shell.status = trf(
                    "wwlite opens profiles and answer files, not {path}",
                    &[("path", &path.display())],
                );
                return;
            }
        }
        #[cfg(feature = "full")]
        {
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
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(winweaver_core::pol::EXTENSION))
            {
                self.open_pol_path(shell, path);
                return;
            }
            #[cfg(feature = "mcp")]
            if path
                .extension()
                .is_some_and(|e| e == wwmcp::config::EXTENSION)
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
        }
        if path.extension().is_some_and(|e| e == themes::EXTENSION) {
            shell.open_theme_path(path);
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

    /// Saves the profile in front, in a browser: as a download of its file, whatever `ask` says,
    /// since the tab has no file there.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn save_active(&mut self, shell: &mut Shell<WinWeaver>, _ask: bool) {
        let Some(Doc::Profile(doc)) = shell.docs.get_mut(shell.active) else {
            shell.status = tr("Nothing to save: the active tab is not a profile").to_owned();
            return;
        };
        if doc.profile.created.is_empty() {
            doc.profile.created = clock::now_utc_iso();
        }
        doc.profile.modified = clock::now_utc_iso();
        let name = format!("{}.{}", file_stem(&doc.profile.name), profile::EXTENSION);
        let text = doc.profile.to_json();
        match ww::store::download(&name, text.as_bytes(), "application/json") {
            Ok(()) => {
                doc.dirty = false;
                doc.changed = None;
                session::remove_draft(&shell.drafts_dir, &doc.draft_key);
                shell.status = trf("Downloaded {name}", &[("name", &name)]);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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
        // The folder of the file is made by the file layer when it writes (`ww_base::files`).
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
        #[cfg(not(feature = "full"))]
        {
            let text = include_str!("../../../../LICENSE").to_owned();
            shell.show_or_open(
                |d| matches!(d, Doc::Markdown(m) if m.title == "LICENSE.txt"),
                || Doc::Markdown(Box::new(MdDoc::plain("LICENSE.txt", None, text))),
            );
        }
        #[cfg(feature = "full")]
        shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.page == Some(pages::LICENSE_ID)),
            || Doc::Markdown(Box::new(pages::license())),
        );
    }

    pub(crate) fn open_guide(&mut self, shell: &mut Shell<WinWeaver>) {
        #[cfg(not(feature = "full"))]
        {
            let text = pages::guide_lite_text(&i18n::current());
            shell.show_or_open(
                |d| matches!(d, Doc::Markdown(m) if m.title == pages::GUIDE_LITE_TITLE),
                || Doc::Markdown(Box::new(MdDoc::plain(pages::GUIDE_LITE_TITLE, None, text))),
            );
        }
        #[cfg(feature = "full")]
        shell.show_or_open(
            |d| matches!(d, Doc::Markdown(m) if m.page == Some(pages::GUIDE_ID)),
            || Doc::Markdown(Box::new(pages::guide())),
        );
    }

    /// A file a page links to: a Markdown page at its heading, else the text as it is.
    fn open_linked_file(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        path: PathBuf,
        anchor: Option<String>,
    ) {
        let md = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if md {
            self.open_md_path(shell, path);
        } else {
            let title = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "text".to_owned());
            shell.open_file(
                &path,
                |d| matches!(d, Doc::Markdown(m) if m.path.as_ref() == Some(&path)),
                |p| winweaver_core::jsonfile::read_text(p, MARKDOWN_MAX_BYTES),
                |text, path| Doc::Markdown(Box::new(MdDoc::plain(title, Some(path), text))),
            );
        }
        if let Some(Doc::Markdown(m)) = shell.docs.get_mut(shell.active) {
            m.jump = anchor.is_some();
            m.anchor = anchor;
        }
    }

    pub(crate) fn open_md_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "page.md".to_owned());
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Markdown(m) if m.path.as_ref() == Some(&path)),
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
        #[cfg(feature = "full")]
        if ext == "json"
            && let Ok(report) = Report::parse(&text, &name)
        {
            self.open_report_from(shell, report, &path);
            return;
        }
        let shown = path.display().to_string();
        if self.open_profile_text(shell, &text, &name, ext, &shown) {
            shell.remember_recent(&path);
        }
    }

    /// Opens the profile a text holds (an answer file, or a profile of WinWeaver or WinKickOff
    /// as JSON) as a new, unsaved profile; `shown` names the file in the status bar. Whether it
    /// opened.
    fn open_profile_text(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        text: &str,
        name: &str,
        ext: &str,
        shown: &str,
    ) -> bool {
        let opened = if ext == "xml" {
            import::profile_from_answer_file(text, name)
        } else {
            import::profile_from_json(text, name)
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
                    &[("path", &shown)],
                );
                true
            }
            Err(e) => {
                shell.open_failed(e);
                false
            }
        }
    }

    /// Opens a file the browser handed over by its name and its bytes (the light edition in a
    /// browser): a profile, an answer file with the profile it embeds, or an older profile. The
    /// tab has no file, so a save is a download.
    #[cfg(any(not(feature = "full"), target_arch = "wasm32"))]
    pub(crate) fn open_any_bytes(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        name: &str,
        bytes: Vec<u8>,
    ) {
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != profile::EXTENSION && ext != "xml" && ext != "json" {
            shell.status = trf(
                "wwlite opens profiles and answer files, not {path}",
                &[("path", &name)],
            );
            return;
        }
        if bytes.len() as u64 > jsonfile::MAX_BYTES {
            shell.open_failed(format!("{name}: larger than {} bytes", jsonfile::MAX_BYTES));
            return;
        }
        let Some(text) = jsonfile::decode_bom(&bytes) else {
            shell.open_failed(format!("{name}: not UTF-8"));
            return;
        };
        if ext != profile::EXTENSION {
            self.open_profile_text(shell, &text, name, &ext, name);
            return;
        }
        match Profile::parse(&text, name) {
            Ok(mut profile) => {
                let unknown = profile.split_unknown(&self.catalog);
                let doc = ProfileDoc::new(profile, None, &self.catalog);
                shell.open_tab(Doc::Profile(Box::new(doc)));
                shell.status = if unknown.is_empty() {
                    trf("Opened {path}", &[("path", &name)])
                } else {
                    trf(
                        "Opened {path}: {n} options of catalogs that are not loaded are left out of the build",
                        &[("path", &name), ("n", &unknown.len())],
                    )
                };
            }
            Err(e) => shell.open_failed(e),
        }
    }

    /// Runs an act of the acts of the files: a new profile, the pages of the program.
    pub(crate) fn apply_file(&mut self, shell: &mut Shell<WinWeaver>, act: FileAct) {
        match act {
            FileAct::NewProfile(preset) => {
                let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                    Some((name, text)) => preset_profile(name, text, &self.catalog),
                    None => Profile::new("New profile", self.catalog.version()),
                };
                let doc = ProfileDoc::new(profile, None, &self.catalog);
                shell.open_tab(Doc::Profile(Box::new(doc)));
            }
            #[cfg(feature = "full")]
            FileAct::OpenReport => self.open_report(shell),
            FileAct::OpenGuide => self.open_guide(shell),
            #[cfg(feature = "full")]
            FileAct::OpenDocs => {
                self.open_page(shell, pages::DOCS_ROOT, None);
            }
            FileAct::OpenLicense => self.open_license(shell),
        }
    }
}
