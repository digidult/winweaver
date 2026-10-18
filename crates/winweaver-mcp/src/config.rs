//! The configuration file of an MCP server, `mcp/<name>.wwmcp` next to the executable: the
//! transport and, for HTTP, the port and the token, the mode, the switches of this PC, the registry
//! and the library, the profile the workspace starts with, and whether the log keeps the whole
//! exchange. The window edits one in the MCP tab and saves it at every change; a start without the
//! window takes one with `--mcp FILE` or `--config FILE`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use winweaver_core::jsonfile;

use crate::{Mode, RegistryAccess};

/// The extension of the files, the folder they live in next to the executable and the name of
/// the one the window opens first.
pub const EXTENSION: &str = "wwmcp";
pub const FOLDER: &str = "mcp";
pub const DEFAULT_NAME: &str = "default";
pub const FORMAT: &str = "winweaver-mcp";
pub const VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 8765;
/// The shortest token the HTTP transport takes.
pub const TOKEN_MIN: usize = 32;
const MAX_BYTES: u64 = 64 * 1024;

/// How a client reaches the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// http://127.0.0.1:<port>/mcp with the bearer token.
    Http,
    /// The standard input and output of a process the client starts.
    Stdio,
}

impl Transport {
    pub fn name(self) -> &'static str {
        match self {
            Transport::Http => "http",
            Transport::Stdio => "stdio",
        }
    }
}

/// A configuration file. Every field but the format has a default, so a short file is enough.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub format: String,
    pub version: u32,
    pub transport: Transport,
    pub port: u16,
    /// The bearer token of HTTP; the window makes one when it is empty.
    pub token: String,
    pub mode: Mode,
    /// Whether the tool read_this_pc may read the settings of this PC.
    pub read_pc: bool,
    pub registry: RegistryAccess,
    /// Whether the tools may change the bookmarks and the catalog files.
    pub edit_library: bool,
    /// The preset or the profile file of the profiles folder the workspace starts with; empty
    /// for the first preset.
    pub profile: String,
    /// Whether the log keeps every request and answer whole (passwords, keys and tokens left
    /// out); off, the log keeps the call, its identifiers, its result and its time.
    pub detailed_log: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            transport: Transport::Http,
            port: DEFAULT_PORT,
            token: String::new(),
            mode: Mode::Read,
            read_pc: false,
            registry: RegistryAccess::Off,
            edit_library: false,
            profile: String::new(),
            detailed_log: false,
        }
    }
}

impl Config {
    /// The folder of the files next to an executable.
    pub fn folder(exe_dir: &Path) -> PathBuf {
        exe_dir.join(FOLDER)
    }

    /// The file the window opens first.
    pub fn default_path(exe_dir: &Path) -> PathBuf {
        Self::folder(exe_dir).join(format!("{DEFAULT_NAME}.{EXTENSION}"))
    }

    /// Reads a file strictly: an unknown field, a wrong format or a bad value is an error.
    pub fn load(path: &Path) -> Result<Self, String> {
        let config: Self = jsonfile::read(path, MAX_BYTES).map_err(|e| e.to_string())?;
        let name = jsonfile::display_name(path);
        if config.format != FORMAT || config.version > VERSION {
            return Err(format!("{name}: not an MCP configuration of this version"));
        }
        config.check().map_err(|e| format!("{name}: {e}"))?;
        Ok(config)
    }

    /// The values a server can start with: a port of the user range, a token of letters and
    /// digits that is empty or long enough.
    pub fn check(&self) -> Result<(), String> {
        if self.port < 1024 {
            return Err(format!("the port {} is below 1024", self.port));
        }
        let letters = self.token.chars().all(|c| c.is_ascii_alphanumeric());
        if !self.token.is_empty() && (self.token.len() < TOKEN_MIN || !letters) {
            return Err(format!("the token has at least {TOKEN_MIN} letters and digits"));
        }
        Ok(())
    }

    /// Writes the file, with its folder.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.check()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_goes_to_a_file_and_back() {
        let name = format!("winweaver-mcp-config-{}", std::process::id());
        let dir = std::env::temp_dir().join(name);
        let path = Config::default_path(&dir);
        let write = |text: &str| std::fs::write(&path, text).unwrap();
        let config = Config {
            token: "a".repeat(40),
            mode: Mode::Edit,
            registry: RegistryAccess::Read,
            detailed_log: true,
            ..Config::default()
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), config);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"mode\": \"edit\""), "{text}");
        assert!(text.contains("\"registry\": \"read\""), "{text}");
        assert!(text.contains("\"transport\": \"http\""), "{text}");
        // A short file takes the defaults; an unknown field or a short token is refused.
        write(r#"{"format": "winweaver-mcp", "transport": "stdio"}"#);
        let short = Config::load(&path).unwrap();
        assert_eq!(short.transport, Transport::Stdio);
        assert_eq!(short.mode, Mode::Read);
        write(r#"{"format": "winweaver-mcp", "colour": 1}"#);
        assert!(Config::load(&path).is_err());
        write(r#"{"format": "winweaver-mcp", "token": "short"}"#);
        assert!(Config::load(&path).is_err());
        write(r#"{"format": "winweaver-mcp", "mode": "all"}"#);
        assert!(Config::load(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
