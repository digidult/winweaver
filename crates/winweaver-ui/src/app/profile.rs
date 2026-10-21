//! The profile tab: the selection, the tree, the check and the build.

use crate::*;

impl App {
    fn active_profile(&mut self) -> Option<&mut ProfileDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => Some(doc),
            _ => None,
        }
    }

    /// The search of the active tab, `None` for a tab without one.
    fn active_search(&mut self) -> Option<&mut Search> {
        match self.docs.get_mut(self.active)? {
            Doc::Profile(d) => Some(&mut d.search),
            Doc::Diff(d) => Some(&mut d.search),
            Doc::Markdown(d) => Some(&mut d.search),
            Doc::Catalog(d) => Some(&mut d.search),
            Doc::Bookmarks => Some(&mut self.marks_search),
            Doc::Mcp => Some(&mut self.mcp_search),
            Doc::RegFile(d) => Some(&mut d.search),
            Doc::Users(d) => Some(&mut d.search),
            Doc::Settings | Doc::Theme(_) | Doc::Registry(_) | Doc::CatalogEdit(_) => None,
        }
    }

    /// After the search of a tab is cleared, the selected item stays in view: the groups above
    /// it open and its row is scrolled to at the next frame.
    fn reveal_selection(&mut self) {
        let catalog = &self.catalog;
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => {
                let mut next = match &doc.sel {
                    Sel::Rule(id) => doc
                        .focus
                        .clone()
                        .or_else(|| catalog.rule(id).map(|r| r.group.clone())),
                    Sel::Group(id) => catalog.group(id).and_then(|g| g.parent.clone()),
                    Sel::Form(_) => Some(winweaver_core::catalog::ROOT_UNATTEND.to_owned()),
                    Sel::Unknown(_) => Some(UNKNOWN_GROUP.to_owned()),
                    Sel::None => None,
                };
                while let Some(g) = next {
                    next = catalog.group(&g).and_then(|x| x.parent.clone());
                    doc.expanded.insert(g);
                }
                doc.reveal = true;
            }
            Some(Doc::Catalog(c)) => {
                let mut chain = Vec::new();
                if let Some(file) = c.catalog(&self.imports) {
                    let group = |id: &str| file.groups.iter().find(|x| x.id == id);
                    let rule = |id: &str| file.rules.iter().find(|r| r.id == id);
                    // The groups above the selected policy, or above the selected group.
                    let mut next = match (c.sel.as_deref(), c.group.as_deref()) {
                        (Some(id), _) => rule(id).map(|r| r.group.clone()),
                        (None, Some(id)) => group(id).and_then(|g| g.parent.clone()),
                        (None, None) => None,
                    };
                    while let Some(g) = next {
                        next = file
                            .groups
                            .iter()
                            .find(|x| x.id == g)
                            .and_then(|x| x.parent.clone());
                        chain.push(g);
                    }
                }
                c.expanded.extend(chain);
                c.reveal = true;
            }
            Some(Doc::Diff(d)) => d.reveal = true,
            Some(Doc::Bookmarks) => self.marks_tab.reveal = true,
            Some(Doc::Mcp) => self.mcp.journal_reveal = true,
            Some(Doc::RegFile(d)) => d.follow = true,
            Some(Doc::Users(d)) => d.follow = true,
            _ => {}
        }
    }

    fn refresh_active(&mut self) {
        let catalog = &self.catalog;
        if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
            doc.refresh(catalog);
        }
    }

    fn crumbs(&self) -> Vec<(String, Sel)> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let mut crumbs = vec![(doc.profile.name.clone(), Sel::None)];
        let group = match &doc.sel {
            Sel::None | Sel::Form(_) | Sel::Unknown(_) => None,
            Sel::Group(id) if id == UNKNOWN_GROUP => None,
            Sel::Group(id) => Some(id.clone()),
            Sel::Rule(id) => doc
                .focus
                .clone()
                .or_else(|| self.catalog.rule(id).map(|r| r.group.clone())),
        };
        let in_unknown = match &doc.sel {
            Sel::Unknown(_) => true,
            Sel::Group(id) => id == UNKNOWN_GROUP,
            _ => false,
        };
        if in_unknown {
            crumbs.push((
                tr("Unknown options").to_owned(),
                Sel::Group(UNKNOWN_GROUP.to_owned()),
            ));
        }
        if let Sel::Unknown(id) = &doc.sel {
            crumbs.push((id.clone(), Sel::Unknown(id.clone())));
        }
        if let Some(group) = group {
            for id in self.catalog.path_of(&group) {
                if let Some(g) = self.catalog.group(id) {
                    crumbs.push((
                        views::gtitle(texts, g).to_owned(),
                        Sel::Group(id.to_owned()),
                    ));
                }
            }
        }
        if let Sel::Rule(id) = &doc.sel
            && let Some(rule) = self.catalog.rule(id)
        {
            crumbs.push((views::rtitle(texts, rule).to_owned(), Sel::Rule(id.clone())));
        }
        if let Sel::Form(f) = &doc.sel {
            crumbs.push((tr(f.title()).to_owned(), Sel::Form(*f)));
        }
        crumbs
    }

    fn check_active(&mut self) {
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            self.status = tr("Nothing to check: the active tab is not a profile").to_owned();
            return;
        };
        doc.issues = validate::check(catalog, &doc.profile, &doc.resolution);
        self.status = validate::summary(&doc.issues);
    }

    /// Checks the profile, builds the answer file and writes it with its scripts into the
    /// output folder; errors and warnings go to the messages panel.
    fn build_active(&mut self) {
        self.check_active();
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        let errors = validate::errors(&doc.issues);
        if errors > 0 {
            self.status = trf("Fix the errors before the build: {n}", &[("n", &errors)]);
            return;
        }
        let build = match render::build(catalog, &doc.profile, winweaver_core::BUILD) {
            Ok(b) => b,
            Err(e) => {
                doc.issues.insert(
                    0,
                    Issue {
                        severity: Severity::Error,
                        target: Target::Profile,
                        text: trf("Build: {e}", &[("e", &e)]),
                    },
                );
                self.status = trf("Build failed: {e}", &[("e", &e)]);
                return;
            }
        };
        for w in &build.warnings {
            doc.issues.push(Issue {
                severity: Severity::Warning,
                target: Target::Profile,
                text: trf("Build: {w}", &[("w", &w)]),
            });
        }
        let dir = output_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.status = trf(
                "Build: cannot create {path}: {e}",
                &[("path", &dir.display()), ("e", &e)],
            );
            return;
        }
        let xml_path = dir.join("autounattend.xml");
        if let Err(e) = write_file(&xml_path, build.xml.as_bytes()) {
            self.status = trf("Build: {e}", &[("e", &e)]);
            return;
        }
        for (name, text) in &build.scripts {
            let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
            if let Err(e) = write_file(&dir.join(name), text.as_bytes()) {
                self.status = trf("Build: {e}", &[("e", &e)]);
                return;
            }
        }
        self.status = trf(
            "Built {path} ({rules} rules, {scripts} scripts, {warnings} warnings)",
            &[
                ("path", &xml_path.display()),
                ("rules", &build.rule_ids.len()),
                ("scripts", &build.scripts.len()),
                ("warnings", &build.warnings.len()),
            ],
        );
        self.shown_file = Some((self.status.clone(), xml_path.clone()));
    }

    /// The rule ids of the selection of the active profile: a rule, the rules of a group, or every
    /// rule when nothing (or a form) is selected.
    fn selection_ids(&self) -> Vec<String> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        match &doc.sel {
            Sel::Rule(id) => vec![id.clone()],
            Sel::Group(group) => self
                .catalog
                .rules_under(group)
                .iter()
                .map(|r| r.id.clone())
                .collect(),
            Sel::None | Sel::Form(_) => self.catalog.rules().iter().map(|r| r.id.clone()).collect(),
            Sel::Unknown(_) => Vec::new(),
        }
    }

    /// Shows a policy of an import in the tree of a profile: an import that is not loaded is, the
    /// first profile tab comes to the front, the groups above the policy open and it is selected.
    fn show_in_profile(&mut self, import: &str, rule: &str) {
        if !import.is_empty() && !self.imports.iter().any(|f| f.id == import) {
            if let Err(e) = self.load_import(import) {
                self.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                return;
            }
            self.refresh_import_list();
            self.rebuild_catalog();
        }
        let Some(index) = self.docs.iter().position(|d| matches!(d, Doc::Profile(_))) else {
            self.status = tr("Open a profile first").to_owned();
            return;
        };
        let mut groups = Vec::new();
        let mut group = self.catalog.rule(rule).map(|r| r.group.clone());
        while let Some(g) = group {
            group = self.catalog.group(&g).and_then(|x| x.parent.clone());
            groups.push(g);
        }
        self.active = index;
        if let Some(Doc::Profile(doc)) = self.docs.get_mut(index) {
            doc.expanded.extend(groups);
            doc.select(Sel::Rule(rule.to_owned()));
            doc.focus = None;
            doc.reveal = true;
        }
    }
}
