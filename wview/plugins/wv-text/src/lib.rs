//! Markdown pages, text files and XML files in one plugin: the page of W# draws them
//! (`ww::mdview`), a text as it is, with the search of the tab and the settings of the window
//! (Wrap lines, Width of pages). A text is UTF-8, or UTF-16 by its mark or by the `charset` of its
//! MIME type. The plugin opens bytes in memory too (a manifest inside a program), and every
//! textual type no other plugin names falls back to its `text/plain` (`wwrt::mime::fallbacks`).
//! Its setting: the largest file it reads whole; a larger one opens in parts (`large`).

mod large;

use std::path::Path;

use wwrt::egui;
use wwrt::mime;
use wwrt::plugin::{Format, Host, Plugin, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::Search;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::mdview::{self, MdDoc};

/// The largest file the plugin reads by default, in MB; it reads a file whole.
const MAX_MB: u32 = 64;

static FORMATS: [Format; 3] = [
    Format {
        name: "Markdown pages",
        extensions: &["md", "markdown"],
        mimes: &["text/markdown"],
    },
    Format {
        name: "Text files",
        extensions: &["txt", "log", "ini", "cfg", "csv"],
        mimes: &["text/plain", "text/csv"],
    },
    Format {
        name: "XML files",
        extensions: &["xml", "manifest"],
        mimes: &["application/xml"],
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

/// The text of some bytes: UTF-8, or UTF-16 by its mark or by the `charset` of its type.
pub fn decoded(bytes: &[u8], mime: &str) -> String {
    let wide = |rest: &[u8], big: bool| {
        let (pairs, _) = rest.as_chunks::<2>();
        let units = pairs.iter().map(|p| {
            if big {
                u16::from_be_bytes(*p)
            } else {
                u16::from_le_bytes(*p)
            }
        });
        char::decode_utf16(units)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    };
    if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        return wide(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        return wide(rest, true);
    }
    let charset = mime::parameter(mime, "charset").map(|c| c.to_ascii_lowercase());
    match charset.as_deref() {
        Some("utf-16le" | "utf-16") => wide(bytes, false),
        Some("utf-16be") => wide(bytes, true),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// The view of a text: a Markdown page or a text as it is.
fn view_of(title: String, path: Option<&Path>, text: String, page: bool) -> Box<dyn View> {
    let lines = text.lines().count();
    let path = path.map(Path::to_path_buf);
    let doc = if page {
        MdDoc::new(title, path, text)
    } else {
        MdDoc::plain(title, path, text)
    };
    Box::new(TextView { doc, lines })
}

/// The largest text the settings let the plugin read.
fn largest(settings: &Value) -> (u32, u64) {
    let max_mb: u32 = setting(settings, "max_mb", MAX_MB);
    (max_mb, u64::from(max_mb) * 1024 * 1024)
}

pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn formats(&self) -> &'static [Format] {
        &FORMATS
    }

    fn open(&self, path: &Path, settings: &Value) -> Result<Box<dyn View>, String> {
        let (_, max) = largest(settings);
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > max {
            return large::LargeText::open(path).map(|v| Box::new(v) as Box<dyn View>);
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let page = path.extension().is_some_and(|e| {
            FORMATS[0]
                .extensions
                .contains(&e.to_string_lossy().to_lowercase().as_str())
        });
        Ok(view_of(title, Some(path), decoded(&bytes, ""), page))
    }

    fn open_bytes(
        &self,
        name: &str,
        mime: &str,
        bytes: Vec<u8>,
        settings: &Value,
    ) -> Result<Box<dyn View>, String> {
        let (max_mb, max) = largest(settings);
        if bytes.len() as u64 > max {
            return Err(trf("larger than {mb} MB", &[("mb", &max_mb)]));
        }
        let page = mime::same(mime, "text/markdown");
        Ok(view_of(name.to_owned(), None, decoded(&bytes, mime), page))
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
                ui.label(tr("Largest file read whole"));
                let field = egui::DragValue::new(&mut max_mb)
                    .range(1..=1024)
                    .suffix(format!(" {}", tr("MB")));
                ui.add(field).changed()
            })
            .inner;
        if changed {
            set_setting(settings, "max_mb", max_mb);
        }
        ui.weak(tr(
            "A larger file opens in parts: its lines are counted while it is read, and the search lists the lines that hold it.",
        ));
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
    use super::*;

    #[test]
    fn a_text_of_utf8_or_utf16_reads_by_its_mark_or_its_charset() {
        let wide: Vec<u8> = "\u{0444}x"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let mut marked = vec![0xFF, 0xFE];
        marked.extend(&wide);
        assert_eq!(decoded(&marked, ""), "\u{0444}x");
        assert_eq!(decoded(&wide, "text/plain; charset=utf-16le"), "\u{0444}x");
        let big: Vec<u8> = "ab".encode_utf16().flat_map(u16::to_be_bytes).collect();
        assert_eq!(decoded(&big, "application/xml; charset=UTF-16BE"), "ab");
        assert_eq!(decoded(b"\xEF\xBB\xBF<a/>", "application/xml"), "<a/>");
        assert_eq!(decoded("\u{0444}".as_bytes(), ""), "\u{0444}");
    }

    #[test]
    fn bytes_open_as_a_text_or_a_page() {
        let manifest = b"<?xml version=\"1.0\"?>\r\n<assembly/>\r\n".to_vec();
        let view = TextPlugin
            .open_bytes("a.exe, MANIFEST", "application/xml", manifest, &Value::Null)
            .unwrap();
        assert_eq!(view.status(), "2 lines");
        let big = vec![b'x'; 2 * 1024 * 1024];
        let small = wwrt::serde_json::json!({ "max_mb": 1 });
        assert!(
            TextPlugin
                .open_bytes("big", "text/plain", big, &small)
                .is_err()
        );
    }

    #[test]
    fn every_text_of_the_plugin_is_translated() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let wanted = wwrt::testing::plugin_texts(&src);
        let missing = wwrt::testing::missing(&super::TRANSLATIONS, &wanted);
        assert!(missing.is_empty(), "{missing:#?}");
    }
}
