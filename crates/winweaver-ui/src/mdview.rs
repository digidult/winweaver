//! Markdown tabs (`docs/spec/01-ui.md` section 6 of the ideas, `03-features.md` section 9): the
//! guide embedded in the executable and any `.md` file, rendered with egui_commonmark.

use std::path::PathBuf;

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// The guide shown at the first start and from the menu (F1).
pub const GUIDE: &str = include_str!("../../../data/docs/guide.md");
/// The translated guides: the language code and the page.
pub const GUIDES: [(&str, &str); 2] = [
    ("uk", include_str!("../../../data/lang/uk/guide.md")),
    ("ru", include_str!("../../../data/lang/ru/guide.md")),
];
pub const GUIDE_TITLE: &str = "Guide.md";

/// The widest a page is laid out, so that lines stay readable on a wide window.
const PAGE_WIDTH: f32 = 860.0;

pub struct MdDoc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
    cache: CommonMarkCache,
    pub search: crate::Search,
}

impl MdDoc {
    pub fn new(title: impl Into<String>, path: Option<PathBuf>, text: String) -> Self {
        Self {
            title: title.into(),
            path,
            text,
            cache: CommonMarkCache::default(),
            search: crate::Search::default(),
        }
    }

    pub fn guide() -> Self {
        Self::new(GUIDE_TITLE, None, guide_text(&crate::i18n::current()))
    }

    pub fn is_guide(&self) -> bool {
        self.path.is_none() && self.title == GUIDE_TITLE
    }
}

/// The guide in a language: a file `lang/<code>/guide.md` next to the executable, the embedded
/// translation, or the English page.
pub fn guide_text(code: &str) -> String {
    let file = crate::lang_dir().join(code).join("guide.md");
    if code != "en"
        && let Ok(text) = winweaver_core::jsonfile::read_text_bom(&file, 4 * 1024 * 1024)
    {
        return text;
    }
    GUIDES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(GUIDE, |(_, text)| text)
        .to_owned()
}

/// The blocks of a page that hold every word of a search, each with the heading above it: the
/// paragraphs, lists and code blocks between blank lines.
pub fn matching_blocks(text: &str, words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut heading = String::new();
    let mut in_code = false;
    let mut block = String::new();
    let flush = |block: &mut String, heading: &str, out: &mut Vec<String>| {
        let lower = block.to_lowercase();
        if !block.trim().is_empty() && words.iter().all(|w| lower.contains(w.as_str())) {
            if heading.is_empty() || block.trim_start().starts_with('#') {
                out.push(block.clone());
            } else {
                out.push(format!("{heading}\n\n{block}"));
            }
        }
        block.clear();
    };
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        if !in_code && line.trim().is_empty() {
            flush(&mut block, &heading, &mut out);
            continue;
        }
        if !in_code && line.starts_with('#') {
            flush(&mut block, &heading, &mut out);
            heading = format!("#### {}", line.trim_start_matches('#').trim());
        }
        block.push_str(line);
        block.push('\n');
    }
    flush(&mut block, &heading, &mut out);
    out
}

pub fn md_view(ui: &mut egui::Ui, doc: &mut MdDoc) {
    let words = doc.search.words();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.set_max_width(PAGE_WIDTH);
            if words.is_empty() {
                CommonMarkViewer::new().show(ui, &mut doc.cache, &doc.text);
                return;
            }
            let blocks = matching_blocks(&doc.text, &words);
            ui.weak(crate::i18n::trf(
                "{n} places hold the search",
                &[("n", &blocks.len())],
            ));
            for block in &blocks {
                ui.separator();
                CommonMarkViewer::new().show(ui, &mut doc.cache, block);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_finds_the_blocks_with_their_headings() {
        let text = "# Guide\n\nFirst steps here.\n\n## Build\n\nCopy autounattend.xml to the stick.\n\n```\nkeep code\n\nwhole\n```\n";
        let words = vec!["autounattend".to_owned()];
        let blocks = matching_blocks(text, &words);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].starts_with("#### Build"), "{blocks:?}");
        let code = matching_blocks(text, &["whole".to_owned()]);
        assert_eq!(code.len(), 1);
        assert!(code[0].contains("keep code"), "a code block stays whole");
    }
}
