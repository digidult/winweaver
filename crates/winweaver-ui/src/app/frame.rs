//! The frame: the chrome around the active tab, the tab itself, the dialogs over the window
//! and the work after the frame.

use crate::*;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.exit {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.apply_theme(ui.ctx());
        self.color_title_bar(ui, frame);
        let title = self.window_title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        let mut acts = self.shortcuts(ui.ctx());
        let (tabs_frame, tool_frame, nav_frame, status_frame) = Self::chrome_frames(ui);
        egui::Panel::top("tabs")
            .frame(tabs_frame)
            .show_separator_line(false)
            .show(ui, |ui| self.tabs_row(ui, &mut acts));
        let compact = ui.available_width() < COMPACT_WIDTH;
        if !compact && self.has_tools() {
            let search_here = !self.has_nav() && self.has_search();
            let hint = self.search_hint_now();
            egui::Panel::top("toolbar")
                .frame(tool_frame)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.toolbar(ui, &mut acts, false);
                        if search_here {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    self.search_field(ui, &mut acts, hint, 0.0);
                                },
                            );
                        }
                    });
                });
        }
        if self.has_nav() || (compact && self.has_tools()) {
            egui::Panel::top("nav")
                .frame(nav_frame)
                .show(ui, |ui| self.nav_row(ui, &mut acts, compact));
        }
        self.messages_panel(ui, &mut acts);
        egui::Panel::bottom("status")
            .frame(status_frame)
            .show(ui, |ui| self.status_bar(ui, &mut acts));
        match self.docs.get(self.active).map(Doc::kind) {
            Some(DocKind::Profile) => self.draw_profile(ui, &mut acts),
            Some(DocKind::Diff) => self.draw_diff(ui, &mut acts),
            Some(DocKind::Markdown) => self.draw_markdown(ui),
            Some(DocKind::Bookmarks) => self.draw_marks(ui, &mut acts),
            Some(DocKind::Mcp) => self.draw_mcp(ui, &mut acts),
            Some(DocKind::Settings) => self.draw_settings(ui),
            Some(DocKind::Catalog) => self.draw_catalog(ui, &mut acts),
            Some(DocKind::Theme) => self.draw_theme(ui, &mut acts),
            Some(DocKind::Registry) => self.draw_registry(ui, &mut acts),
            Some(DocKind::CatalogEdit) => self.draw_catalog_editor(ui, &mut acts),
            Some(DocKind::RegFile) => self.draw_reg_file(ui, &mut acts),
            Some(DocKind::Users) => self.draw_users(ui, &mut acts),
            None => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label(tr("No document is open. Use Menu, New or Ctrl+N."));
                });
            }
        }
        self.confirm_view(ui.ctx(), &mut acts);
        self.ask_view(ui.ctx(), &mut acts);
        self.users_import_view(ui.ctx(), &mut acts);
        let links: Vec<String> = ui.ctx().output_mut(|o| {
            let mut found = Vec::new();
            o.commands.retain(|c| match c {
                egui::OutputCommand::OpenUrl(u) if is_doc_link(&u.url) => {
                    found.push(u.url.clone());
                    false
                }
                _ => true,
            });
            found
        });
        acts.extend(links.into_iter().map(Act::OpenDoc));
        acts.append(&mut self.pending_acts);
        self.apply(acts);
        self.poll_running(ui.ctx());
        self.poll_import(ui.ctx());
        self.poll_pc_read(ui.ctx());
        self.poll_registry_search(ui.ctx());
        let closing = self.exit || ui.ctx().input(|i| i.viewport().close_requested());
        self.write_drafts(closing);
        if closing {
            self.remember_tabs();
        }
        self.persist_settings();
        self.persist_mcp();
        self.log_status();
    }
}

impl App {
    /// The profile tab: the tree on the left, the detail panel in the middle.
    fn draw_profile(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let settings = &self.settings;
        let p = style::palette(ui);
        egui::Panel::left("tree")
            .resizable(true)
            .default_size(380.0)
            .frame(
                egui::Frame::new()
                    .fill(p.card)
                    .inner_margin(egui::Margin::symmetric(0, 4)),
            )
            .show(ui, |ui| {
                views::tree_view(
                    ui,
                    &self.catalog,
                    texts,
                    doc,
                    &doc.search,
                    settings.inline_switches,
                    !settings.hide_unknown,
                    acts,
                );
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| {
                views::detail_view(
                    ui,
                    &self.catalog,
                    texts,
                    &self.imports,
                    doc,
                    settings,
                    acts,
                );
            });
    }

    /// The diff tab: the list of the differences on the left, the selected one in the middle.
    fn draw_diff(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let marked = self
            .current_bookmark()
            .is_some_and(|b| self.marks.find(&b).is_some());
        let Some(Doc::Diff(doc)) = self.docs.get(self.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let p = style::palette(ui);
        let (default, range) = list_split(ui);
        egui::Panel::left("diff-list")
            .resizable(true)
            .default_size(default)
            .size_range(range)
            .frame(egui::Frame::new().fill(p.card))
            .show(ui, |ui| {
                diffview::diff_view(ui, &self.catalog, texts, doc, acts);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| {
                diffview::diff_detail(
                    ui,
                    &self.catalog,
                    texts,
                    doc,
                    &self.settings,
                    marked,
                    acts,
                );
            });
    }

    fn draw_markdown(&mut self, ui: &mut egui::Ui) {
        if let Some(Doc::Markdown(doc)) = self.docs.get_mut(self.active) {
            egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc));
        }
    }

    /// The MCP tab: the configuration, and with the server running the journal under it and
    /// the selected call on the right.
    fn draw_mcp(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Mcp(doc)) = self.docs.get(self.active) else {
            return;
        };
        let search = &doc.search;
        let panel = &mut self.mcp;
        let imports: Vec<String> = self.imports.iter().map(|f| f.id.clone()).collect();
        let catalog = &self.catalog;
        let texts = texts_for(&self.texts, &i18n::current());
        let p = style::palette(ui);
        let settings_frame = egui::Frame::new().fill(p.bg);
        if panel.run.is_some() {
            let max = (ui.available_height() - 120.0).max(160.0);
            egui::Panel::top("mcp-settings")
                .resizable(true)
                .default_size(420.0)
                .size_range(120.0..=max)
                .frame(settings_frame)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| mcpview::mcp_view(ui, panel, acts));
                });
            if panel.detail_open {
                egui::Panel::right("mcp-detail")
                    .resizable(true)
                    .default_size(380.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.bg)
                            .inner_margin(egui::Margin::same(12)),
                    )
                    .show(ui, |ui| {
                        mcpview::detail_view(ui, panel, catalog, texts, &imports, acts)
                    });
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.card))
                .show(ui, |ui| mcpview::journal_view(ui, panel, search));
        } else {
            egui::CentralPanel::default()
                .frame(settings_frame)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| mcpview::mcp_view(ui, panel, acts));
                });
        }
    }

    /// The bookmark tab: the list on the left, the selected bookmark in the middle.
    fn draw_marks(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Bookmarks(doc)) = self.docs.get(self.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let p = style::palette(ui);
        let (default, range) = list_split(ui);
        egui::Panel::left("marks-list")
            .resizable(true)
            .default_size(default)
            .size_range(range)
            .frame(egui::Frame::new().fill(p.card))
            .show(ui, |ui| {
                marksview::marks_view(
                    ui,
                    &self.catalog,
                    texts,
                    &self.imports,
                    &self.marks,
                    &self.marks_path,
                    &doc.search,
                    doc,
                    acts,
                );
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| {
                marksview::marks_detail(
                    ui,
                    &self.catalog,
                    texts,
                    &self.imports,
                    &self.marks,
                    doc,
                    &self.settings,
                    acts,
                );
            });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        let p = style::palette(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| self.settings_view(ui));
    }

    /// The tab of an import: its tree and the selected policy.
    fn draw_catalog(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let shown = match self.docs.get(self.active) {
            Some(Doc::Catalog(doc)) => doc.catalog(&self.imports).map(|f| (f, doc)),
            _ => None,
        };
        match shown {
            Some((file, doc)) => catalogview::catalog_view(
                ui,
                file,
                doc,
                &self.catalog,
                &self.imports,
                &self.settings,
                acts,
            ),
            None => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label(tr("This import is no longer there."));
                });
            }
        }
    }

    fn draw_theme(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let families = style::installed_families();
        let p = style::palette(ui);
        if let Some(Doc::Theme(doc)) = self.docs.get_mut(self.active) {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.bg))
                .show(ui, |ui| {
                    themeview::theme_view(ui, doc, &families, acts);
                });
        }
    }

    fn draw_registry(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let texts = texts_for(&self.texts, &i18n::current());
        if let Some(Doc::Registry(doc)) = self.docs.get_mut(self.active) {
            regview::registry_view(ui, doc, &self.catalog, texts, acts);
        }
    }

    fn draw_catalog_editor(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if let Some(Doc::CatalogEdit(doc)) = self.docs.get_mut(self.active) {
            cataledit::editor_view(ui, doc, acts);
        }
    }

    fn draw_reg_file(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let texts = texts_for(&self.texts, &i18n::current());
        if let Some(Doc::RegFile(doc)) = self.docs.get_mut(self.active) {
            regfileview::regfile_view(ui, doc, &self.catalog, texts, &self.imports, acts);
        }
    }

    fn draw_users(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if let Some(Doc::Users(doc)) = self.docs.get_mut(self.active) {
            usersview::users_view(ui, doc, acts);
        }
    }
}
