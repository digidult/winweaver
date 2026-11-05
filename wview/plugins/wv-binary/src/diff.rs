//! A file compared with a structure saved of it (`.wwbin`): the two trees matched node by node and
//! a line for every node with its mark (the same, changed, added in the file, removed from it),
//! and the comments and bookmarks of the lines kept in a `.wwbindiff` (docs/plugins.md section
//! 6.4).
//!
//! A node is matched among its siblings by its key when it has one (`sections[.text]`), else by
//! its name (`[3]` for an item of no key), and the second node of one name among its siblings is
//! `name#2`: a new section does not move the others. The path of a line, its names joined by dots,
//! is the key of its note. A value, a text, the SHA-256 of raw bytes or the count of an array
//! that differs is a change, and a structure changes when a node under it does; an offset alone
//! is none.
//!
//! ```json
//! { "format": "wwbindiff", "version": 1, "file": "kernel32.dll",
//!   "structure": "kernel32.dll.wwbin",
//!   "notes": [ { "key": "sections[.text].characteristics", "comment": "...", "bookmark": true } ] }
//! ```
//!
//! The paths of the file and of the structure are relative to the folder of the `.wwbindiff` when
//! they are inside it, else absolute.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use wwrt::serde::{Deserialize, Serialize};

use crate::engine::{Data, Tree};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Same,
    Changed,
    /// Only in the file.
    Added,
    /// Only in the structure.
    Removed,
}

/// A line of the comparison: a node of either tree or of both.
#[derive(Clone, Debug)]
pub struct Line {
    pub path: String,
    pub label: String,
    pub mark: Mark,
    /// The node in the structure saved and in the file.
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub children: Vec<usize>,
}

/// The note of a line.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
pub struct Note {
    pub key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bookmark: bool,
}

/// The `.wwbindiff` file.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(crate = "wwrt::serde", deny_unknown_fields)]
pub struct DiffFile {
    pub format: String,
    pub version: u32,
    pub file: String,
    pub structure: String,
    #[serde(default)]
    pub notes: Vec<Note>,
}

/// The largest `.wwbindiff` read, in bytes.
const LARGEST: u64 = 16 << 20;

/// The name of a node among its siblings, before the count of its twins.
fn segment(tree: &Tree, node: usize) -> String {
    let n = &tree.nodes[node];
    match &n.key {
        Some(k) => format!("[{k}]"),
        None => n.name.clone(),
    }
}

/// The children of a node with their names among them: `name`, then `name#2` for its twin.
fn named(tree: &Tree, node: usize) -> Vec<(String, usize)> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    tree.nodes[node]
        .children
        .iter()
        .map(|c| {
            let s = segment(tree, *c);
            let n = seen.entry(s.clone()).or_insert(0);
            *n += 1;
            let name = if *n == 1 { s } else { format!("{s}#{n}") };
            (name, *c)
        })
        .collect()
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else if name.starts_with('[') {
        format!("{path}{name}")
    } else {
        format!("{path}.{name}")
    }
}

/// Whether two nodes hold the same value, their children left aside. `sha` gives the SHA-256 of
/// a raw range of the file; the structure keeps it as its note.
fn same_value(
    old: &Tree,
    a: usize,
    new: &Tree,
    b: usize,
    sha: &mut dyn FnMut(usize) -> String,
) -> bool {
    match (&old.nodes[a].data, &new.nodes[b].data) {
        (Data::Raw, Data::Raw) => {
            old.nodes[a].size == new.nodes[b].size && old.nodes[a].note == sha(b)
        }
        (Data::Missing, Data::Missing) => true,
        (x, y) => x == y,
    }
}

struct Walk<'a> {
    old: &'a Tree,
    new: &'a Tree,
    sha: &'a mut dyn FnMut(usize) -> String,
    lines: Vec<Line>,
}

impl Walk<'_> {
    fn line(
        &mut self,
        path: String,
        label: String,
        old: Option<usize>,
        new: Option<usize>,
        parent: Option<usize>,
    ) -> usize {
        let index = self.lines.len();
        if let Some(p) = parent {
            self.lines[p].children.push(index);
        }
        self.lines.push(Line {
            path,
            label,
            mark: Mark::Same,
            old,
            new,
            children: Vec::new(),
        });
        index
    }

    /// The line of a pair of nodes (or of one) and the lines under it; gives its index.
    fn walk(
        &mut self,
        path: String,
        old: Option<usize>,
        new: Option<usize>,
        parent: Option<usize>,
    ) -> usize {
        let label = match (new, old) {
            (Some(n), _) => self.new.label(n),
            (None, Some(o)) => self.old.label(o),
            (None, None) => String::new(),
        };
        let me = self.line(path.clone(), label, old, new, parent);
        let olds = old.map(|o| named(self.old, o)).unwrap_or_default();
        let news = new.map(|n| named(self.new, n)).unwrap_or_default();
        let by_name: HashMap<&str, usize> = olds.iter().map(|(s, i)| (s.as_str(), *i)).collect();
        let mut changed = false;
        for (name, n) in &news {
            let o = by_name.get(name.as_str()).copied();
            let child = self.walk(join(&path, name), o, Some(*n), Some(me));
            changed |= self.lines[child].mark != Mark::Same;
        }
        let in_new: std::collections::HashSet<&str> =
            news.iter().map(|(s, _)| s.as_str()).collect();
        for (name, o) in &olds {
            if !in_new.contains(name.as_str()) {
                self.walk(join(&path, name), Some(*o), None, Some(me));
                changed = true;
            }
        }
        self.lines[me].mark = match (old, new) {
            (None, Some(_)) => Mark::Added,
            (Some(_), None) => Mark::Removed,
            (Some(o), Some(n)) => {
                if changed || !same_value(self.old, o, self.new, n, self.sha) {
                    Mark::Changed
                } else {
                    Mark::Same
                }
            }
            (None, None) => Mark::Same,
        };
        me
    }
}

/// The lines of a comparison of the structure `old` with the tree `new` of the file, the root
/// first.
pub fn compare(old: &Tree, new: &Tree, sha: &mut dyn FnMut(usize) -> String) -> Vec<Line> {
    let mut walk = Walk {
        old,
        new,
        sha,
        lines: Vec::new(),
    };
    walk.walk(String::new(), Some(0), Some(0), None);
    walk.lines
}

/// The count of the values of each mark (the lines of no lines under them): changed, added,
/// removed.
pub fn counts(lines: &[Line]) -> (usize, usize, usize) {
    let count = |m: Mark| {
        lines
            .iter()
            .filter(|l| l.mark == m && l.children.is_empty())
            .count()
    };
    (
        count(Mark::Changed),
        count(Mark::Added),
        count(Mark::Removed),
    )
}

/// A path as a `.wwbindiff` keeps it: relative to its folder when inside it.
pub fn kept(path: &Path, dir: &Path) -> String {
    match path.strip_prefix(dir) {
        Ok(inside) => inside.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

/// A path of a `.wwbindiff` as a path of the disk.
pub fn resolved(text: &str, dir: &Path) -> PathBuf {
    let path = PathBuf::from(text);
    if path.is_absolute() {
        path
    } else {
        dir.join(path)
    }
}

impl DiffFile {
    pub fn new(file: &Path, structure: &Path, at: &Path, notes: &BTreeMap<String, Note>) -> Self {
        let dir = at.parent().unwrap_or(Path::new(""));
        DiffFile {
            format: "wwbindiff".to_owned(),
            version: 1,
            file: kept(file, dir),
            structure: kept(structure, dir),
            notes: notes
                .values()
                .filter(|n| !n.comment.is_empty() || n.bookmark)
                .cloned()
                .collect(),
        }
    }

    /// A `.wwbindiff` read: strict, at most 16 MB.
    pub fn read(path: &Path) -> Result<Self, String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > LARGEST {
            return Err(format!("larger than {LARGEST} bytes"));
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let file: DiffFile = wwrt::serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if file.format != "wwbindiff" || file.version != 1 {
            return Err("not a .wwbindiff of version 1".to_owned());
        }
        Ok(file)
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = wwrt::serde_json::to_string_pretty(self).map_err(|e| e.to_string())? + "\n";
        wwrt::ww::dialogs::write_file(path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Limits, read};
    use crate::format::Format;

    const PAIRS: &str = r#"{ "format": "wwformat", "version": 1, "name": "Pairs",
        "extensions": [], "root": "file", "types": {
            "file": { "fields": [
                { "name": "count", "type": "u8" },
                { "name": "pairs", "type": "pair", "count": "count", "key": "name" },
                { "name": "tail", "type": "raw", "size": "2" } ] },
            "pair": { "fields": [
                { "name": "name", "type": "text", "size": "1" },
                { "name": "value", "type": "u8" } ] } } }"#;

    fn tree(bytes: Vec<u8>) -> Tree {
        let format = Format::parse(PAIRS).unwrap();
        let mut b = bytes;
        read(&format, &mut b, Limits::default())
    }

    fn line<'a>(lines: &'a [Line], path: &str) -> &'a Line {
        lines.iter().find(|l| l.path == path).unwrap_or_else(|| {
            panic!(
                "no line {path}: {:?}",
                lines.iter().map(|l| &l.path).collect::<Vec<_>>()
            )
        })
    }

    #[test]
    fn the_nodes_are_matched_by_their_keys() {
        // a=1 b=2, then the file: a new pair x before them, b changed, the tail the same.
        let mut old = tree(vec![2, b'a', 1, b'b', 2, 7, 7]);
        let new = tree(vec![3, b'x', 9, b'a', 1, b'b', 5, 7, 7]);
        // The structure keeps the SHA-256 of raw bytes as their note: the tail is the same.
        for n in &mut old.nodes {
            if n.data == Data::Raw {
                n.note = "same".to_owned();
            }
        }
        let lines = compare(&old, &new, &mut |_| "same".to_owned());
        assert_eq!(
            line(&lines, "pairs[a]").mark,
            Mark::Same,
            "a new item moves no other"
        );
        assert_eq!(line(&lines, "pairs[b].value").mark, Mark::Changed);
        assert_eq!(
            line(&lines, "pairs[b]").mark,
            Mark::Changed,
            "a structure with a change in it"
        );
        assert_eq!(line(&lines, "pairs[x]").mark, Mark::Added);
        assert_eq!(
            line(&lines, "pairs[x].value").mark,
            Mark::Added,
            "the nodes under it too"
        );
        assert_eq!(line(&lines, "tail").mark, Mark::Same, "moved, not changed");
        assert_eq!(line(&lines, "count").mark, Mark::Changed);
        assert_eq!(lines[0].mark, Mark::Changed);
        let back = compare(&new, &old, &mut |_| "same".to_owned());
        assert_eq!(line(&back, "pairs[x]").mark, Mark::Removed);
        let (changed, added, removed) = counts(&lines);
        assert_eq!(
            (changed, added, removed),
            (2, 2, 0),
            "the count and b; the name and value of x"
        );
    }

    #[test]
    fn twins_of_a_name_are_told_apart() {
        let old = tree(vec![2, b'a', 1, b'a', 2, 0, 0]);
        let new = tree(vec![2, b'a', 1, b'a', 3, 0, 0]);
        let lines = compare(&old, &new, &mut |_| String::new());
        assert_eq!(line(&lines, "pairs[a]").mark, Mark::Same);
        assert_eq!(line(&lines, "pairs[a]#2").mark, Mark::Changed);
    }

    #[test]
    fn a_comparison_file_keeps_its_notes_and_its_paths() {
        let dir = std::env::temp_dir().join(format!("wv-binary-diff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let at = dir.join("k.dll.wwbindiff");
        let mut notes = BTreeMap::new();
        notes.insert(
            "sections[.text].characteristics".to_owned(),
            Note {
                key: "sections[.text].characteristics".to_owned(),
                comment: "Writable now".to_owned(),
                bookmark: true,
            },
        );
        notes.insert(
            "empty".to_owned(),
            Note {
                key: "empty".to_owned(),
                ..Note::default()
            },
        );
        let outside = std::env::temp_dir().join("elsewhere.wwbin");
        let file = DiffFile::new(&dir.join("k.dll"), &outside, &at, &notes);
        assert_eq!(file.file, "k.dll", "inside its folder: relative");
        assert_eq!(PathBuf::from(&file.structure), outside, "outside: absolute");
        assert_eq!(file.notes.len(), 1, "a note of nothing is left out");
        file.write(&at).unwrap();
        let back = DiffFile::read(&at).unwrap();
        assert_eq!(back, file);
        assert_eq!(resolved(&back.file, &dir), dir.join("k.dll"));
        std::fs::write(
            &at,
            r#"{ "format": "wwbindiff", "version": 1, "file": "a", "structure": "b", "x": 1 }"#,
        )
        .unwrap();
        assert!(DiffFile::read(&at).is_err(), "strict");
    }
}
