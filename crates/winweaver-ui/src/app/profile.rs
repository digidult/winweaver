//! The profile tab: the selection, the tree, the check and the build.

use crate::*;

impl WinWeaver {
    pub(crate) fn active_profile<'a>(
        &self,
        shell: &'a mut Shell<WinWeaver>,
    ) -> Option<&'a mut ProfileDoc> {
        match shell.docs.get_mut(shell.active) {
            Some(Doc::Profile(doc)) => Some(doc),
            _ => None,
        }
    }

    /// After the search of a tab is cleared, the selected item stays in view: the groups above
    /// it open and its row is scrolled to at the next frame.
    pub(crate) fn reveal_selection(&mut self, shell: &mut Shell<WinWeaver>) {
        let catalog = &self.catalog;
        match shell.docs.get_mut(shell.active) {
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
            Some(Doc::Bookmarks(d)) => d.reveal = true,
            Some(Doc::Mcp(_)) => self.mcp.journal_reveal = true,
            Some(Doc::RegFile(d)) => d.follow = true,
            Some(Doc::Users(d)) => d.follow = true,
            _ => {}
        }
    }

    pub(crate) fn refresh_active(&mut self, shell: &mut Shell<WinWeaver>) {
        let catalog = &self.catalog;
        if let Some(Doc::Profile(doc)) = shell.docs.get_mut(shell.active) {
            doc.refresh(catalog);
        }
    }

    pub(crate) fn crumbs(&self, shell: &Shell<WinWeaver>) -> Vec<(String, Sel)> {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
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

    pub(crate) fn check_active(&mut self, shell: &mut Shell<WinWeaver>) {
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = shell.docs.get_mut(shell.active) else {
            shell.status = tr("Nothing to check: the active tab is not a profile").to_owned();
            return;
        };
        doc.issues = validate::check(catalog, &doc.profile, &doc.resolution);
        shell.status = validate::summary(&doc.issues);
    }

    /// Checks the profile, builds the answer file and writes it with its scripts into the
    /// output folder; errors and warnings go to the messages panel.
    pub(crate) fn build_active(&mut self, shell: &mut Shell<WinWeaver>) {
        self.check_active(shell);
        let catalog = &self.catalog;
        let Some(Doc::Profile(doc)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        let errors = validate::errors(&doc.issues);
        if errors > 0 {
            shell.status = trf("Fix the errors before the build: {n}", &[("n", &errors)]);
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
                shell.status = trf("Build failed: {e}", &[("e", &e)]);
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
            shell.status = trf(
                "Build: cannot create {path}: {e}",
                &[("path", &dir.display()), ("e", &e)],
            );
            return;
        }
        let xml_path = dir.join(render::ANSWER_FILE);
        if let Err(e) = build.write(&xml_path) {
            shell.status = trf("Build: {e}", &[("e", &e)]);
            return;
        }
        shell.status = trf(
            "Built {path} ({rules} rules, {scripts} scripts, {warnings} warnings)",
            &[
                ("path", &xml_path.display()),
                ("rules", &build.rule_ids.len()),
                ("scripts", &build.scripts.len()),
                ("warnings", &build.warnings.len()),
            ],
        );
        shell.shown_file = Some((shell.status.clone(), xml_path.clone()));
    }

    /// The rule ids of the selection of the active profile: a rule, the rules of a group, or every
    /// rule when nothing (or a form) is selected.
    pub(crate) fn selection_ids(&self, shell: &Shell<WinWeaver>) -> Vec<String> {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
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
    pub(crate) fn show_in_profile(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        import: &str,
        rule: &str,
    ) {
        if !import.is_empty() && !self.imports.iter().any(|f| f.id == import) {
            if let Err(e) = self.load_import(shell, import) {
                shell.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                return;
            }
            self.refresh_import_list();
            self.rebuild_catalog(shell);
        }
        let Some(index) = shell.docs.iter().position(|d| matches!(d, Doc::Profile(_))) else {
            shell.status = tr("Open a profile first").to_owned();
            return;
        };
        let mut groups = Vec::new();
        let mut group = self.catalog.rule(rule).map(|r| r.group.clone());
        while let Some(g) = group {
            group = self.catalog.group(&g).and_then(|x| x.parent.clone());
            groups.push(g);
        }
        shell.active = index;
        if let Some(Doc::Profile(doc)) = shell.docs.get_mut(index) {
            doc.expanded.extend(groups);
            doc.select(Sel::Rule(rule.to_owned()));
            doc.focus = None;
            doc.reveal = true;
        }
    }

    /// Runs an act of the acts of the profile tab.
    pub(crate) fn apply_profile(&mut self, shell: &mut Shell<WinWeaver>, act: ProfileAct) {
        match act {
            ProfileAct::Select(sel) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.select(sel);
                    doc.focus = None;
                }
            }
            ProfileAct::Reveal(sel) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.select(sel);
                    doc.focus = None;
                }
                self.reveal_selection(shell);
            }
            ProfileAct::SelectIn(sel, group) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.select(sel);
                    doc.focus = group;
                }
            }
            ProfileAct::ShowIn(rule, group) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.select(Sel::Rule(rule));
                    doc.focus = Some(group);
                }
                self.reveal_selection(shell);
            }
            ProfileAct::Toggle(id, on) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.profile.set_on(&id, on);
                }
                self.refresh_active(shell);
            }
            ProfileAct::SetGroup(group, on) => {
                let ids: Vec<String> = self
                    .catalog
                    .rules_under(&group)
                    .iter()
                    .filter(|r| !on || !r.id.starts_with("admx."))
                    .map(|r| r.id.clone())
                    .collect();
                if let Some(doc) = self.active_profile(shell) {
                    for id in &ids {
                        doc.profile.set_on(id, on);
                    }
                }
                self.refresh_active(shell);
            }
            ProfileAct::SetParam(id, name, value) => {
                let default = self
                    .catalog
                    .rule(&id)
                    .and_then(|r| r.params.get(&name))
                    .map(|p| p.default_value());
                if let Some(doc) = self.active_profile(shell) {
                    let choice = doc.profile.rules.entry(id).or_default();
                    if default
                        .as_ref()
                        .is_some_and(|d| check::same_value(d, &value))
                    {
                        choice.params.shift_remove(&name);
                    } else {
                        choice.params.insert(name, value);
                    }
                }
                self.refresh_active(shell);
            }
            ProfileAct::OnlyOn => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.only_on = !doc.only_on;
                    doc.reveal = true;
                }
            }
            ProfileAct::Expand(group, open) => {
                if let Some(doc) = self.active_profile(shell) {
                    if open {
                        doc.expanded.insert(group);
                    } else {
                        doc.expanded.remove(&group);
                    }
                }
            }
            ProfileAct::Back => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.go(-1);
                }
            }
            ProfileAct::Forward => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.go(1);
                }
            }
            ProfileAct::Check => {
                self.check_active(shell);
                shell.messages_open = true;
            }
            ProfileAct::ShowInProfile(import, rule) => self.show_in_profile(shell, &import, &rule),
            ProfileAct::ShowGroupInProfile(group) => {
                match shell.docs.iter().position(|d| matches!(d, Doc::Profile(_))) {
                    Some(index) => {
                        shell.active = index;
                        self.apply(shell, Act::Profile(ProfileAct::Reveal(Sel::Group(group))));
                    }
                    None => shell.status = tr("Open a profile first").to_owned(),
                }
            }
            ProfileAct::Build => self.build_active(shell),
            ProfileAct::SetInstall(install) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.profile.install = *install;
                }
                self.refresh_active(shell);
            }
            ProfileAct::SetAccounts(accounts) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.profile.accounts = accounts;
                }
                self.refresh_active(shell);
            }
            ProfileAct::SetLanguages(languages) => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.profile.languages = languages;
                }
                self.refresh_active(shell);
            }
            ProfileAct::Fold => {
                shell.settings.extra.desc_open = !shell.settings.extra.desc_open;
                shell.settings_dirty = true;
            }
            ProfileAct::ClearMessages => {
                if let Some(doc) = self.active_profile(shell) {
                    doc.issues.clear();
                }
            }
        }
    }
}
