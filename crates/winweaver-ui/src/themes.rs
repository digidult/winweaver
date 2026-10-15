//! Colour themes (`docs/spec/01-ui.md` section 14): Latte and Matrix are embedded, a file
//! `themes/<id>.json` next to the executable adds a theme. A theme names a light or dark base and
//! the colours it changes; a colour it leaves out keeps the base.

use std::collections::BTreeMap;
use std::path::Path;

use eframe::egui::Color32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_themes_load_and_colour() {
        let list = themes(&std::env::temp_dir().join("winweaver-no-themes"));
        assert_eq!(list.len(), 2);
        let matrix = list.iter().find(|t| t.id == "matrix").unwrap();
        assert!(matrix.dark);
        let p = crate::style::Palette::from_colors(matrix.dark, &matrix.colors);
        assert_eq!(p.bg, Color32::from_rgb(0, 0, 0));
        assert_eq!(color("#00ff41"), Some(Color32::from_rgb(0, 255, 65)));
        assert_eq!(color("00ff41"), None);
    }
}
