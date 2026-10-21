//! WinWeaver on the window of WW: what the shell asks of the program (`ww::App`): its documents,
//! its acts, the command bar, the menus, the navigation row, the messages, the status bar, its
//! settings, its keys and the work after a frame.

use eframe::egui::{self, Key, Modifiers, RichText};
use ww::commands::Item;
use ww::{App, Messages, Nav, Shell, ShellAct, ShellDoc};

use crate::commands::{Menu, keys};
use crate::*;

impl App for WinWeaver {
    type Doc = Doc;
    type Act = Act;
    type Extra = Extra;

    const NAME: &'static str = "WinWeaver";
    const BUILD: &'static str = winweaver_core::BUILD;
    const NEW_HINT: &'static str = "New profile (Ctrl+N)";

    fn languages() -> &'static [(&'static str, &'static str)] {
        &LANGUAGES
    }

    fn themes() -> &'static [(&'static str, &'static str)] {
        &THEMES
    }

    fn system_language() -> Option<String> {
        winweaver_win::ui_language()
    }

    fn shell_doc(doc: ShellDoc) -> Doc {
        Doc::Shell(doc)
    }

    fn shell_act(act: Act) -> Result<ShellAct<Self>, Act> {
        match act {
            Act::Shell(a) => Ok(a),
            other => Err(other),
        }
    }

    /// The first tab: the preset Office.
    fn started(&mut self, shell: &mut Shell<Self>) {
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0)
            .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &self.catalog));
        let doc = ProfileDoc::new(profile, None, &self.catalog);
        shell.open_tab(Doc::Profile(Box::new(doc)));
    }

    /// The files next to the executable: the MCP configuration, the bookmarks, the imports.
    fn load(&mut self, shell: &mut Shell<Self>) {
        self.load_mcp_config(shell);
        match Bookmarks::load_or_default(&self.marks_path) {
            Ok(marks) => self.marks = marks,
            Err(e) => shell.status = trf("Bookmarks not read: {e}", &[("e", &e)]),
        }
        self.load_imports(shell);
    }

    fn restored(&mut self, shell: &mut Shell<Self>) {
        if shell.settings.extra.show_guide_at_start {
            self.open_guide(shell);
            shell.settings.extra.show_guide_at_start = false;
            shell.settings_dirty = true;
        }
    }

    fn new_document(&mut self, shell: &mut Shell<Self>) {
        self.apply_file(shell, FileAct::NewProfile(None));
    }

    fn open_path(&mut self, shell: &mut Shell<Self>, path: PathBuf) {
        self.open_any(shell, path);
    }

    fn open_dialog(&self, _shell: &Shell<Self>) -> rfd::FileDialog {
        open_dialog()
    }

    fn save(&mut self, shell: &mut Shell<Self>, ask: bool) {
        match shell.active_doc().map(Doc::kind) {
            Some(DocKind::Mcp) if ask => self.save_mcp_as(shell),
            Some(DocKind::Mcp) => self.persist_mcp(),
            Some(DocKind::Diff) if ask => self.save_active_diff(shell),
            Some(DocKind::Diff) => self.save_diff(shell),
            Some(DocKind::CatalogEdit) => self.save_catalog_edit(shell),
            Some(DocKind::RegFile) => self.save_reg_doc(shell, ask),
            Some(DocKind::Users) => self.save_users_doc(shell, ask),
            _ => self.save_active(shell, ask),
        }
    }

    fn restore_draft(
        &mut self,
        shell: &mut Shell<Self>,
        key: String,
        path: Option<PathBuf>,
        document: serde_json::Value,
    ) -> bool {
        self.restore_profile_draft(shell, key, path, document)
    }

    fn open_link(&mut self, shell: &mut Shell<Self>, link: &str) {
        self.open_doc(shell, link);
    }

    fn search_cleared(&mut self, shell: &mut Shell<Self>) {
        self.reveal_selection(shell);
    }

    fn settings_opened(&mut self, _shell: &mut Shell<Self>) {
        self.refresh_import_list();
    }

    fn draw(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        match shell.active_doc().map(Doc::kind) {
            Some(DocKind::Profile) => self.draw_profile(shell, ui, acts),
            Some(DocKind::Diff) => self.draw_diff(shell, ui, acts),
            Some(DocKind::Markdown) => self.draw_markdown(shell, ui),
            Some(DocKind::Bookmarks) => self.draw_marks(shell, ui, acts),
            Some(DocKind::Mcp) => self.draw_mcp(shell, ui, acts),
            Some(DocKind::Catalog) => self.draw_catalog(shell, ui, acts),
            Some(DocKind::Registry) => self.draw_registry(shell, ui, acts),
            Some(DocKind::CatalogEdit) => self.draw_catalog_editor(shell, ui, acts),
            Some(DocKind::RegFile) => self.draw_reg_file(shell, ui, acts),
            Some(DocKind::Users) => self.draw_users(shell, ui, acts),
            // The settings and the editor of a theme are drawn by the window.
            Some(DocKind::Settings | DocKind::Theme) | None => {}
        }
    }

    fn commands(&self, shell: &Shell<Self>) -> Vec<Item<Act>> {
        self.tab_commands(shell)
    }

    fn menu_popup(&self, shell: &Shell<Self>, menu: usize, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if let Some(menu) = Menu::from_id(menu) {
            self.bar_menu(shell, menu, ui, acts);
        }
    }

    fn menu(&self, shell: &Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        self.menu_items(shell, ui, acts);
    }

    /// The history of a profile tab, the breadcrumb of a profile or a catalog tab, the star of
    /// an option.
    fn nav(&self, shell: &Shell<Self>) -> Option<Nav<Act>> {
        let history = match shell.active_doc() {
            Some(Doc::Profile(doc)) => Some((
                doc.can_go(-1),
                doc.can_go(1),
                Act::Profile(ProfileAct::Back),
                Act::Profile(ProfileAct::Forward),
            )),
            _ => None,
        };
        let star = self
            .current_bookmark(shell)
            .map(|b| (self.marks.find(&b).is_some(), Act::Marks(MarksAct::Toggle)));
        Some(Nav {
            history,
            crumbs: self.nav_crumbs(shell),
            star,
        })
    }

    /// The checks and warnings of a profile tab.
    fn messages(&self, shell: &Shell<Self>) -> Option<Messages<Act>> {
        let Some(Doc::Profile(doc)) = shell.active_doc() else {
            return None;
        };
        if doc.issues.is_empty() {
            return None;
        }
        Some(Messages {
            error: validate::errors(&doc.issues) > 0,
            summary: validate::summary(&doc.issues),
            clear: Act::Profile(ProfileAct::ClearMessages),
        })
    }

    fn messages_body(&self, shell: &Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Profile(doc)) = shell.active_doc() else {
            return;
        };
        let p = style::palette(ui);
        let texts = texts_for(&self.texts, &i18n::current());
        for issue in &doc.issues {
            ui.horizontal(|ui| {
                let (tag, color) = match issue.severity {
                    Severity::Error => (tr("Error"), ui.visuals().error_fg_color),
                    Severity::Warning => (tr("Warning"), p.warn),
                    Severity::Info => (tr("Info"), p.muted),
                };
                ui.label(RichText::new(tag).color(color).size(style::size(ui, 12.0)));
                let (label, sel) = match &issue.target {
                    Target::Rule(id) => (
                        self.catalog.rule(id).map_or_else(
                            || id.clone(),
                            |r| views::rtitle(texts, r).to_owned(),
                        ),
                        Sel::Rule(id.clone()),
                    ),
                    Target::Install => (tr("Installation").to_owned(), Sel::Form(Form::Install)),
                    Target::Accounts => (tr("Accounts").to_owned(), Sel::Form(Form::Accounts)),
                    Target::Languages => (tr("Languages").to_owned(), Sel::Form(Form::Languages)),
                    Target::Profile => (doc.profile.name.clone(), Sel::None),
                    // Their group, unless the settings hide it from the tree.
                    Target::Unknown if shell.settings.extra.hide_unknown => {
                        (doc.profile.name.clone(), Sel::None)
                    }
                    Target::Unknown => (
                        tr("Unknown options").to_owned(),
                        Sel::Group(UNKNOWN_GROUP.to_owned()),
                    ),
                };
                if ui.link(label).clicked() {
                    acts.push(Act::Profile(ProfileAct::Reveal(sel)));
                }
                views::linked_text(
                    ui,
                    &self.catalog,
                    texts,
                    issue.text.as_str(),
                    p.msg_fg,
                    views::RuleLink::Profile,
                    acts,
                );
            });
        }
    }

    /// The state of the profile and the catalog.
    fn status_left(&self, shell: &Shell<Self>) -> Vec<String> {
        let mut texts = Vec::new();
        let state = match shell.active_doc() {
            Some(Doc::Profile(d)) if d.dirty => tr("Modified"),
            Some(Doc::Profile(d)) if d.path.is_some() => tr("Saved"),
            Some(Doc::Profile(_)) => tr("Not saved yet"),
            _ => "",
        };
        if !state.is_empty() {
            texts.push(state.to_owned());
        }
        texts.push(trf(
            "Catalog {version}, {n} rules",
            &[
                ("version", &self.catalog.version()),
                ("n", &self.catalog.rules().len()),
            ],
        ));
        texts
    }

    /// The MCP server, while it runs.
    fn status_right(&self, _shell: &Shell<Self>, ui: &mut egui::Ui) {
        if let Some(run) = &self.mcp.run {
            let p = style::palette(ui);
            ui.label(
                RichText::new(trf(
                    "MCP: {url}, mode {mode}",
                    &[
                        ("url", &run.http.url()),
                        ("mode", &run.server.options().mode.name()),
                    ],
                ))
                .size(style::size(ui, 11.0))
                .color(p.ok),
            );
        }
    }

    fn busy(&self, _shell: &Shell<Self>) -> bool {
        self.pc_read.is_some()
    }

    fn settings_sections(&self) -> Vec<&'static str> {
        SettingsSection::ALL.iter().map(|s| s.title()).collect()
    }

    fn settings_general(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui) {
        self.general_settings(shell, ui);
    }

    fn settings_section(
        &mut self,
        shell: &mut Shell<Self>,
        section: usize,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        match SettingsSection::ALL.get(section) {
            Some(SettingsSection::Templates) => {
                style::block(ui, |ui| self.imports_view(ui, acts));
            }
            Some(SettingsSection::Files) => {
                style::block(ui, |ui| self.files_settings(shell, ui));
            }
            None => {}
        }
    }

    /// The keys of WinWeaver beyond the ones of the window; Escape cancels a plan waiting for
    /// its confirmation.
    fn shortcuts(&self, _shell: &Shell<Self>, input: &mut egui::InputState) -> Vec<Act> {
        let mut acts = Vec::new();
        let keyed: [(&keys::Keys, Act); 7] = [
            (&keys::FOLD, Act::Profile(ProfileAct::Fold)),
            (&keys::BOOKMARK, Act::Marks(MarksAct::Toggle)),
            (&keys::GUIDE, Act::File(FileAct::OpenGuide)),
            (&keys::BUILD, Act::Profile(ProfileAct::Build)),
            (&keys::CHECK, Act::Profile(ProfileAct::Check)),
            (&keys::BACK, Act::Profile(ProfileAct::Back)),
            (&keys::FORWARD, Act::Profile(ProfileAct::Forward)),
        ];
        for (key, act) in keyed {
            if input.consume_shortcut(&key.shortcut()) {
                acts.push(act);
            }
        }
        if self.confirm.is_some() && input.consume_key(Modifiers::NONE, Key::Escape) {
            acts.push(Act::Pc(PcAct::Cancel));
        }
        acts
    }

    fn apply(&mut self, shell: &mut Shell<Self>, act: Act) {
        match act {
            Act::Profile(a) => self.apply_profile(shell, a),
            Act::File(a) => self.apply_file(shell, a),
            Act::Settings(a) => self.apply_settings(a),
            Act::Pc(a) => self.apply_pc(shell, a),
            Act::Users(a) => self.apply_users(shell, a),
            Act::Diff(a) => self.apply_diff(shell, a),
            Act::Marks(a) => self.apply_marks(shell, a),
            Act::Mcp(a) => self.apply_mcp(shell, a),
            Act::Catalog(a) => self.apply_catalog(shell, a),
            Act::Import(a) => self.apply_import(shell, a),
            Act::Registry(a) => self.apply_registry(shell, a),
            Act::RegFile(a) => self.apply_reg_file(shell, a),
            // The window runs its own acts (`shell_act`); one that comes here goes back to it.
            Act::Shell(a) => shell.pending.push(Act::Shell(a)),
        }
    }

    fn dialogs(&mut self, _shell: &mut Shell<Self>, ctx: &egui::Context, acts: &mut Vec<Act>) {
        self.confirm_view(ctx, acts);
        self.users_import_view(ctx, acts);
    }

    fn after_frame(&mut self, shell: &mut Shell<Self>, ctx: &egui::Context) {
        self.poll_running(shell, ctx);
        self.poll_import(shell, ctx);
        self.poll_pc_read(shell, ctx);
        self.poll_registry_search(shell, ctx);
        self.persist_mcp();
    }
}

impl WinWeaver {
    /// The breadcrumb of the active tab: a label and what a click on it does.
    fn nav_crumbs(&self, shell: &Shell<WinWeaver>) -> Vec<(String, Option<Act>)> {
        match shell.active_doc() {
            Some(Doc::Profile(doc)) => self
                .crumbs(shell)
                .into_iter()
                .map(|(label, sel)| {
                    let act = match sel {
                        Sel::Rule(_) => Act::Profile(ProfileAct::SelectIn(sel, doc.focus.clone())),
                        _ => Act::Profile(ProfileAct::Select(sel)),
                    };
                    (label, Some(act))
                })
                .collect(),
            Some(Doc::Catalog(c)) => {
                let Some(file) = c.catalog(&self.imports) else {
                    return Vec::new();
                };
                let mut out = vec![(
                    file.name.clone(),
                    Some(Act::Catalog(CatalogAct::Select(None))),
                )];
                let lang = i18n::current();
                let rule = c
                    .sel
                    .as_deref()
                    .and_then(|id| file.rules.iter().find(|r| r.id == id));
                let mut group = match rule {
                    Some(r) => Some(r.group.clone()),
                    None => c.group.clone(),
                };
                let mut chain = Vec::new();
                while let Some(g) = group {
                    let Some(found) = file.groups.iter().find(|x| x.id == g) else {
                        break;
                    };
                    let title = catalogview::group_title(file, &lang, found).to_owned();
                    chain.push((
                        title,
                        Some(Act::Catalog(CatalogAct::Select(Some(TreeKey::Group(g))))),
                    ));
                    group = found.parent.clone();
                }
                out.extend(chain.into_iter().rev());
                if let Some(rule) = rule {
                    let (title, _) = catalogview::rule_texts(file, &lang, rule);
                    out.push((title.to_owned(), None));
                }
                out
            }
            _ => Vec::new(),
        }
    }
}
