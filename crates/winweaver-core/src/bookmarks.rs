//! Bookmark files (`.wwmarks`): positions in documents with a comment
//! (`docs/spec/02-documents.md` section 6).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::jsonfile::{self, JsonError};
use crate::paths;

pub const FORMAT: &str = "winweaver-bookmarks";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "wwmarks";
/// The default file next to the executable, created at the first bookmark.
pub const DEFAULT_FILE: &str = "bookmarks.wwmarks";
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum BookmarksError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error("{name}: {source}")]
    Io {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{name}: not a WinWeaver bookmark file (format {found})")]
    Format { name: String, found: String },
    #[error("{name}: bookmark file version {found} is newer than this program knows ({VERSION})")]
    Version { name: String, found: u32 },
}

/// The kind of document of a bookmark of an option.
pub const OPTION: &str = "profile";
/// The document of a bookmark of the registry: `path` is the key, `item` the value (empty for the key).
pub const REGISTRY: &str = "registry";

/// A bookmark of an option: its id (a rule, a group or a form) in `item`, the profile it was made
/// in as a link (`path`, empty for an unsaved profile or a catalog), a comment and the time. Its
/// title comes from the catalog when it is shown. Files of version 1 may also hold positions in
/// other documents (`diff`, `markdown`) with a `title`; they are read and kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Bookmark {
    pub document: String,
    pub path: String,
    pub item: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    pub comment: String,
    pub created: String,
}

impl Bookmark {
    /// The bookmark of an option, with the profile it was made in.
    pub fn option(item: &str, path: &str) -> Self {
        Self {
            document: OPTION.to_owned(),
            path: path.to_owned(),
            item: item.to_owned(),
            ..Self::default()
        }
    }

    /// The bookmark of a key of the registry, or of one of its values.
    pub fn registry(key: &str, value: &str) -> Self {
        Self {
            document: REGISTRY.to_owned(),
            path: key.to_owned(),
            item: value.to_owned(),
            ..Self::default()
        }
    }

    pub fn is_registry(&self) -> bool {
        self.document == REGISTRY
    }

    /// Whether this is the bookmark of an option.
    pub fn is_option(&self) -> bool {
        self.document == OPTION && !self.item.is_empty()
    }

    /// Whether two bookmarks point at the same place: the same option wherever it was marked,
    /// or the same position of the same document.
    pub fn same_position(&self, other: &Bookmark) -> bool {
        if self.is_option() && other.is_option() {
            return self.item == other.item;
        }
        if self.is_registry() && other.is_registry() {
            return self.path.eq_ignore_ascii_case(&other.path)
                && self.item.eq_ignore_ascii_case(&other.item);
        }
        self.document == other.document && self.path == other.path && self.item == other.item
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmarks {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub items: Vec<Bookmark>,
}

impl Default for Bookmarks {
    fn default() -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            items: Vec::new(),
        }
    }
}

impl Bookmarks {
    pub fn find(&self, position: &Bookmark) -> Option<usize> {
        self.items.iter().position(|b| b.same_position(position))
    }

    /// Adds the bookmark or, when its position is already there, removes it; `true` when added.
    pub fn toggle(&mut self, bookmark: Bookmark) -> bool {
        match self.find(&bookmark) {
            Some(i) => {
                self.items.remove(i);
                false
            }
            None => {
                self.items.push(bookmark);
                true
            }
        }
    }

    /// The file, or the empty list when the file does not exist.
    pub fn load_or_default(path: &Path) -> Result<Self, BookmarksError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load(path)
    }

    /// The file with the documents inside the program folder named by absolute paths again.
    pub fn load(path: &Path) -> Result<Self, BookmarksError> {
        let name = jsonfile::display_name(path);
        let text = jsonfile::read_text(path, MAX_BYTES)?;
        let mut marks = Self::parse(&text, &name)?;
        marks.resolve_in(&paths::program_dir());
        Ok(marks)
    }

    /// The bookmarks as a file keeps them: a document inside `base` relative to it.
    pub fn stored_in(&self, base: &Path) -> Self {
        let mut marks = self.clone();
        for b in marks.items.iter_mut().filter(|b| !b.is_registry()) {
            b.path = paths::stored_text(&b.path, base);
        }
        marks
    }

    /// The bookmarks of a file with the documents kept relative to `base` made absolute.
    pub fn resolve_in(&mut self, base: &Path) {
        for b in self.items.iter_mut().filter(|b| !b.is_registry()) {
            b.path = paths::resolved_text(&b.path, base);
        }
    }

    pub fn parse(text: &str, name: &str) -> Result<Self, BookmarksError> {
        let marks: Self = jsonfile::parse(text, name)?;
        if marks.format != FORMAT {
            return Err(BookmarksError::Format {
                name: name.to_owned(),
                found: marks.format,
            });
        }
        if marks.version > VERSION {
            return Err(BookmarksError::Version {
                name: name.to_owned(),
                found: marks.version,
            });
        }
        Ok(marks)
    }

    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    pub fn save(&self, path: &Path) -> Result<(), BookmarksError> {
        let name = jsonfile::display_name(path);
        let io = |source: std::io::Error| BookmarksError::Io {
            name: name.clone(),
            source,
        };
        let tmp = path.with_extension("tmp");
        let text = self.stored_in(&paths::program_dir()).to_json();
        std::fs::write(&tmp, text).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(item: &str) -> Bookmark {
        Bookmark {
            document: "profile".into(),
            path: "profiles/office.wwprofile".into(),
            item: item.into(),
            title: item.to_uppercase(),
            ..Bookmark::default()
        }
    }

    #[test]
    fn an_option_is_one_bookmark_in_every_profile_and_keeps_no_title() {
        let mut marks = Bookmarks::default();
        assert!(marks.toggle(Bookmark::option(
            "defender.pua",
            "profiles/office.wwprofile"
        )));
        let elsewhere = Bookmark::option("defender.pua", "profiles/strict.wwprofile");
        assert_eq!(marks.find(&elsewhere), Some(0));
        assert_eq!(marks.find(&Bookmark::option("defender.pua", "")), Some(0));
        assert!(!marks.to_json().contains("\"title\""));
        let page = Bookmark {
            document: "markdown".into(),
            path: "docs/guide.md".into(),
            item: "#apply".into(),
            title: "Apply".into(),
            ..Bookmark::default()
        };
        assert!(marks.toggle(page.clone()));
        assert!(marks.to_json().contains("\"title\": \"Apply\""));
        assert_eq!(marks.find(&page), Some(1));
        assert!(!marks.toggle(elsewhere));
        assert_eq!(marks.items, vec![page]);
    }

    #[test]
    fn toggle_find_and_round_trip() {
        let mut marks = Bookmarks::default();
        assert!(marks.toggle(mark("defender.pua")));
        assert!(marks.toggle(mark("asr")));
        assert_eq!(marks.find(&mark("asr")), Some(1));
        assert!(!marks.toggle(mark("defender.pua")));
        assert_eq!(marks.items.len(), 1);
        let dir = std::env::temp_dir().join(format!("winweaver-marks-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(DEFAULT_FILE);
        assert_eq!(
            Bookmarks::load_or_default(&path).unwrap(),
            Bookmarks::default()
        );
        marks.save(&path).unwrap();
        assert_eq!(Bookmarks::load(&path).unwrap(), marks);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(Bookmarks::parse(r#"{"format": "x", "version": 1}"#, "x").is_err());
    }

    #[test]
    fn a_profile_inside_the_program_folder_is_kept_relative() {
        let base = std::env::temp_dir().join("ww-marks").join("WinWeaver");
        let file = base.join("profiles").join("Office.wwprofile");
        let path = file.display().to_string();
        let mut marks = Bookmarks::default();
        marks.toggle(Bookmark::option("defender.pua", &path));
        marks.toggle(Bookmark::option("asr", "Strict"));
        marks.toggle(Bookmark::registry("HKEY_CURRENT_USER\\Software", ""));
        let kept = marks.stored_in(&base);
        let first = &kept.items[0].path;
        assert!(first.starts_with('.'), "{first}");
        assert_eq!(kept.items[1..], marks.items[1..]);
        let mut read = kept.clone();
        read.resolve_in(&base);
        assert_eq!(read, marks);
    }
}
