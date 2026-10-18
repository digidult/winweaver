//! The tab of a registry file (`.reg`, list "Development further" of 18.10.2026, item 3), shown as
//! a structure rather than as text: on the left the outline of the file, its keys and its comments
//! in their order; on the right the selected key with its values in the types of regedit and the
//! comments between them, the way to the registry tab and to the rules of the loaded catalogs that
//! write it. A key, a value or a comment is edited in a window over the tab and lines are removed
//! after a question; Save writes every line back, the ones the program does not read too.

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use eframe::egui;
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{Action, Catalog};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::regfile::{Line, RegFile};
use winweaver_core::registry::{self, Data};

use crate::i18n::{tr, trf};
use crate::regview::value_title;
use crate::views::{self, rtitle};
use crate::{Act, Search, list, style, tree};

/// The types the editor of a value offers.
const TYPES: [&str; 7] = [
    "REG_SZ",
    "REG_EXPAND_SZ",
    "REG_MULTI_SZ",
    "REG_DWORD",
    "REG_QWORD",
    "REG_BINARY",
    "REG_NONE",
];
/// The most characters of data a row shows.
const SHOWN_MAX: usize = 200;
/// The most rules the head of a key lists.
const RULES_MAX: usize = 12;
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

/// The rules of a catalog by the keys they write (in lower case), each with the value it writes in
/// lower case, `None` for the values of a list.
type Writers = HashMap<String, Vec<(String, Option<String>)>>;

fn writers_of(catalog: &Catalog) -> Writers {
    let mut out = Writers::new();
    for rule in catalog.rules() {
        for action in &rule.actions {
            let (path, name) = match action {
                Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. } => {
                    (path, Some(name.to_lowercase()))
                }
                Action::RegList { path, .. } => (path, None),
                _ => continue,
            };
            if let Some(key) = registry::normalize(path) {
                let list = out.entry(key.to_lowercase()).or_default();
                let entry = (rule.id.clone(), name);
                if !list.contains(&entry) {
                    list.push(entry);
                }
            }
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
    writers: Option<(usize, Writers)>,
}

impl RegFileDoc {
    pub fn new(file: RegFile, path: PathBuf) -> Self {
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
        }
    }

    pub fn title(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
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
        let Some((_, writers)) = &self.writers else {
            return Vec::new();
        };
        let value = value.map(str::to_lowercase);
        let mut out: Vec<String> = Vec::new();
        for (id, name) in writers.get(&catalog_key(path)).into_iter().flatten() {
            let fits = match (&value, name) {
                (Some(v), Some(n)) => v == n,
                _ => true,
            };
            if fits && !out.contains(id) {
                out.push(id.clone());
            }
        }
        out
    }
}

/// The row of an entry in the outline.
fn outline_row(lines: &[Line], item: &Item, index: usize) -> tree::Row<Option<usize>> {
    let (title, right, icon, muted) = match item {
        Item::Key { line, lines: range } => {
            let (path, delete) = match &lines[*line] {
                Line::Key { path, delete } => (path.clone(), *delete),
                _ => (String::new(), false),
            };
            let values = range
                .clone()
                .filter(|&i| matches!(lines[i], Line::Value { .. }))
                .count();
            if delete {
                (path, String::new(), "\u{1F5D1}", true)
            } else {
                (path, values.to_string(), "\u{1F4C1}", false)
            }
        }
        Item::Notes { lines: range } => {
            let first = range.clone().find_map(|i| match &lines[i] {
                Line::Comment(t) => Some(t.trim_start_matches(';').trim().to_owned()),
                Line::Other(t) => Some(t.trim().to_owned()),
                Line::Value { name, .. } => Some(value_title(name).to_owned()),
                Line::Blank | Line::Key { .. } => None,
            });
            let right = if range.len() > 1 {
                range.len().to_string()
            } else {
                String::new()
            };
            (first.unwrap_or_default(), right, ";", true)
        }
    };
    tree::Row {
        key: Some(index),
        depth: 0,
        title,
        right,
        check: tree::Check::None,
        open: None,
        muted,
        icon: Some(icon),
        switch: None,
    }
}

/// The outline: the whole file first, then the entries the search finds.
fn outline(ui: &mut egui::Ui, doc: &mut RegFileDoc, items: &[Item], selected: Option<usize>) {
    let lines = &doc.file.lines;
    let words = doc.search.words();
    let keys = lines
        .iter()
        .filter(|l| matches!(l, Line::Key { .. }))
        .count();
    let mut rows = vec![tree::Row {
        key: None,
        depth: 0,
        title: tr("The whole file").to_owned(),
        right: keys.to_string(),
        check: tree::Check::None,
        open: None,
        muted: false,
        icon: Some("\u{1F4C4}"),
        switch: None,
    }];
    for (i, item) in items.iter().enumerate() {
        if !words.is_empty() {
            let text: Vec<String> = lines[item.lines()].iter().map(search_text).collect();
            let text = text.join("\n");
            if !words.iter().all(|w| text.contains(w.as_str())) {
                continue;
            }
        }
        rows.push(outline_row(lines, item, i));
    }
    let shown_at = rows.iter().position(|r| r.key == selected);
    let scroll = if doc.follow {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    let id = egui::Id::new(("regfile-outline", doc.path.display().to_string()));
    let shown = tree::show(ui, id, &rows, shown_at, scroll);
    if shown.scrolled || shown_at.is_none() {
        doc.follow = false;
    }
    if shown.moved {
        doc.follow = true;
    }
    for event in shown.events {
        if let tree::Event::Select(key) = event {
            doc.at = key.map(|i| items[i].anchor());
            doc.line = None;
            doc.error = None;
        }
    }
}

/// A rule as a link to the profile, with the links to the catalogs it is in.
fn rule_link(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = catalog.rule(id) else {
        return;
    };
    ui.horizontal_wrapped(|ui| {
        if ui.link(rtitle(texts, rule)).clicked() {
            acts.push(Act::ShowInProfile(String::new(), id.to_owned()));
        }
        views::catalog_links(ui, imports, id, acts);
    });
}

/// The head of a key: its path, what the file does with it, the registry tab and the rules that
/// write it or its selected value.
#[allow(clippy::too_many_arguments)]
fn key_head(
    ui: &mut egui::Ui,
    doc: &RegFileDoc,
    line: usize,
    values: usize,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    acts: &mut Vec<Act>,
) {
    let Some(Line::Key { path, delete }) = doc.file.lines.get(line) else {
        return;
    };
    let value = doc.selected_value();
    views::header(ui, |ui| {
        let title = egui::RichText::new(path.as_str())
            .font(style::mono(ui, 14.0))
            .strong();
        style::copy_label(ui, title, true);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if *delete {
                style::chip(ui, tr("Deleted with its subkeys"), true);
            } else {
                style::chip(ui, &trf("{n} values", &[("n", &values)]), false);
            }
            if in_default_user(path) {
                style::chip(ui, tr("Default user profile"), false);
            }
            if ui.link(tr("Open in the registry")).clicked() {
                acts.push(Act::OpenRegistry(path.clone(), value.clone()));
            }
        });
        if in_default_user(path) {
            let text = tr(
                "The default user profile: load its hive here before the import, as the comment at the top of the file says.",
            );
            ui.label(style::muted(ui, text, 12.0));
        }
    });
    let rules = doc.rules_of(path, value.as_deref());
    views::block(ui, |ui| {
        match &value {
            Some(v) => {
                let name = value_title(v);
                style::section_title(ui, &trf("Rules that write {name}", &[("name", &name)]));
            }
            None => style::section_title(ui, tr("Rules that write it")),
        }
        ui.add_space(4.0);
        if rules.is_empty() {
            ui.weak(tr("No rule of the loaded catalogs writes it."));
        }
        for id in rules.iter().take(RULES_MAX) {
            rule_link(ui, catalog, texts, imports, id, acts);
        }
        if rules.len() > RULES_MAX {
            let more = rules.len() - RULES_MAX;
            ui.weak(trf("and {n} more", &[("n", &more)]));
        }
    });
    ui.add_space(10.0);
}

/// The head of the whole file: its name, its header, what it holds and how it is imported.
fn file_head(ui: &mut egui::Ui, doc: &RegFileDoc) {
    let lines = &doc.file.lines;
    let count = |f: &dyn Fn(&Line) -> bool| lines.iter().filter(|l| f(l)).count();
    let keys = count(&|l| matches!(l, Line::Key { delete: false, .. }));
    let gone_keys = count(&|l| matches!(l, Line::Key { delete: true, .. }));
    let values = count(&|l| matches!(l, Line::Value { data: Some(_), .. }));
    let gone_values = count(&|l| matches!(l, Line::Value { data: None, .. }));
    let comments = count(&|l| matches!(l, Line::Comment(_)));
    let other = count(&|l| matches!(l, Line::Other(_)));
    let default_user = lines
        .iter()
        .any(|l| matches!(l, Line::Key { path, .. } if in_default_user(path)));
    let facts = [
        (keys, trf("{n} keys", &[("n", &keys)])),
        (gone_keys, trf("{n} keys deleted", &[("n", &gone_keys)])),
        (values, trf("{n} values", &[("n", &values)])),
        (
            gone_values,
            trf("{n} values deleted", &[("n", &gone_values)]),
        ),
        (comments, trf("{n} comments", &[("n", &comments)])),
        (other, trf("{n} lines not read", &[("n", &other)])),
    ];
    views::header(ui, |ui| {
        ui.label(style::strong(ui, doc.title(), 18.0));
        let path = style::muted(ui, doc.path.display().to_string(), 12.0);
        style::copy_label(ui, path, true);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, &doc.file.header, true);
            for (n, text) in &facts {
                if *n > 0 {
                    style::chip(ui, text, false);
                }
            }
        });
    });
    views::block(ui, |ui| {
        style::section_title(ui, tr("Import"));
        ui.add_space(4.0);
        ui.label(tr(
            "Windows imports a registry file with regedit or reg import, as administrator for the keys of the computer. Try it on a test PC first.",
        ));
        if default_user {
            ui.label(tr(
                "Its keys of the default user profile need its hive loaded first, as the comment at the top of the file says.",
            ));
        }
        if other > 0 {
            let p = style::palette(ui);
            ui.colored_label(
                p.warn,
                tr("The lines the program does not read are kept as they are and saved again."),
            );
        }
    });
}

/// The lines of an entry as a list of name, type and data; the key itself is in the head.
fn lines_list(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    range: Range<usize>,
    key: Option<usize>,
    catalog: &Catalog,
    acts: &mut Vec<Act>,
) {
    let lines = &doc.file.lines;
    let key_path = key.and_then(|k| match &lines[k] {
        Line::Key { path, .. } => Some(path.clone()),
        _ => None,
    });
    let columns = [
        list::Column::new(tr("Name"), 220.0),
        list::Column::new(tr("Type"), 150.0),
        list::Column::new(tr("Data"), 400.0),
    ];
    let mut rows = Vec::new();
    for i in range {
        let (cells, button) = match &lines[i] {
            Line::Blank | Line::Key { .. } => continue,
            Line::Value { name, data } => {
                let (kind, shown) = match data {
                    Some(d) => (
                        list::Cell::new(d.type_name()).muted(),
                        list::Cell::new(short(&d.display())),
                    ),
                    None => (list::Cell::new(tr("deleted")).warn(), list::Cell::new("")),
                };
                let button = key_path
                    .as_ref()
                    .map(|_| ("\u{1F5C2}", tr("Open in the registry").to_owned()));
                (
                    vec![list::Cell::new(value_title(name)), kind, shown],
                    button,
                )
            }
            Line::Comment(t) => {
                let button = rule_of_comment(t)
                    .filter(|id| catalog.rule(id).is_some())
                    .map(|_| ("\u{27A1}", tr("Show in the profile").to_owned()));
                let text = t.strip_prefix(';').unwrap_or(t).trim();
                let cells = vec![
                    list::Cell::new(";").muted(),
                    list::Cell::new(tr("comment")).muted(),
                    list::Cell::new(text).muted(),
                ];
                (cells, button)
            }
            Line::Other(t) => {
                let cells = vec![
                    list::Cell::new(""),
                    list::Cell::new(tr("not read")).warn(),
                    list::Cell::new(t.as_str()),
                ];
                (cells, None)
            }
        };
        rows.push(list::Row {
            key: i,
            cells,
            button,
        });
    }
    let selected = doc.line.and_then(|l| rows.iter().position(|r| r.key == l));
    let scroll = if doc.follow_line {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    let id = egui::Id::new(("regfile-lines", doc.path.display().to_string()));
    let shown = list::show(ui, id, tr("Lines"), &columns, &rows, selected, scroll);
    if shown.scrolled || selected.is_none() {
        doc.follow_line = false;
    }
    if shown.moved {
        doc.follow_line = true;
    }
    for event in shown.events {
        match event {
            list::Event::Select(i) => {
                doc.line = Some(i);
                doc.error = None;
            }
            list::Event::Open(i) => {
                doc.line = Some(i);
                doc.start_edit(i);
            }
            list::Event::Button(i) => match &doc.file.lines[i] {
                Line::Value { name, .. } => {
                    if let Some(k) = &key_path {
                        acts.push(Act::OpenRegistry(k.clone(), Some(name.clone())));
                    }
                }
                Line::Comment(t) => {
                    if let Some(id) = rule_of_comment(t) {
                        acts.push(Act::ShowInProfile(String::new(), id.to_owned()));
                    }
                }
                _ => {}
            },
            list::Event::Delete(i) => {
                doc.line = Some(i);
                acts.push(Act::AskRegFileRemove);
            }
        }
    }
}

/// The tab: the outline on the left, the selected entry on the right, the editor over them.
pub fn regfile_view(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    acts: &mut Vec<Act>,
) {
    let count = catalog.rules().len();
    if doc.writers.as_ref().is_none_or(|(n, _)| *n != count) {
        doc.writers = Some((count, writers_of(catalog)));
    }
    let items = items(&doc.file.lines);
    let selected = doc.item(&items);
    let p = style::palette(ui);
    egui::Panel::left("regfile-outline")
        .resizable(true)
        .default_size(380.0)
        .frame(
            egui::Frame::new()
                .fill(p.card)
                .inner_margin(egui::Margin::symmetric(0, 4)),
        )
        .show(ui, |ui| outline(ui, doc, &items, selected));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg))
        .show(ui, |ui| {
            if let Some(e) = &doc.error {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.colored_label(p.warn, e);
                });
            }
            match selected.map(|i| &items[i]) {
                None => {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| file_head(ui, doc));
                }
                Some(Item::Key { line, lines }) => {
                    let values = lines
                        .clone()
                        .filter(|&i| matches!(doc.file.lines[i], Line::Value { .. }))
                        .count();
                    key_head(ui, doc, *line, values, catalog, texts, imports, acts);
                    lines_list(ui, doc, lines.clone(), Some(*line), catalog, acts);
                }
                Some(Item::Notes { lines }) => {
                    views::header(ui, |ui| {
                        ui.label(style::strong(ui, tr("Comments"), 16.0));
                        let n = lines.len();
                        let text = trf("{n} lines outside a key, kept in place", &[("n", &n)]);
                        ui.label(style::muted(ui, text, 12.0));
                    });
                    ui.add_space(10.0);
                    lines_list(ui, doc, lines.clone(), None, catalog, acts);
                }
            }
        });
    editor(ui.ctx(), doc);
}

/// The editor of a key, a value or a comment, over the window.
fn editor(ctx: &egui::Context, doc: &mut RegFileDoc) {
    let Some(edit) = &mut doc.edit else {
        return;
    };
    let title = match (edit.kind, edit.line) {
        (EditKind::Key, None) => tr("New key"),
        (EditKind::Key, Some(_)) => tr("Edit the key"),
        (EditKind::Value, None) => tr("New value"),
        (EditKind::Value, Some(_)) => tr("Edit the value"),
        (EditKind::Comment, _) => tr("Edit the comment"),
        (EditKind::Raw, _) => tr("Edit the line"),
    };
    let mut close = false;
    let mut apply = false;
    egui::Window::new(title)
        .id(egui::Id::new("regfile-editor"))
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("regfile-edit")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| match edit.kind {
                    EditKind::Key => {
                        ui.label(tr("Key"));
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.name)
                                .desired_width(420.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut edit.delete, tr("Delete this key with its subkeys"));
                        ui.end_row();
                    }
                    EditKind::Value => value_fields(ui, edit),
                    EditKind::Comment | EditKind::Raw => {
                        ui.label(tr("Text"));
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.name)
                                .desired_width(420.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.end_row();
                    }
                });
            if let Some(e) = &edit.error {
                ui.colored_label(style::palette(ui).warn, e);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr("OK")).clicked() {
                    apply = true;
                }
                if ui.button(tr("Cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            });
        });
    if apply {
        doc.apply_edit();
    }
    if close {
        doc.edit = None;
    }
}

/// The fields of a value in the editor: the name, the deletion, the type and the data.
fn value_fields(ui: &mut egui::Ui, edit: &mut Edit) {
    ui.label(tr("Name"));
    ui.add(
        egui::TextEdit::singleline(&mut edit.name)
            .desired_width(420.0)
            .hint_text(tr("(Default)")),
    );
    ui.end_row();
    ui.label("");
    ui.checkbox(&mut edit.delete, tr("Delete this value"));
    ui.end_row();
    if edit.delete {
        return;
    }
    ui.label(tr("Type"));
    egui::ComboBox::from_id_salt("regfile-type")
        .selected_text(edit.type_name.as_str())
        .show_ui(ui, |ui| {
            for t in TYPES {
                ui.selectable_value(&mut edit.type_name, t.to_owned(), t);
            }
        });
    ui.end_row();
    ui.label(tr("Data"));
    let multi = matches!(
        edit.type_name.as_str(),
        "REG_MULTI_SZ" | "REG_BINARY" | "REG_NONE"
    );
    let hint = match edit.type_name.as_str() {
        "REG_DWORD" | "REG_QWORD" => tr("A number: decimal, or hexadecimal after 0x"),
        "REG_MULTI_SZ" => tr("One string per line"),
        "REG_BINARY" | "REG_NONE" => tr("Bytes as hexadecimal pairs"),
        _ => "",
    };
    let field = if multi {
        egui::TextEdit::multiline(&mut edit.text).desired_rows(8)
    } else {
        egui::TextEdit::singleline(&mut edit.text)
    };
    ui.add(
        field
            .desired_width(420.0)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace),
    );
    ui.end_row();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RegFile {
        let text = [
            "Windows Registry Editor Version 5.00",
            "",
            "; The head of the file",
            "",
            "; Rule a.b",
            "[HKEY_CURRENT_USER\\Software\\A]",
            "\"One\"=dword:00000001",
            "; between the values",
            "\"Two\"=-",
            "",
            "; alone",
            "",
            "[-HKEY_CURRENT_USER\\Software\\Old]",
            "",
            "",
        ]
        .join("\r\n");
        RegFile::parse(&text).unwrap()
    }

    #[test]
    fn the_outline_keeps_the_comments_in_their_places() {
        let file = sample();
        let found = items(&file.lines);
        // The head, the key with the comment above it and the one between its values, the
        // comment alone, the deleted key.
        assert_eq!(found.len(), 4, "{found:?}");
        assert_eq!(found[0], Item::Notes { lines: 1..2 });
        assert_eq!(
            found[1],
            Item::Key {
                line: 4,
                lines: 3..8,
            }
        );
        assert_eq!(found[2], Item::Notes { lines: 9..10 });
        assert!(matches!(found[3], Item::Key { line: 11, .. }));
        assert_eq!(rule_of_comment("; Rule a.b"), Some("a.b"));
        assert_eq!(rule_of_comment("; The head"), None);
        assert_eq!(
            catalog_key("HKEY_USERS\\UnattendDefault\\Software\\X"),
            "hkey_current_user\\software\\x"
        );
    }

    #[test]
    fn a_value_is_edited_added_and_removed() {
        let mut doc = RegFileDoc::new(sample(), PathBuf::from("x.reg"));
        // A new value of the key goes after its last value.
        doc.at = Some(4);
        doc.start_new(EditKind::Value);
        let edit = doc.edit.as_mut().unwrap();
        assert_eq!(edit.at, 8);
        edit.name = "Three".to_owned();
        edit.type_name = "REG_DWORD".to_owned();
        edit.text = "0x10".to_owned();
        assert!(doc.apply_edit());
        assert_eq!(
            doc.file.lines[8],
            Line::Value {
                name: "Three".to_owned(),
                data: Some(Data::Dword(16)),
            }
        );
        // Bad data stays in the editor with its error.
        doc.start_edit(8);
        doc.edit.as_mut().unwrap().text = "x".to_owned();
        assert!(!doc.apply_edit());
        assert!(doc.edit.as_ref().unwrap().error.is_some());
        doc.edit = None;
        // The key goes with its values; its comments stay.
        doc.line = None;
        let (lines, _) = doc.removal().unwrap();
        assert_eq!(lines, vec![4, 5, 7, 8]);
        doc.remove_lines(&lines);
        let text = doc.file.to_text();
        assert!(text.contains("; Rule a.b"), "{text}");
        assert!(text.contains("; between the values"), "{text}");
        assert!(!text.contains("\"One\""), "{text}");
        // A new key goes before the blank lines at the end, after a blank line.
        doc.start_new(EditKind::Key);
        doc.edit.as_mut().unwrap().name = "HKLM\\Software\\New".to_owned();
        assert!(doc.apply_edit());
        let text = doc.file.to_text();
        let key = "[HKEY_LOCAL_MACHINE\\Software\\New]";
        assert!(text.contains(key), "{text}");
        assert_eq!(doc.file.lines.last(), Some(&Line::Blank));
    }
}
