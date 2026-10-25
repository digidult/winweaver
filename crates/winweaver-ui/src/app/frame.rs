//! The active tab drawn by its kind.

use crate::*;

impl WinWeaver {
    /// The profile tab: the tree on the left, the detail panel in the middle.
    pub(crate) fn draw_profile(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let settings = &shell.settings;
        let p = style::palette(ui);
        widgets::side_panel(ui, "tree", 380.0, |ui| {
            views::tree_view(
                ui,
                &self.catalog,
                texts,
                doc,
                &doc.search,
                settings.extra.inline_switches,
                !settings.extra.hide_unknown,
                acts,
            );
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| {
                views::detail_view(ui, &self.catalog, texts, &self.imports, doc, settings, acts);
            });
    }

    #[cfg(feature = "full")]
    /// The diff tab: the list of the differences on the left, the selected one in the middle.
    pub(crate) fn draw_diff(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let marked = self
            .current_bookmark(shell)
            .is_some_and(|b| self.full.marks.find(&b).is_some());
        let Some(Doc::Diff(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let p = style::palette(ui);
        let (default, range) = widgets::list_split(ui);
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
                diffview::diff_detail(ui, &self.catalog, texts, doc, &shell.settings, marked, acts);
            });
    }

    pub(crate) fn draw_markdown(&mut self, shell: &mut Shell<WinWeaver>, ui: &mut egui::Ui) {
        let wrap = shell.settings.wrap_lines;
        let page = shell.settings.page_width;
        if let Some(Doc::Markdown(doc)) = shell.docs.get_mut(shell.active) {
            egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc, wrap, page));
        }
    }

    #[cfg(feature = "full")]
    /// The MCP tab: the configuration, and with the server running the journal under it and
    /// the selected call on the right.
    pub(crate) fn draw_mcp(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let Some(Doc::Mcp(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let search = &doc.search;
        let panel = &mut self.full.mcp;
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

    #[cfg(feature = "full")]
    /// The bookmark tab: the list on the left, the selected bookmark in the middle.
    pub(crate) fn draw_marks(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let Some(Doc::Bookmarks(doc)) = shell.docs.get(shell.active) else {
            return;
        };
        let texts = texts_for(&self.texts, &i18n::current());
        let p = style::palette(ui);
        let (default, range) = widgets::list_split(ui);
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
                    &self.full.marks,
                    &self.full.marks_path,
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
                    &self.full.marks,
                    doc,
                    &shell.settings,
                    acts,
                );
            });
    }

    #[cfg(feature = "full")]
    /// The tab of an import: its tree and the selected policy.
    pub(crate) fn draw_catalog(
        &self,
        shell: &Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let shown = match shell.docs.get(shell.active) {
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
                &shell.settings,
                acts,
            ),
            None => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label(tr("This import is no longer there."));
                });
            }
        }
    }

    #[cfg(feature = "full")]
    pub(crate) fn draw_registry(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let texts = texts_for(&self.texts, &i18n::current());
        if let Some(Doc::Registry(doc)) = shell.docs.get_mut(shell.active) {
            regview::registry_view(ui, doc, &self.catalog, texts, acts);
        }
    }

    #[cfg(feature = "full")]
    pub(crate) fn draw_catalog_editor(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        if let Some(Doc::CatalogEdit(doc)) = shell.docs.get_mut(shell.active) {
            cataledit::editor_view(ui, doc, acts);
        }
    }

    #[cfg(feature = "full")]
    pub(crate) fn draw_reg_file(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        let texts = texts_for(&self.texts, &i18n::current());
        let options = regfileview::Options {
            tree: shell.settings.extra.regfile_tree,
            writers_open: shell.settings.extra.writers_open,
        };
        if let Some(Doc::RegFile(doc)) = shell.docs.get_mut(shell.active) {
            let (catalog, imports) = (&self.catalog, &self.imports);
            regfileview::regfile_view(ui, doc, catalog, texts, imports, &options, acts);
        }
    }

    #[cfg(feature = "full")]
    pub(crate) fn draw_users(
        &mut self,
        shell: &mut Shell<WinWeaver>,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
    ) {
        if let Some(Doc::Users(doc)) = shell.docs.get_mut(shell.active) {
            usersview::users_view(ui, doc, acts);
        }
    }
}
