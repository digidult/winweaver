//! The settings tab and the settings that take effect at once.

use crate::*;

impl App {
    fn open_settings(&mut self) {
        match self.docs.iter().position(|d| matches!(d, Doc::Settings)) {
            Some(i) => self.active = i,
            None => {
                self.docs.push(Doc::Settings);
                self.active = self.docs.len() - 1;
            }
        }
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        let p = style::palette(ui);
        views::header(ui, |ui| {
            ui.label(style::strong(ui, tr("Settings"), 18.0));
        });
        ui.horizontal_top(|ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 0,
                    top: 10,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(200.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let sections = [tr("General"), tr("Policy templates"), tr("Files")];
                        for (i, title) in sections.into_iter().enumerate() {
                            let selected = self.settings_section == i;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(200.0, style::row_height(ui)),
                                egui::Sense::click(),
                            );
                            if selected {
                                ui.painter()
                                    .rect_filled(rect, egui::CornerRadius::same(6), p.sel);
                            } else if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    egui::CornerRadius::same(6),
                                    p.hover,
                                );
                            }
                            ui.painter().text(
                                egui::pos2(rect.left() + 12.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                title,
                                style::font(ui, 13.0),
                                if selected { p.sel_text } else { p.text },
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    true,
                                    selected,
                                    title,
                                )
                            });
                            if response.clicked() {
                                self.settings_section = i;
                            }
                        }
                    });
                });
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings-page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        match self.settings_section {
                            1 => {
                                let mut acts = Vec::new();
                                views::block(ui, |ui| self.imports_view(ui, &mut acts));
                                self.pending_acts.extend(acts);
                            }
                            2 => views::block(ui, |ui| self.files_settings(ui)),
                            _ => self.general_settings(ui),
                        }
                        ui.add_space(12.0);
                    });
            });
        });
        if before != self.settings {
            self.settings_dirty = true;
            if before.language != self.settings.language {
                self.apply_language();
            }
        }
    }

    /// The folders next to the executable and the recent files.
    fn files_settings(&mut self, ui: &mut egui::Ui) {
        style::section_title(ui, tr("Files"));
        ui.add_space(4.0);
        ui.label(trf(
            "Settings file: {path}",
            &[("path", &Settings::path().display())],
        ));
        let folders = [
            (
                trf(
                    "Profiles folder: {path}",
                    &[("path", &profiles_dir().display())],
                ),
                profiles_dir(),
            ),
            (
                trf(
                    "Output folder: {path}",
                    &[("path", &output_dir().display())],
                ),
                output_dir(),
            ),
            (
                trf(
                    "Policy templates folder: {path}",
                    &[("path", &self.imports_dir.display())],
                ),
                self.imports_dir.clone(),
            ),
        ];
        for (text, path) in folders {
            ui.horizontal_wrapped(|ui| {
                ui.label(text);
                if ui.small_button(tr("Open")).clicked() {
                    self.pending_acts.push(Act::ShowInFolder(path));
                }
            });
        }
        ui.add_space(8.0);
        if !self.settings.recent.is_empty() && ui.button(tr("Forget recent files")).clicked() {
            self.pending_acts.push(Act::AskForgetRecent);
        }
    }

    /// Language, theme, the fonts of the theme and the behaviour of the window, in cards.
    fn general_settings(&mut self, ui: &mut egui::Ui) {
        views::block(ui, |ui| {
            style::section_title(ui, tr("Language and theme"));
            ui.add_space(4.0);
            self.language_and_theme(ui);
        });
        views::block(ui, |ui| self.theme_fonts(ui));
        views::block(ui, |ui| {
            style::section_title(ui, tr("Window"));
            ui.add_space(4.0);
            ui.checkbox(
                &mut self.settings.desc_open,
                tr("Show the description panel (Ctrl+D)"),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Open at first in the description:"));
                let parts = &mut self.settings.desc_sections;
                ui.checkbox(&mut parts.what, tr("What it does"));
                ui.checkbox(&mut parts.risk, tr("Risk"));
                ui.checkbox(&mut parts.writes, tr("What it writes"));
                ui.checkbox(&mut parts.deps, tr("Dependencies and conflicts"));
                ui.checkbox(&mut parts.more, tr("Versions, check and rollback"));
                ui.checkbox(&mut parts.reference, tr("Reference"));
            });
            ui.checkbox(
                &mut self.settings.show_guide_at_start,
                tr("Open the guide at the next start"),
            );
            ui.checkbox(
                &mut self.settings.inline_switches,
                tr("Switches at the end of the rows of the selected group"),
            );
            ui.checkbox(
                &mut self.settings.hide_unknown,
                tr("Hide the options of the profile whose catalog is not loaded"),
            );
            ui.add_space(4.0);
            self.renderer_choice(ui);
            ui.checkbox(
                &mut self.settings.restore_tabs,
                tr("Open the tabs of the last session at the start"),
            );
        });
    }

    /// The renderer of the next start, among the ones of this executable.
    fn renderer_choice(&mut self, ui: &mut egui::Ui) {
        let current = Renderer::parse(&self.settings.renderer);
        ui.horizontal(|ui| {
            ui.label(tr("Renderer"));
            egui::ComboBox::from_id_salt("renderer")
                .selected_text(current.map_or(tr("Automatic"), Renderer::title))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.renderer,
                        String::new(),
                        tr("Automatic"),
                    );
                    for &r in Renderer::ALL {
                        ui.selectable_value(
                            &mut self.settings.renderer,
                            r.id().to_owned(),
                            r.title(),
                        );
                    }
                });
        });
        let order: Vec<&str> = Renderer::ALL.iter().map(|r| r.title()).collect();
        ui.weak(trf("A change takes effect at the next start. Automatic tries {order} in this order and takes the first that starts; OpenGL takes less memory, DirectX 12 also draws without a driver of the graphics card.", &[("order", &order.join(", "))]));
        if !self.graphics.is_empty() {
            ui.weak(trf(
                "Draws now with {graphics}",
                &[("graphics", &self.graphics)],
            ));
        }
    }

    fn language_and_theme(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(tr("Language"));
            let langs = i18n::languages(&lang_dir());
            let current = if self.settings.language.is_empty() {
                tr("Follow Windows").to_owned()
            } else {
                langs
                    .iter()
                    .find(|l| l.code == self.settings.language)
                    .map_or_else(|| self.settings.language.clone(), |l| l.name.clone())
            };
            egui::ComboBox::from_id_salt("language")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.language,
                        String::new(),
                        tr("Follow Windows"),
                    );
                    for l in &langs {
                        ui.selectable_value(&mut self.settings.language, l.code.clone(), &l.name);
                    }
                });
        });
        let shown = self.shown_theme_id(ui.visuals().dark_mode);
        ui.horizontal(|ui| {
            ui.label(tr("Theme"));
            let current = if self.settings.custom_theme.is_empty()
                && self.settings.theme == ThemeChoice::System
            {
                tr("Follow Windows").to_owned()
            } else {
                themes::find(&self.themes, &shown).name
            };
            let mut choice = if self.settings.theme == ThemeChoice::System
                && self.settings.custom_theme.is_empty()
            {
                String::new()
            } else {
                shown.clone()
            };
            let before = choice.clone();
            egui::ComboBox::from_id_salt("theme")
                .selected_text(current)
                .width(200.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, String::new(), tr("Follow Windows"))
                        .on_hover_text(tr("Light or Dark, as Windows shows its apps"));
                    for t in &self.themes {
                        ui.selectable_value(&mut choice, t.id.clone(), &t.name);
                    }
                });
            if choice != before {
                if choice.is_empty() {
                    self.settings.theme = ThemeChoice::System;
                    self.settings.custom_theme.clear();
                } else {
                    let dark = themes::find(&self.themes, &choice).dark;
                    self.settings.theme = if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    };
                    self.settings.custom_theme = choice;
                }
            }
            if ui
                .button(tr("Edit the theme"))
                .on_hover_text(tr(
                    "Opens the theme file in its editor: colours, fonts, Save as",
                ))
                .clicked()
            {
                self.pending_acts.push(Act::EditTheme(shown.clone()));
            }
            if ui.button(tr("Open a theme file...")).clicked() {
                self.pending_acts.push(Act::OpenThemeFile);
            }
        });
    }

    /// The fonts of the theme the window shows, saved in its file when one changes.
    fn theme_fonts(&mut self, ui: &mut egui::Ui) {
        let shown = self.shown_theme_id(ui.visuals().dark_mode);
        let mut theme = themes::find(&self.themes, &shown);
        style::section_title(
            ui,
            &trf("Fonts of the theme {name}", &[("name", &theme.name)]),
        );
        let families = style::installed_families();
        ui.add_space(4.0);
        if themeview::fonts_grid(ui, "settings-fonts", &mut theme, &families) {
            let path = theme.file_in(&themes_dir());
            match theme.save(&path) {
                Ok(()) => {
                    self.themes = themes::themes(&themes_dir());
                    self.status = trf("Saved {path}", &[("path", &path.display())]);
                    self.shown_file = Some((self.status.clone(), path.clone()));
                }
                Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
            }
        }
        ui.label(style::muted(
            ui,
            trf(
                "The fonts belong to the theme {name}: a change is saved in its file in the themes folder.",
                &[("name", &theme.name)],
            ),
            12.0,
        ));
    }

    /// The language the window shows: the setting, or the language of Windows when it says to
    /// follow Windows, English when there is no file for it.
    fn effective_language(&self) -> String {
        let wanted = if self.settings.language.is_empty() {
            winweaver_win::ui_language().unwrap_or_else(|| "en".to_owned())
        } else {
            self.settings.language.clone()
        };
        if i18n::languages(&lang_dir())
            .iter()
            .any(|l| l.code == wanted)
        {
            wanted
        } else {
            "en".to_owned()
        }
    }

    fn apply_language(&mut self) {
        let code = self.effective_language();
        i18n::set_language(&code, &lang_dir());
    }

    fn persist_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        if let Err(e) = self.settings.save_to(&Settings::path()) {
            self.status = trf("Settings not saved: {e}", &[("e", &e)]);
        }
    }
}
