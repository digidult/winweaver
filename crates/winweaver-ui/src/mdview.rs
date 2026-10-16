//! Markdown tabs (`docs/spec/01-ui.md` section 6 of the ideas, `03-features.md` section 9): the
//! guide embedded in the executable and any `.md` file, rendered with egui_commonmark.

use std::path::PathBuf;

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// The guide shown at the first start and from the menu (F1).
pub const GUIDE: &str = include_str!("../../../data/docs/guide.md");
/// The translated guides: the language code and the page.
pub const GUIDES: [(&str, &str); 8] = [
    ("uk", include_str!("../../../data/lang/uk/guide.md")),
    ("ru", include_str!("../../../data/lang/ru/guide.md")),
    ("de", include_str!("../../../data/lang/de/guide.md")),
    ("es", include_str!("../../../data/lang/es/guide.md")),
    ("fr", include_str!("../../../data/lang/fr/guide.md")),
    ("id", include_str!("../../../data/lang/id/guide.md")),
    ("it", include_str!("../../../data/lang/it/guide.md")),
    ("tr", include_str!("../../../data/lang/tr/guide.md")),
];
pub const GUIDE_TITLE: &str = "Guide.md";
/// The parameter reference of the catalog, carried over with its data (`data/reference/`): the
/// file name and the page. The `doc` field of a rule links into it, a heading as `#anchor`.
pub const REFERENCE: [(&str, &str); 22] = [
    (
        "00-architecture.md",
        include_str!("../../../data/reference/00-architecture.md"),
    ),
    (
        "01-windows-pe.md",
        include_str!("../../../data/reference/01-windows-pe.md"),
    ),
    (
        "02-specialize-xml.md",
        include_str!("../../../data/reference/02-specialize-xml.md"),
    ),
    (
        "03-oobe-accounts-languages.md",
        include_str!("../../../data/reference/03-oobe-accounts-languages.md"),
    ),
    (
        "04-users-and-media.md",
        include_str!("../../../data/reference/04-users-and-media.md"),
    ),
    (
        "05-printing.md",
        include_str!("../../../data/reference/05-printing.md"),
    ),
    (
        "06-windows-update.md",
        include_str!("../../../data/reference/06-windows-update.md"),
    ),
    (
        "07-defender.md",
        include_str!("../../../data/reference/07-defender.md"),
    ),
    (
        "08-accounts-uac-lsa-remote.md",
        include_str!("../../../data/reference/08-accounts-uac-lsa-remote.md"),
    ),
    (
        "09-network-smb-firewall.md",
        include_str!("../../../data/reference/09-network-smb-firewall.md"),
    ),
    (
        "10-removable-scripts-browser.md",
        include_str!("../../../data/reference/10-removable-scripts-browser.md"),
    ),
    (
        "11-logging-audit.md",
        include_str!("../../../data/reference/11-logging-audit.md"),
    ),
    (
        "12-telemetry-consumer-ai.md",
        include_str!("../../../data/reference/12-telemetry-consumer-ai.md"),
    ),
    (
        "13-services-apps.md",
        include_str!("../../../data/reference/13-services-apps.md"),
    ),
    (
        "14-default-user-profile.md",
        include_str!("../../../data/reference/14-default-user-profile.md"),
    ),
    (
        "15-per-user-script.md",
        include_str!("../../../data/reference/15-per-user-script.md"),
    ),
    (
        "16-post-oobe.md",
        include_str!("../../../data/reference/16-post-oobe.md"),
    ),
    (
        "17-cross-links.md",
        include_str!("../../../data/reference/17-cross-links.md"),
    ),
    (
        "18-browsers.md",
        include_str!("../../../data/reference/18-browsers.md"),
    ),
    (
        "19-more-privacy.md",
        include_str!("../../../data/reference/19-more-privacy.md"),
    ),
    (
        "20-explorer-namespaces.md",
        include_str!("../../../data/reference/20-explorer-namespaces.md"),
    ),
    (
        "README.md",
        include_str!("../../../data/reference/README.md"),
    ),
];

/// The widest a page is laid out, so that lines stay readable on a wide window.
const PAGE_WIDTH: f32 = 860.0;

pub struct MdDoc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
    cache: CommonMarkCache,
    pub search: crate::Search,
    /// The heading to show, by its anchor, and whether to scroll to it at the next frame.
    pub anchor: Option<String>,
    pub jump: bool,
}

impl MdDoc {
    pub fn new(title: impl Into<String>, path: Option<PathBuf>, text: String) -> Self {
        Self {
            title: title.into(),
            path,
            text,
            cache: CommonMarkCache::default(),
            search: crate::Search::default(),
            anchor: None,
            jump: false,
        }
    }

    pub fn guide() -> Self {
        Self::new(GUIDE_TITLE, None, guide_text(&crate::i18n::current()))
    }

    pub fn is_guide(&self) -> bool {
        self.path.is_none() && self.title == GUIDE_TITLE
    }

    /// The license of the program, built into the executable, so that it travels with the exe alone.
    pub fn license() -> Self {
        Self::new(LICENSE_TITLE, None, license_text())
    }

    pub fn is_license(&self) -> bool {
        self.path.is_none() && self.title == LICENSE_TITLE
    }
}

const LICENSE_TITLE: &str = "LICENSE";

/// The text of `LICENSE` as a Markdown page: its first line (the name of the license) a heading.
pub fn license_text() -> String {
    let text = include_str!("../../../LICENSE").replace("\r\n", "\n");
    match text.split_once('\n') {
        Some((first, rest)) if !first.starts_with('#') => format!("# {first}\n{rest}"),
        _ => text,
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
    fn the_license_is_built_in_as_a_page() {
        let text = license_text();
        assert!(text.starts_with("# MIT License\n"), "{text}");
        assert!(text.contains("Permission is hereby granted"));
        assert!(MdDoc::license().is_license() && !MdDoc::guide().is_license());
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
        let browsers = REFERENCE
            .iter()
            .find(|(n, _)| *n == "18-browsers.md")
            .unwrap()
            .1;
        assert!(split_at_heading(browsers, "microsoft-edge").is_some());
    }
}
