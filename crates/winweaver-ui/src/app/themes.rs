//! The themes: the one shown, their editor, the colours of the title bar.

use crate::*;

impl App {
    /// The theme of the settings: a colour theme over its light or dark base, or the plain theme.
    /// The id of the theme the settings choose for a side when they follow Windows.
    pub(crate) fn shown_theme_id(&self, dark_mode: bool) -> String {
        if !self.settings.custom_theme.is_empty() {
            return self.settings.custom_theme.clone();
        }
        let dark = match self.settings.theme {
            ThemeChoice::System => dark_mode,
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
        };
        if dark { "dark" } else { "light" }.to_owned()
    }

    /// The light and dark themes of the window and the preference: the theme in the editor in
    /// front, else the one of the settings, else Light and Dark as Windows shows its apps.
    pub(crate) fn wanted_theme(&self) -> (themes::Theme, themes::Theme, egui::ThemePreference) {
        let light = themes::find(&self.themes, "light");
        let dark = themes::find(&self.themes, "dark");
        let chosen = match self.docs.get(self.active) {
            Some(Doc::Theme(d)) => Some(d.theme.clone()),
            _ => (!self.settings.custom_theme.is_empty())
                .then(|| themes::find(&self.themes, &self.settings.custom_theme)),
        };
        match chosen {
            Some(t) if t.dark => (light, t, egui::ThemePreference::Dark),
            Some(t) => (t, dark, egui::ThemePreference::Light),
            None => (light, dark, self.settings.theme.preference()),
        }
    }

    pub(crate) fn apply_visuals(&mut self, ctx: &egui::Context) {
        let wanted = self.wanted_theme();
        if self.applied_theme.as_ref() == Some(&wanted) {
            return;
        }
        let (light, dark, preference) = &wanted;
        style::apply(
            ctx,
            (light.palette(), light.fonts()),
            (dark.palette(), dark.fonts()),
        );
        ctx.set_theme(*preference);
        self.applied_theme = Some(wanted);
    }

    /// Opens the editor of a theme, or brings its tab to the front.
    pub(crate) fn edit_theme(&mut self, theme: themes::Theme) {
        if self.focus_tab(|d| matches!(d, Doc::Theme(t) if t.theme.id == theme.id)) {
            return;
        }
        self.docs
            .push(Doc::Theme(Box::new(themeview::ThemeDoc::new(theme))));
        self.active = self.docs.len() - 1;
    }

    pub(crate) fn open_theme_path(&mut self, path: PathBuf) {
        match themes::Theme::load(&path) {
            Ok(theme) => {
                self.edit_theme(theme);
                self.remember_recent(&path);
            }
            Err(e) => self.open_failed(e),
        }
    }

    /// Writes the theme of the active tab to its file, or to one the user names.
    pub(crate) fn save_theme(&mut self, ask: bool) {
        let Some(Doc::Theme(d)) = self.docs.get(self.active) else {
            return;
        };
        let dir = themes_dir();
        let path = if ask {
            let proposed = format!("{}.{}", file_stem(&d.theme.name), themes::EXTENSION);
            let dialog = rfd::FileDialog::new()
                .add_filter(tr("WinWeaver theme"), &[themes::EXTENSION])
                .set_title(tr("Save the theme as"))
                .set_file_name(proposed);
            let dialog = if std::fs::create_dir_all(&dir).is_ok() {
                dialog.set_directory(&dir)
            } else {
                dialog
            };
            let Some(path) = dialog.save_file() else {
                return;
            };
            if path.extension().is_some() {
                path
            } else {
                path.with_extension(themes::EXTENSION)
            }
        } else {
            d.theme.file_in(&dir)
        };
        let Some(Doc::Theme(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.theme.save(&path) {
            Ok(()) => {
                d.theme.id = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                d.theme.path = Some(path.clone());
                d.dirty = false;
                self.saved(&path);
                self.themes = themes::themes(&dir);
                self.remember_recent(&path);
            }
            Err(e) => self.save_failed(e),
        }
    }

    /// The title bar in the colours of the theme (Windows 11; the dark frame on Windows 10), given
    /// again whenever the colours change, also when the theme follows Windows.
    pub(crate) fn color_title_bar(&mut self, ui: &egui::Ui, frame: &eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let v = ui.visuals();
        let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
        let colors = (
            v.dark_mode,
            rgb(v.panel_fill),
            rgb(v.text_color()),
            rgb(v.window_stroke.color),
        );
        if self.title_colors == Some(colors) {
            return;
        }
        if let Ok(handle) = frame.window_handle()
            && let RawWindowHandle::Win32(w) = handle.as_raw()
        {
            winweaver_win::color_title_bar(w.hwnd.get(), colors.0, colors.1, colors.2, colors.3);
            self.title_colors = Some(colors);
        }
    }

    /// Runs an act of the acts of the themes.
    pub(crate) fn apply_theme(&mut self, act: ThemeAct) {
        match act {
            ThemeAct::Edit(id) => {
                let theme = themes::find(&self.themes, &id);
                self.edit_theme(theme);
            }
            ThemeAct::OpenFile => {
                let dir = themes_dir();
                let dialog = rfd::FileDialog::new()
                    .add_filter(tr("WinWeaver theme"), &[themes::EXTENSION, "json"])
                    .set_title(tr("Open a theme file"));
                let dialog = widgets::in_folder(dialog, &dir);
                if let Some(path) = dialog.pick_file() {
                    self.open_theme_path(path);
                }
            }
            ThemeAct::Use(id) => {
                let dark = themes::find(&self.themes, &id).dark;
                self.settings.theme = if dark {
                    ThemeChoice::Dark
                } else {
                    ThemeChoice::Light
                };
                self.settings.custom_theme = id;
                self.settings_dirty = true;
            }
        }
    }
}
