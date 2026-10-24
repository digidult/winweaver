//! The tab of a registry file (`.reg`), shown as a structure rather than as text: on the left the
//! outline of the file, its keys and its comments in their order; on the right the selected key
//! with its values in the types of regedit and the comments between them, the way to the registry
//! tab and to the rules of the loaded catalogs that write it. A key, a value or a comment is edited
//! in a window over the tab and lines are removed after a question; Save writes every line back,
//! the ones the program does not read too.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;

use eframe::egui;
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::regfile::{Change, Line, RegFile};
use winweaver_core::registry::{self, Data, NEW_TYPES};

use crate::i18n::{tr, trf};
use crate::regview::{self, value_title};
use crate::views::{self, rtitle};
use crate::{
    Act, ProfileAct, Question, RegFileAct, RegistryAct, Search, Tab, icons, list, matches_words,
    regedit, style, tree, widgets,
};

mod editor;
#[cfg(test)]
mod tests;
mod view;

pub(crate) use editor::*;
pub(crate) use view::*;

/// What of a registry file is applied to this PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyScope {
    Value,
    Key,
    File,
}

/// The types the editor of a value offers: the ones of the registry, and REG_NONE as a file may
/// hold it.
fn value_types() -> Vec<&'static str> {
    NEW_TYPES.iter().copied().chain(["REG_NONE"]).collect()
}
/// The most characters of data a row shows.
const SHOWN_MAX: usize = 200;
/// Where the registry files of the plans load the default user profile.
const DEFAULT_USER: &str = "HKEY_USERS\\UnattendDefault";

/// An entry of the outline: a key with its values, the comments right above it and the ones
/// between its values; or comments and other lines outside a key.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Key { line: usize, lines: Range<usize> },
    Notes { lines: Range<usize> },
}

impl Item {
    pub fn lines(&self) -> Range<usize> {
        match self {
            Item::Key { lines, .. } | Item::Notes { lines } => lines.clone(),
        }
    }

    /// The line the outline selects: the key, or the first line of the comments.
    pub fn anchor(&self) -> usize {
        match self {
            Item::Key { line, .. } => *line,
            Item::Notes { lines } => lines.start,
        }
    }
}

/// The entries of the lines of a file, in their order; blank lines between them belong to none.
pub fn items(lines: &[Line]) -> Vec<Item> {
    let n = lines.len();
    let note = |l: &Line| matches!(l, Line::Comment(_) | Line::Other(_));
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if lines[i] == Line::Blank {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        while j < n && note(&lines[j]) {
            j += 1;
        }
        if j < n && matches!(lines[j], Line::Key { .. }) {
            // The key goes on over its values and over the lines between them.
            let mut end = j + 1;
            loop {
                let mut k = end;
                while k < n && !matches!(lines[k], Line::Key { .. } | Line::Value { .. }) {
                    k += 1;
                }
                if k < n && matches!(lines[k], Line::Value { .. }) {
                    end = k + 1;
                } else {
                    break;
                }
            }
            out.push(Item::Key {
                line: j,
                lines: start..end,
            });
            i = end;
        } else {
            // Comments, or a value before the first key.
            let end = if j > start { j } else { start + 1 };
            out.push(Item::Notes { lines: start..end });
            i = end;
        }
    }
    out
}

/// Whether a key is in the default user profile as the files of the plans load it.
fn in_default_user(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower
        .strip_prefix(DEFAULT_USER.to_lowercase().as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('\\'))
}

/// The key of a path as the catalog names it, in lower case: the default user profile of a plan
/// is the current user there, as `DU:` is read.
/// A key as the outline shows it: the short name of its hive, and only the last three parts of a
/// longer path, so that the keys of one branch are told apart; the head of the tab shows it whole.
fn short_path(path: &str) -> String {
    let (hive, rest) = path.split_once('\\').unwrap_or((path, ""));
    let short = registry::HIVES
        .iter()
        .find(|(long, _)| long.eq_ignore_ascii_case(hive))
        .map_or(hive, |(_, s)| *s);
    let parts: Vec<&str> = rest.split('\\').filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => short.to_owned(),
        1..=3 => format!("{short}\\{}", parts.join("\\")),
        n => format!("{short}\\...\\{}", parts[n - 3..].join("\\")),
    }
}

fn catalog_key(path: &str) -> String {
    let path = registry::normalize(path).unwrap_or_else(|| path.to_owned());
    let lower = path.to_lowercase();
    if in_default_user(&lower) {
        let rest = &lower[DEFAULT_USER.len()..];
        return format!("hkey_current_user{rest}");
    }
    lower
}

/// The rule a comment of the file of a plan names (`; Rule <id>`).
fn rule_of_comment(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(';')?.trim();
    let id = rest.strip_prefix("Rule ")?.trim();
    (!id.is_empty() && !id.contains(' ')).then_some(id)
}

/// The text of a line the search looks at, in lower case.
fn search_text(line: &Line) -> String {
    match line {
        Line::Blank => String::new(),
        Line::Comment(t) | Line::Other(t) => t.to_lowercase(),
        Line::Key { path, .. } => path.to_lowercase(),
        Line::Value { name, data } => {
            let shown = data.as_ref().map(Data::display).unwrap_or_default();
            format!("{name} {shown}").to_lowercase()
        }
    }
}

/// Data shortened for a row.
fn short(text: &str) -> String {
    if text.chars().count() > SHOWN_MAX {
        text.chars().take(SHOWN_MAX).collect::<String>() + "..."
    } else {
        text.to_owned()
    }
}

/// A text in one line.
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

/// What the editor over the tab changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Key,
    Value,
    Comment,
    /// A line the program does not read, edited as its text.
    Raw,
}

/// The editor over the tab.
pub struct Edit {
    pub kind: EditKind,
    /// The line it changes; `None` for a new line, which goes in at `at`.
    pub line: Option<usize>,
    pub at: usize,
    /// The path of a key, the name of a value or the text of a comment after its `;`.
    pub name: String,
    pub type_name: String,
    pub text: String,
    /// The key is deleted with its subkeys, the value is deleted.
    pub delete: bool,
    pub error: Option<String>,
}

pub struct RegFileDoc {
    pub path: PathBuf,
    pub file: RegFile,
    pub dirty: bool,
    /// A line of the selected entry of the outline; `None` for the whole file.
    pub at: Option<usize>,
    /// The selected line of the entry, on the right.
    pub line: Option<usize>,
    pub edit: Option<Edit>,
    pub error: Option<String>,
    pub search: Search,
    /// The selected entry and the selected line come into view at the next frame.
    pub follow: bool,
    pub follow_line: bool,
    /// The rules of the catalog by the keys they write, with the count of rules it was made of.
    writers: Option<regview::Writers>,
    /// The keys folded in the tree of the outline, in lower case.
    pub closed: HashSet<String>,
    pub draft: ww::DraftState,
}

impl RegFileDoc {
    pub fn new(file: RegFile, path: PathBuf) -> Self {
        let draft = ww::DraftState::new(Some(&path));
        Self {
            path,
            file,
            dirty: false,
            at: None,
            line: None,
            edit: None,
            error: None,
            search: Search::default(),
            follow: false,
            follow_line: false,
            writers: None,
            closed: HashSet::new(),
            draft,
        }
    }

    /// The changes of the registry of the selected value, the selected key with its values, or
    /// the whole file; `None` when that part is not selected.
    pub fn changes(&self, scope: ApplyScope) -> Option<Vec<Change>> {
        let changes = match scope {
            ApplyScope::File => self.file.changes(None),
            ApplyScope::Key => {
                let items = items(&self.file.lines);
                let item = &items[self.item(&items)?];
                if !matches!(item, Item::Key { .. }) {
                    return None;
                }
                self.file.changes(Some(item.lines()))
            }
            ApplyScope::Value => {
                let line = self.line?;
                if !matches!(self.file.lines.get(line), Some(Line::Value { .. })) {
                    return None;
                }
                self.file.changes(Some(line..line + 1))
            }
        };
        (!changes.is_empty()).then_some(changes)
    }

    /// The question before a part of the file changes this PC, with the act of a yes.
    pub fn apply_question(&self, scope: ApplyScope) -> Option<Question> {
        let changes = self.changes(scope)?;
        let deleted = changes
            .iter()
            .filter(|c| matches!(c, Change::DeleteKey(_) | Change::DeleteValue { .. }))
            .count();
        let mut text = trf(
            "Apply {n} changes of the registry to this PC? The keys of the computer need WinWeaver started as administrator.",
            &[("n", &changes.len())],
        );
        if deleted > 0 {
            text.push(' ');
            text.push_str(&trf(
                "{n} of them delete a key or a value, which no undo brings back.",
                &[("n", &deleted)],
            ));
        }
        Some(Question::new(
            tr("Change this PC"),
            text,
            tr("Apply"),
            RegFileAct::Apply(scope),
        ))
    }

    /// The question before the removal of the selected key or line, with the act of a yes.
    pub fn remove_question(&self) -> Option<Question> {
        let (lines, text) = self.removal()?;
        Some(Question::new(
            tr("Remove from the file"),
            text,
            tr("Remove"),
            RegFileAct::Remove(lines),
        ))
    }

    /// The entry of the outline the selection is in; `None` for the whole file.
    pub fn item(&self, items: &[Item]) -> Option<usize> {
        let at = self.at?;
        items
            .iter()
            .position(|it| it.lines().end > at)
            .or(items.len().checked_sub(1))
    }

    /// The key of the selected entry: its line and its path.
    pub fn selected_key(&self) -> Option<(usize, String)> {
        let items = items(&self.file.lines);
        let Item::Key { line, .. } = &items[self.item(&items)?] else {
            return None;
        };
        match &self.file.lines[*line] {
            Line::Key { path, .. } => Some((*line, path.clone())),
            _ => None,
        }
    }

    /// The name of the selected value.
    pub fn selected_value(&self) -> Option<String> {
        match self.file.lines.get(self.line?)? {
            Line::Value { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// The line Edit changes: the selected line, else the key or the first comment of the entry.
    pub fn target(&self) -> Option<usize> {
        if let Some(l) = self.line
            && l < self.file.lines.len()
        {
            return Some(l);
        }
        let items = items(&self.file.lines);
        self.item(&items).map(|i| items[i].anchor())
    }

    /// The editor of a line of the file.
    pub fn start_edit(&mut self, line: usize) {
        let mut edit = Edit {
            kind: EditKind::Value,
            line: Some(line),
            at: line,
            name: String::new(),
            type_name: "REG_SZ".to_owned(),
            text: String::new(),
            delete: false,
            error: None,
        };
        match self.file.lines.get(line) {
            Some(Line::Key { path, delete }) => {
                edit.kind = EditKind::Key;
                edit.name = path.clone();
                edit.delete = *delete;
            }
            Some(Line::Value { name, data }) => {
                edit.name = name.clone();
                match data {
                    Some(d) if !d.editable() => {
                        let text = tr("This type of value is shown, not edited.");
                        self.error = Some(text.to_owned());
                        return;
                    }
                    Some(d) => {
                        edit.type_name = d.type_name();
                        edit.text = d.edit_text();
                    }
                    None => edit.delete = true,
                }
            }
            Some(Line::Comment(text)) => {
                edit.kind = EditKind::Comment;
                edit.name = text.strip_prefix(';').unwrap_or(text).to_owned();
            }
            Some(Line::Other(text)) => {
                edit.kind = EditKind::Raw;
                edit.name = text.clone();
            }
            Some(Line::Blank) | None => return,
        }
        self.error = None;
        self.edit = Some(edit);
    }

    /// The editor of a new key at the end of the file, or of a new value of the selected key after
    /// its last value.
    pub fn start_new(&mut self, kind: EditKind) {
        let at = if kind == EditKind::Value {
            let items = items(&self.file.lines);
            let item = self.item(&items).map(|i| items[i].clone());
            let Some(Item::Key { line, lines }) = item else {
                let text = tr("Select a key first: a new value goes under it.");
                self.error = Some(text.to_owned());
                return;
            };
            let last = lines
                .rev()
                .find(|&i| matches!(self.file.lines[i], Line::Value { .. }));
            last.unwrap_or(line) + 1
        } else {
            // Before the blank lines that end the file.
            let mut at = self.file.lines.len();
            while at > 0 && self.file.lines[at - 1] == Line::Blank {
                at -= 1;
            }
            at
        };
        let kind = if kind == EditKind::Value {
            kind
        } else {
            EditKind::Key
        };
        self.error = None;
        self.edit = Some(Edit {
            kind,
            line: None,
            at,
            name: String::new(),
            type_name: "REG_SZ".to_owned(),
            text: String::new(),
            delete: false,
            error: None,
        });
    }

    /// Puts the line of the editor into the file; false, with the error in the editor, for a path,
    /// a name or data the registry does not take.
    pub fn apply_edit(&mut self) -> bool {
        let Some(edit) = &mut self.edit else {
            return false;
        };
        let line = match edit.kind {
            EditKind::Key => match registry::normalize(&edit.name) {
                Some(path) => Line::Key {
                    path,
                    delete: edit.delete,
                },
                None => {
                    let text = trf(
                        "Not a path of the registry: {path}",
                        &[("path", &edit.name)],
                    );
                    edit.error = Some(text);
                    return false;
                }
            },
            EditKind::Value => {
                let long = edit.name.chars().count() > registry::NAME_MAX;
                if long || edit.name.chars().any(char::is_control) {
                    let text = tr("The name is too long or has a control character.");
                    edit.error = Some(text.to_owned());
                    return false;
                }
                let data = if edit.delete {
                    None
                } else {
                    match Data::parse(&edit.type_name, &edit.text) {
                        Ok(d) => Some(d),
                        Err(e) => {
                            edit.error = Some(e);
                            return false;
                        }
                    }
                };
                Line::Value {
                    name: edit.name.clone(),
                    data,
                }
            }
            EditKind::Comment => Line::Comment(format!(";{}", one_line(&edit.name))),
            EditKind::Raw => Line::parse(&one_line(&edit.name)),
        };
        let lines = &mut self.file.lines;
        let key = edit.kind == EditKind::Key;
        let at = match edit.line {
            Some(i) if i < lines.len() => {
                lines[i] = line;
                i
            }
            _ => {
                let mut at = edit.at.min(lines.len());
                if key && at > 0 && lines[at - 1] != Line::Blank {
                    lines.insert(at, Line::Blank);
                    at += 1;
                }
                lines.insert(at, line);
                at
            }
        };
        self.edit = None;
        self.at = Some(at);
        self.line = (!key).then_some(at);
        self.dirty = true;
        self.follow = true;
        self.follow_line = true;
        self.error = None;
        true
    }

    /// What Remove takes away and the question that asks first: the selected line, else the key
    /// of the entry with its values (its comments stay), or every line of an entry of comments.
    pub fn removal(&self) -> Option<(Vec<usize>, String)> {
        let lines = &self.file.lines;
        if let Some(l) = self.line
            && let Some(line) = lines.get(l)
        {
            let text = match line {
                Line::Value { name, .. } => trf(
                    "Remove the value {name} from the file?",
                    &[("name", &value_title(name))],
                ),
                Line::Key { .. } => return self.key_removal(l),
                Line::Blank => return None,
                Line::Comment(_) | Line::Other(_) => {
                    tr("Remove this line from the file?").to_owned()
                }
            };
            return Some((vec![l], text));
        }
        let items = items(lines);
        match &items[self.item(&items)?] {
            Item::Key { line, .. } => self.key_removal(*line),
            Item::Notes { lines: range } => Some((
                range.clone().collect(),
                trf(
                    "Remove these {n} lines from the file?",
                    &[("n", &range.len())],
                ),
            )),
        }
    }

    fn key_removal(&self, key: usize) -> Option<(Vec<usize>, String)> {
        let lines = &self.file.lines;
        let Line::Key { path, .. } = lines.get(key)? else {
            return None;
        };
        let items = items(lines);
        let range = items.iter().find(|it| it.anchor() == key)?.lines();
        let mut out = vec![key];
        out.extend(range.filter(|&i| matches!(lines[i], Line::Value { .. })));
        let text = trf(
            "Remove the key {path} and its {n} values from the file? Its comments stay.",
            &[("path", path), ("n", &(out.len() - 1))],
        );
        Some((out, text))
    }

    /// Removes lines of the file.
    pub fn remove_lines(&mut self, lines: &[usize]) {
        let mut sorted = lines.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for &i in sorted.iter().rev() {
            if i < self.file.lines.len() {
                self.file.lines.remove(i);
            }
        }
        let first = sorted.first().copied().unwrap_or(0);
        let last = self.file.lines.len().checked_sub(1);
        self.at = last.map(|last| first.min(last));
        self.line = None;
        self.dirty = true;
        self.follow = true;
    }

    /// The rules of the loaded catalogs that write a key, or a value of it.
    fn rules_of(&self, path: &str, value: Option<&str>) -> Vec<String> {
        regview::Writers::ids_for(&self.writers, &catalog_key(path), value).to_vec()
    }
}
