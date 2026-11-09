//! The configuration files of WinWeaver up to 0.1.46 (the format `winweaver-mcp`): the fields of
//! the server and the options of WinWeaver at the top level. A file of that format is written
//! again in the format of W# (`ww-mcp`), the options of WinWeaver in its branch `winweaver`.

use std::path::Path;

use serde::Deserialize;
use wwmcp::config::{Config, EXTENSION, MAX_BYTES, Transport};
use wwmcp::{Choice, Mode};

/// The format of the old files.
pub const FORMAT: &str = "winweaver-mcp";

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Old {
    format: String,
    version: u32,
    transport: Transport,
    port: u16,
    token: String,
    mode: Mode,
    read_pc: bool,
    registry: String,
    edit_library: bool,
    profile: String,
    detailed_log: bool,
}

impl Default for Old {
    fn default() -> Self {
        let new = Config::default();
        Self {
            format: String::new(),
            version: 1,
            transport: new.transport,
            port: new.port,
            token: String::new(),
            mode: Mode::Read,
            read_pc: false,
            registry: "off".to_owned(),
            edit_library: false,
            profile: String::new(),
            detailed_log: false,
        }
    }
}

/// The configuration of a text of the old format, in the format of W#; none for a text of
/// another format.
pub fn convert(text: &str, name: &str) -> Option<Result<Config, String>> {
    let probe: serde_json::Value = serde_json::from_str(text).ok()?;
    if probe.get("format").and_then(serde_json::Value::as_str) != Some(FORMAT) {
        return None;
    }
    let old: Old = match ww_base::jsonfile::parse(text, name) {
        Ok(o) => o,
        Err(e) => return Some(Err(e.to_string())),
    };
    let mut config = Config {
        transport: old.transport,
        port: old.port,
        token: old.token,
        mode: old.mode,
        detailed_log: old.detailed_log,
        ..Config::default()
    };
    let options = crate::options();
    let choice = |key: &str| -> Choice {
        options
            .iter()
            .find(|c| c.key == key)
            .cloned()
            .expect("an option of WinWeaver")
    };
    let switch = |on: bool| if on { "on" } else { "off" };
    let set = [
        ("read_pc", switch(old.read_pc).to_owned()),
        ("registry", old.registry),
        ("edit_library", switch(old.edit_library).to_owned()),
        ("profile", old.profile),
    ];
    for (key, value) in set {
        if let Err(e) = config.set_option(&choice(key), crate::SOURCE, &value) {
            return Some(Err(format!("{name}: {e}")));
        }
    }
    let _ = (old.format, old.version);
    Some(config.check().map(|()| config))
}

/// Writes a file of the old format again in the format of W#: whether it did.
pub fn migrate(path: &Path) -> Result<bool, String> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(false);
    };
    if meta.len() > MAX_BYTES {
        return Ok(false);
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let name = ww_base::jsonfile::display_name(path);
    match convert(&text, &name) {
        None => Ok(false),
        Some(Err(e)) => Err(e),
        Some(Ok(config)) => config.save(path).map(|()| true),
    }
}

/// Writes every file of the old format of a folder again in the format of W#; the names of the
/// files written, and the reasons of the ones that were not.
pub fn migrate_folder(dir: &Path) -> (Vec<String>, Vec<String>) {
    let mut done = Vec::new();
    let mut failed = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (done, failed);
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != EXTENSION) {
            continue;
        }
        let name = ww_base::jsonfile::display_name(&path);
        match migrate(&path) {
            Ok(true) => done.push(name),
            Ok(false) => {}
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    (done, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_configuration_takes_the_format_of_w_sharp() {
        let old = r#"{"format": "winweaver-mcp", "version": 1, "transport": "stdio", "port": 9000,
            "mode": "edit", "read_pc": true, "registry": "read", "edit_library": false,
            "profile": "Strict", "detailed_log": true}"#;
        let config = convert(old, "a.wwmcp").unwrap().unwrap();
        assert_eq!(config.transport, Transport::Stdio);
        assert_eq!(
            (config.port, config.mode, config.detailed_log),
            (9000, Mode::Edit, true)
        );
        let ours = config.source(crate::SOURCE);
        assert_eq!(ours.options["read_pc"], "on");
        assert_eq!(ours.options["registry"], "read");
        assert_eq!(ours.options["profile"], "Strict");
        assert!(!ours.options.contains_key("edit_library"));
        assert!(convert(r#"{"format": "ww-mcp"}"#, "b").is_none());
        assert!(convert("not json", "c").is_none());
        let wrong = r#"{"format": "winweaver-mcp", "registry": "all"}"#;
        assert!(convert(wrong, "d").unwrap().is_err());
        let dir = std::env::temp_dir().join(format!("winweaver-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("old.wwmcp");
        std::fs::write(&path, old).unwrap();
        std::fs::write(dir.join("other.txt"), old).unwrap();
        let (done, failed) = migrate_folder(&dir);
        assert_eq!((done, failed.len()), (vec!["old.wwmcp".to_owned()], 0));
        assert_eq!(Config::load(&path).unwrap(), config);
        assert!(!migrate(&path).unwrap(), "written once");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
