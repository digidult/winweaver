//! Markdown pages and text files, two formats in one plugin: the page of W# draws both
//! (`ww::mdview`), a text as it is, with the search of the tab and the settings of the window
//! (Wrap lines, Width of pages). Its setting: the largest file it opens.

use std::path::Path;

use wwrt::egui;
use wwrt::plugin::{Format, Host, Plugin, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::Search;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::mdview::{self, MdDoc};

/// The largest file the plugin reads by default, in MB; it reads a file whole.
const MAX_MB: u32 = 64;

static FORMATS: [Format; 2] = [
    Format {
        name: "Markdown pages",
        extensions: &["md", "markdown"],
    },
    Format {
        name: "Text files",
        extensions: &["txt", "log", "ini", "cfg", "csv"],
    },
];

/// The name of the plugin (translated by the host).
const NAME: &str = "Text and Markdown";

/// The texts of the plugin in the languages of the window.
static TRANSLATIONS: [(&str, &str); 8] = [
    ("uk", include_str!("../lang/uk.json")),
    ("ru", include_str!("../lang/ru.json")),
    ("de", include_str!("../lang/de.json")),
    ("es", include_str!("../lang/es.json")),
    ("fr", include_str!("../lang/fr.json")),
    ("id", include_str!("../lang/id.json")),
    ("it", include_str!("../lang/it.json")),
    ("tr", include_str!("../lang/tr.json")),
];

pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn formats(&self) -> &'static [Format] {
        &FORMATS
    }

    fn open(&self, path: &Path, settings: &Value) -> Result<Box<dyn View>, String> {
        let max_mb: u32 = setting(settings, "max_mb", MAX_MB);
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > u64::from(max_mb) * 1024 * 1024 {
            return Err(trf("larger than {mb} MB", &[("mb", &max_mb)]));
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        let text = String::from_utf8_lossy(bytes).into_owned();
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let page = path.extension().is_some_and(|e| {
            FORMATS[0]
                .extensions
                .contains(&e.to_string_lossy().to_lowercase().as_str())
        });
        let lines = text.lines().count();
        let doc = if page {
            MdDoc::new(title, Some(path.to_path_buf()), text)
        } else {
            MdDoc::plain(title, Some(path.to_path_buf()), text)
        };
        Ok(Box::new(TextView { doc, lines }))
    }

    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &TRANSLATIONS
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&self, ui: &mut egui::Ui, settings: &mut Value) -> bool {
        let mut max_mb: u32 = setting(settings, "max_mb", MAX_MB);
        let changed = ui
            .horizontal(|ui| {
                ui.label(tr("Largest file to open"));
                let field = egui::DragValue::new(&mut max_mb)
                    .range(1..=1024)
                    .suffix(format!(" {}", tr("MB")));
                ui.add(field).changed()
            })
            .inner;
        if changed {
            set_setting(settings, "max_mb", max_mb);
        }
        changed
    }
}

struct TextView {
    doc: MdDoc,
    lines: usize,
}

impl View for TextView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        egui::CentralPanel::default().show(ui, |ui| {
            mdview::md_view(ui, &mut self.doc, host.wrap_lines, host.page_width);
        });
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.doc.search)
    }

    fn status(&self) -> String {
        trf("{n} lines", &[("n", &self.lines)])
    }
}

wwrt::plugin!(TextPlugin);

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
