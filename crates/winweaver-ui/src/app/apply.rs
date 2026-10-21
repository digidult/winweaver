//! The acts of a frame: the keyboard, the question over the window, the handler that hands
//! every act to its subsystem, and the acts of the chrome itself.

use crate::*;

impl App {
    /// The question over the window, when there is one.
    pub(crate) fn ask_view(&self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(ask) = &self.question else {
            return;
        };
        widgets::Modal::new(ask.title.as_str()).show(ctx, |ui| {
            ui.label(ask.text.as_str());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(ask.yes.as_str()).clicked() {
                    acts.push(Act::Answer(true));
                }
                if ui.button(tr("Cancel")).clicked() {
                    acts.push(Act::Answer(false));
                }
            });
        });
    }

    pub(crate) fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        use crate::commands::keys;
        use egui::{Key, Modifiers};
        let mut acts = Vec::new();
        let active = self.active;
        let count = self.docs.len();
        let confirming = self.confirm.is_some();
        let asking = self.question.is_some();
        let mut focus_search = false;
        ctx.input_mut(|i| {
            if i.consume_shortcut(&keys::FIND.shortcut()) {
                focus_search = true;
            }
            if i.consume_shortcut(&keys::FOLD.shortcut()) {
                acts.push(Act::Nav(NavAct::Fold));
            }
            if i.consume_shortcut(&keys::SAVE_AS.shortcut()) {
                acts.push(Act::File(FileAct::SaveAs));
            }
            if i.consume_shortcut(&keys::SAVE.shortcut()) {
                acts.push(Act::File(FileAct::Save));
            }
            if i.consume_shortcut(&keys::OPEN.shortcut()) {
                acts.push(Act::File(FileAct::Open));
            }
            if i.consume_shortcut(&keys::NEW.shortcut()) {
                acts.push(Act::File(FileAct::NewProfile(None)));
            }
            if i.consume_shortcut(&keys::BOOKMARK.shortcut()) {
                acts.push(Act::Marks(MarksAct::Toggle));
            }
            if i.consume_shortcut(&keys::CLOSE.shortcut()) {
                acts.push(Act::Nav(NavAct::CloseTab(active, false)));
            }
            if i.consume_shortcut(&keys::SETTINGS.shortcut()) {
                acts.push(Act::Settings(SettingsAct::Open));
            }
            if i.consume_shortcut(&keys::GUIDE.shortcut()) {
                acts.push(Act::File(FileAct::OpenGuide));
            }
            if i.consume_shortcut(&keys::BUILD.shortcut()) {
                acts.push(Act::Profile(ProfileAct::Build));
            }
            if i.consume_shortcut(&keys::CHECK.shortcut()) {
                acts.push(Act::Profile(ProfileAct::Check));
            }
            if i.consume_shortcut(&keys::BACK.shortcut()) {
                acts.push(Act::Profile(ProfileAct::Back));
            }
            if i.consume_shortcut(&keys::FORWARD.shortcut()) {
                acts.push(Act::Profile(ProfileAct::Forward));
            }
            if count > 1 && i.consume_shortcut(&keys::PREVIOUS_TAB.shortcut()) {
                acts.push(Act::Nav(NavAct::Activate((active + count - 1) % count)));
            }
            if count > 1 && i.consume_shortcut(&keys::NEXT_TAB.shortcut()) {
                acts.push(Act::Nav(NavAct::Activate((active + 1) % count)));
            }
            if active > 0 && i.consume_shortcut(&keys::TAB_LEFT.shortcut()) {
                acts.push(Act::Nav(NavAct::MoveTab(active, active - 1)));
            }
            if active + 1 < count && i.consume_shortcut(&keys::TAB_RIGHT.shortcut()) {
                acts.push(Act::Nav(NavAct::MoveTab(active, active + 1)));
            }
            if confirming && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::Pc(PcAct::Cancel));
            }
            if asking && i.consume_key(Modifiers::NONE, Key::Escape) {
                acts.push(Act::Answer(false));
            }
            for file in &i.raw.dropped_files {
                acts.push(Act::File(FileAct::OpenPath(file.path().to_path_buf())));
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
                Act::Profile(a) => self.apply_profile(a),
                Act::Nav(a) => self.apply_nav(a),
                Act::File(a) => self.apply_file(a),
                Act::Settings(a) => self.apply_settings(a),
                Act::Theme(a) => self.apply_theme(a),
                Act::Pc(a) => self.apply_pc(a),
                Act::Users(a) => self.apply_users(a),
                Act::Diff(a) => self.apply_diff(a),
                Act::Marks(a) => self.apply_marks(a),
                Act::Mcp(a) => self.apply_mcp(a),
                Act::Catalog(a) => self.apply_catalog(a),
                Act::Import(a) => self.apply_import(a),
                Act::Registry(a) => self.apply_registry(a),
                Act::RegFile(a) => self.apply_reg_file(a),
                Act::Ask(question) => self.ask(*question),
                Act::Answer(yes) => {
                    if let Some(question) = self.question.take()
                        && yes
                    {
                        self.apply(vec![question.act]);
                    }
                }
            }
        }
    }

    /// Shows a question over the window; a yes runs its act, a no drops it.
    pub(crate) fn ask(&mut self, question: Question) {
        self.question = Some(question);
    }

    /// The act of closing a tab discarding its changes: a question first when it has some.
    pub(crate) fn discard_act(&self, i: usize) -> Act {
        match self.docs.get(i) {
            Some(d) if d.dirty() => crate::question::close_discard(&d.title(), i).into(),
            _ => Act::Nav(NavAct::CloseTab(i, true)),
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

    /// Runs an act of the acts of the chrome: the tabs, the panels, the view of a tab, a message.
    pub(crate) fn apply_nav(&mut self, act: NavAct) {
        match act {
            NavAct::Activate(i) => {
                if i < self.docs.len() {
                    self.active = i;
                }
            }
            NavAct::MoveTab(from, to) => self.move_tab(from, to),
            NavAct::CloseTab(i, force) => {
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
            NavAct::ShowInFolder(path) => {
                // A folder of the program that is not there yet is made first.
                if !path.exists() && path.extension().is_none() {
                    let _ = std::fs::create_dir_all(&path);
                }
                if let Err(e) = winweaver_win::show_in_folder(&path) {
                    self.status = trf("The folder was not opened: {e}", &[("e", &e)]);
                }
            }
            NavAct::Fold => {
                self.settings.extra.desc_open = !self.settings.extra.desc_open;
                self.settings_dirty = true;
            }
            NavAct::ToggleMessages => self.messages_open = !self.messages_open,
            NavAct::SearchCleared => self.reveal_selection(),
            NavAct::Revealed => match self.docs.get_mut(self.active) {
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
            NavAct::Follow => match self.docs.get_mut(self.active) {
                Some(Doc::Profile(doc)) => doc.follow = true,
                Some(Doc::Catalog(c)) => c.follow = true,
                Some(Doc::Diff(d)) => d.follow = true,
                Some(Doc::Bookmarks(d)) => d.follow = true,
                _ => {}
            },
            NavAct::OpenDoc(link) => self.open_doc(&link),
            NavAct::Exit => self.exit = true,
            NavAct::Message(text) => self.status = text,
        }
    }
}
