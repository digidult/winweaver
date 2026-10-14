//! Markdown tabs (`docs/spec/01-ui.md` section 6 of the ideas, `03-features.md` section 9): the
//! guide embedded in the executable and any `.md` file, rendered with egui_commonmark.

use std::path::PathBuf;

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// The guide shown at the first start and from the menu (F1).
pub const GUIDE: &str = include_str!("../../../data/docs/guide.md");
pub const GUIDE_TITLE: &str = "Guide.md";

/// The widest a page is laid out, so that lines stay readable on a wide window.
const PAGE_WIDTH: f32 = 860.0;

pub struct MdDoc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
    cache: CommonMarkCache,
}

impl MdDoc {
    pub fn new(title: impl Into<String>, path: Option<PathBuf>, text: String) -> Self {
        Self {
            title: title.into(),
            path,
            text,
            cache: CommonMarkCache::default(),
        }
    }

    pub fn guide() -> Self {
        Self::new(GUIDE_TITLE, None, GUIDE.to_owned())
    }

    pub fn is_guide(&self) -> bool {
        self.path.is_none() && self.title == GUIDE_TITLE
    }
}

pub fn md_view(ui: &mut egui::Ui, doc: &mut MdDoc) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.set_max_width(PAGE_WIDTH);
            CommonMarkViewer::new().show(ui, &mut doc.cache, &doc.text);
        });
}
