//! The settings of the window: `settings.json` next to the executable (`docs/spec/02-documents.md`
//! section 7). The window is the only writer; a missing or unreadable file means the defaults.

use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};
use winweaver_core::{jsonfile, paths};

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
    /// The id of a colour theme of themes/ or the embedded ones, empty for the colours of the theme.
    pub custom_theme: String,
    /// The tabs with files open at the last close are opened again at the start.
    pub restore_tabs: bool,
    /// The files of the tabs open at the last close, and the active one among them.
    pub open_tabs: Vec<PathBuf>,
    pub active_tab: usize,
    /// The port and the token of the MCP server of an earlier version: read once to make
    /// `mcp/default.wwmcp`, never written again.
    #[serde(skip_serializing)]
    pub mcp_port: u16,
    #[serde(skip_serializing)]
    pub mcp_token: String,
    /// The ids of the imports of policy templates that are loaded; the other catalog files of
    /// imports/ are never read. `None` in the settings of an earlier version, whose imports were
    /// all read: then every import but the hidden ones is loaded.
    pub loaded_imports: Option<Vec<String>>,
    /// The imports an earlier version hid; read once to fill `loaded_imports`, never written.
    #[serde(skip_serializing)]
    pub hidden_imports: Vec<String>,
    /// The options of a profile whose catalog is not loaded are left out of the tree.
    pub hide_unknown: bool,
    /// The parts of the description of an option open at first.
    pub desc_sections: DescSections,
    /// The renderer the window tries first (`opengl`, `directx`), empty for Automatic; a name this
    /// executable lacks is Automatic. `--renderer` overrides it for one start.
    pub renderer: String,
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
            custom_theme: String::new(),
            restore_tabs: true,
            open_tabs: Vec::new(),
            active_tab: 0,
            mcp_port: 8765,
            mcp_token: String::new(),
            loaded_imports: None,
            hidden_imports: Vec::new(),
            hide_unknown: false,
            renderer: String::new(),
            desc_sections: DescSections::default(),
        }
    }
}

/// The parts of the description of an option open at first; folded ones open with a click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DescSections {
    pub what: bool,
    pub risk: bool,
    pub writes: bool,
    pub deps: bool,
    pub more: bool,
    pub reference: bool,
}

impl Default for DescSections {
    fn default() -> Self {
        Self {
            what: true,
            risk: true,
            writes: true,
            deps: false,
            more: false,
            reference: false,
        }
    }
}

/// The folder of the executable, or the current folder when it cannot be found.
pub fn exe_dir() -> PathBuf {
    paths::program_dir()
}

impl Settings {
    pub fn path() -> PathBuf {
        exe_dir().join("settings.json")
    }

    /// The settings of a file; the defaults when the file does not exist. The files the settings
    /// keep relative to their folder (the program folder) are absolute in memory.
    pub fn load_from(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut settings: Self = jsonfile::read(path, MAX_BYTES).map_err(|e| e.to_string())?;
        if settings.format != FORMAT {
            return Err(format!(
                "{}: not a WinWeaver settings file",
                jsonfile::display_name(path)
            ));
        }
        let base = folder_of(path);
        for file in settings.recent.iter_mut().chain(&mut settings.open_tabs) {
            *file = paths::resolved(file, &base);
        }
        Ok(settings)
    }

    /// Writes the settings through a temporary file that replaces the target; a file inside the
    /// folder of the settings is kept relative to it, so the program folder can move.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        let base = folder_of(path);
        let mut kept = self.clone();
        for file in kept.recent.iter_mut().chain(&mut kept.open_tabs) {
            *file = paths::stored(file, &base);
        }
        let mut text = serde_json::to_string_pretty(&kept).map_err(|e| e.to_string())?;
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

/// The folder of a file, absolute; the current folder for a bare name.
fn folder_of(path: &Path) -> PathBuf {
    let folder = path.parent().unwrap_or(Path::new(""));
    std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf())
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

    #[test]
    fn files_inside_the_program_folder_are_kept_relative() {
        let dir = std::env::temp_dir().join(format!("winweaver-relative-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let inside = dir.join("profiles").join("Office.wwprofile");
        let outside = std::env::temp_dir().join("Elsewhere.wwprofile");
        let mut settings = Settings::default();
        settings.remember(&outside);
        settings.remember(&inside);
        settings.open_tabs = vec![inside.clone()];
        settings.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let kept = Path::new(".").join("profiles").join("Office.wwprofile");
        let quoted = serde_json::to_string(&kept).unwrap();
        assert!(text.contains(&quoted), "{text}");
        assert!(!text.contains(&serde_json::to_string(&inside).unwrap()));
        assert!(text.contains(&serde_json::to_string(&outside).unwrap()));
        let moved = std::env::temp_dir().join(format!("winweaver-moved-{}", std::process::id()));
        std::fs::create_dir_all(&moved).unwrap();
        std::fs::copy(&path, moved.join("settings.json")).unwrap();
        let loaded = Settings::load_from(&moved.join("settings.json")).unwrap();
        let there = moved.join("profiles").join("Office.wwprofile");
        assert_eq!(loaded.recent, vec![there.clone(), outside]);
        assert_eq!(loaded.open_tabs, vec![there]);
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&moved).unwrap();
    }
}
