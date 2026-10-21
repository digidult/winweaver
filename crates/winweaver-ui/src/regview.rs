//! The registry tab (customer list of 18.10.2026, item 9): the keys of this PC as a tree on the left,
//! the values of the selected key on the right, an address row on top, an editor of a value in the
//! types of regedit and a search under the selected key at the bottom, which runs in a thread of
//! its own, the rules that write the selection above the values, and the permissions of the key in
//! a window. Reading and writing go through acts applied after the frame, with the registry of the
//! window (`App::registry`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;

use eframe::egui;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::registry::{self, Found, HIVES, Key, NEW_TYPES, Registry};
use winweaver_core::security::{Entry, Rights, Scope};

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
use crate::{Act, Question, RegistryAct, icons, regedit, style, tree, views, widgets};

/// The most rules the line above the values names.
const WRITERS_MAX: usize = 12;

/// The rules of the catalog that write the selected key or value, kept until the selection or the
/// catalog changes.
pub struct Writers {
    key: String,
    value: Option<String>,
    rules: usize,
    pub ids: Vec<String>,
}

impl Writers {
    /// Fills the cache for a key (or a value of it) unless it holds that already.
    pub fn refresh(
        cache: &mut Option<Writers>,
        catalog: &Catalog,
        path: &str,
        value: Option<&str>,
    ) {
        let key = path.to_lowercase();
        let rules = catalog.rules().len();
        let fresh = cache
            .as_ref()
            .is_some_and(|w| w.key == key && w.value.as_deref() == value && w.rules == rules);
        if !fresh {
            let found = registry::writers(catalog, path, value);
            *cache = Some(Writers {
                key,
                value: value.map(str::to_owned),
                rules,
                ids: found.iter().map(|r| r.id.clone()).collect(),
            });
        }
    }

    /// The ids of the cache, when it is for this key and value.
    pub fn ids_for<'a>(
        cache: &'a Option<Writers>,
        path: &str,
        value: Option<&str>,
    ) -> &'a [String] {
        match cache {
            Some(w) if w.key.eq_ignore_ascii_case(path) && w.value.as_deref() == value => &w.ids,
            _ => &[],
        }
    }
}

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
    writers: Option<Writers>,
    /// The window of the rights of the selected key.
    pub rights: Option<RightsEdit>,
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
            writers: None,
            rights: None,
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
        icon: Some(icons::FOLDER),
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

/// The rules that write the selected value, or the key when no value is selected, as links to
/// the profile.
fn writers_line(
    ui: &mut egui::Ui,
    doc: &mut RegistryDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    acts: &mut Vec<Act>,
) {
    Writers::refresh(&mut doc.writers, catalog, &doc.path, doc.value.as_deref());
    let Some(w) = &doc.writers else {
        return;
    };
    if w.ids.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        match &w.value {
            Some(v) => {
                let name = value_title(v);
                ui.weak(trf("Rules that write {name}", &[("name", &name)]));
            }
            None => {
                ui.weak(tr("Rules that write it"));
            }
        }
        for id in w.ids.iter().take(WRITERS_MAX) {
            let title = catalog
                .rule(id)
                .map_or(id.as_str(), |r| views::rtitle(texts, r));
            if ui.link(title).on_hover_text(id).clicked() {
                acts.push(views::RuleLink::Elsewhere.act(id));
            }
        }
        if w.ids.len() > WRITERS_MAX {
            let more = w.ids.len() - WRITERS_MAX;
            ui.weak(trf("and {n} more", &[("n", &more)]));
        }
    });
    ui.add_space(6.0);
}

/// The tab: the address row, the tree of keys, the rules that write the selection, the values,
/// the editor.
pub fn registry_view(
    ui: &mut egui::Ui,
    doc: &mut RegistryDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    acts: &mut Vec<Act>,
) {
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
                    acts.push(Act::Registry(RegistryAct::Go(doc.address.clone(), None)));
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
    widgets::side_panel(ui, "registry-tree", 360.0, |ui| {
        let mut out = Vec::new();
        for (hive, _) in HIVES {
            rows(doc, &mut out, hive, hive, 0);
        }
        let selected = out
            .iter()
            .position(|r| r.key.eq_ignore_ascii_case(&doc.path));
        let scroll = tree::Scroll::wanted(doc.reveal, false);
        let shown = tree::show(ui, egui::Id::new("registry-tree"), &out, selected, scroll);
        if shown.scrolled || selected.is_none() {
            doc.reveal = false;
        }
        for event in shown.events {
            match event {
                tree::Event::Select(path) => acts.push(Act::Registry(RegistryAct::Go(path, None))),
                tree::Event::Expand(path, open) => {
                    acts.push(Act::Registry(RegistryAct::Expand(path, open)))
                }
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
        .show(ui, |ui| {
            writers_line(ui, doc, catalog, texts, acts);
            values(ui, doc, acts);
        });
    editor(ui.ctx(), doc, acts);
    rights_window(ui.ctx(), doc, acts);
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
                acts.push(Act::Registry(RegistryAct::FindStop));
            }
        } else if ui.button(tr("Find")).clicked() || enter {
            acts.push(Act::Registry(RegistryAct::Find));
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
                    acts.push(Act::Registry(RegistryAct::Go(
                        hit.path.clone(),
                        hit.value.clone(),
                    )));
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
        acts.push(Act::Registry(RegistryAct::Edit));
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
    widgets::Modal::new(title).width(560.0).show(ctx, |ui| {
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
                    let types = (edit.kind == EditKind::NewValue).then_some(&NEW_TYPES[..]);
                    regedit::type_and_data(
                        ui,
                        "registry-type",
                        types,
                        &mut edit.type_name,
                        &mut edit.text,
                        380.0,
                    );
                }
            });
        if let Some(e) = &edit.error {
            ui.colored_label(style::palette(ui).warn, e);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(tr("OK")).clicked() {
                acts.push(Act::Registry(RegistryAct::Save));
            }
            if ui.button(tr("Cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
    });
    if close {
        doc.edit = None;
    }
}

/// The rights of a key in the window over the tab: its owner and its access list, the entries of
/// the key itself changed here, the inherited ones shown.
pub struct RightsEdit {
    pub path: String,
    pub rights: Rights,
    /// The names of the accounts by their security identifiers, read when the window opens.
    pub names: HashMap<String, String>,
    /// The account typed for a new entry.
    pub add: String,
    pub error: Option<String>,
    pub changed: bool,
}

/// What an entry gives, as regedit names it.
fn access_title(entry: &Entry) -> String {
    if entry.full() {
        tr("Full control").to_owned()
    } else if entry.read() {
        tr("Read").to_owned()
    } else {
        let mask = format!("0x{:x}", entry.mask);
        trf("Special ({mask})", &[("mask", &mask)])
    }
}

fn scope_title(scope: Scope) -> &'static str {
    match scope {
        Scope::Key => tr("This key only"),
        Scope::KeyAndSubkeys => tr("This key and subkeys"),
        Scope::Subkeys => tr("Subkeys only"),
    }
}

/// The controls of an entry of the key itself: allow or deny, the access, where it applies;
/// whether one changed.
fn entry_controls(ui: &mut egui::Ui, i: usize, entry: &mut Entry) -> bool {
    let mut changed = false;
    let mut deny = entry.deny;
    let kind = if deny { tr("Deny") } else { tr("Allow") };
    egui::ComboBox::from_id_salt(("rights-type", i))
        .selected_text(kind)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut deny, false, tr("Allow"));
            ui.selectable_value(&mut deny, true, tr("Deny"));
        });
    if deny != entry.deny {
        entry.deny = deny;
        changed = true;
    }
    let mut pick = None;
    let (full, read) = (entry.full(), entry.read());
    egui::ComboBox::from_id_salt(("rights-access", i))
        .selected_text(access_title(entry))
        .show_ui(ui, |ui| {
            if ui.selectable_label(full, tr("Full control")).clicked() {
                pick = Some(true);
            }
            if ui.selectable_label(read && !full, tr("Read")).clicked() {
                pick = Some(false);
            }
        });
    if let Some(f) = pick {
        entry.set_full(f);
        changed = true;
    }
    let mut scope = entry.scope;
    egui::ComboBox::from_id_salt(("rights-scope", i))
        .selected_text(scope_title(scope))
        .show_ui(ui, |ui| {
            for s in [Scope::Key, Scope::KeyAndSubkeys, Scope::Subkeys] {
                ui.selectable_value(&mut scope, s, scope_title(s));
            }
        });
    if scope != entry.scope {
        entry.scope = scope;
        changed = true;
    }
    changed
}

/// The window of the rights of a key: the owner, the inheritance, the entries and a new one.
fn rights_window(ctx: &egui::Context, doc: &mut RegistryDoc, acts: &mut Vec<Act>) {
    let Some(edit) = &mut doc.rights else {
        return;
    };
    let mut close = false;
    let mut gone = None;
    widgets::Modal::new(tr("Permissions"))
        .id("registry-rights")
        .width(680.0)
        .show(ctx, |ui| {
            ui.weak(&edit.path);
            ui.add_space(4.0);
            let owner = edit
                .names
                .get(&edit.rights.owner)
                .unwrap_or(&edit.rights.owner);
            ui.label(trf("Owner: {name}", &[("name", owner)]));
            let mut inherit = !edit.rights.protected;
            let text = tr("Inherit the entries of the key above");
            if ui.checkbox(&mut inherit, text).changed() {
                edit.rights.protected = !inherit;
                edit.changed = true;
            }
            ui.add_space(6.0);
            let mut changed = false;
            egui::Grid::new("registry-rights-grid")
                .num_columns(5)
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for title in [tr("Account"), tr("Type"), tr("Access"), tr("Applies to")] {
                        ui.strong(title);
                    }
                    ui.label("");
                    ui.end_row();
                    for (i, entry) in edit.rights.entries.iter_mut().enumerate() {
                        let name = edit.names.get(&entry.sid).unwrap_or(&entry.sid);
                        ui.label(name.as_str()).on_hover_text(&entry.sid);
                        if entry.inherited || entry.other.is_some() {
                            ui.weak(if entry.deny { tr("Deny") } else { tr("Allow") });
                            ui.weak(access_title(entry));
                            ui.weak(scope_title(entry.scope));
                            ui.weak(if entry.inherited {
                                tr("inherited")
                            } else {
                                tr("kept as it is")
                            });
                        } else {
                            changed |= entry_controls(ui, i, entry);
                            let remove = ui.small_button(icons::CLOSE);
                            if remove.on_hover_text(tr("Remove the entry")).clicked() {
                                gone = Some(i);
                            }
                        }
                        ui.end_row();
                    }
                });
            if changed {
                edit.changed = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut edit.add)
                        .hint_text(tr("An account or a group of this PC"))
                        .desired_width(260.0),
                );
                if ui.button(tr("Add")).clicked() {
                    acts.push(Act::Registry(RegistryAct::RightsAdd));
                }
            });
            if let Some(e) = &edit.error {
                ui.colored_label(style::palette(ui).warn, e);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let apply = ui.add_enabled(edit.changed, egui::Button::new(tr("Apply")));
                if apply.clicked() {
                    acts.push(Question::change_rights(&edit.path).into());
                }
                if ui.button(tr("Cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            });
        });
    if let Some(i) = gone {
        edit.rights.entries.remove(i);
        edit.changed = true;
    }
    if close {
        doc.rights = None;
    }
}
