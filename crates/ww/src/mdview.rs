//! Markdown tabs: a page of the program or any `.md` file, rendered with egui_commonmark, with
//! the search of the tab and the anchors of the headings.

use std::path::PathBuf;

use crate::doc::{Document, Search, matches_words};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// The widest a page is laid out, so that lines stay readable on a wide window.
const PAGE_WIDTH: f32 = 860.0;

pub struct MdDoc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
    cache: CommonMarkCache,
    pub search: Search,
    /// The heading to show, by its anchor, and whether to scroll to it at the next frame.
    pub anchor: Option<String>,
    pub jump: bool,
    /// The page of the program the document shows, by its id; a file or a text has none.
    pub page: Option<&'static str>,
}

impl MdDoc {
    pub fn new(title: impl Into<String>, path: Option<PathBuf>, text: String) -> Self {
        Self {
            title: title.into(),
            path,
            text,
            cache: CommonMarkCache::default(),
            search: Search::default(),
            anchor: None,
            jump: false,
            page: None,
        }
    }

    /// A page of the program, opened once by its id.
    pub fn page(id: &'static str, title: impl Into<String>, text: String) -> Self {
        let mut doc = Self::new(title, None, text);
        doc.page = Some(id);
        doc
    }

}

impl Document for MdDoc {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

/// The anchor of a heading as GitHub writes it: lower case, spaces as hyphens, other punctuation
/// left out.
pub fn slug(heading: &str) -> String {
    let mut out = String::new();
    for c in heading.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c == ' ' || c == '-' {
            out.push('-');
        } else if c == '_' {
            out.push('_');
        }
    }
    out
}

/// The page cut before the heading of an anchor, outside code blocks.
fn split_at_heading<'a>(text: &'a str, anchor: &str) -> Option<(&'a str, &'a str)> {
    let mut offset = 0;
    let mut in_code = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        if !in_code && t.starts_with('#') && slug(t.trim_start_matches('#')) == anchor {
            return Some((&text[..offset], &text[offset..]));
        }
        offset += line.len();
    }
    None
}

/// The blocks of a page that hold every word of a search, each with the heading above it: the
/// paragraphs, lists and code blocks between blank lines.
pub fn matching_blocks(text: &str, words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut heading = String::new();
    let mut in_code = false;
    let mut block = String::new();
    let flush = |block: &mut String, heading: &str, out: &mut Vec<String>| {
        if !block.trim().is_empty() && matches_words(block, words) {
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
                let jump = if doc.jump {
                    doc.anchor
                        .as_deref()
                        .and_then(|a| split_at_heading(&doc.text, a))
                } else {
                    None
                };
                match jump {
                    Some((before, after)) => {
                        CommonMarkViewer::new().show(ui, &mut doc.cache, before);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(1.0, 1.0), egui::Sense::hover());
                        ui.scroll_to_rect(rect, Some(egui::Align::Min));
                        CommonMarkViewer::new().show(ui, &mut doc.cache, after);
                        doc.jump = false;
                    }
                    None => {
                        CommonMarkViewer::new().show(ui, &mut doc.cache, &doc.text);
                    }
                }
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

    #[test]
    fn anchors_find_their_headings() {
        assert_eq!(slug(" Microsoft Edge"), "microsoft-edge");
        assert_eq!(slug("Table of removed apps"), "table-of-removed-apps");
        let text = "# Top\n\nText.\n\n## Microsoft Edge\n\nKey.\n";
        let (before, after) = split_at_heading(text, "microsoft-edge").unwrap();
        assert!(before.ends_with("Text.\n\n") && after.starts_with("## Microsoft Edge"));
    }
}
