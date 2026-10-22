//! Markdown tabs: a page of the program or any `.md` file, rendered with egui_commonmark, with
//! the search of the tab and the anchors of the headings.

use std::path::{Path, PathBuf};

use crate::doc::{Document, Search, matches_words};
use crate::settings::TableWidth;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// The widest a page is laid out, so that lines stay readable on a wide window.
const PAGE_WIDTH: f32 = 860.0;
/// The space between the columns of a table, and the least width of a column.
const TABLE_GAP: f32 = 12.0;
const COLUMN_MIN: f32 = 40.0;

/// A part of a page: Markdown for the viewer, or a table the page draws itself, so that its
/// width is the one of the settings and its cells wrap inside it.
enum Segment<'a> {
    Markdown(&'a str),
    Table(Vec<Vec<&'a str>>),
}

/// The cells of a row of a table, trimmed, without the bars at the ends.
fn cells(line: &str) -> Vec<&str> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    line.split('|').map(str::trim).collect()
}

/// The line under the header of a table: `---`, `:--`, `--:` or `:-:` in every cell.
fn is_separator(line: &str) -> bool {
    let parts = cells(line);
    line.contains('-')
        && !parts.is_empty()
        && parts.iter().all(|p| {
            let p = p.trim_start_matches(':').trim_end_matches(':');
            !p.is_empty() && p.chars().all(|c| c == '-')
        })
}

/// The page cut into Markdown and tables; a table inside a block of code stays Markdown.
fn segments(text: &str) -> Vec<Segment<'_>> {
    let mut starts = Vec::new();
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        starts.push(pos);
        pos += line.len();
    }
    starts.push(text.len());
    let line = |i: usize| text[starts[i]..starts[i + 1]].trim_end_matches(['\n', '\r']);
    let n = starts.len() - 1;
    let mut out = Vec::new();
    let mut from = 0;
    let mut fence = false;
    let mut i = 0;
    while i < n {
        let l = line(i).trim_start();
        if l.starts_with("```") || l.starts_with("~~~") {
            fence = !fence;
        }
        let table = !fence && l.contains('|') && i + 1 < n && is_separator(line(i + 1));
        if !table {
            i += 1;
            continue;
        }
        let mut end = i + 2;
        while end < n && line(end).contains('|') && !line(end).trim().is_empty() {
            end += 1;
        }
        if from < i {
            out.push(Segment::Markdown(&text[starts[from]..starts[i]]));
        }
        let rows = (i..end)
            .filter(|r| *r != i + 1)
            .map(|r| cells(line(r)))
            .collect();
        out.push(Segment::Table(rows));
        from = end;
        i = end;
    }
    if from < n {
        out.push(Segment::Markdown(&text[starts[from]..]));
    }
    out
}

/// A table of a page: a grid of its width, the header strong, the cells wrapped.
fn table(ui: &mut egui::Ui, cache: &mut CommonMarkCache, rows: &[Vec<&str>], width: f32, n: usize) {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let col = ((width - TABLE_GAP * (cols as f32 - 1.0)) / cols as f32).max(COLUMN_MIN);
    let p = crate::style::palette(ui);
    ui.add_space(4.0);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, p.border))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            egui::Grid::new(("page-table", n))
                .num_columns(cols)
                .spacing([TABLE_GAP, 4.0])
                .striped(true)
                .min_col_width(col)
                .max_col_width(col)
                .show(ui, |ui| {
                    for (r, row) in rows.iter().enumerate() {
                        for c in 0..cols {
                            let cell = row.get(c).copied().unwrap_or("");
                            ui.scope(|ui| {
                                ui.set_max_width(col);
                                if r == 0 {
                                    ui.strong(cell);
                                } else {
                                    CommonMarkViewer::new().show(ui, cache, cell);
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add_space(4.0);
}

/// A text of a page: its Markdown through the viewer, its tables as grids of `table_width`.
fn render(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    text: &str,
    table_width: f32,
    tables: &mut usize,
) {
    for segment in segments(text) {
        match segment {
            Segment::Markdown(md) => {
                if !md.trim().is_empty() {
                    CommonMarkViewer::new().show(ui, cache, md);
                }
            }
            Segment::Table(rows) => {
                *tables += 1;
                table(ui, cache, &rows, table_width, *tables);
            }
        }
    }
}

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

/// The page; `wrap` folds its lines to the width of the view (a readable width at most), else
/// the view scrolls sideways. A table or a block of code wider than the view scrolls sideways
/// either way.
pub fn md_view(ui: &mut egui::Ui, doc: &mut MdDoc, wrap: bool, tables: TableWidth) {
    let words = doc.search.words();
    let view_width = ui.available_width();
    let width = if wrap {
        view_width.min(PAGE_WIDTH)
    } else {
        f32::INFINITY
    };
    let table_width = match tables {
        TableWidth::View => width.min(view_width) - 2.0 * TABLE_GAP,
        TableWidth::Pixels(n) => n as f32,
        TableWidth::Characters(n) => {
            let font = egui::TextStyle::Body.resolve(ui.style());
            n as f32 * ui.fonts_mut(|f| f.glyph_width(&font, 'n'))
        }
    };
    let mut tables = 0;
    let code_bg = crate::style::palette(ui).code_bg;
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        // The code of a page takes the colour of the theme, inline and in blocks (a block
        // is a read-only text field).
        ui.visuals_mut().code_bg_color = code_bg;
        ui.visuals_mut().extreme_bg_color = code_bg;
        if width.is_finite() {
            ui.set_max_width(width);
        }
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
                    render(ui, &mut doc.cache, before, table_width, &mut tables);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(1.0, 1.0), egui::Sense::hover());
                    ui.scroll_to_rect(rect, Some(egui::Align::Min));
                    render(ui, &mut doc.cache, after, table_width, &mut tables);
                    doc.jump = false;
                }
                None => {
                    render(ui, &mut doc.cache, &doc.text, table_width, &mut tables);
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
            render(ui, &mut doc.cache, block, table_width, &mut tables);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_cut_into_markdown_and_tables() {
        let text = "Intro\n\n| A | B |\n|---|:-:|\n| 1 | two words |\n| 3 |\n\nAfter\n\n```\n| not | a table |\n|---|---|\n```\n";
        let parts = segments(text);
        assert_eq!(parts.len(), 3);
        assert!(matches!(parts[0], Segment::Markdown(m) if m == "Intro\n\n"));
        let Segment::Table(rows) = &parts[1] else {
            panic!("a table");
        };
        assert_eq!(rows, &vec![vec!["A", "B"], vec!["1", "two words"], vec!["3"]]);
        assert!(matches!(parts[2], Segment::Markdown(m) if m.starts_with("\nAfter") && m.contains("not | a table")));
        assert!(is_separator("| :--- | ---: |"));
        assert!(!is_separator("| a | b |"));
    }

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
