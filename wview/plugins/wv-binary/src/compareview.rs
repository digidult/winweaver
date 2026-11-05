//! The views of saved structures: the comparison of a file with its `.wwbin` (the tree of the lines
//! on the left, with Changed only; the line selected on the right, both values, its comment and its
//! bookmark), and a `.wwbin` opened alone (its tree, or its page).

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use wwrt::egui::{self, RichText};
use wwrt::plugin::{Host, Tool, View};
use wwrt::ww::Search;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::mdview::{self, MdDoc};
use wwrt::ww::tree::{self, Check, Event, Row, Scroll};
use wwrt::ww::widgets;

use crate::diff::{self, Line, Mark, Note};
use crate::engine::Tree;
use crate::view::{markdown, tree_rows};
use crate::wwbin::Saved;

/// A file compared with a structure saved of it.
pub struct Comparison {
    pub structure: PathBuf,
    pub saved: Saved,
    pub lines: Vec<Line>,
    pub notes: BTreeMap<String, Note>,
    /// The `.wwbindiff` the comparison was read from or saved to.
    pub file: Option<PathBuf>,
    /// The notes changed since the last save.
    pub dirty: bool,
    open: HashSet<usize>,
    selected: usize,
    follow: bool,
    changed_only: bool,
}

fn icon(mark: Mark) -> Option<&'static str> {
    match mark {
        Mark::Same => None,
        Mark::Changed => Some("\u{270F}"),
        Mark::Added => Some("\u{2795}"),
        Mark::Removed => Some("\u{2796}"),
    }
}

fn mark_text(mark: Mark) -> &'static str {
    match mark {
        Mark::Same => tr("The same"),
        Mark::Changed => tr("Changed"),
        Mark::Added => tr("Added in the file"),
        Mark::Removed => tr("Removed from the file"),
    }
}

impl Comparison {
    /// A comparison open at its changes: every structure with one open, the first one selected.
    pub fn new(structure: PathBuf, saved: Saved, lines: Vec<Line>) -> Self {
        let mut open: HashSet<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.mark == Mark::Changed && !l.children.is_empty())
            .map(|(i, _)| i)
            .collect();
        open.insert(0);
        let selected = lines
            .iter()
            .position(|l| l.mark != Mark::Same && l.children.is_empty())
            .unwrap_or(0);
        Comparison {
            structure,
            saved,
            lines,
            notes: BTreeMap::new(),
            file: None,
            dirty: false,
            open,
            selected,
            follow: false,
            changed_only: true,
        }
    }

    pub fn changed_only(&self) -> bool {
        self.changed_only
    }

    pub fn toggle_changed_only(&mut self) {
        self.changed_only = !self.changed_only;
    }

    /// The value of a line on one side, `-` where the side has no node.
    fn values(&self, tree: &Tree, line: &Line) -> (String, String) {
        let old = line.old.map(|o| {
            let mut v = self.saved.tree.value(o);
            // The structure keeps no way of showing a number: the one of the file is taken.
            if let (Some(n), crate::engine::Data::Int(x)) =
                (line.new, &self.saved.tree.nodes[o].data)
            {
                let names = &self.saved.tree.nodes[o].note;
                v = crate::engine::number(*x, tree.nodes[n].show);
                if !names.is_empty() {
                    v = format!("{v} {names}");
                }
            }
            v
        });
        let new = line.new.map(|n| tree.value(n));
        let none = || "-".to_owned();
        (old.unwrap_or_else(none), new.unwrap_or_else(none))
    }

    fn title(&self, tree: &Tree, line: &Line) -> String {
        let (old, new) = self.values(tree, line);
        let value = match line.mark {
            Mark::Changed if line.children.is_empty() => format!("{old} \u{2192} {new}"),
            Mark::Removed => old,
            _ => new,
        };
        if value.is_empty() || value == "-" {
            line.label.clone()
        } else {
            format!("{}: {value}", line.label)
        }
    }

    /// The rows of the open lines, the ones of no change left out with Changed only.
    pub fn rows(&self, tree: &Tree) -> Vec<Row<usize>> {
        let mut out = Vec::new();
        let mut stack = vec![(0usize, 0usize)];
        while let Some((i, depth)) = stack.pop() {
            let line = &self.lines[i];
            if self.changed_only && line.mark == Mark::Same && i != 0 {
                continue;
            }
            let shown: Vec<usize> = line
                .children
                .iter()
                .copied()
                .filter(|c| !self.changed_only || self.lines[*c].mark != Mark::Same)
                .collect();
            let open = (!shown.is_empty()).then(|| self.open.contains(&i));
            let note = self.notes.get(&line.path);
            let mut right = String::new();
            if note.is_some_and(|n| n.bookmark) {
                right.push('\u{2605}');
            }
            if note.is_some_and(|n| !n.comment.is_empty()) {
                right.push_str(" \u{1F4AC}");
            }
            out.push(Row {
                key: i,
                depth,
                title: self.title(tree, line),
                right: right.trim().to_owned(),
                check: Check::None,
                open,
                muted: line.mark == Mark::Same,
                icon: icon(line.mark),
                switch: None,
            });
            if open == Some(true) {
                for c in shown.iter().rev() {
                    stack.push((*c, depth + 1));
                }
            }
        }
        out
    }

    /// Draws the comparison; gives the node of the file to show in the tree and the bytes.
    pub fn ui(&mut self, ui: &mut egui::Ui, tree: &Tree) -> Option<usize> {
        let rows = self.rows(tree);
        let selected = rows.iter().position(|r| r.key == self.selected);
        let id = ui.make_persistent_id("wv-binary-compare");
        let output = widgets::side_panel(ui, id, 520.0, |ui| {
            tree::show(ui, id, &rows, selected, Scroll::wanted(false, self.follow))
        });
        self.follow = output.moved;
        for event in output.events {
            match event {
                Event::Select(i) => self.selected = i,
                Event::Expand(i, true) => {
                    self.open.insert(i);
                }
                Event::Expand(i, false) => {
                    self.open.remove(&i);
                }
                Event::Toggle(_) | Event::Switch(..) => {}
            }
        }
        egui::CentralPanel::default()
            .show(ui, |ui| self.detail(ui, tree))
            .inner
    }

    fn detail(&mut self, ui: &mut egui::Ui, tree: &Tree) -> Option<usize> {
        let line = self.lines[self.selected].clone();
        let path = if line.path.is_empty() {
            line.label.clone()
        } else {
            line.path.clone()
        };
        ui.label(RichText::new(&path).monospace().strong());
        ui.label(mark_text(line.mark));
        ui.add_space(6.0);
        let (old, new) = self.values(tree, &line);
        let place = |t: &Tree, n: Option<usize>| {
            n.map(|n| {
                (
                    format!("0x{:X}", t.nodes[n].offset),
                    t.nodes[n].size.to_string(),
                )
            })
            .unwrap_or(("-".to_owned(), "-".to_owned()))
        };
        let (old_at, old_size) = place(&self.saved.tree, line.old);
        let (new_at, new_size) = place(tree, line.new);
        egui::Grid::new("wv-binary-compare-detail")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.strong(tr("In the structure"));
                ui.strong(tr("In the file"));
                ui.end_row();
                for (name, a, b) in [
                    (tr("Value"), &old, &new),
                    (tr("Offset"), &old_at, &new_at),
                    (tr("Size"), &old_size, &new_size),
                ] {
                    ui.label(name);
                    ui.label(a);
                    ui.label(b);
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        let mut note = self.notes.get(&line.path).cloned().unwrap_or(Note {
            key: line.path.clone(),
            ..Note::default()
        });
        let mut changed = ui.checkbox(&mut note.bookmark, tr("Bookmark")).changed();
        ui.label(tr("Comment"));
        changed |= ui
            .add(
                egui::TextEdit::multiline(&mut note.comment)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            )
            .changed();
        if changed {
            self.notes.insert(line.path.clone(), note);
            self.dirty = true;
        }
        ui.add_space(8.0);
        let mut show = None;
        if let Some(n) = line.new
            && ui.button(tr("Show in the tree and bytes")).clicked()
        {
            show = Some(n);
        }
        show
    }

    pub fn status(&self) -> String {
        let (changed, added, removed) = diff::counts(&self.lines);
        let mut line = trf(
            "Compared with {structure}: {changed} changed, {added} added, {removed} removed",
            &[
                (
                    "structure",
                    &self
                        .structure
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                ),
                ("changed", &changed),
                ("added", &added),
                ("removed", &removed),
            ],
        );
        if self.dirty {
            line.push_str(", ");
            line.push_str(tr("the notes are not saved"));
        }
        line
    }
}

/// A `.wwbin` opened alone: its tree, or its page.
pub struct SavedView {
    title: String,
    saved: Saved,
    open: HashSet<usize>,
    selected: usize,
    follow: bool,
    page: Option<MdDoc>,
    text: bool,
}

impl SavedView {
    pub fn new(title: String, saved: Saved) -> Self {
        SavedView {
            title,
            saved,
            open: HashSet::from([0]),
            selected: 0,
            follow: false,
            page: None,
            text: false,
        }
    }
}

impl View for SavedView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        if self.text {
            let (wrap, width) = (host.wrap_lines, host.page_width);
            let (tree, title, format) = (&self.saved.tree, &self.title, &self.saved.format);
            let doc = self.page.get_or_insert_with(|| {
                MdDoc::new(title.clone(), None, markdown(tree, title, format))
            });
            egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc, wrap, width));
            return;
        }
        let rows = tree_rows(&self.saved.tree, &self.open);
        let selected = rows.iter().position(|r| r.key == self.selected && !r.muted);
        egui::CentralPanel::default().show(ui, |ui| {
            let id = ui.make_persistent_id("wv-binary-saved");
            let output = tree::show(ui, id, &rows, selected, Scroll::wanted(false, self.follow));
            self.follow = output.moved;
            for event in output.events {
                match event {
                    Event::Select(n) => self.selected = n,
                    Event::Expand(n, true) => {
                        self.open.insert(n);
                    }
                    Event::Expand(n, false) => {
                        self.open.remove(&n);
                    }
                    Event::Toggle(_) | Event::Switch(..) => {}
                }
            }
        });
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                id: "tree",
                icon: "\u{1F333}",
                label: "Tree",
            },
            Tool {
                id: "text",
                icon: "\u{1F4C4}",
                label: "Text",
            },
        ]
    }

    fn act(&mut self, id: &str, _host: &mut Host) {
        self.text = id == "text";
        if self.text && self.page.is_none() {
            let (tree, title) = (&self.saved.tree, &self.title);
            self.page = Some(MdDoc::new(
                title.clone(),
                None,
                markdown(tree, title, &self.saved.format),
            ));
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        if self.text {
            self.page.as_mut().map(|p| &mut p.search)
        } else {
            None
        }
    }

    fn status(&self) -> String {
        trf(
            "The structure of {name} ({size} bytes, SHA-256 {sha}), {format}, {nodes} nodes",
            &[
                ("name", &self.saved.name),
                ("size", &self.saved.size),
                ("sha", &self.saved.sha256),
                ("format", &self.saved.format),
                ("nodes", &self.saved.tree.nodes.len()),
            ],
        )
    }
}
