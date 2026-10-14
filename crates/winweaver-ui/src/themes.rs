//! Colour themes (`docs/spec/01-ui.md` section 14): Latte and Matrix are embedded, a file
//! `themes/<id>.json` next to the executable adds a theme. A theme names a light or dark base and
//! the colours it changes; a colour it leaves out keeps the base.

use std::collections::BTreeMap;
use std::path::Path;

use eframe::egui::{self, Color32};
use serde::Deserialize;
use winweaver_core::jsonfile;

pub const EMBEDDED: [(&str, &str); 2] = [
    ("latte", include_str!("../../../data/themes/latte.json")),
    ("matrix", include_str!("../../../data/themes/matrix.json")),
];
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub dark: bool,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

/// The embedded themes and the files of the folder, by id; a file with the id of an embedded
/// theme replaces it.
pub fn themes(dir: &Path) -> Vec<Theme> {
    let mut out: Vec<Theme> = EMBEDDED
        .iter()
        .filter_map(|(id, text)| {
            jsonfile::parse::<Theme>(text, id).ok().map(|mut t| {
                t.id = (*id).to_owned();
                t
            })
        })
        .collect();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for path in entries.filter_map(Result::ok).map(|e| e.path()) {
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Ok(mut t) = jsonfile::read::<Theme>(&path, MAX_BYTES) {
                t.id = id;
                out.retain(|o| o.id != t.id);
                out.push(t);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// A colour written as `#rrggbb`.
pub fn color(text: &str) -> Option<Color32> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// The visuals of a theme: its base with the colours it names.
pub fn visuals(theme: &Theme) -> egui::Visuals {
    let mut v = if theme.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let c = |key: &str| theme.colors.get(key).and_then(|t| color(t));
    if let Some(bg) = c("background") {
        v.panel_fill = bg;
        v.window_fill = bg;
        v.faint_bg_color = bg;
    }
    if let Some(fg) = c("foreground") {
        v.override_text_color = Some(fg);
    }
    if let Some(field) = c("field") {
        v.extreme_bg_color = field;
        v.text_edit_bg_color = Some(field);
    }
    if let Some(sel) = c("select") {
        v.selection.bg_fill = sel;
    }
    if let Some(sel_fg) = c("select_foreground") {
        v.selection.stroke.color = sel_fg;
    }
    if let Some(border) = c("border") {
        v.window_stroke.color = border;
        v.widgets.noninteractive.bg_stroke.color = border;
    }
    if let Some(button) = c("button") {
        v.widgets.inactive.bg_fill = button;
        v.widgets.inactive.weak_bg_fill = button;
    }
    if let Some(active) = c("button_active") {
        v.widgets.hovered.bg_fill = active;
        v.widgets.hovered.weak_bg_fill = active;
        v.widgets.active.bg_fill = active;
        v.widgets.active.weak_bg_fill = active;
    }
    if let Some(muted) = c("muted") {
        v.widgets.noninteractive.fg_stroke.color = muted;
    }
    if let Some(link) = c("link") {
        v.hyperlink_color = link;
    }
    if let Some(e) = c("error") {
        v.error_fg_color = e;
    }
    if let Some(w) = c("warning") {
        v.warn_fg_color = w;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_themes_load_and_colour() {
        let list = themes(&std::env::temp_dir().join("winweaver-no-themes"));
        assert_eq!(list.len(), 2);
        let matrix = list.iter().find(|t| t.id == "matrix").unwrap();
        assert!(matrix.dark);
        let v = visuals(matrix);
        assert_eq!(v.panel_fill, Color32::from_rgb(0, 0, 0));
        assert_eq!(color("#00ff41"), Some(Color32::from_rgb(0, 255, 65)));
        assert_eq!(color("00ff41"), None);
    }
}
