//! The tab of the comparison of two catalogs: the policies that differ in the tree of their
//! categories, the selected one side by side, and the report as a Markdown page or file.

use std::collections::HashSet;

use eframe::egui;
use winweaver_core::admx::{self, CatalogFile};
use winweaver_core::catalog::{ROOT_POLICIES, Rule};

use crate::ShellAct;
use crate::catalogview::{group_title, rule_texts};
use crate::i18n::{tr, trf};
use crate::{Act, CatalogAct, Search, matches_words, style, tree, views, widgets};

/// Where a policy of the comparison is: only in the first catalog, only in the second, or in
/// both with differences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    First,
    Second,
    Both,
}

impl Side {
    /// The number of the side among the switches of the tab.
    fn index(self) -> usize {
        match self {
            Side::First => 0,
            Side::Second => 1,
            Side::Both => 2,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Side::First => "\u{2796}",
            Side::Second => "\u{2795}",
            Side::Both => "\u{2260}",
        }
    }
}

/// A policy that differs: its id, its side, what differs and the titles of its categories.
pub struct Entry {
    pub id: String,
    pub side: Side,
    pub what: Vec<&'static str>,
    pub path: Vec<String>,
}

/// A row of the tree of a comparison: a category (the titles above it joined) or a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmpKey {
    Category(String),
    Policy(String),
}

/// An open comparison of two catalogs.
pub struct CompareDoc {
    pub title: String,
    pub first: Box<CatalogFile>,
    pub second: Box<CatalogFile>,
    /// The count of the policies that are the same.
    pub same: usize,
    pub entries: Vec<Entry>,
    pub sel: Option<CmpKey>,
    pub expanded: HashSet<String>,
    pub search: Search,
    /// Which policies the tree shows: only in the first, only in the second, different.
    pub shown: [bool; 3],
    /// The selected row is scrolled to the middle at the next frame.
    pub reveal: bool,
    /// The selected row follows into view at the next frame.
    pub follow: bool,
}

/// The joiner of the titles of a category in its key.
const JOIN: char = '\u{1F}';

impl CompareDoc {
    /// The comparison of two catalog files, with the categories of the first level open.
    pub fn new(first: &CatalogFile, second: &CatalogFile) -> Self {
        let lang = crate::i18n::current();
        let c = admx::compare(first, second);
        let mut entries = Vec::new();
        let path = |f: &CatalogFile, id: &str| {
            f.rules
                .iter()
                .find(|r| r.id == id)
                .map(|r| category_path(f, &lang, r))
                .unwrap_or_default()
        };
        for (id, _) in &c.only_first {
            entries.push(Entry {
                id: id.clone(),
                side: Side::First,
                what: Vec::new(),
                path: path(first, id),
            });
        }
        for (id, _) in &c.only_second {
            entries.push(Entry {
                id: id.clone(),
                side: Side::Second,
                what: Vec::new(),
                path: path(second, id),
            });
        }
        for d in &c.changed {
            entries.push(Entry {
                id: d.id.clone(),
                side: Side::Both,
                what: d.what.clone(),
                path: path(first, &d.id),
            });
        }
        let expanded = entries
            .iter()
            .filter_map(|e| e.path.first().cloned())
            .collect();
        let title = trf(
            "{first} and {second}",
            &[("first", &first.name), ("second", &second.name)],
        );
        Self {
            title,
            first: Box::new(first.clone()),
            second: Box::new(second.clone()),
            same: c.same,
            entries,
            sel: None,
            expanded,
            search: Search::default(),
            shown: [true; 3],
            reveal: false,
            follow: false,
        }
    }

    /// The policy of a side, if that catalog has it.
    fn rule<'a>(&'a self, side: Side, id: &str) -> Option<&'a Rule> {
        let file = if side == Side::Second {
            &self.second
        } else {
            &self.first
        };
        file.rules.iter().find(|r| r.id == id)
    }

    /// The title of a policy: of the first catalog, else of the second.
    fn policy_title(&self, lang: &str, id: &str) -> String {
        if let Some(r) = self.rule(Side::First, id) {
            return rule_texts(&self.first, lang, r).0.to_owned();
        }
        self.rule(Side::Second, id)
            .map(|r| rule_texts(&self.second, lang, r).0.to_owned())
            .unwrap_or_else(|| id.to_owned())
    }

    /// Whether the tree shows an entry: its side is switched on and it holds every word.
    fn shows(&self, lang: &str, entry: &Entry, words: &[String]) -> bool {
        if !self.shown[entry.side.index()] {
            return false;
        }
        let hay = format!(
            "{} {} {}",
            entry.id,
            self.policy_title(lang, &entry.id),
            entry.path.join(" ")
        );
        matches_words(&hay, words)
    }
}

/// The titles of the categories above a policy, from the top; the root group of an import is
/// its name, not a category.
fn category_path(file: &CatalogFile, lang: &str, rule: &Rule) -> Vec<String> {
    let mut path = Vec::new();
    let mut next = Some(rule.group.clone());
    let mut guard = 0;
    while let Some(id) = next {
        guard += 1;
        let Some(group) = file.groups.iter().find(|g| g.id == id) else {
            break;
        };
        let parent = group.parent.as_deref();
        if guard > file.groups.len() || parent == Some(ROOT_POLICIES) || parent.is_none() {
            break;
        }
        path.push(group_title(file, lang, group).to_owned());
        next = group.parent.clone();
    }
    path.reverse();
    path
}

/// The categories and policies of the tree, by the titles of the categories.
#[derive(Default)]
struct Node {
    children: Vec<(String, Node)>,
    entries: Vec<usize>,
}

impl Node {
    fn count(&self) -> usize {
        let below: usize = self.children.iter().map(|(_, c)| c.count()).sum();
        self.entries.len() + below
    }

    /// The category of a title under this one, made when missing.
    fn child(&mut self, title: &str) -> &mut Node {
        let i = match self.children.iter().position(|(t, _)| t == title) {
            Some(i) => i,
            None => {
                self.children.push((title.to_owned(), Node::default()));
                self.children.len() - 1
            }
        };
        &mut self.children[i].1
    }
}

/// The rows of a category of the tree and of everything in it.
fn rows_of(
    rows: &mut Vec<tree::Row<CmpKey>>,
    doc: &CompareDoc,
    lang: &str,
    node: &Node,
    key: &str,
    depth: usize,
    open_all: bool,
) {
    for (title, child) in &node.children {
        let child_key = if key.is_empty() {
            title.clone()
        } else {
            format!("{key}{JOIN}{title}")
        };
        let open = open_all || doc.expanded.contains(&child_key);
        rows.push(tree::Row {
            key: CmpKey::Category(child_key.clone()),
            depth,
            title: title.clone(),
            right: child.count().to_string(),
            check: tree::Check::None,
            open: Some(open),
            muted: false,
            icon: None,
            switch: None,
        });
        if open {
            rows_of(rows, doc, lang, child, &child_key, depth + 1, open_all);
        }
    }
    for &i in &node.entries {
        let entry = &doc.entries[i];
        rows.push(tree::Row {
            key: CmpKey::Policy(entry.id.clone()),
            depth,
            title: doc.policy_title(lang, &entry.id),
            right: String::new(),
            check: tree::Check::None,
            open: None,
            muted: false,
            icon: Some(entry.side.icon()),
            switch: None,
        });
    }
}

/// The tab: the switches and the tree on the left, the selected policy side by side.
pub fn compare_view(ui: &mut egui::Ui, doc: &CompareDoc, acts: &mut Vec<Act>) {
    let lang = crate::i18n::current();
    let words = doc.search.words();
    let mut root = Node::default();
    for (i, entry) in doc.entries.iter().enumerate() {
        if !doc.shows(&lang, entry, &words) {
            continue;
        }
        let mut node = &mut root;
        for title in &entry.path {
            node = node.child(title);
        }
        node.entries.push(i);
    }
    let mut rows = Vec::new();
    rows_of(&mut rows, doc, &lang, &root, "", 0, !words.is_empty());
    let selected = doc
        .sel
        .as_ref()
        .and_then(|k| rows.iter().position(|r| &r.key == k));
    let scroll = tree::Scroll::wanted(doc.reveal, doc.follow);
    if selected.is_none() && scroll != tree::Scroll::None {
        acts.push(Act::Shell(ShellAct::Revealed));
    }
    widgets::side_panel(ui, "compare-tree", 380.0, |ui| {
        let counts = [
            doc.entries.iter().filter(|e| e.side == Side::First).count(),
            doc.entries
                .iter()
                .filter(|e| e.side == Side::Second)
                .count(),
            doc.entries.iter().filter(|e| e.side == Side::Both).count(),
        ];
        let labels = [
            trf("Only in {name}", &[("name", &doc.first.name)]),
            trf("Only in {name}", &[("name", &doc.second.name)]),
            tr("Different").to_owned(),
        ];
        for (i, label) in labels.iter().enumerate() {
            let side = [Side::First, Side::Second, Side::Both][i];
            let mut on = doc.shown[i];
            let text = format!("{} {label} ({})", side.icon(), counts[i]);
            if ui.checkbox(&mut on, text).changed() {
                acts.push(Act::Catalog(CatalogAct::CompareShow(i)));
            }
        }
        ui.label(style::muted(
            ui,
            trf("{n} policies are the same", &[("n", &doc.same)]),
            11.0,
        ));
        ui.separator();
        if rows.is_empty() {
            ui.weak(tr("Nothing differs here."));
        }
        let id = egui::Id::new(("compare-tree", doc.title.as_str()));
        let out = tree::show(ui, id, &rows, selected, scroll);
        for event in out.events {
            match event {
                tree::Event::Select(k) => acts.push(Act::Catalog(CatalogAct::CompareSelect(k))),
                tree::Event::Expand(CmpKey::Category(k), open) => {
                    acts.push(Act::Catalog(CatalogAct::CompareExpand(k, open)));
                }
                _ => {}
            }
        }
        if out.moved {
            acts.push(Act::Shell(ShellAct::Follow));
        }
        if out.scrolled {
            acts.push(Act::Shell(ShellAct::Revealed));
        }
    });
    let p = style::palette(ui);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| detail(ui, doc, &lang, acts));
        });
}

/// The selected policy of both catalogs, or the counts of the comparison.
fn detail(ui: &mut egui::Ui, doc: &CompareDoc, lang: &str, acts: &mut Vec<Act>) {
    let Some(CmpKey::Policy(id)) = &doc.sel else {
        views::header(ui, |ui| {
            ui.label(style::strong(ui, &doc.title, 18.0));
        });
        views::block(ui, |ui| {
            ui.label(trf(
                "{same} policies are the same, {changed} differ, {first_only} are only in {first} and {second_only} only in {second}.",
                &[
                    ("same", &doc.same),
                    ("changed", &doc.entries.iter().filter(|e| e.side == Side::Both).count()),
                    ("first_only", &doc.entries.iter().filter(|e| e.side == Side::First).count()),
                    ("second_only", &doc.entries.iter().filter(|e| e.side == Side::Second).count()),
                    ("first", &doc.first.name),
                    ("second", &doc.second.name),
                ],
            ));
            ui.add_space(4.0);
            ui.weak(tr("Select a policy to see it in both catalogs."));
        });
        return;
    };
    let Some(entry) = doc.entries.iter().find(|e| &e.id == id) else {
        return;
    };
    views::header(ui, |ui| {
        ui.label(style::strong(ui, doc.policy_title(lang, id), 18.0));
        views::id_line(ui, id);
        let state = match entry.side {
            Side::First => trf("Only in {name}", &[("name", &doc.first.name)]),
            Side::Second => trf("Only in {name}", &[("name", &doc.second.name)]),
            Side::Both => {
                let what: Vec<&str> = entry
                    .what
                    .iter()
                    .map(|w| crate::difference_text(w))
                    .collect();
                trf("Different: {what}", &[("what", &what.join(", "))])
            }
        };
        style::chip(ui, &state, true);
        if !entry.path.is_empty() {
            ui.label(style::muted(ui, entry.path.join(" \u{203A} "), 12.0));
        }
    });
    legend(ui);
    ui.columns(2, |cols| {
        side(&mut cols[0], &doc.first, &doc.second, lang, id, acts);
        side(&mut cols[1], &doc.second, &doc.first, lang, id, acts);
    });
}

/// How a field of a policy compares with the same field of the other catalog.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Same,
    Differs,
    /// The field is not in this catalog, the other one has it.
    Absent,
    /// The field is only in this catalog.
    OnlyHere,
}

impl Mark {
    /// The mark of a field from its value here and in the other catalog, `None` when absent.
    fn of<T: PartialEq>(here: Option<T>, there: Option<T>) -> Self {
        match (here, there) {
            (Some(a), Some(b)) if a == b => Mark::Same,
            (Some(_), Some(_)) => Mark::Differs,
            (None, Some(_)) => Mark::Absent,
            (Some(_), None) => Mark::OnlyHere,
            (None, None) => Mark::Same,
        }
    }

    /// The colour and the word of a mark; none for a field that is the same.
    fn look(self, ui: &egui::Ui) -> Option<(egui::Color32, &'static str)> {
        let p = style::palette(ui);
        match self {
            Mark::Same => None,
            Mark::Differs => Some((p.warn, tr("differs"))),
            Mark::Absent => Some((ui.visuals().error_fg_color, tr("absent"))),
            Mark::OnlyHere => Some((p.ok, tr("only here"))),
        }
    }
}

/// The colours of the marks, above the two sides.
fn legend(ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for mark in [Mark::Differs, Mark::Absent, Mark::OnlyHere] {
            if let Some((color, word)) = mark.look(ui) {
                ui.label(egui::RichText::new("\u{25A0}").color(color));
                ui.label(style::muted(ui, word, 12.0));
            }
        }
    });
}

/// A field of a side in a frame tinted by its mark, with the word of the mark after its title.
fn field(ui: &mut egui::Ui, mark: Mark, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let look = mark.look(ui);
    let mut frame = egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 3));
    if let Some((color, _)) = look {
        frame = frame
            .fill(color.gamma_multiply(0.12))
            .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.6)))
            .corner_radius(egui::CornerRadius::same(4));
    }
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            if !title.is_empty() {
                ui.label(style::strong(ui, title, 12.0));
            }
            if let Some((color, word)) = look {
                let size = style::size(ui, 11.0);
                ui.label(egui::RichText::new(word).color(color).size(size));
            }
        });
        add(ui);
    });
    ui.add_space(4.0);
}

/// One catalog's side of a policy: its texts, its values, its values when off, its parameters,
/// each field marked as the same, different, absent or only here against the other catalog.
fn side(
    ui: &mut egui::Ui,
    file: &CatalogFile,
    other: &CatalogFile,
    lang: &str,
    id: &str,
    acts: &mut Vec<Act>,
) {
    views::block(ui, |ui| {
        style::section_title(ui, &file.name);
        let Some(rule) = file.rules.iter().find(|r| r.id == id) else {
            ui.weak(tr("Not in this catalog."));
            return;
        };
        let twin = other.rules.iter().find(|r| r.id == id);
        let off_id = format!("{id}.off");
        let off = file.rules.iter().find(|r| r.id == off_id);
        let twin_off = other.rules.iter().find(|r| r.id == off_id);
        let (title, summary) = rule_texts(file, lang, rule);
        let path = category_path(file, lang, rule);
        let texts = twin.map(|r| rule_texts(other, lang, r));
        let twin_path = twin.map(|r| category_path(other, lang, r));
        let text = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        let mark = Mark::of(Some(title.to_owned()), texts.map(|t| t.0.to_owned()));
        field(ui, mark, "", |ui| {
            ui.label(style::strong(ui, title, 13.0));
        });
        let there = twin_path.filter(|p| !p.is_empty());
        let mark = Mark::of((!path.is_empty()).then(|| path.clone()), there);
        if !path.is_empty() || mark != Mark::Same {
            field(ui, mark, tr("Category"), |ui| {
                ui.label(style::muted(ui, path.join(" \u{203A} "), 12.0));
            });
        }
        let mark = Mark::of(text(summary), texts.and_then(|t| text(t.1)));
        if !summary.is_empty() || mark != Mark::Same {
            field(ui, mark, tr("Description"), |ui| {
                style::selectable_text(ui, ("compare-summary", &file.id, id), summary);
            });
        }
        let mark = Mark::of(Some(&rule.actions), twin.map(|r| &r.actions));
        field(ui, mark, tr("Values"), |ui| views::writes(ui, rule, acts));
        let mark = Mark::of(off.map(|r| &r.actions), twin_off.map(|r| &r.actions));
        if off.is_some() || mark != Mark::Same {
            field(ui, mark, tr("Values when off"), |ui| {
                if let Some(off) = off {
                    views::writes(ui, off, acts);
                }
            });
        }
        let params = (!rule.params.is_empty()).then_some(&rule.params);
        let twin_params = twin.and_then(|r| (!r.params.is_empty()).then_some(&r.params));
        let mark = Mark::of(params, twin_params);
        if params.is_some() || mark != Mark::Same {
            field(ui, mark, tr("Parameters"), |ui| {
                for (name, param) in &rule.params {
                    let default = param.default_value();
                    ui.label(trf(
                        "{title} ({name}), default {value}",
                        &[
                            ("title", &param.title()),
                            ("name", name),
                            ("value", &default),
                        ],
                    ));
                }
            });
        }
    });
}
