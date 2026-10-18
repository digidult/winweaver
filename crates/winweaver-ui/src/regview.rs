//! The registry tab (customer list of 18.10.2026, item 9): the keys of this PC as a tree on the left,
//! the values of the selected key on the right, an address row on top, an editor of a value in the
//! types of regedit and a search under the selected key at the bottom, which runs in a thread of
//! its own. The rights of keys are not there yet; their button says so. Reading and writing go
//! through acts applied after the frame, with the registry of the window (`App::registry`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;

use eframe::egui;
use winweaver_core::registry::{self, Found, HIVES, Key, NEW_TYPES, Registry};

/// The most places a search of the tab lists.
pub const SEARCH_LIMIT: usize = 5000;

/// A search that runs: its stop, the keys read so far and its result when it ends.
pub struct Running {
    pub stop: Arc<AtomicBool>,
    pub read: Arc<AtomicUsize>,
    pub result: Receiver<Found>,
}

/// A search nobody waits for any more (its tab closed) stops at the next key.
impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The search of the tab, as Find of regedit: what it looks for and at, and the places found.
pub struct Search {
    pub open: bool,
    pub text: String,
    pub keys: bool,
    pub values: bool,
    pub data: bool,
    pub whole: bool,
    /// The key the last search went under.
    pub root: String,
    pub running: Option<Running>,
    pub found: Option<Found>,
    /// The text field takes the focus at the next frame.
    pub focus: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            open: false,
            text: String::new(),
            keys: true,
            values: true,
            data: true,
            whole: false,
            root: String::new(),
            running: None,
            found: None,
            focus: false,
        }
    }
}

impl Search {
    /// What the search looks for, from the fields.
    pub fn query(&self) -> registry::Query {
        registry::Query {
            text: self.text.trim().to_owned(),
            keys: self.keys,
            values: self.values,
            data: self.data,
            whole: self.whole,
        }
    }
}

use crate::i18n::{tr, trf};
use crate::{Act, style, tree};

/// What the editor of the tab makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    /// A value that is there.
    Value,
    NewValue,
    NewKey,
}

/// The editor over the tab.
pub struct Edit {
    pub kind: EditKind,
    pub name: String,
    pub type_name: String,
    pub text: String,
    pub error: Option<String>,
}

pub struct RegistryDoc {
    /// The selected key, in the form of regedit.
    pub path: String,
    /// The text of the address row.
    pub address: String,
    /// The open keys of the tree, in lower case, and the subkeys read of each.
    pub expanded: HashSet<String>,
    children: HashMap<String, Vec<String>>,
    /// The key shown on the right, read when the selection changes and after a change.
    pub key: Option<Key>,
    pub error: Option<String>,
    /// The selected value by name.
    pub value: Option<String>,
    pub edit: Option<Edit>,
    /// The selected key comes into view at the next frame.
    pub reveal: bool,
    pub search: Search,
}

/// The name of the default value as regedit shows it.
pub(crate) fn value_title(name: &str) -> &str {
    if name.is_empty() {
        tr("(Default)")
    } else {
        name
    }
}

impl RegistryDoc {
    pub fn new(reg: &dyn Registry, path: &str, value: Option<&str>) -> Self {
        let mut doc = Self {
            path: HIVES[2].0.to_owned(),
            address: HIVES[2].0.to_owned(),
            expanded: HashSet::new(),
            children: HashMap::new(),
            key: None,
            error: None,
            value: None,
            edit: None,
            reveal: false,
            search: Search::default(),
        };
        doc.go(reg, path, value);
        doc
    }

    pub fn title(&self) -> String {
        tr("Registry").to_owned()
    }

    /// Shows a key: its parents open in the tree, its values read, a value selected.
    pub fn go(&mut self, reg: &dyn Registry, path: &str, value: Option<&str>) {
        let Some(path) = registry::normalize(path) else {
            self.error = Some(trf(
                "Not a path of the registry: {path}",
                &[("path", &path)],
            ));
            return;
        };
        let mut up = registry::parent(&path);
        while let Some(p) = up {
            self.expanded.insert(p.to_lowercase());
            self.load(reg, p);
            up = registry::parent(p);
        }
        self.address = path.clone();
        self.path = path;
        self.value = value.map(str::to_owned);
        self.reveal = true;
        self.refresh(reg);
    }

    fn load(&mut self, reg: &dyn Registry, path: &str) {
        if let Ok(k) = reg.read(path) {
            self.children.insert(path.to_lowercase(), k.subkeys);
        }
    }

    /// Reads the selected key again and the subkeys of the open keys.
    pub fn refresh(&mut self, reg: &dyn Registry) {
        match reg.read(&self.path) {
            Ok(k) => {
                self.children
                    .insert(self.path.to_lowercase(), k.subkeys.clone());
                if let Some(v) = &self.value
                    && !k.values.iter().any(|x| x.name.eq_ignore_ascii_case(v))
                {
                    self.value = None;
                }
                self.error = None;
                self.key = Some(k);
            }
            Err(e) => {
                self.key = None;
                self.error = Some(e);
            }
        }
        let open: Vec<String> = self.expanded.iter().cloned().collect();
        for p in open {
            self.load(reg, &p);
        }
    }

    /// Opens or closes a key of the tree, reading its subkeys when it opens.
    pub fn expand(&mut self, reg: &dyn Registry, path: &str, open: bool) {
        if open {
            self.expanded.insert(path.to_lowercase());
            self.load(reg, path);
        } else {
            self.expanded.remove(&path.to_lowercase());
        }
    }

    /// The editor of the selected value, or of a new value or key.
    pub fn start_edit(&mut self, kind: EditKind, type_name: &str) {
        let edit = match kind {
            EditKind::Value => {
                let Some(v) = self.selected() else {
                    return;
                };
                if !v.data.editable() {
                    self.error = Some(tr("This type of value is shown, not edited.").to_owned());
                    return;
                }
                Edit {
                    kind,
                    name: v.name.clone(),
                    type_name: v.data.type_name(),
                    text: v.data.edit_text(),
                    error: None,
                }
            }
            EditKind::NewValue | EditKind::NewKey => Edit {
                kind,
                name: String::new(),
                type_name: type_name.to_owned(),
                text: String::new(),
                error: None,
            },
        };
        self.edit = Some(edit);
    }

    pub fn selected(&self) -> Option<&registry::Value> {
        let name = self.value.as_ref()?;
        self.key
            .as_ref()?
            .values
            .iter()
            .find(|v| v.name.eq_ignore_ascii_case(name))
    }
}

/// The rows of the tree under a key.
fn rows(
    doc: &RegistryDoc,
    out: &mut Vec<tree::Row<String>>,
    path: &str,
    title: &str,
    depth: usize,
) {
    let open = doc.expanded.contains(&path.to_lowercase());
    out.push(tree::Row {
        key: path.to_owned(),
        depth,
        title: title.to_owned(),
        right: String::new(),
        check: tree::Check::None,
        open: Some(open),
        muted: false,
        icon: Some("\u{1F4C1}"),
        switch: None,
    });
    if !open {
        return;
    }
    if let Some(children) = doc.children.get(&path.to_lowercase()) {
        for child in children {
            rows(doc, out, &format!("{path}\\{child}"), child, depth + 1);
        }
    }
}

/// The tab: the address row, the tree of keys, the values, the editor.
pub fn registry_view(ui: &mut egui::Ui, doc: &mut RegistryDoc, acts: &mut Vec<Act>) {
    let p = style::palette(ui);
    egui::Panel::top("registry-address")
        .frame(
            egui::Frame::new()
                .fill(p.card)
                .inner_margin(egui::Margin::symmetric(10, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Key"));
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut doc.address)
                        .desired_width(ui.available_width() - 70.0)
                        .font(egui::TextStyle::Monospace),
                );
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(tr("Go")).clicked() || enter {
                    acts.push(Act::RegistryGo(doc.address.clone(), None));
                }
            });
        });
    if doc.search.open {
        egui::Panel::bottom("registry-search")
            .resizable(true)
            .default_size(220.0)
            .frame(
                egui::Frame::new()
                    .fill(p.card)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ui, |ui| search_panel(ui, doc, acts));
    }
    egui::Panel::left("registry-tree")
        .resizable(true)
        .default_size(360.0)
        .frame(
            egui::Frame::new()
                .fill(p.card)
                .inner_margin(egui::Margin::symmetric(0, 4)),
        )
        .show(ui, |ui| {
            let mut out = Vec::new();
            for (hive, _) in HIVES {
                rows(doc, &mut out, hive, hive, 0);
            }
            let selected = out
                .iter()
                .position(|r| r.key.eq_ignore_ascii_case(&doc.path));
            let scroll = if doc.reveal {
                tree::Scroll::Center
            } else {
                tree::Scroll::None
            };
            let shown = tree::show(ui, egui::Id::new("registry-tree"), &out, selected, scroll);
            if shown.scrolled || selected.is_none() {
                doc.reveal = false;
            }
            for event in shown.events {
                match event {
                    tree::Event::Select(path) => acts.push(Act::RegistryGo(path, None)),
                    tree::Event::Expand(path, open) => acts.push(Act::RegistryExpand(path, open)),
                    _ => {}
                }
            }
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(p.bg)
                .inner_margin(egui::Margin::same(12)),
        )
        .show(ui, |ui| values(ui, doc, acts));
    editor(ui.ctx(), doc, acts);
}

/// What a search found, in a line.
fn found_text(found: &Found, root: &str) -> String {
    let mut text = trf(
        "{hits} found under {root} in {keys} keys",
        &[
            ("hits", &found.hits.len()),
            ("root", &root),
            ("keys", &found.keys),
        ],
    );
    if !found.complete {
        text.push_str("; ");
        text.push_str(tr("stopped or at the limit, so there may be more"));
    }
    if found.unreadable > 0 {
        let unread = trf("{n} keys could not be read", &[("n", &found.unreadable)]);
        text.push_str("; ");
        text.push_str(&unread);
    }
    text
}

/// The search at the bottom of the tab: the text and what it looks at, then the places found; a
/// click on one shows it.
fn search_panel(ui: &mut egui::Ui, doc: &mut RegistryDoc, acts: &mut Vec<Act>) {
    let path = doc.path.clone();
    let s = &mut doc.search;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Find"));
        let edit = ui.add(
            egui::TextEdit::singleline(&mut s.text)
                .desired_width(260.0)
                .hint_text(tr("Text in the names or the data")),
        );
        if s.focus {
            edit.request_focus();
            s.focus = false;
        }
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.checkbox(&mut s.keys, tr("Keys"));
        ui.checkbox(&mut s.values, tr("Values"));
        ui.checkbox(&mut s.data, tr("Data"));
        ui.checkbox(&mut s.whole, tr("Match whole string only"));
        if s.running.is_some() {
            if ui.button(tr("Stop")).clicked() {
                acts.push(Act::RegistryFindStop);
            }
        } else if ui.button(tr("Find")).clicked() || enter {
            acts.push(Act::RegistryFind);
        }
    });
    let status = match (&s.running, &s.found) {
        (Some(r), _) => trf(
            "Searching under {root}: {n} keys read",
            &[("root", &s.root), ("n", &r.read.load(Ordering::Relaxed))],
        ),
        (None, Some(found)) => found_text(found, &s.root),
        (None, None) => trf(
            "Searches under the selected key: {root}",
            &[("root", &path)],
        ),
    };
    ui.weak(status);
    let Some(found) = &s.found else {
        return;
    };
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for hit in &found.hits {
                let text = match &hit.value {
                    Some(v) => format!("{}    {}", hit.path, value_title(v)),
                    None => hit.path.clone(),
                };
                let label = egui::RichText::new(text).monospace();
                if ui.selectable_label(false, label).clicked() {
                    acts.push(Act::RegistryGo(hit.path.clone(), hit.value.clone()));
                }
            }
        });
}

/// The values of the selected key: a click selects, a double click edits.
fn values(ui: &mut egui::Ui, doc: &mut RegistryDoc, acts: &mut Vec<Act>) {
    if let Some(e) = &doc.error {
        ui.colored_label(style::palette(ui).warn, e);
        ui.add_space(6.0);
    }
    let Some(key) = &doc.key else {
        return;
    };
    let mut select = None;
    let mut edit = false;
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        egui::Grid::new("registry-values")
            .num_columns(3)
            .striped(true)
            .spacing([18.0, 4.0])
            .show(ui, |ui| {
                ui.strong(tr("Name"));
                ui.strong(tr("Type"));
                ui.strong(tr("Data"));
                ui.end_row();
                for v in &key.values {
                    let on = doc
                        .value
                        .as_ref()
                        .is_some_and(|s| s.eq_ignore_ascii_case(&v.name));
                    let r = ui.selectable_label(on, value_title(&v.name));
                    if r.clicked() {
                        select = Some(v.name.clone());
                    }
                    if r.double_clicked() {
                        select = Some(v.name.clone());
                        edit = true;
                    }
                    ui.monospace(v.data.type_name());
                    let mut shown = v.data.display();
                    if shown.chars().count() > 200 {
                        shown = shown.chars().take(200).collect::<String>() + "...";
                    }
                    ui.label(egui::RichText::new(shown).monospace());
                    ui.end_row();
                }
            });
        if key.values.is_empty() {
            ui.weak(tr("This key has no values."));
        }
    });
    if let Some(name) = select {
        doc.value = Some(name);
    }
    if edit {
        acts.push(Act::RegistryEdit);
    }
}

/// The editor of a value or of a new key, over the window.
fn editor(ctx: &egui::Context, doc: &mut RegistryDoc, acts: &mut Vec<Act>) {
    let Some(edit) = &mut doc.edit else {
        return;
    };
    let title = match edit.kind {
        EditKind::Value => tr("Edit the value"),
        EditKind::NewValue => tr("New value"),
        EditKind::NewKey => tr("New key"),
    };
    let mut close = false;
    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.weak(&doc.path);
            ui.add_space(6.0);
            egui::Grid::new("registry-edit")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(tr("Name"));
                    ui.add_enabled(
                        edit.kind != EditKind::Value,
                        egui::TextEdit::singleline(&mut edit.name).desired_width(380.0),
                    );
                    ui.end_row();
                    if edit.kind != EditKind::NewKey {
                        ui.label(tr("Type"));
                        if edit.kind == EditKind::NewValue {
                            egui::ComboBox::from_id_salt("registry-type")
                                .selected_text(edit.type_name.as_str())
                                .show_ui(ui, |ui| {
                                    for t in NEW_TYPES {
                                        ui.selectable_value(&mut edit.type_name, t.to_owned(), t);
                                    }
                                });
                        } else {
                            ui.monospace(&edit.type_name);
                        }
                        ui.end_row();
                        ui.label(tr("Data"));
                        let multi = matches!(
                            edit.type_name.as_str(),
                            "REG_MULTI_SZ" | "REG_BINARY" | "REG_NONE"
                        );
                        let hint = match edit.type_name.as_str() {
                            "REG_DWORD" | "REG_QWORD" => {
                                tr("A number: decimal, or hexadecimal after 0x")
                            }
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
                                .desired_width(380.0)
                                .hint_text(hint)
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
                    acts.push(Act::RegistrySave);
                }
                if ui.button(tr("Cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            });
        });
    if close {
        doc.edit = None;
    }
}
