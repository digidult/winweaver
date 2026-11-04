//! The template of a native plugin of wview (the SDK, docs/plugins.md). It opens `.hello` files
//! and shows their first line large in the middle of the tab: the wheel and + and - change its
//! size, a drag and the arrows move it, a double click and 0 put it back. Its setting: the colour
//! of the text. Every part a plugin has is here once: the formats, the view with the keys and the
//! mouse, the cursor, a command of the bar, a setting, the status bar and the translations.

use std::path::Path;

use wwrt::egui::{self, Align2, Key, Vec2};
use wwrt::input::{self, Cursor};
use wwrt::plugin::{Format, Host, Plugin, Tool, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::style;

/// The name of the plugin (translated by the host).
const NAME: &str = "Hello, the template of a plugin";
/// The largest file the plugin reads.
const MAX_BYTES: u64 = 64 * 1024;
/// The size of the text at first, in points.
const SIZE: f32 = 48.0;
/// The keys of the view while it has the focus.
const KEYS: [Key; 8] = [
    Key::Plus,
    Key::Equals,
    Key::Minus,
    Key::Num0,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
];

static FORMATS: [Format; 1] = [Format {
    name: "Hello files",
    extensions: &["hello"],
}];

/// The texts of the plugin in other languages: a file `lang/<code>.json` each, in the form of the
/// `ui.json` of a program on W#. Two of them here; a plugin gives the ones it has.
static TRANSLATIONS: [(&str, &str); 2] = [
    ("uk", include_str!("../lang/uk.json")),
    ("ru", include_str!("../lang/ru.json")),
];

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn formats(&self) -> &'static [Format] {
        &FORMATS
    }

    fn open(&self, path: &Path, _settings: &Value) -> Result<Box<dyn View>, String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > MAX_BYTES {
            return Err(tr("The file is too large for a hello").to_owned());
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let line = text.lines().next().unwrap_or("").trim();
        let text = if line.is_empty() { "Hello" } else { line };
        Ok(Box::new(HelloView {
            text: text.to_owned(),
            size: SIZE,
            offset: Vec2::ZERO,
        }))
    }

    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &TRANSLATIONS
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&self, ui: &mut egui::Ui, settings: &mut Value) -> bool {
        let mut accent: bool = setting(settings, "accent", true);
        let changed = ui
            .checkbox(&mut accent, tr("Draw the text in the accent colour"))
            .changed();
        if changed {
            set_setting(settings, "accent", accent);
        }
        changed
    }
}

struct HelloView {
    text: String,
    size: f32,
    /// Where the text is from the centre of the view, in points.
    offset: Vec2,
}

impl HelloView {
    fn reset(&mut self) {
        self.size = SIZE;
        self.offset = Vec2::ZERO;
    }
}

impl View for HelloView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let accent: bool = setting(host.settings, "accent", true);
        egui::CentralPanel::default().show(ui, |ui| {
            let rect = ui.max_rect();
            let (_, input) = input::area(ui, ui.id().with("hello"), rect, &KEYS);
            for (key, _) in &input.keys {
                match key {
                    Key::Plus | Key::Equals => self.size = (self.size * 1.2).min(400.0),
                    Key::Minus => self.size = (self.size / 1.2).max(8.0),
                    Key::Num0 => self.reset(),
                    Key::ArrowLeft => self.offset.x -= 10.0,
                    Key::ArrowRight => self.offset.x += 10.0,
                    Key::ArrowUp => self.offset.y -= 10.0,
                    Key::ArrowDown => self.offset.y += 10.0,
                    _ => {}
                }
            }
            if input.wheel.y != 0.0 {
                self.size = (self.size * (-input.wheel.y * 0.002).exp()).clamp(8.0, 400.0);
            }
            if input.double_clicked {
                self.reset();
            }
            if input.dragging {
                self.offset += input.drag;
                input::cursor(ui, Cursor::Grabbing);
            } else if input.hovered {
                input::cursor(ui, Cursor::Grab);
            }
            let colour = if accent {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().text_color()
            };
            ui.painter_at(rect).text(
                rect.center() + self.offset,
                Align2::CENTER_CENTER,
                &self.text,
                style::font(ui, self.size),
                colour,
            );
        });
    }

    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            id: "reset",
            icon: "\u{21BA}",
            label: "Reset",
        }]
    }

    fn act(&mut self, id: &str) {
        if id == "reset" {
            self.reset();
        }
    }

    fn status(&self) -> String {
        trf(
            "{size} points, moved by {x}, {y}",
            &[
                ("size", &format!("{:.0}", self.size)),
                ("x", &format!("{:.0}", self.offset.x)),
                ("y", &format!("{:.0}", self.offset.y)),
            ],
        )
    }
}

wwrt::plugin!(HelloPlugin);

#[cfg(test)]
mod tests {
    #[test]
    fn every_text_of_the_plugin_is_translated() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let wanted = wwrt::testing::plugin_texts(&src);
        let missing = wwrt::testing::missing(&super::TRANSLATIONS, &wanted);
        assert!(missing.is_empty(), "{missing:#?}");
    }
}
