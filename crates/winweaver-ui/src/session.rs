//! The session of the window: drafts of unsaved profiles, the log file (`docs/spec/02-documents.md`
//! section 8, `03-features.md` section 11). The open tabs are kept in the settings.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use winweaver_core::clock;
use winweaver_core::jsonfile;
use winweaver_core::profile::Profile;

pub const DRAFT_FORMAT: &str = "winweaver-draft";
pub const DRAFT_VERSION: u32 = 1;
const DRAFT_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The unsaved state of a profile, written next to the executable until it is saved or
/// discarded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub format: String,
    pub version: u32,
    /// The file the profile belongs to, empty for a profile never saved.
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub saved: String,
    pub profile: Profile,
}

/// FNV-1a, 64 bits: a stable name for the draft of a file.
pub fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The draft key of a profile file.
pub fn key_for_path(path: &Path) -> String {
    format!("p-{:016x}", fnv(&path.to_string_lossy().to_lowercase()))
}

/// A draft key for a profile that has no file yet, unique in this session.
pub fn new_key() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("n-{start}-{n}")
}

fn draft_file(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}.json"))
}

/// Writes the draft of a profile through a temporary file.
pub fn write_draft(dir: &Path, key: &str, path: Option<&Path>, profile: &Profile) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let draft = Draft {
        format: DRAFT_FORMAT.to_owned(),
        version: DRAFT_VERSION,
        path: path.map(|p| p.display().to_string()).unwrap_or_default(),
        saved: clock::now_utc_iso(),
        profile: profile.clone(),
    };
    let mut text = serde_json::to_string_pretty(&draft).map_err(|e| e.to_string())?;
    text.push('\n');
    let file = draft_file(dir, key);
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &file).map_err(|e| format!("{}: {e}", file.display()))
}

/// Deletes the draft of a key, if there is one.
pub fn remove_draft(dir: &Path, key: &str) {
    let _ = std::fs::remove_file(draft_file(dir, key));
}

/// The drafts of a folder with their keys, oldest first, and the names of the files that could
/// not be read (they stay where they are).
pub fn read_drafts(dir: &Path) -> (Vec<(String, Draft)>, Vec<String>) {
    let mut drafts = Vec::new();
    let mut broken = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (drafts, broken);
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for file in files {
        let name = jsonfile::display_name(&file);
        let read = jsonfile::read::<Draft>(&file, DRAFT_MAX_BYTES)
            .map_err(|e| e.to_string())
            .and_then(|d| {
                if d.format == DRAFT_FORMAT {
                    Ok(d)
                } else {
                    Err(format!("{name}: not a draft"))
                }
            });
        match read {
            Ok(draft) => {
                let key = file
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                drafts.push((key, draft));
            }
            Err(e) => broken.push(e),
        }
    }
    drafts.sort_by(|a, b| a.1.saved.cmp(&b.1.saved));
    (drafts, broken)
}

/// Appends a line to the log of the day in `dir`; a log that cannot be written is ignored.
pub fn log(dir: &Path, level: &str, text: &str) {
    let now = clock::now_utc_iso();
    let day = now.get(..10).unwrap_or("unknown");
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let file = dir.join(format!("winweaver-{day}.log"));
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    {
        let line = text.replace(['\r', '\n'], " ");
        let _ = writeln!(f, "[{now}] [{level}] {line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_round_trip_and_broken_files_are_listed() {
        let dir = std::env::temp_dir().join(format!("winweaver-drafts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let profile = Profile::new("Draft", "0.11");
        let key = key_for_path(Path::new("C:\\Profiles\\Office.wwprofile"));
        assert_eq!(key, key_for_path(Path::new("c:\\profiles\\office.wwprofile")));
        write_draft(&dir, &key, Some(Path::new("C:\\Profiles\\Office.wwprofile")), &profile).unwrap();
        let other = new_key();
        assert_ne!(other, new_key());
        write_draft(&dir, &other, None, &profile).unwrap();
        std::fs::write(dir.join("broken.json"), "{").unwrap();
        let (drafts, broken) = read_drafts(&dir);
        assert_eq!(drafts.len(), 2);
        assert_eq!(broken.len(), 1);
        assert!(drafts.iter().any(|(k, d)| k == &key && d.path.ends_with("Office.wwprofile")));
        remove_draft(&dir, &key);
        remove_draft(&dir, &other);
        assert_eq!(read_drafts(&dir).0.len(), 0);
        log(&dir, "INFO", "a line\nwith a break");
        let logs: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("winweaver-"))
            .collect();
        assert_eq!(logs.len(), 1);
        let text = std::fs::read_to_string(logs[0].path()).unwrap();
        assert!(text.contains("[INFO] a line with a break"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
