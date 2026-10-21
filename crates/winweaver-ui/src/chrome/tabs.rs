//! The tab row: the menu button, the tabs (a click, a drag, the menu of the right button) and the new-profile button.

use super::*;

impl App {
    /// The tab row: the menu button, the tabs with their icons, the new tab button.
    pub(crate) fn tabs_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let p = Self::palette_now(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let menu = style::icon_button(ui, icons::MENU, tr("Menu"), true, Some(p.text));
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
                    acts.push(Act::Nav(NavAct::MoveTab(from, to)));
                }
            }
            if style::icon_button(ui, "+", tr("New profile (Ctrl+N)"), true, None).clicked() {
                acts.push(Act::File(FileAct::NewProfile(None)));
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
            .min(TAB_TITLE_WIDTH);
        let dirty = doc.dirty();
        let dot = if dirty { TAB_DOT } else { 0.0 };
        let width = TAB_PADDING + TAB_ICON + dot + text_w + TAB_GAP + TAB_CLOSE;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, TAB_HEIGHT), Sense::click_and_drag());
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
            let mut x = rect.left() + TAB_PADDING;
            painter.text(
                egui::pos2(x + 6.0, rect.center().y),
                Align2::CENTER_CENTER,
                doc.kind().icon(),
                style::font(ui, 12.0),
                color,
            );
            x += TAB_ICON;
            if dirty {
                painter.circle_filled(egui::pos2(x + 2.0, rect.center().y), 3.0, p.accent);
                x += TAB_DOT;
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
                icons::CLOSE,
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
            acts.push(Act::Nav(NavAct::CloseTab(i, false)));
        } else if response.clicked() {
            acts.push(Act::Nav(NavAct::Activate(i)));
        }
        if response.middle_clicked() {
            acts.push(Act::Nav(NavAct::CloseTab(i, false)));
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
                acts.push(Act::Nav(NavAct::MoveTab(i, i - 1)));
                ui.close();
            }
            if ui
                .add_enabled(
                    i < last,
                    egui::Button::new(tr("Move the tab right (Ctrl+Shift+PgDn)")),
                )
                .clicked()
            {
                acts.push(Act::Nav(NavAct::MoveTab(i, i + 1)));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Close (Ctrl+W)")).clicked() {
                acts.push(Act::Nav(NavAct::CloseTab(i, false)));
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
            item(
                ui,
                tr("Empty profile"),
                keys::NEW.text,
                Act::File(FileAct::NewProfile(None)),
            );
            ui.separator();
            for (name, _) in PRESETS {
                item(
                    ui,
                    &trf("Profile from the preset {name}", &[("name", &name)]),
                    "",
                    Act::File(FileAct::NewProfile(Some(name))),
                );
            }
            ui.separator();
            item(ui, tr("Users file"), "", Act::Users(UsersAct::New));
            item(
                ui,
                tr("Users file of the accounts of this PC"),
                "",
                Act::Users(UsersAct::OfThisPc),
            );
        });
        item(ui, tr("Open..."), keys::OPEN.text, Act::File(FileAct::Open));
        ui.add_enabled_ui(!self.settings.recent.is_empty(), |ui| {
            ui.menu_button(tr("Open recent"), |ui| {
                for path in &self.settings.recent {
                    item(
                        ui,
                        &path.display().to_string(),
                        "",
                        Act::File(FileAct::OpenPath(path.clone())),
                    );
                }
            });
        });
        item(ui, tr("Save"), keys::SAVE.text, Act::File(FileAct::Save));
        item(
            ui,
            tr("Save as..."),
            keys::SAVE_AS.text,
            Act::File(FileAct::SaveAs),
        );
        item(
            ui,
            tr("Close tab"),
            keys::CLOSE.text,
            Act::Nav(NavAct::CloseTab(self.active, false)),
        );
        item(
            ui,
            tr("Close tab, discard changes"),
            "",
            self.discard_act(self.active),
        );
        ui.separator();
        item(
            ui,
            tr("Guide"),
            keys::GUIDE.text,
            Act::File(FileAct::OpenGuide),
        );
        item(ui, tr("License"), "", Act::File(FileAct::OpenLicense));
        item(ui, tr("Bookmarks"), "", Act::Marks(MarksAct::Open));
        item(
            ui,
            tr("Registry"),
            "",
            Act::Registry(RegistryAct::Open("HKEY_LOCAL_MACHINE".to_owned(), None)),
        );
        item(ui, tr("MCP server"), "", Act::Mcp(McpAct::Open));
        item(
            ui,
            tr("Settings"),
            keys::SETTINGS.text,
            Act::Settings(SettingsAct::Open),
        );
        ui.separator();
        item(
            ui,
            tr("About"),
            "",
            Act::Nav(NavAct::Message(trf(
                "WinWeaver {build}, catalog {catalog}, settings {path}",
                &[
                    ("build", &winweaver_core::BUILD),
                    ("catalog", &self.catalog.version()),
                    ("path", &Settings::path().display()),
                ],
            ))),
        );
        item(ui, tr("Exit"), "", Act::Nav(NavAct::Exit));
    }
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
