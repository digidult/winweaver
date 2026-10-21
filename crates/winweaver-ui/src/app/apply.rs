//! The acts of a frame: the keyboard, the tabs and the handler that runs every act.

use crate::*;

impl App {
    /// The question over the window, when there is one.
    pub(crate) fn ask_view(&self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(ask) = &self.ask else {
            return;
        };
        egui::Window::new(ask.title.as_str())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(ask.text.as_str());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(ask.yes.as_str()).clicked() {
                        acts.push(Act::AskYes);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        acts.push(Act::AskNo);
                    }
                });
            });
    }

    pub(crate) fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let mut acts = Vec::new();
        let active = self.active;
        let count = self.docs.len();
        let confirming = self.confirm.is_some();
        let asking = self.ask.is_some();
        let mut focus_search = false;
        ctx.input_mut(|i| {
            let sc = |m, k| KeyboardShortcut::new(m, k);
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::F)) {
                focus_search = true;
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::D)) {
                acts.push(Act::Fold);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::S)) {
                acts.push(Act::SaveAs);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::S)) {
                acts.push(Act::Save);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::O)) {
                acts.push(Act::Open);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::N)) {
                acts.push(Act::NewProfile(None));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::B)) {
                acts.push(Act::ToggleBookmark);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::W)) {
                acts.push(Act::CloseTab(active, false));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::Comma)) {
                acts.push(Act::OpenSettings);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F1)) {
                acts.push(Act::OpenGuide);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F9)) {
                acts.push(Act::Build);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F7)) {
                acts.push(Act::Check);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowLeft)) {
                acts.push(Act::Back);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowRight)) {
                acts.push(Act::Forward);
            }
            if count > 1 && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab)) {
                acts.push(Act::Activate((active + count - 1) % count));
            }
            if count > 1 && i.consume_shortcut(&sc(Modifiers::CTRL, Key::Tab)) {
                acts.push(Act::Activate((active + 1) % count));
            }
            if active > 0
                && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::PageUp))
            {
                acts.push(Act::MoveTab(active, active - 1));
            }
            if active + 1 < count
                && i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::PageDown))
            {
                acts.push(Act::MoveTab(active, active + 1));
            }
            if confirming && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::CancelPlan);
            }
            if asking && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::AskNo);
            }
            for file in &i.raw.dropped_files {
                acts.push(Act::OpenPath(file.path().to_path_buf()));
            }
        });
        if focus_search && let Some(search) = self.active_search() {
            search.open = true;
            search.focus = true;
        }
        acts
    }

    pub(crate) fn apply(&mut self, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Select(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = None;
                    }
                }
                Act::Reveal(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = None;
                    }
                    self.reveal_selection();
                }
                Act::SelectIn(sel, group) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                        doc.focus = group;
                    }
                }
                Act::ShowIn(rule, group) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(Sel::Rule(rule));
                        doc.focus = Some(group);
                    }
                    self.reveal_selection();
                }
                Act::OpenCatalogAt(import, rule) => {
                    self.open_catalog(&import);
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active)
                        && c.id == import
                    {
                        c.group = None;
                        c.sel = Some(rule);
                    }
                    self.reveal_selection();
                }
                Act::Toggle(id, on) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.set_on(&id, on);
                    }
                    self.refresh_active();
                }
                Act::SetGroup(group, on) => {
                    let ids: Vec<String> = self
                        .catalog
                        .rules_under(&group)
                        .iter()
                        .filter(|r| !on || !r.id.starts_with("admx."))
                        .map(|r| r.id.clone())
                        .collect();
                    if let Some(doc) = self.active_profile() {
                        for id in &ids {
                            doc.profile.set_on(id, on);
                        }
                    }
                    self.refresh_active();
                }
                Act::SetParam(id, name, value) => {
                    let default = self
                        .catalog
                        .rule(&id)
                        .and_then(|r| r.params.get(&name))
                        .map(|p| p.default_value());
                    if let Some(doc) = self.active_profile() {
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
                    self.refresh_active();
                }
                Act::Expand(group, open) => {
                    if let Some(doc) = self.active_profile() {
                        if open {
                            doc.expanded.insert(group);
                        } else {
                            doc.expanded.remove(&group);
                        }
                    }
                }
                Act::Activate(i) => {
                    if i < self.docs.len() {
                        self.active = i;
                    }
                }
                Act::MoveTab(from, to) => self.move_tab(from, to),
                Act::CloseTab(i, force) => {
                    if i < self.docs.len() {
                        if self.docs[i].dirty() && !force {
                            self.status = tr("The tab has unsaved changes: save it (Ctrl+S) or close it from the menu discarding them").to_owned();
                        } else {
                            if let Doc::Profile(d) = &self.docs[i] {
                                session::remove_draft(&self.drafts_dir, &d.draft_key);
                            }
                            self.docs.remove(i);
                            if self.active >= self.docs.len() {
                                self.active = self.docs.len().saturating_sub(1);
                            }
                        }
                    }
                }
                Act::NewProfile(preset) => {
                    let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                        Some((name, text)) => Profile::parse(text, name)
                            .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog)),
                        None => Profile::new("New profile", self.catalog.version()),
                    };
                    let doc = ProfileDoc::new(profile, None, &self.catalog);
                    self.docs.push(Doc::Profile(Box::new(doc)));
                    self.active = self.docs.len() - 1;
                }
                Act::Open => {
                    if let Some(path) = pick_any() {
                        self.open_path(path);
                    }
                }
                Act::OpenPath(path) => self.open_path(path),
                Act::OpenSettings => {
                    self.themes = themes::themes(&themes_dir());
                    self.refresh_import_list();
                    self.open_settings();
                }
                Act::ImportOpenCatalog => self.import_open_catalog(),
                Act::OpenSettingsSection(section) => self.open_settings().section = section,
                Act::Save => match self.docs.get(self.active) {
                    Some(Doc::Mcp(_)) => self.persist_mcp(),
                    Some(Doc::Diff(_)) => self.save_diff(),
                    Some(Doc::Theme(_)) => self.save_theme(false),
                    Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                    Some(Doc::RegFile(_)) => self.save_reg_doc(false),
                    Some(Doc::Users(_)) => self.save_users_doc(false),
                    _ => self.save_active(false),
                },
                Act::SaveAs => match self.docs.get(self.active) {
                    Some(Doc::Mcp(_)) => self.save_mcp_as(),
                    Some(Doc::Diff(_)) => self.save_active_diff(),
                    Some(Doc::Theme(_)) => self.save_theme(true),
                    Some(Doc::CatalogEdit(_)) => self.save_catalog_edit(),
                    Some(Doc::RegFile(_)) => self.save_reg_doc(true),
                    Some(Doc::Users(_)) => self.save_users_doc(true),
                    _ => self.save_active(true),
                },
                Act::EditTheme(id) => {
                    let theme = themes::find(&self.themes, &id);
                    self.edit_theme(theme);
                }
                Act::OpenThemeFile => {
                    let dir = themes_dir();
                    let dialog = rfd::FileDialog::new()
                        .add_filter(tr("WinWeaver theme"), &[themes::EXTENSION, "json"])
                        .set_title(tr("Open a theme file"));
                    let dialog = if dir.is_dir() {
                        dialog.set_directory(dir)
                    } else {
                        dialog
                    };
                    if let Some(path) = dialog.pick_file() {
                        self.open_theme_path(path);
                    }
                }
                Act::UseTheme(id) => {
                    let dark = themes::find(&self.themes, &id).dark;
                    self.settings.theme = if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    };
                    self.settings.custom_theme = id;
                    self.settings_dirty = true;
                }
                Act::Back => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(-1);
                    }
                }
                Act::Forward => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(1);
                    }
                }
                Act::ToggleBookmark => {
                    if let Some(mut mark) = self.current_bookmark() {
                        if let Some(j) = self.marks.find(&mark)
                            && !self.marks.items[j].comment.trim().is_empty()
                        {
                            self.ask_remove_mark(j);
                            continue;
                        }
                        mark.created = clock::now_utc_iso();
                        let added = self.marks.toggle(mark);
                        if self.save_marks() {
                            let text = if added {
                                tr("Bookmark added ({n} bookmarks in {path})")
                            } else {
                                tr("Bookmark removed ({n} bookmarks in {path})")
                            };
                            self.status = text
                                .replace("{n}", &self.marks.items.len().to_string())
                                .replace("{path}", &self.marks_path.display().to_string());
                        }
                    }
                }
                Act::OpenMcp => self.open_mcp(),
                Act::McpStart => self.start_mcp(),
                Act::McpStop => {
                    self.mcp.run = None;
                    self.status = tr("The MCP server stopped").to_owned();
                }
                Act::McpRenewToken => match winweaver_mcp::new_token() {
                    Ok(t) => self.mcp.token = t,
                    Err(e) => self.mcp.error = e,
                },
                Act::McpOpen(name) => {
                    let profile = self
                        .mcp
                        .run
                        .as_ref()
                        .and_then(|r| r.server.workspace_profile(&name));
                    if let Some(profile) = profile {
                        let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                        doc.dirty = true;
                        self.docs.push(Doc::Profile(Box::new(doc)));
                        self.active = self.docs.len() - 1;
                    }
                }
                Act::OpenMarks => self.open_marks(),
                Act::GoToMark(i) => self.go_to_mark(i),
                Act::SetMarkComment(i, text) => {
                    if let Some(b) = self.marks.items.get_mut(i) {
                        b.comment = text;
                        self.save_marks();
                    }
                }
                Act::RemoveMark(i) => {
                    if i < self.marks.items.len() {
                        self.marks.items.remove(i);
                        if let Some(m) = self.marks_doc() {
                            m.sel = match m.sel {
                                Some(s) if s == i => None,
                                Some(s) if s > i => Some(s - 1),
                                other => other,
                            };
                        }
                        self.save_marks();
                    }
                }
                Act::RemoveMarkAsk(i) => self.ask_remove_mark(i),
                Act::MarkSelect(i) => {
                    if let Some(m) = self.marks_doc() {
                        m.sel = Some(i);
                    }
                }
                Act::ShowInFolder(path) => {
                    // A folder of the program that is not there yet is made first.
                    if !path.exists() && path.extension().is_none() {
                        let _ = std::fs::create_dir_all(&path);
                    }
                    if let Err(e) = winweaver_win::show_in_folder(&path) {
                        self.status = trf("The folder was not opened: {e}", &[("e", &e)]);
                    }
                }
                Act::AskYes => {
                    if let Some(ask) = self.ask.take() {
                        self.apply(vec![ask.act]);
                    }
                }
                Act::AskNo => self.ask = None,
                Act::Fold => {
                    self.settings.desc_open = !self.settings.desc_open;
                    self.settings_dirty = true;
                }
                Act::Check => {
                    self.check_active();
                    self.messages_open = true;
                }
                Act::ReadPc => self.read_pc(false),
                Act::ReadSelection => self.read_pc(true),
                Act::CheckOnPc => self.check_on_pc(),
                Act::SaveScript(kind) => self.save_script(kind),
                Act::SaveRegFile(revert) => self.save_plan_reg(revert),
                Act::RegFileEdit => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active)
                        && let Some(line) = d.target()
                    {
                        d.start_edit(line);
                    }
                }
                Act::RegFileNew(kind) => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) {
                        d.start_new(kind);
                    }
                }
                Act::AskRegFileRemove => {
                    if let Some(Doc::RegFile(d)) = self.docs.get(self.active)
                        && let Some((lines, text)) = d.removal()
                    {
                        self.ask = Some(Ask {
                            title: tr("Remove from the file").to_owned(),
                            text,
                            yes: tr("Remove").to_owned(),
                            act: Act::RegFileRemove(lines),
                        });
                    }
                }
                Act::RegFileRemove(lines) => {
                    if let Some(Doc::RegFile(d)) = self.docs.get_mut(self.active) {
                        d.remove_lines(&lines);
                    }
                }
                Act::OpenReport => self.open_report(),
                Act::NewUsersFile => self.new_users_doc(Vec::new()),
                Act::UsersOfThisPc => match winweaver_win::users_of_this_pc() {
                    Ok(read) => self.new_users_doc(read),
                    Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
                },
                Act::UsersAdd => {
                    if let Some(d) = self.active_users() {
                        let n = d.file.users.len() + 1;
                        d.add(users::User {
                            name: format!("User{n}"),
                            ..users::User::default()
                        });
                    }
                }
                Act::UsersDuplicate => {
                    if let Some(d) = self.active_users() {
                        d.duplicate();
                    }
                }
                Act::AskUsersRemove => {
                    let found = self
                        .active_users()
                        .and_then(|d| d.selected.zip(d.selected_user().map(|u| u.name.clone())));
                    if let Some((i, name)) = found {
                        self.ask = Some(Ask {
                            title: tr("Remove the account from the file").to_owned(),
                            text: trf(
                                "Remove {name} from the file? An account of a PC stays.",
                                &[("name", &name)],
                            ),
                            yes: tr("Remove").to_owned(),
                            act: Act::UsersRemove(i),
                        });
                    }
                }
                Act::UsersRemove(i) => {
                    if let Some(d) = self.active_users() {
                        d.remove(i);
                    }
                }
                Act::UsersMove(down) => {
                    if let Some(d) = self.active_users() {
                        d.move_selected(down);
                    }
                }
                Act::UsersUnlock => self.users_unlock(false, false),
                Act::UsersSetMaster => self.users_unlock(true, false),
                Act::UsersSetPassword => self.users_unlock(false, true),
                Act::UsersLock => {
                    if let Some(d) = self.active_users() {
                        d.lock();
                    }
                }
                Act::UsersCheck => {
                    if let Some(d) = self.active_users() {
                        d.check();
                        let errors = d.issues.iter().filter(|i| i.error).count();
                        self.status = trf(
                            "Checked {n} accounts: {errors} errors",
                            &[("n", &d.file.users.len()), ("errors", &errors)],
                        );
                    }
                }
                Act::UsersReadPc => match winweaver_win::users_of_this_pc() {
                    Ok(read) => {
                        if let Some(d) = self.active_users() {
                            let n = d.merge(read);
                            self.status = trf("{n} accounts of this PC added", &[("n", &n)]);
                        }
                    }
                    Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
                },
                Act::AskUsersRun(operation, all) => self.ask_users_run(operation, all),
                Act::UsersRun(operation, names) => self.run_users(operation, &names),
                Act::UsersScript => self.save_users_script(),
                Act::UsersIntoProfile => self.users_into_profile(),
                Act::UsersFromProfile => self.users_from_profile(),
                Act::DiffUnsaved => self.diff_unsaved(),
                Act::ImportTemplates(source) => self.start_import(source),
                Act::ToggleImport(id) => {
                    if self.imports.iter().any(|f| f.id == id) {
                        self.unload_import(&id);
                    } else if let Err(e) = self.load_import(&id) {
                        self.status = trf("The import was not loaded: {e}", &[("e", &e)]);
                    }
                    self.refresh_import_list();
                    self.rebuild_catalog();
                }
                Act::RemoveImport(id) => {
                    let _ = std::fs::remove_file(
                        self.imports_dir
                            .join(format!("{id}.{}", admx::CATALOG_EXTENSION)),
                    );
                    self.unload_import(&id);
                    self.refresh_import_list();
                    self.rebuild_catalog();
                }
                Act::LoadCatalogs => self.load_catalogs_for_unknown(),
                Act::AskRemoveImport(id) => {
                    let name = self
                        .import_list
                        .iter()
                        .find(|e| e.id == id)
                        .map_or_else(|| id.clone(), |e| e.name.clone());
                    self.ask = Some(Ask {
                        title: tr("Remove the import").to_owned(),
                        text: trf(
                            "Remove {name}? Its catalog file is deleted from the imports folder, and the options of profiles from it become unknown.",
                            &[("name", &name)],
                        ),
                        yes: tr("Remove").to_owned(),
                        act: Act::RemoveImport(id),
                    });
                }
                Act::AskCloseDiscard(i) => match self.docs.get(i) {
                    Some(d) if d.dirty() => {
                        self.ask = Some(Ask {
                            title: tr("Discard the changes").to_owned(),
                            text: trf(
                                "Close {title} and discard its unsaved changes?",
                                &[("title", &d.title())],
                            ),
                            yes: tr("Discard").to_owned(),
                            act: Act::CloseTab(i, true),
                        });
                    }
                    Some(_) => self.apply(vec![Act::CloseTab(i, true)]),
                    None => {}
                },
                Act::AskForgetRecent => {
                    self.ask = Some(Ask {
                        title: tr("Forget the recent files").to_owned(),
                        text: tr(
                            "Forget the list of the recent files? The files stay where they are.",
                        )
                        .to_owned(),
                        yes: tr("Forget").to_owned(),
                        act: Act::ForgetRecent,
                    });
                }
                Act::ForgetRecent => {
                    self.settings.recent.clear();
                    self.settings_dirty = true;
                }
                Act::AskRenewToken => {
                    self.ask = Some(Ask {
                        title: tr("Renew the token").to_owned(),
                        text: tr("Renew the token of the MCP server? The clients set up with the old one lose their access until they get the new one.")
                            .to_owned(),
                        yes: tr("Renew").to_owned(),
                        act: Act::McpRenewToken,
                    });
                }
                Act::CompareCatalog(other) => self.compare_catalog(other),
                Act::OpenRegistry(path, value) => self.open_registry(&path, value.as_deref()),
                Act::RegistryGo(path, value) => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.go(&*reg, &path, value.as_deref());
                    }
                }
                Act::RegistryExpand(path, open) => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.expand(&*reg, &path, open);
                    }
                }
                Act::RegistryRefresh => {
                    let reg = self.registry.clone();
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.refresh(&*reg);
                    }
                }
                Act::RegistryEdit => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.start_edit(regview::EditKind::Value, "");
                    }
                }
                Act::RegistryNew(kind, type_name) => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.start_edit(kind, type_name);
                    }
                }
                Act::RegistrySave => self.registry_save(),
                Act::RegistrySearch => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active) {
                        d.search.open = !d.search.open;
                        d.search.focus = d.search.open;
                    }
                }
                Act::RegistryFind => self.registry_find(),
                Act::RegistryFindStop => {
                    if let Some(Doc::Registry(d)) = self.docs.get_mut(self.active)
                        && let Some(running) = &d.search.running
                    {
                        running
                            .stop
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                Act::AskRegistryDelete => self.ask_registry_delete(),
                Act::RegistryRights => self.registry_rights(),
                Act::RegistryRightsAdd => self.registry_rights_add(),
                Act::AskRegistryRights => {
                    if let Some(Doc::Registry(d)) = self.docs.get(self.active)
                        && let Some(edit) = &d.rights
                    {
                        self.ask = Some(Ask {
                            title: tr("Change the rights").to_owned(),
                            text: trf(
                                "Change the rights of {path}? A wrong access list can shut Windows or a program out of the key.",
                                &[("path", &edit.path)],
                            ),
                            yes: tr("Change").to_owned(),
                            act: Act::RegistryRightsSave,
                        });
                    }
                }
                Act::RegistryRightsSave => self.registry_rights_save(),
                Act::RegistryDelete(path, value) => self.registry_delete(&path, value.as_deref()),
                Act::BookmarkRegistry => self.bookmark_registry(),
                Act::EditCatalog(which) => self.edit_catalog(which),
                Act::NewCatalog => self.new_catalog(),
                Act::AskCatalogEditRemove(id) => {
                    self.ask = Some(Ask {
                        title: tr("Remove the policy").to_owned(),
                        text: trf(
                            "Remove the policy {id} from the catalog? It leaves the file at the next save.",
                            &[("id", &id)],
                        ),
                        yes: tr("Remove").to_owned(),
                        act: Act::CatalogEditRemove(id),
                    });
                }
                Act::CatalogEditRemove(id) => {
                    if let Some(Doc::CatalogEdit(d)) = self.docs.get_mut(self.active) {
                        d.remove(&id);
                    }
                }
                Act::ExportImport(id) => self.export_import(&id),
                Act::StartRename(id, name) => self.open_settings().renaming = Some((id, name)),
                Act::OpenCatalog(id) => self.open_catalog(&id),
                Act::CatalogSelect(key) => {
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) {
                        if let Some(group) = key.strip_prefix("g:") {
                            c.group = Some(group.to_owned());
                            c.sel = None;
                        } else if key.is_empty() {
                            c.group = None;
                            c.sel = None;
                        } else {
                            c.group = None;
                            c.sel = Some(key);
                        }
                    }
                }
                Act::CatalogReveal(key) => {
                    self.apply(vec![Act::CatalogSelect(key)]);
                    self.reveal_selection();
                }
                Act::CatalogExpand(gid, open) => {
                    if let Some(Doc::Catalog(c)) = self.docs.get_mut(self.active) {
                        if open {
                            c.expanded.insert(gid);
                        } else {
                            c.expanded.remove(&gid);
                        }
                    }
                }
                Act::ShowInProfile(import, rule) => self.show_in_profile(&import, &rule),
                Act::ShowGroupInProfile(group) => {
                    match self.docs.iter().position(|d| matches!(d, Doc::Profile(_))) {
                        Some(index) => {
                            self.active = index;
                            self.apply(vec![Act::Reveal(Sel::Group(group))]);
                        }
                        None => self.status = tr("Open a profile first").to_owned(),
                    }
                }
                Act::CancelRename => self.open_settings().renaming = None,
                Act::RenameImport(id, name) => self.rename_import(&id, &name),
                Act::PlanNative(revert) => self.plan_native(revert),
                Act::PlanUndo => self.plan_undo(),
                Act::ConfirmPlan => self.run_plan(),
                Act::CancelPlan => self.confirm = None,
                Act::Build => self.build_active(),
                Act::OpenGuide => self.open_guide(),
                Act::OpenLicense => self.open_license(),
                Act::Compare => {
                    if let Some(path) = pick_profile() {
                        match Profile::load(&path) {
                            Ok(other) => self.compare_active(other, Some(&path)),
                            Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                        }
                    }
                }
                Act::CompareWith(name) => {
                    if let Some((name, text)) = PRESETS.iter().find(|p| p.0 == name) {
                        match Profile::parse(text, name) {
                            Ok(other) => self.compare_active(other, None),
                            Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                        }
                    }
                }
                Act::ApplyDiff => self.apply_active_diff(),
                Act::SaveDiffAs => self.save_active_diff(),
                Act::DiffSelect(key) => {
                    if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                        d.sel = Some(key);
                    }
                }
                Act::DiffComment(key, text) => {
                    if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                        if text.trim().is_empty() {
                            d.diff.comments.shift_remove(&key);
                        } else {
                            d.diff.comments.insert(key, text);
                        }
                        d.dirty = true;
                    }
                }
                Act::DiffGoTo(key, other) => self.go_to_diff_item(&key, other),
                Act::DiffApplyItem(key) => self.apply_diff_item(&key),
                Act::SetInstall(install) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.install = install;
                    }
                    self.refresh_active();
                }
                Act::SetAccounts(accounts) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.accounts = accounts;
                    }
                    self.refresh_active();
                }
                Act::SetLanguages(languages) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.languages = languages;
                    }
                    self.refresh_active();
                }
                Act::ClearMessages => {
                    if let Some(doc) = self.active_profile() {
                        doc.issues.clear();
                    }
                }
                Act::ToggleMessages => self.messages_open = !self.messages_open,
                Act::SearchCleared => self.reveal_selection(),
                Act::Revealed => match self.docs.get_mut(self.active) {
                    Some(Doc::Profile(doc)) => {
                        doc.reveal = false;
                        doc.follow = false;
                    }
                    Some(Doc::Catalog(c)) => {
                        c.reveal = false;
                        c.follow = false;
                    }
                    Some(Doc::Diff(d)) => {
                        d.reveal = false;
                        d.follow = false;
                    }
                    Some(Doc::Bookmarks(d)) => {
                        d.reveal = false;
                        d.follow = false;
                    }
                    _ => {}
                },
                Act::Follow => match self.docs.get_mut(self.active) {
                    Some(Doc::Profile(doc)) => doc.follow = true,
                    Some(Doc::Catalog(c)) => c.follow = true,
                    Some(Doc::Diff(d)) => d.follow = true,
                    Some(Doc::Bookmarks(d)) => d.follow = true,
                    _ => {}
                },
                Act::OpenDoc(link) => self.open_doc(&link),
                Act::Exit => self.exit = true,
                Act::Message(text) => self.status = text,
            }
        }
    }

    /// Moves a tab to another place of the row; the tab in front stays in front.
    pub(crate) fn move_tab(&mut self, from: usize, to: usize) {
        let n = self.docs.len();
        if from >= n || to >= n || from == to {
            return;
        }
        let doc = self.docs.remove(from);
        self.docs.insert(to, doc);
        self.active = if self.active == from {
            to
        } else if from < self.active && self.active <= to {
            self.active - 1
        } else if to <= self.active && self.active < from {
            self.active + 1
        } else {
            self.active
        };
    }
}
