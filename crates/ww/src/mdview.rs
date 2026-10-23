//! Markdown tabs: a page of the program or any `.md` file, drawn by `markdown` (every paragraph
//! one text of egui), with the search of the tab and the anchors of the headings.

use std::path::{Path, PathBuf};

use crate::doc::{Document, Search, matches_words};
use crate::markdown::{self, Block, Outcome, View};
use crate::settings::PageWidth;
use eframe::egui;

pub struct MdDoc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
    /// The blocks of the text, parsed again when the text changes (its hash).
    parsed: Option<(u64, Vec<Block>)>,
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
            parsed: None,
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

/// The page, as wide as `page` says; `wrap` folds its lines to that width, else the view scrolls
/// sideways. Its tables fill the width of the page; a table or a block of code wider than the view scrolls
/// sideways either way. A link to a heading of the page brings it into view; any other link is
/// handed to egui (the shell opens the pages of the program, the system the web).
pub fn md_view(ui: &mut egui::Ui, doc: &mut MdDoc, wrap: bool, page: PageWidth) {
    let words = doc.search.words();
    let hash = crate::session::fnv(&doc.text);
    if doc.parsed.as_ref().is_none_or(|(h, _)| *h != hash) {
        doc.parsed = Some((hash, markdown::parse(&doc.text)));
    }
    let mut out = Outcome::default();
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let page = match page {
            PageWidth::View => ui.available_width(),
            PageWidth::Pixels(n) => n as f32,
            PageWidth::Characters(n) => {
                let font = crate::style::font(ui, 13.0);
                n as f32 * ui.fonts_mut(|f| f.glyph_width(&font, 'n'))
            }
        };
        let view = View {
            width: if wrap { page } else { f32::INFINITY },
            page,
            anchor: if doc.jump { doc.anchor.clone() } else { None },
        };
        if words.is_empty() {
            if let Some((_, blocks)) = &doc.parsed {
                markdown::show(ui, blocks, &view, &mut out);
            }
            return;
        }
        let places = matching_blocks(&doc.text, &words);
        ui.weak(crate::i18n::trf(
            "{n} places hold the search",
            &[("n", &places.len())],
        ));
        for place in &places {
            ui.separator();
            markdown::show(ui, &markdown::parse(place), &view, &mut out);
        }
    });
    if doc.jump {
        doc.jump = false;
    }
    match out.link {
        Some(link) if link.starts_with('#') => {
            doc.anchor = Some(link.trim_start_matches('#').to_owned());
            doc.jump = true;
            ui.ctx().request_repaint();
        }
        Some(link) => ui.ctx().open_url(egui::OpenUrl::same_tab(link)),
        None => {}
    }
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
    fn anchors_are_the_slugs_of_the_headings() {
        assert_eq!(slug(" Microsoft Edge"), "microsoft-edge");
        assert_eq!(slug("Table of removed apps"), "table-of-removed-apps");
        assert_eq!(slug("7. Work status"), "7-work-status");
    }
}
