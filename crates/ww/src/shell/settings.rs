//! The settings tab: General (the language, the theme and its fonts, the window) and the
//! sections of the program, chosen in a list on the left.

use eframe::egui::{self, Margin};

use super::{App, ShellAct, ShellDoc, Window};
use crate::doc::Document;
use crate::folders::dir;
use crate::i18n::{self, tr, trf};
use crate::renderer::Renderer;
use crate::settings::ThemeChoice;
use crate::{style, themes, themeview};

impl<A: App> Window<A> {
    pub(super) fn settings_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<A::Act>) {
        let p = style::palette(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ui, |ui| self.settings_page(ui, acts));
    }

    fn settings_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<A::Act>) {
        let before = self.shell.settings.clone();
        let p = style::palette(ui);
        let mut section = match self.shell.active_doc().and_then(Document::shell_doc) {
            Some(ShellDoc::Settings(d)) => d.section,
            _ => 0,
        };
        let mut titles = vec![tr("General")];
        titles.extend(self.app.settings_sections());
        section = section.min(titles.len() - 1);
        style::header(ui, |ui| {
            ui.label(style::strong(ui, tr("Settings"), 18.0));
        });
        ui.horizontal_top(|ui| {
            egui::Frame::new()
                .inner_margin(Margin {
                    left: 8,
                    right: 0,
                    top: 10,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(200.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (i, title) in titles.iter().enumerate() {
                            let selected = section == i;
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
                                *title,
                                style::font(ui, 13.0),
                                if selected { p.sel_text } else { p.text },
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    true,
                                    selected,
                                    *title,
                                )
                            });
                            if response.clicked() {
                                section = i;
                            }
                        }
                    });
                });
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings-page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        if section == 0 {
                            self.general_settings(ui, acts);
                        } else {
                            self.app
                                .settings_section(&mut self.shell, section - 1, ui, acts);
                        }
                        ui.add_space(12.0);
                    });
            });
        });
        if let Some(ShellDoc::Settings(d)) = self
            .shell
            .active_doc_mut()
            .and_then(Document::shell_doc_mut)
        {
            d.section = section;
        }
        if before != self.shell.settings {
            self.shell.settings_dirty = true;
            if before.language != self.shell.settings.language {
                self.shell.apply_language();
            }
        }
    }

    /// Language, theme, the fonts of the theme and the behaviour of the window, in cards; the
    /// program adds its own lines to the card of the window.
    fn general_settings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<A::Act>) {
        style::block(ui, |ui| {
            style::section_title(ui, tr("Language and theme"));
            ui.add_space(4.0);
            self.language_and_theme(ui, acts);
        });
        style::block(ui, |ui| self.theme_fonts(ui));
        style::block(ui, |ui| {
            style::section_title(ui, tr("Window"));
            ui.add_space(4.0);
            self.app.settings_general(&mut self.shell, ui);
            self.renderer_choice(ui);
            ui.checkbox(
                &mut self.shell.settings.restore_tabs,
                tr("Open the tabs of the last session at the start"),
            );
            ui.checkbox(
                &mut self.shell.settings.wrap_lines,
                tr("Wrap lines to the width of the view"),
            );
        });
    }

    /// The renderer of the next start, among the ones of this executable.
    fn renderer_choice(&mut self, ui: &mut egui::Ui) {
        let settings = &mut self.shell.settings;
        let current = Renderer::parse(&settings.renderer);
        ui.horizontal(|ui| {
            ui.label(tr("Renderer"));
            egui::ComboBox::from_id_salt("renderer")
                .selected_text(current.map_or(tr("Automatic"), Renderer::title))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.renderer, String::new(), tr("Automatic"));
                    for &r in Renderer::ALL {
                        ui.selectable_value(&mut settings.renderer, r.id().to_owned(), r.title());
                    }
                });
        });
        let order: Vec<&str> = Renderer::ALL.iter().map(|r| r.title()).collect();
        ui.weak(trf("A change takes effect at the next start. Automatic tries {order} in this order and takes the first that starts; OpenGL takes less memory, DirectX 12 also draws without a driver of the graphics card.", &[("order", &order.join(", "))]));
        if !self.shell.graphics.is_empty() {
            ui.weak(trf(
                "Draws now with {graphics}",
                &[("graphics", &self.shell.graphics)],
            ));
        }
    }

    fn language_and_theme(&mut self, ui: &mut egui::Ui, acts: &mut Vec<A::Act>) {
        let shell = &mut self.shell;
        ui.horizontal(|ui| {
            ui.label(tr("Language"));
            let langs = i18n::languages(&dir("lang"));
            let current = if shell.settings.language.is_empty() {
                tr("Follow Windows").to_owned()
            } else {
                langs
                    .iter()
                    .find(|l| l.code == shell.settings.language)
                    .map_or_else(|| shell.settings.language.clone(), |l| l.name.clone())
            };
            egui::ComboBox::from_id_salt("language")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut shell.settings.language,
                        String::new(),
                        tr("Follow Windows"),
                    );
                    for l in &langs {
                        ui.selectable_value(&mut shell.settings.language, l.code.clone(), &l.name);
                    }
                });
        });
        let shown = shell.shown_theme_id(ui.visuals().dark_mode);
        ui.horizontal(|ui| {
            ui.label(tr("Theme"));
            let follows = shell.settings.custom_theme.is_empty()
                && shell.settings.theme == ThemeChoice::System;
            let current = if follows {
                tr("Follow Windows").to_owned()
            } else {
                themes::find(&shell.themes, &shown).name
            };
            let mut choice = if follows {
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
                    for t in &shell.themes {
                        ui.selectable_value(&mut choice, t.id.clone(), &t.name);
                    }
                });
            if choice != before {
                if choice.is_empty() {
                    shell.settings.theme = ThemeChoice::System;
                    shell.settings.custom_theme.clear();
                } else {
                    let dark = themes::find(&shell.themes, &choice).dark;
                    shell.settings.theme = if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    };
                    shell.settings.custom_theme = choice;
                }
            }
            if ui
                .button(tr("Edit the theme"))
                .on_hover_text(tr(
                    "Opens the theme file in its editor: colours, fonts, Save as",
                ))
                .clicked()
            {
                acts.push(ShellAct::EditTheme(shown.clone()).into());
            }
            if ui.button(tr("Open a theme file...")).clicked() {
                acts.push(ShellAct::OpenThemeFile.into());
            }
        });
    }

    /// The fonts of the theme the window shows, saved in its file when one changes.
    fn theme_fonts(&mut self, ui: &mut egui::Ui) {
        let shell = &mut self.shell;
        let shown = shell.shown_theme_id(ui.visuals().dark_mode);
        let mut theme = themes::find(&shell.themes, &shown);
        style::section_title(
            ui,
            &trf("Fonts of the theme {name}", &[("name", &theme.name)]),
        );
        let families = style::installed_families();
        ui.add_space(4.0);
        if themeview::fonts_grid(ui, "settings-fonts", &mut theme, &families) {
            let folder = dir("themes");
            let path = theme.file_in(&folder);
            match theme.save(&path) {
                Ok(()) => {
                    shell.themes = themes::themes(&folder);
                    shell.saved(&path);
                }
                Err(e) => shell.save_failed(e),
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
}
