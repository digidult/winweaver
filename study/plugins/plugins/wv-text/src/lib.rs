//! Markdown pages and text files, two formats in one plugin: the page of W# draws both
//! (`ww::mdview`), a text as it is, with the search of the tab and the settings of the window
//! (Wrap lines, Width of pages).

use std::path::Path;

use wwrt::egui;
use wwrt::plugin::{Format, Host, Plugin, View};
use wwrt::ww::Search;
use wwrt::ww::mdview::{self, MdDoc};

/// The largest file the plugin reads; it reads a file whole.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

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

struct TextPlugin;

impl Plugin for TextPlugin {
    fn name(&self) -> &'static str {
        "Text and Markdown"
    }

    fn formats(&self) -> &'static [Format] {
        &FORMATS
    }

    fn open(&self, path: &Path) -> Result<Box<dyn View>, String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > MAX_BYTES {
            return Err(format!("larger than {} MB", MAX_BYTES / 1024 / 1024));
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
}

struct TextView {
    doc: MdDoc,
    lines: usize,
}

impl View for TextView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &Host) {
        egui::CentralPanel::default().show(ui, |ui| {
            mdview::md_view(ui, &mut self.doc, host.wrap_lines, host.page_width);
        });
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.doc.search)
    }

    fn status(&self) -> String {
        format!("{} lines", self.lines)
    }
}

wwrt::plugin!(TextPlugin);
