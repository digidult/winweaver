//! The settings of the window: `settings.json` next to the executable (`docs/spec/02-documents.md`
//! section 7). The window is the only writer; a missing or unreadable file means the defaults.

use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};
use winweaver_core::jsonfile;

pub const FORMAT: &str = "winweaver-settings";
pub const VERSION: u32 = 1;
const MAX_BYTES: u64 = 1024 * 1024;
const RECENT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::System => egui::ThemePreference::System,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub format: String,
    pub version: u32,
    /// The language of the interface and the catalog texts: a code, empty to follow Windows.
    pub language: String,
    pub theme: ThemeChoice,
    /// Whether the description panel is shown.
    pub desc_open: bool,
    /// The recently opened or saved profiles, the newest first.
    pub recent: Vec<PathBuf>,
    pub search_history: Vec<String>,
    /// The guide opens at the start; the window clears it after the first start.
    pub show_guide_at_start: bool,
    /// The rules of the selected group show a switch at the end of their rows.
    pub inline_switches: bool,
    /// The tabs with files open at the last close are opened again at the start.
    pub restore_tabs: bool,
    /// The files of the tabs open at the last close, and the active one among them.
    pub open_tabs: Vec<PathBuf>,
    pub active_tab: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            language: String::new(),
            theme: ThemeChoice::System,
            desc_open: true,
            recent: Vec::new(),
            search_history: Vec::new(),
            show_guide_at_start: true,
            inline_switches: true,
            restore_tabs: true,
            open_tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

/// The folder of the executable, or the current folder when it cannot be found.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Settings {
    pub fn path() -> PathBuf {
        exe_dir().join("settings.json")
    }

    /// The settings of a file; the defaults when the file does not exist.
    pub fn load_from(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let settings: Self = jsonfile::read(path, MAX_BYTES).map_err(|e| e.to_string())?;
        if settings.format != FORMAT {
            return Err(format!(
                "{}: not a WinWeaver settings file",
                jsonfile::display_name(path)
            ));
        }
        Ok(settings)
    }

    /// Writes the settings through a temporary file that replaces the target.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    /// Puts a file first in the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("winweaver-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
        let mut settings = Settings {
            language: "uk".to_owned(),
            theme: ThemeChoice::Dark,
            ..Settings::default()
        };
        settings.remember(Path::new("a.wwprofile"));
        settings.remember(Path::new("b.wwprofile"));
        settings.remember(Path::new("a.wwprofile"));
        settings.save_to(&path).unwrap();
        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded, settings);
        assert_eq!(
            loaded.recent,
            vec![PathBuf::from("a.wwprofile"), PathBuf::from("b.wwprofile")]
        );
        std::fs::write(&path, "{\"format\": \"other\"}").unwrap();
        assert!(Settings::load_from(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
