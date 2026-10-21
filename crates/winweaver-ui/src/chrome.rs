//! The chrome of the window around the documents (`docs/spec/01-ui.md` sections 1 to 5 and 13):
//! the tab row with the menu, the command bar of the active tab (its own commands, icons above
//! the labels), the navigation row where it means something (back and forward, the breadcrumb,
//! the star, the search of the tab), the message bar with its own height, and the status bar.

use eframe::egui::{self, Align, Align2, CornerRadius, Layout, Margin, RichText, Sense, Stroke};

use crate::i18n::{tr, trf};
use crate::style::{self, Palette};
use crate::{
    Act, App, Doc, Form, PRESETS, ScriptKind, Sel, Settings, SettingsSection, Tab, texts_for,
    views,
};
use winweaver_core::validate::{self, Severity, Target};

/// A frame of a bar: the card colour with a line under it.
fn bar_frame(p: &Palette, margin: Margin) -> egui::Frame {
    egui::Frame::new().fill(p.card).inner_margin(margin)
}

impl App {
    /// The frame colours of the panels of the chrome.
    fn palette_now(ui: &egui::Ui) -> Palette {
        style::palette(ui)
    }

    /// The tab row: the menu button, the tabs with their icons, the new tab button.
    pub(crate) fn tabs_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let p = Self::palette_now(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let menu = style::icon_button(ui, "\u{2630}", tr("Menu"), true, Some(p.text));
            egui::Popup::menu(&menu).show(|ui| {
                ui.set_min_width(260.0);
                self.menu_items(ui, acts);
            });
            ui.add_space(4.0);
            let tabs: Vec<egui::Response> = self
                .docs
                .iter()
                .enumerate()
                .map(|(i, doc)| self.tab(ui, &p, i, doc, acts))
                .collect();
            let rects: Vec<egui::Rect> = tabs.iter().map(|r| r.rect).collect();
            let pointer = ui.ctx().pointer_interact_pos();
            if let (Some(from), Some(pos)) = (tabs.iter().position(|r| r.dragged()), pointer) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                let (_, x) = drop_place(&rects, from, pos.x);
                let row = rects[from];
                ui.painter().vline(
                    x,
                    row.top() + 4.0..=row.bottom() - 4.0,
                    Stroke::new(2.0, p.accent),
                );
            }
            if let (Some(from), Some(pos)) = (tabs.iter().position(|r| r.drag_stopped()), pointer) {
                let (to, _) = drop_place(&rects, from, pos.x);
                if to != from {
                    acts.push(Act::MoveTab(from, to));
                }
            }
            if style::icon_button(ui, "+", tr("New profile (Ctrl+N)"), true, None).clicked() {
                acts.push(Act::NewProfile(None));
            }
        });
    }

    /// A tab of the row: a click brings it to the front, a drag moves it, the right button opens
    /// a menu that moves or closes it.
    fn tab(
        &self,
        ui: &mut egui::Ui,
        p: &Palette,
        i: usize,
        doc: &Doc,
        acts: &mut Vec<Act>,
    ) -> egui::Response {
        let active = i == self.active;
        let title = doc.title();
        let font = style::font(ui, 12.5);
        let text_w = ui
            .painter()
            .layout_no_wrap(title.clone(), font.clone(), p.text)
            .size()
            .x
            .min(200.0);
        let dirty = doc.dirty();
        let width = 12.0 + 16.0 + if dirty { 10.0 } else { 0.0 } + text_w + 8.0 + 22.0;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click_and_drag());
        let close_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 15.0, rect.center().y),
            egui::vec2(18.0, 18.0),
        );
        let close = ui.interact(close_rect, response.id.with("close"), Sense::click());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let corners = CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            };
            if active {
                painter.rect_filled(rect, corners, p.card);
            } else if response.hovered() {
                painter.rect_filled(rect, corners, p.hover);
            }
            let color = if active { p.text } else { p.muted };
            let mut x = rect.left() + 12.0;
            painter.text(
                egui::pos2(x + 6.0, rect.center().y),
                Align2::CENTER_CENTER,
                doc.kind().icon(),
                style::font(ui, 12.0),
                color,
            );
            x += 18.0;
            if dirty {
                painter.circle_filled(egui::pos2(x + 2.0, rect.center().y), 3.0, p.accent);
                x += 10.0;
            }
            let galley = painter.layout_no_wrap(title.clone(), font, color);
            let clip = egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + text_w, rect.bottom()),
            );
            painter.with_clip_rect(clip).galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                color,
            );
            if close.hovered() {
                painter.rect_filled(close_rect, CornerRadius::same(4), p.hover);
            }
            painter.text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "\u{2715}",
                style::font(ui, 10.0),
                if active || response.hovered() {
                    p.muted
                } else {
                    p.off
                },
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, &title)
        });
        close.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr("Close (Ctrl+W)"))
        });
        if close.clicked() {
            acts.push(Act::CloseTab(i, false));
        } else if response.clicked() {
            acts.push(Act::Activate(i));
        }
        if response.middle_clicked() {
            acts.push(Act::CloseTab(i, false));
        }
        let last = self.docs.len().saturating_sub(1);
        response.context_menu(|ui| {
            if ui
                .add_enabled(
                    i > 0,
                    egui::Button::new(tr("Move the tab left (Ctrl+Shift+PgUp)")),
                )
                .clicked()
            {
                acts.push(Act::MoveTab(i, i - 1));
                ui.close();
            }
            if ui
                .add_enabled(
                    i < last,
                    egui::Button::new(tr("Move the tab right (Ctrl+Shift+PgDn)")),
                )
                .clicked()
            {
                acts.push(Act::MoveTab(i, i + 1));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Close (Ctrl+W)")).clicked() {
                acts.push(Act::CloseTab(i, false));
                ui.close();
            }
        });
        let hint = doc.path().map(|p| p.display().to_string());
        match hint {
            Some(h) => response.on_hover_text(h),
            None => response,
        }
    }

    /// The items of the menu button.
    fn menu_items(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let mut item = |ui: &mut egui::Ui, text: &str, keys: &str, act: Act| {
            let mut button = egui::Button::new(text);
            if !keys.is_empty() {
                button = button.shortcut_text(keys);
            }
            if ui.add(button).clicked() {
                acts.push(act);
                ui.close();
            }
        };
        ui.menu_button(tr("New"), |ui| {
            item(ui, tr("Empty profile"), "Ctrl+N", Act::NewProfile(None));
            ui.separator();
            for (name, _) in PRESETS {
                item(
                    ui,
                    &trf("Profile from the preset {name}", &[("name", &name)]),
                    "",
                    Act::NewProfile(Some(name)),
                );
            }
            ui.separator();
            item(ui, tr("Users file"), "", Act::NewUsersFile);
            item(
                ui,
                tr("Users file of the accounts of this PC"),
                "",
                Act::UsersOfThisPc,
            );
        });
        item(ui, tr("Open..."), "Ctrl+O", Act::Open);
        ui.add_enabled_ui(!self.settings.recent.is_empty(), |ui| {
            ui.menu_button(tr("Open recent"), |ui| {
                for path in &self.settings.recent {
                    item(
                        ui,
                        &path.display().to_string(),
                        "",
                        Act::OpenPath(path.clone()),
                    );
                }
            });
        });
        item(ui, tr("Save"), "Ctrl+S", Act::Save);
        item(ui, tr("Save as..."), "Ctrl+Shift+S", Act::SaveAs);
        item(
            ui,
            tr("Close tab"),
            "Ctrl+W",
            Act::CloseTab(self.active, false),
        );
        item(
            ui,
            tr("Close tab, discard changes"),
            "",
            Act::AskCloseDiscard(self.active),
        );
        ui.separator();
        item(ui, tr("Guide"), "F1", Act::OpenGuide);
        item(ui, tr("License"), "", Act::OpenLicense);
        item(ui, tr("Bookmarks"), "", Act::OpenMarks);
        item(
            ui,
            tr("Registry"),
            "",
            Act::OpenRegistry("HKEY_LOCAL_MACHINE".to_owned(), None),
        );
        item(ui, tr("MCP server"), "", Act::OpenMcp);
        item(ui, tr("Settings"), "Ctrl+,", Act::OpenSettings);
        ui.separator();
        item(
            ui,
            tr("About"),
            "",
            Act::Message(trf(
                "WinWeaver {build}, catalog {catalog}, settings {path}",
                &[
                    ("build", &winweaver_core::BUILD),
                    ("catalog", &self.catalog.version()),
                    ("path", &Settings::path().display()),
                ],
            )),
        );
        item(ui, tr("Exit"), "", Act::Exit);
    }

    /// Whether the active tab has commands in the command bar.
    pub(crate) fn has_tools(&self) -> bool {
        self.docs
            .get(self.active)
            .is_none_or(|d| d.kind().has_tools())
    }

    /// The command bar of the active tab; `compact` draws the icons alone.
    pub(crate) fn toolbar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let tool = |ui: &mut egui::Ui, icon: &str, text: &str, keys: &str, enabled: bool| {
            let hint = if keys.is_empty() {
                text.to_owned()
            } else {
                format!("{text} ({keys})")
            };
            style::tool_button(ui, icon, text, &hint, enabled, compact, false).clicked()
        };
        let split = |ui: &mut egui::Ui, icon: &str, text: &str| {
            style::tool_button(ui, icon, text, text, true, compact, true)
        };
        let sep = |ui: &mut egui::Ui| {
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(2.0);
        };
        match self.docs.get(self.active) {
            Some(Doc::Profile(_)) => {
                if tool(ui, "\u{1F4C4}", tr("New"), "Ctrl+N", true) {
                    acts.push(Act::NewProfile(None));
                }
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
                sep(ui);
                if tool(ui, "\u{2714}", tr("Check"), "F7", true) {
                    acts.push(Act::Check);
                }
                if tool(ui, "\u{2699}", tr("Build"), "F9", true) {
                    acts.push(Act::Build);
                }
                sep(ui);
                let pc = split(ui, "\u{1F5A5}", tr("This PC"));
                egui::Popup::menu(&pc).show(|ui| {
                    ui.set_min_width(300.0);
                    this_pc_menu(ui, acts);
                });
                let compare = split(ui, "\u{21C6}", tr("Compare"));
                egui::Popup::menu(&compare).show(|ui| {
                    ui.set_min_width(220.0);
                    if ui.button(tr("Compare with a file...")).clicked() {
                        acts.push(Act::Compare);
                        ui.close();
                    }
                    ui.separator();
                    for (name, _) in PRESETS {
                        if ui
                            .button(trf("With the preset {name}", &[("name", &name)]))
                            .clicked()
                        {
                            acts.push(Act::CompareWith(name));
                            ui.close();
                        }
                    }
                    ui.separator();
                    let dirty = self.docs.get(self.active).is_some_and(Doc::dirty);
                    let unsaved = egui::Button::new(tr("Unsaved changes"));
                    if ui.add_enabled(dirty, unsaved).clicked() {
                        acts.push(Act::DiffUnsaved);
                        ui.close();
                    }
                });
                sep(ui);
                let catalogs = split(ui, "\u{1F4E6}", tr("Catalogs"));
                egui::Popup::menu(&catalogs).show(|ui| {
                    ui.set_min_width(300.0);
                    self.catalogs_menu(ui, acts);
                });
            }
            Some(Doc::Diff(_)) => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
                if tool(ui, "\u{1F4BE}", tr("Save as"), "Ctrl+Shift+S", true) {
                    acts.push(Act::SaveDiffAs);
                }
                sep(ui);
                if tool(ui, "\u{2714}", tr("Apply"), "", true) {
                    acts.push(Act::ApplyDiff);
                }
            }
            Some(Doc::Markdown(_)) | Some(Doc::Bookmarks(_)) => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
            }
            Some(Doc::Mcp(_)) => {
                let running = self.mcp.run.is_some();
                if tool(ui, "\u{25B6}", tr("Start"), "", !running) {
                    acts.push(Act::McpStart);
                }
                if tool(ui, "\u{25A0}", tr("Stop"), "", running) {
                    acts.push(Act::McpStop);
                }
            }
            Some(Doc::Theme(_)) => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::OpenThemeFile);
                }
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
                if tool(ui, "\u{1F4BE}", tr("Save as"), "Ctrl+Shift+S", true) {
                    acts.push(Act::SaveAs);
                }
            }
            Some(Doc::Catalog(c)) if c.file.is_some() => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
                sep(ui);
                if tool(ui, "\u{1F4E5}", tr("Import"), "", true) {
                    acts.push(Act::ImportOpenCatalog);
                }
                self.compare_tool(ui, acts, compact);
                if tool(ui, "\u{270F}", tr("Edit"), "", true) {
                    acts.push(Act::EditCatalog(crate::CatalogRef::Tab(self.active)));
                }
            }
            Some(Doc::Catalog(c)) => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
                sep(ui);
                let selected = c.sel.clone();
                if tool(
                    ui,
                    "\u{27A1}",
                    tr("Show in the profile"),
                    "",
                    selected.is_some(),
                ) && let Some(rule) = selected
                {
                    acts.push(Act::ShowInProfile(c.id.clone(), rule));
                }
                if tool(ui, "\u{1F4E4}", tr("Export"), "", true) {
                    acts.push(Act::ExportImport(c.id.clone()));
                }
                self.compare_tool(ui, acts, compact);
                if tool(ui, "\u{270F}", tr("Edit"), "", true) {
                    acts.push(Act::EditCatalog(crate::CatalogRef::Import(c.id.clone())));
                }
            }
            Some(Doc::CatalogEdit(_)) => {
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
            }
            Some(Doc::RegFile(d)) => {
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
                if tool(ui, "\u{1F4BE}", tr("Save as"), "Ctrl+Shift+S", true) {
                    acts.push(Act::SaveAs);
                }
                sep(ui);
                let key = d.selected_key();
                if tool(ui, "\u{1F4C1}", tr("New key"), "", true) {
                    acts.push(Act::RegFileNew(crate::regfileview::EditKind::Key));
                }
                if tool(ui, "\u{2795}", tr("New value"), "", key.is_some()) {
                    acts.push(Act::RegFileNew(crate::regfileview::EditKind::Value));
                }
                if tool(ui, "\u{270F}", tr("Edit"), "", d.target().is_some()) {
                    acts.push(Act::RegFileEdit);
                }
                if tool(ui, "\u{1F5D1}", tr("Delete"), "", d.removal().is_some()) {
                    acts.push(Act::AskRegFileRemove);
                }
                sep(ui);
                if tool(
                    ui,
                    "\u{1F5C2}",
                    tr("Open in the registry"),
                    "",
                    key.is_some(),
                ) && let Some((_, path)) = key
                {
                    acts.push(Act::OpenRegistry(path, d.selected_value()));
                }
            }
            Some(Doc::Users(d)) => {
                use winweaver_core::users::Operation;
                if tool(ui, "\u{1F4BE}", tr("Save"), "Ctrl+S", true) {
                    acts.push(Act::Save);
                }
                if tool(ui, "\u{1F4BE}", tr("Save as"), "Ctrl+Shift+S", true) {
                    acts.push(Act::SaveAs);
                }
                sep(ui);
                let one = d.selected_user().is_some();
                if tool(ui, "\u{2795}", tr("Add"), "", true) {
                    acts.push(Act::UsersAdd);
                }
                if tool(ui, "\u{29C9}", tr("Duplicate"), "", one) {
                    acts.push(Act::UsersDuplicate);
                }
                if tool(ui, "\u{1F5D1}", tr("Remove"), "", one) {
                    acts.push(Act::AskUsersRemove);
                }
                if tool(ui, "\u{2191}", tr("Up"), "", one) {
                    acts.push(Act::UsersMove(false));
                }
                if tool(ui, "\u{2193}", tr("Down"), "", one) {
                    acts.push(Act::UsersMove(true));
                }
                sep(ui);
                if d.unlocked() {
                    if tool(ui, "\u{1F513}", tr("Lock"), "", true) {
                        acts.push(Act::UsersLock);
                    }
                    if tool(ui, "\u{1F511}", tr("Password"), "", one) {
                        acts.push(Act::UsersSetPassword);
                    }
                    if tool(ui, "\u{1F510}", tr("Master password"), "", true) {
                        acts.push(Act::UsersSetMaster);
                    }
                } else if tool(ui, "\u{1F512}", tr("Unlock"), "", true) {
                    acts.push(Act::UsersUnlock);
                }
                sep(ui);
                if tool(ui, "\u{2714}", tr("Check"), "", true) {
                    acts.push(Act::UsersCheck);
                }
                if tool(ui, "\u{1F5A5}", tr("Read this PC"), "", true) {
                    acts.push(Act::UsersReadPc);
                }
                let create = Operation::Create {
                    update_existing: false,
                };
                if tool(ui, "\u{25B6}", tr("Create on this PC"), "", d.unlocked()) {
                    acts.push(Act::AskUsersRun(create, true));
                }
                if tool(ui, "\u{1F511}", tr("Apply passwords"), "", d.unlocked()) {
                    acts.push(Act::AskUsersRun(Operation::Passwords, true));
                }
                if tool(ui, "\u{1F4DC}", tr("Script"), "", true) {
                    acts.push(Act::UsersScript);
                }
            }
            Some(Doc::Registry(d)) => {
                if tool(ui, "\u{21BB}", tr("Refresh"), "", true) {
                    acts.push(Act::RegistryRefresh);
                }
                sep(ui);
                if tool(ui, "\u{1F4C1}", tr("New key"), "", true) {
                    acts.push(Act::RegistryNew(crate::regview::EditKind::NewKey, ""));
                }
                if tool(ui, "\u{2795}", tr("New value"), "", true) {
                    acts.push(Act::RegistryNew(
                        crate::regview::EditKind::NewValue,
                        "REG_SZ",
                    ));
                }
                if tool(ui, "\u{270F}", tr("Edit"), "", d.selected().is_some()) {
                    acts.push(Act::RegistryEdit);
                }
                if tool(ui, "\u{1F5D1}", tr("Delete"), "", true) {
                    acts.push(Act::AskRegistryDelete);
                }
                sep(ui);
                if tool(ui, "\u{2605}", tr("Bookmark"), "", true) {
                    acts.push(Act::BookmarkRegistry);
                }
                if tool(ui, "\u{1F50D}", tr("Find"), "", true) {
                    acts.push(Act::RegistrySearch);
                }
                if tool(ui, "\u{1F512}", tr("Permissions"), "", true) {
                    acts.push(Act::RegistryRights);
                }
            }
            Some(Doc::Settings(_)) => {}
            None => {
                if tool(ui, "\u{1F4C4}", tr("New"), "Ctrl+N", true) {
                    acts.push(Act::NewProfile(None));
                }
                if tool(ui, "\u{1F4C2}", tr("Open"), "Ctrl+O", true) {
                    acts.push(Act::Open);
                }
            }
        }
    }

    /// Compare in the bar of a catalog tab: a menu of the loaded imports and the tabs of catalog
    /// files to compare the catalog of the tab with.
    fn compare_tool(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let own = match self.docs.get(self.active) {
            Some(Doc::Catalog(c)) if c.file.is_none() => Some(c.id.clone()),
            _ => None,
        };
        let mut others: Vec<(String, crate::CatalogRef)> = self
            .imports
            .iter()
            .filter(|f| Some(&f.id) != own.as_ref())
            .map(|f| (f.name.clone(), crate::CatalogRef::Import(f.id.clone())))
            .collect();
        for (i, d) in self.docs.iter().enumerate() {
            if i != self.active
                && let Doc::Catalog(c) = d
                && let Some(f) = &c.file
            {
                others.push((f.name.clone(), crate::CatalogRef::Tab(i)));
            }
        }
        let label = tr("Compare");
        let compare = style::tool_button(ui, "\u{21C6}", label, label, true, compact, true);
        egui::Popup::menu(&compare).show(|ui| {
            ui.set_min_width(300.0);
            if others.is_empty() {
                ui.weak(tr(
                    "Load or open another catalog to compare it with this one",
                ));
            }
            for (name, other) in others {
                if ui.button(name).clicked() {
                    acts.push(Act::CompareCatalog(other));
                    ui.close();
                }
            }
        });
    }

    /// The menu of the policy templates: the imports to open in a tab and the import commands.
    fn catalogs_menu(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        if self.imports.is_empty() {
            ui.weak(tr("No policy templates imported yet."));
        }
        for f in &self.imports {
            if ui.button(&f.name).clicked() {
                acts.push(Act::OpenCatalog(f.id.clone()));
                ui.close();
            }
        }
        ui.separator();
        for (text, source) in [
            (
                tr("Import the templates of this Windows"),
                crate::ImportSource::ThisWindows,
            ),
            (tr("Import a folder..."), crate::ImportSource::Folder),
            (
                tr("Import a catalog file..."),
                crate::ImportSource::CatalogFile,
            ),
            (tr("Import an archive..."), crate::ImportSource::Archive),
        ] {
            if ui.button(text).clicked() {
                acts.push(Act::ImportTemplates(source));
                ui.close();
            }
        }
        if ui.button(tr("New catalog")).clicked() {
            acts.push(Act::NewCatalog);
            ui.close();
        }
        ui.separator();
        if ui.button(tr("Settings of the policy templates")).clicked() {
            acts.push(Act::OpenSettingsSection(SettingsSection::Templates));
            ui.close();
        }
    }

    /// The breadcrumb of the active tab: a label and what a click on it does.
    fn nav_crumbs(&self) -> Vec<(String, Option<Act>)> {
        match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => self
                .crumbs()
                .into_iter()
                .map(|(label, sel)| {
                    let act = match sel {
                        Sel::Rule(_) => Act::SelectIn(sel, doc.focus.clone()),
                        _ => Act::Select(sel),
                    };
                    (label, Some(act))
                })
                .collect(),
            Some(Doc::Catalog(c)) => {
                let Some(file) = c.catalog(&self.imports) else {
                    return Vec::new();
                };
                let mut out = vec![(file.name.clone(), Some(Act::CatalogSelect(String::new())))];
                let lang = crate::i18n::current();
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
                    let title = crate::catalogview::group_title(file, &lang, found).to_owned();
                    chain.push((title, Some(Act::CatalogSelect(format!("g:{g}")))));
                    group = found.parent.clone();
                }
                out.extend(chain.into_iter().rev());
                if let Some(rule) = rule {
                    let (title, _) = crate::catalogview::rule_texts(file, &lang, rule);
                    out.push((title.to_owned(), None));
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// Whether the active tab has a navigation row: a history, a breadcrumb or a search.
    pub(crate) fn has_nav(&self) -> bool {
        self.docs
            .get(self.active)
            .is_some_and(|d| d.kind().has_nav())
    }

    /// Whether the active tab has something to search.
    pub(crate) fn has_search(&self) -> bool {
        self.docs
            .get(self.active)
            .is_some_and(|d| d.kind().has_search())
    }

    /// The hint of the search field of the active tab.
    pub(crate) fn search_hint_now(&self) -> &'static str {
        self.docs
            .get(self.active)
            .map_or("", |d| d.kind().search_hint())
    }

    /// The navigation row of the active tab: the history, the breadcrumb (shortened to the first
    /// and the last parts with a menu of the others when the row is narrow), the star, and at the
    /// right end the search and, in a narrow window, the command bar as icons.
    pub(crate) fn nav_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let p = Self::palette_now(ui);
        let history = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => Some((doc.can_go(-1), doc.can_go(1))),
            _ => None,
        };
        let crumbs = self.nav_crumbs();
        let current = self.current_bookmark();
        let star = current.is_some();
        let marked = current.is_some_and(|b| self.marks.find(&b).is_some());
        let hint = self.docs.get(self.active).map_or("", search_hint);
        let search_open = self.active_search().is_some_and(|s| s.open);
        let tools = compact && self.has_tools();
        let tools_id = egui::Id::new("nav-tools-width");
        let tools_w = if tools {
            ui.data(|d| d.get_temp::<f32>(tools_id)).unwrap_or(320.0)
        } else {
            0.0
        };
        let search_w = if hint.is_empty() {
            0.0
        } else if search_open {
            240.0
        } else {
            34.0
        };
        let clone_act = |a: &Act| match a {
            Act::Select(s) => Some(Act::Select(s.clone())),
            Act::SelectIn(s, g) => Some(Act::SelectIn(s.clone(), g.clone())),
            Act::CatalogSelect(s) => Some(Act::CatalogSelect(s.clone())),
            _ => None,
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if let Some((back, forward)) = history {
                if style::icon_button(ui, "\u{2190}", tr("Back (Alt+Left)"), back, None).clicked() {
                    acts.push(Act::Back);
                }
                if style::icon_button(ui, "\u{2192}", tr("Forward (Alt+Right)"), forward, None)
                    .clicked()
                {
                    acts.push(Act::Forward);
                }
                ui.add_space(6.0);
            }
            let star_w = if star { 34.0 } else { 0.0 };
            let room = (ui.available_width() - tools_w - search_w - star_w - 32.0).max(80.0);
            let shown = fit_crumbs(ui, &crumbs, room);
            let hidden: Vec<usize> = (0..crumbs.len())
                .filter(|i| !shown.contains(&Some(*i)))
                .collect();
            for (k, entry) in shown.iter().enumerate() {
                if k > 0 {
                    ui.label(style::muted(ui, "\u{203A}", 13.0));
                }
                match entry {
                    Some(i) => {
                        let (label, act) = &crumbs[*i];
                        let last = *i + 1 == crumbs.len();
                        let text = if last {
                            style::strong(ui, label.as_str(), 13.0)
                        } else {
                            RichText::new(label.as_str())
                        };
                        let clicked = if last && shown.len() == 1 {
                            ui.scope(|ui| {
                                ui.set_max_width(room);
                                ui.add(egui::Label::new(text).truncate().sense(Sense::click()))
                            })
                            .inner
                            .clicked()
                        } else {
                            style::flat_button(ui, text).clicked()
                        };
                        if clicked && let Some(a) = act.as_ref().and_then(clone_act) {
                            acts.push(a);
                        }
                    }
                    None => {
                        let more = style::flat_button(ui, RichText::new("\u{2026}"))
                            .on_hover_text(tr("The other parts of the path"));
                        egui::Popup::menu(&more).show(|ui| {
                            for i in &hidden {
                                let (label, act) = &crumbs[*i];
                                if ui.button(label.as_str()).clicked() {
                                    if let Some(a) = act.as_ref().and_then(clone_act) {
                                        acts.push(a);
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                }
            }
            if star {
                let (icon, label) = if marked {
                    ("\u{2605}", tr("Remove the bookmark (Ctrl+B)"))
                } else {
                    ("\u{2606}", tr("Bookmark this (Ctrl+B)"))
                };
                if style::icon_button(ui, icon, label, true, Some(p.accent)).clicked() {
                    acts.push(Act::ToggleBookmark);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let reserve = if tools { tools_w + 12.0 } else { 0.0 };
                self.search_field(ui, acts, hint, reserve);
                if tools {
                    ui.separator();
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(tools_w, 30.0), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    self.toolbar(&mut child, acts, true);
                    let used = child.min_rect().width().ceil();
                    if (used - tools_w).abs() > 0.5 {
                        ui.data_mut(|d| d.insert_temp(tools_id, used));
                        ui.ctx().request_repaint();
                    }
                }
            });
        });
    }

    /// The search of the active tab, at the right end of the navigation row.
    /// The search of the active tab; `reserve` is the width the row keeps for what comes after it.
    pub(crate) fn search_field(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
        hint: &'static str,
        reserve: f32,
    ) {
        if hint.is_empty() {
            return;
        }
        let history = self.search_history.clone();
        let Some(search) = self.active_search() else {
            return;
        };
        if !search.open {
            if style::icon_button(ui, "\u{1F50D}", tr(hint), true, None).clicked() {
                search.open = true;
                search.focus = true;
            }
            return;
        }
        let had_text = !search.text.trim().is_empty();
        let mut remember = None;
        if !history.is_empty() {
            let more = style::icon_button(ui, "\u{25BE}", tr("Recent searches"), true, None);
            egui::Popup::menu(&more).show(|ui| {
                for h in &history {
                    if ui.button(h).clicked() {
                        search.text = h.clone();
                        ui.close();
                    }
                }
            });
        }
        let id = egui::Id::new("search-field");
        let width = (ui.available_width() - 40.0 - reserve).clamp(160.0, 560.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut search.text)
                .id(id)
                .hint_text(tr(hint))
                .margin(Margin::symmetric(8, 4))
                .desired_width(width),
        );
        if search.focus {
            ui.memory_mut(|m| m.request_focus(id));
            search.focus = false;
        }
        let close = style::icon_button(ui, "\u{2715}", tr("Clear and close (Esc)"), true, None);
        let escaped = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
        if close.clicked() || escaped {
            search.text.clear();
            search.open = false;
        } else if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            remember = Some(search.text.trim().to_owned());
        }
        if had_text && search.text.trim().is_empty() {
            acts.push(Act::SearchCleared);
        }
        if let Some(text) = remember
            && crate::remember_search(&mut self.search_history, &text)
        {
            self.settings.search_history = self.search_history.clone();
            self.settings_dirty = true;
        }
    }

    /// The message bar of the checks and warnings, folded to one line or open with its own
    /// height that the user drags.
    pub(crate) fn messages_panel(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        if doc.issues.is_empty() {
            return;
        }
        let p = Self::palette_now(ui);
        let texts = texts_for(&self.texts, &crate::i18n::current());
        let frame = egui::Frame::new()
            .fill(p.msg_bg)
            .stroke(Stroke::new(1.0, p.msg_border))
            .inner_margin(Margin::symmetric(12, 4));
        let open = self.messages_open;
        let header = |ui: &mut egui::Ui, acts: &mut Vec<Act>| {
            ui.horizontal(|ui| {
                let errors = validate::errors(&doc.issues);
                let icon = if errors > 0 { "\u{26D4}" } else { "\u{26A0}" };
                ui.label(RichText::new(icon).color(p.msg_fg));
                ui.label(RichText::new(validate::summary(&doc.issues)).color(p.msg_fg));
                let toggle = if open { tr("Hide") } else { tr("Show") };
                if ui.link(toggle).clicked() {
                    acts.push(Act::ToggleMessages);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if style::icon_button(
                        ui,
                        "\u{2715}",
                        tr("Hide the messages"),
                        true,
                        Some(p.msg_fg),
                    )
                    .clicked()
                    {
                        acts.push(Act::ClearMessages);
                    }
                });
            });
        };
        if !open {
            egui::Panel::top("messages-bar")
                .frame(frame)
                .show_separator_line(false)
                .show(ui, |ui| header(ui, acts));
            return;
        }
        let max = (ui.available_height() * 0.6).max(120.0);
        egui::Panel::top("messages")
            .frame(frame)
            .resizable(true)
            .default_size(150.0)
            .size_range(60.0..=max)
            .show(ui, |ui| {
                header(ui, acts);
                egui::ScrollArea::vertical()
                    .id_salt("messages-list")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for issue in &doc.issues {
                            ui.horizontal(|ui| {
                                let (tag, color) = match issue.severity {
                                    Severity::Error => (tr("Error"), ui.visuals().error_fg_color),
                                    Severity::Warning => (tr("Warning"), p.warn),
                                    Severity::Info => (tr("Info"), p.muted),
                                };
                                ui.label(
                                    RichText::new(tag).color(color).size(style::size(ui, 12.0)),
                                );
                                let (label, sel) = match &issue.target {
                                    Target::Rule(id) => (
                                        self.catalog.rule(id).map_or_else(
                                            || id.clone(),
                                            |r| views::rtitle(texts, r).to_owned(),
                                        ),
                                        Sel::Rule(id.clone()),
                                    ),
                                    Target::Install => {
                                        (tr("Installation").to_owned(), Sel::Form(Form::Install))
                                    }
                                    Target::Accounts => {
                                        (tr("Accounts").to_owned(), Sel::Form(Form::Accounts))
                                    }
                                    Target::Languages => {
                                        (tr("Languages").to_owned(), Sel::Form(Form::Languages))
                                    }
                                    Target::Profile => (doc.profile.name.clone(), Sel::None),
                                    // Their group, unless the settings hide it from the tree.
                                    Target::Unknown if self.settings.hide_unknown => {
                                        (doc.profile.name.clone(), Sel::None)
                                    }
                                    Target::Unknown => (
                                        tr("Unknown options").to_owned(),
                                        Sel::Group(crate::UNKNOWN_GROUP.to_owned()),
                                    ),
                                };
                                if ui.link(label).clicked() {
                                    acts.push(Act::Reveal(sel));
                                }
                                let link = views::RuleLink::Profile;
                                let text = issue.text.as_str();
                                views::linked_text(
                                    ui,
                                    &self.catalog,
                                    texts,
                                    text,
                                    p.msg_fg,
                                    link,
                                    acts,
                                );
                            });
                        }
                    });
            });
    }

    /// The status bar: the state of the document, the catalog, the MCP server, the last message.
    pub(crate) fn status_bar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let p = Self::palette_now(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            let state = match self.docs.get(self.active) {
                Some(Doc::Profile(d)) if d.dirty => tr("Modified"),
                Some(Doc::Profile(d)) if d.path.is_some() => tr("Saved"),
                Some(Doc::Profile(_)) => tr("Not saved yet"),
                _ => "",
            };
            if !state.is_empty() {
                ui.label(style::muted(ui, state, 11.0));
            }
            ui.label(style::muted(
                ui,
                trf(
                    "Catalog {version}, {n} rules",
                    &[
                        ("version", &self.catalog.version()),
                        ("n", &self.catalog.rules().len()),
                    ],
                ),
                11.0,
            ));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(run) = &self.mcp.run {
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
                // A file the message names, shown in its folder.
                if let Some((text, path)) = &self.shown_file
                    && text == &self.status
                {
                    let open = ui.small_button(tr("Open the folder"));
                    if open.on_hover_text(path.display().to_string()).clicked() {
                        acts.push(Act::ShowInFolder(path.clone()));
                    }
                }
                ui.label(style::muted(ui, self.status.as_str(), 11.0));
                if self.pc_read.is_some() {
                    ui.add(egui::Spinner::new().size(style::size(ui, 12.0)));
                }
            });
        });
    }

    /// The frames of the bars of the chrome.
    pub(crate) fn chrome_frames(
        ui: &egui::Ui,
    ) -> (egui::Frame, egui::Frame, egui::Frame, egui::Frame) {
        let p = Self::palette_now(ui);
        (
            egui::Frame::new().fill(p.bg).inner_margin(Margin {
                left: 6,
                right: 6,
                top: 4,
                bottom: 0,
            }),
            bar_frame(&p, Margin::symmetric(8, 4)),
            bar_frame(&p, Margin::symmetric(8, 4)),
            bar_frame(&p, Margin::symmetric(12, 3)),
        )
    }
}

/// The commands of This PC for the selection.
pub(crate) fn this_pc_menu(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    let mut item = |ui: &mut egui::Ui, text: &str, act: Act| {
        if ui.button(text).clicked() {
            acts.push(act);
            ui.close();
        }
    };
    item(ui, tr("Read this PC into a new profile"), Act::ReadPc);
    item(
        ui,
        tr("Read the selection into this profile"),
        Act::ReadSelection,
    );
    item(ui, tr("Check the selection on this PC"), Act::CheckOnPc);
    ui.separator();
    item(
        ui,
        tr("Save an apply script for the selection..."),
        Act::SaveScript(ScriptKind::Apply),
    );
    item(
        ui,
        tr("Save a return-to-defaults script for the selection..."),
        Act::SaveScript(ScriptKind::Revert),
    );
    item(
        ui,
        tr("Save a registry file for the selection..."),
        Act::SaveRegFile(false),
    );
    item(
        ui,
        tr("Save a return-to-defaults registry file for the selection..."),
        Act::SaveRegFile(true),
    );
    item(
        ui,
        tr("Save an audit script for the selection..."),
        Act::SaveScript(ScriptKind::Audit),
    );
    ui.separator();
    item(
        ui,
        tr("Apply the selection to this PC..."),
        Act::PlanNative(false),
    );
    item(
        ui,
        tr("Return the selection to Windows defaults..."),
        Act::PlanNative(true),
    );
    item(ui, tr("Undo a change from its backup..."), Act::PlanUndo);
    ui.separator();
    item(ui, tr("Open an audit report..."), Act::OpenReport);
}

/// The parts of the breadcrumb that fit a width: all of them, else the first, an ellipsis
/// (`None`) and the last ones, else the last alone.
fn fit_crumbs(ui: &egui::Ui, crumbs: &[(String, Option<Act>)], room: f32) -> Vec<Option<usize>> {
    let n = crumbs.len();
    let width = |i: usize| {
        let font = if i + 1 == n {
            style::strong_font(ui, 13.0)
        } else {
            style::font(ui, 13.0)
        };
        ui.painter()
            .layout_no_wrap(crumbs[i].0.clone(), font, egui::Color32::WHITE)
            .size()
            .x
            + 22.0
    };
    let total = |list: &[Option<usize>]| {
        list.iter().map(|x| x.map_or(26.0, width)).sum::<f32>()
            + 16.0 * list.len().saturating_sub(1) as f32
    };
    let mut list: Vec<Option<usize>> = (0..n).map(Some).collect();
    while total(&list) > room && list.len() > 2 {
        if list[1].is_some() {
            list[1] = None;
        } else if list.len() > 3 {
            list.remove(2);
        } else {
            break;
        }
    }
    if total(&list) > room && n > 0 {
        list = vec![Some(n - 1)];
    }
    list
}

/// Where a dragged tab goes: its new index and the x of the line that shows the place, from the
/// tabs of the row, the index of the dragged one and the x of the pointer.
fn drop_place(rects: &[egui::Rect], from: usize, x: f32) -> (usize, f32) {
    let others: Vec<&egui::Rect> = rects
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != from)
        .map(|(_, r)| r)
        .collect();
    let to = others.iter().filter(|r| r.center().x < x).count();
    let line = match others.get(to) {
        Some(r) => r.left() - 1.0,
        None => others
            .last()
            .map_or(rects[from].right(), |r| r.right() + 1.0),
    };
    (to, line)
}
