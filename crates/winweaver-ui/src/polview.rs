//! The tab of a Group Policy file (`Registry.pol`): its keys as a tree on the left, the entries of
//! the selected key on the right with what each does, the way to the registry tab and to the
//! rules that write a value. Read only. The files of this Windows are read once and kept for the
//! links of the other tabs ([`system`], [`link`]).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use eframe::egui;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::pol::{self, Action, PolFile, Scope};
use winweaver_core::registry;

use crate::i18n::{tr, trf};
use crate::{
    Act, PolAct, ProfileAct, RegistryAct, Search, Tab, icons, list, matches_words, style, tree,
    views, widgets,
};

pub struct PolDoc {
    pub pol: PolFile,
    /// The selected key as the file writes it; `None` for the whole file.
    pub sel: Option<String>,
    /// The selected entry, by its index in the file.
    pub entry: Option<usize>,
    /// The keys folded in the tree, in lower case.
    pub closed: HashSet<String>,
    pub search: Search,
    /// The selection moved with the keys: it comes into view at the next frame.
    pub follow: bool,
    pub follow_entry: bool,
}

impl PolDoc {
    pub fn new(pol: PolFile) -> Self {
        Self {
            pol,
            sel: None,
            entry: None,
            closed: HashSet::new(),
            search: Search::default(),
            follow: false,
            follow_entry: false,
        }
    }

    /// Selects a key, in the form of regedit with its hive or as the file writes it, and a value.
    pub fn select(&mut self, key: &str, value: Option<&str>) {
        let found = self.pol.entries.iter().position(|e| {
            (self.pol.full_key(&e.key).eq_ignore_ascii_case(key) || e.key.eq_ignore_ascii_case(key))
                && value.is_none_or(|v| e.name.eq_ignore_ascii_case(v))
        });
        if let Some(i) = found {
            self.sel = Some(self.pol.entries[i].key.clone());
            self.entry = value.map(|_| i);
            self.follow = true;
            self.follow_entry = true;
            let mut up = self.pol.entries[i].key.to_lowercase();
            while let Some((parent, _)) = up.rsplit_once('\\') {
                self.closed.remove(parent);
                up = parent.to_owned();
            }
        }
    }
}

/// The title of a file: the computer's or the users' of this Windows, else its name.
pub fn title_of(pol: &PolFile) -> String {
    match pol.scope {
        Scope::Computer => tr("Group Policy of the computer").to_owned(),
        Scope::User => tr("Group Policy of the users").to_owned(),
        Scope::Unknown => pol
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Registry.pol".to_owned()),
    }
}

impl Tab for PolDoc {
    fn title(&self) -> String {
        title_of(&self.pol)
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.pol.path)
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }
}

/// What an entry does, in a few words.
pub fn action_title(action: &Action) -> String {
    match action {
        Action::Set(_) => tr("Sets the value").to_owned(),
        Action::Soft(_) => tr("Sets the value when it is not there").to_owned(),
        Action::Delete => tr("Deletes the value").to_owned(),
        Action::DeleteAll => tr("Deletes every value of the key").to_owned(),
        Action::DeleteValues(list) => {
            trf("Deletes the values {names}", &[("names", &list.join(", "))])
        }
        Action::DeleteKeys(list) => trf("Deletes the keys {names}", &[("names", &list.join(", "))]),
        Action::SecureKey(true) => tr("Only the administrators may change the key").to_owned(),
        Action::SecureKey(false) => tr("The key keeps its own rights").to_owned(),
        Action::CreateKey => tr("Makes the key").to_owned(),
    }
}

/// The type and the data of an entry that sets a value.
fn data_of(action: &Action) -> (String, String) {
    match action {
        Action::Set(d) | Action::Soft(d) => {
            let mut shown = d.display();
            if shown.chars().count() > 200 {
                shown = shown.chars().take(200).collect::<String>() + "...";
            }
            (d.type_name(), shown)
        }
        _ => (String::new(), String::new()),
    }
}

/// The keys of the file as a tree of their paths; the count of each is the number of its entries.
fn key_rows(doc: &PolDoc) -> Vec<tree::Row<Option<String>>> {
    struct Part {
        name: String,
        path: String,
        count: usize,
        children: Vec<usize>,
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut roots: Vec<usize> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for e in &doc.pol.entries {
        let mut parent: Option<usize> = None;
        let mut current = String::new();
        for name in e.key.split('\\').filter(|p| !p.is_empty()) {
            if !current.is_empty() {
                current.push('\\');
            }
            current.push_str(name);
            let known = index.get(&current.to_lowercase()).copied();
            let n = match known {
                Some(n) => n,
                None => {
                    parts.push(Part {
                        name: name.to_owned(),
                        path: current.clone(),
                        count: 0,
                        children: Vec::new(),
                    });
                    let n = parts.len() - 1;
                    index.insert(current.to_lowercase(), n);
                    match parent {
                        Some(p) => parts[p].children.push(n),
                        None => roots.push(n),
                    }
                    n
                }
            };
            parent = Some(n);
        }
        if let Some(n) = parent {
            parts[n].count += 1;
        }
    }
    let mut rows = vec![tree::Row {
        key: None,
        depth: 0,
        title: tr("The whole file").to_owned(),
        right: doc.pol.entries.len().to_string(),
        check: tree::Check::None,
        open: None,
        muted: false,
        icon: Some(icons::NEW),
        switch: None,
    }];
    let mut stack: Vec<(usize, usize)> = roots.iter().rev().map(|&n| (n, 0)).collect();
    while let Some((n, depth)) = stack.pop() {
        let part = &parts[n];
        let open = !doc.closed.contains(&part.path.to_lowercase());
        rows.push(tree::Row {
            key: Some(part.path.clone()),
            depth,
            title: part.name.clone(),
            right: if part.count > 0 {
                part.count.to_string()
            } else {
                String::new()
            },
            check: tree::Check::None,
            open: (!part.children.is_empty()).then_some(open),
            muted: part.count == 0,
            icon: Some(icons::FOLDER),
            switch: None,
        });
        if open {
            for &c in part.children.iter().rev() {
                stack.push((c, depth + 1));
            }
        }
    }
    rows
}

/// The tab: the tree of keys, the head of the file or of the key, the entries of the key (or the
/// ones a search finds), the rules that write the selected value.
pub fn pol_view(
    ui: &mut egui::Ui,
    doc: &mut PolDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    acts: &mut Vec<Act>,
) {
    let p = style::palette(ui);
    widgets::side_panel(ui, "pol-keys", 380.0, |ui| {
        let rows = key_rows(doc);
        let selected = rows.iter().position(|r| {
            r.key.as_deref().map(str::to_lowercase) == doc.sel.as_deref().map(str::to_lowercase)
        });
        let scroll = tree::Scroll::wanted(false, doc.follow);
        let id = egui::Id::new(("pol-keys", doc.pol.path.display().to_string()));
        let shown = tree::show(ui, id, &rows, selected, scroll);
        if shown.scrolled || selected.is_none() {
            doc.follow = false;
        }
        if shown.moved {
            doc.follow = true;
        }
        for event in shown.events {
            match event {
                tree::Event::Select(key) => {
                    doc.sel = key;
                    doc.entry = None;
                }
                tree::Event::Expand(Some(key), open) => {
                    if open {
                        doc.closed.remove(&key.to_lowercase());
                    } else {
                        doc.closed.insert(key.to_lowercase());
                    }
                }
                _ => {}
            }
        }
    });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg))
        .show(ui, |ui| {
            let words = doc.search.words();
            let full = doc.sel.as_ref().map(|k| doc.pol.full_key(k));
            views::header(ui, |ui| {
                let title = match &full {
                    Some(key) => key.clone(),
                    None => title_of(&doc.pol),
                };
                style::copy_label(ui, style::strong(ui, title, 16.0), true);
                let path = style::muted(ui, doc.pol.path.display().to_string(), 12.0);
                style::copy_label(ui, path, true);
                ui.horizontal_wrapped(|ui| {
                    let scope = match doc.pol.scope {
                        Scope::Computer => "HKEY_LOCAL_MACHINE",
                        Scope::User => "HKEY_CURRENT_USER",
                        Scope::Unknown => tr("No hive: a file outside Group Policy"),
                    };
                    style::chip(ui, scope, true);
                    style::chip(
                        ui,
                        &trf("{n} entries", &[("n", &doc.pol.entries.len())]),
                        false,
                    );
                    if let Some(key) = &full
                        && doc.pol.scope.hive().is_some()
                        && ui.link(tr("Open in the registry")).clicked()
                    {
                        let value = doc
                            .entry
                            .and_then(|i| doc.pol.entries.get(i))
                            .map(|e| e.name.clone());
                        acts.push(Act::Registry(RegistryAct::Open(key.clone(), value)));
                    }
                });
                ui.label(style::muted(
                    ui,
                    tr("Group Policy writes these values into the registry at every refresh of the policies; a change made in the registry alone is undone then."),
                    12.0,
                ));
            });
            let shown: Vec<usize> = doc
                .pol
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    if !words.is_empty() {
                        let (_, data) = data_of(&e.action);
                        let text = format!("{}\n{}\n{data}", e.key, e.name);
                        return matches_words(&text, &words);
                    }
                    doc.sel
                        .as_ref()
                        .is_none_or(|k| e.key.eq_ignore_ascii_case(k))
                })
                .map(|(i, _)| i)
                .collect();
            entries_list(ui, doc, &shown, full.is_none() || !words.is_empty());
            if let Some(i) = doc.entry
                && let Some(e) = doc.pol.entries.get(i)
            {
                let key = doc.pol.full_key(&e.key);
                let value = (!e.name.is_empty()).then_some(e.name.as_str());
                let rules = registry::writers(catalog, &key, value);
                if !rules.is_empty() {
                    views::block(ui, |ui| {
                        style::section_title(ui, tr("Rules that write it"));
                        ui.add_space(4.0);
                        for rule in rules.iter().take(30) {
                            if ui.link(views::rtitle(texts, rule)).clicked() {
                                acts.push(Act::Profile(ProfileAct::ShowInProfile(
                                    String::new(),
                                    rule.id.clone(),
                                )));
                            }
                        }
                    });
                }
            }
        });
}

/// The entries `shown` as a list; `with_key` adds the column of the key.
fn entries_list(ui: &mut egui::Ui, doc: &mut PolDoc, shown: &[usize], with_key: bool) {
    let mut columns = Vec::new();
    if with_key {
        columns.push(list::Column::new(tr("Key"), 320.0));
    }
    columns.extend([
        list::Column::new(tr("Name"), 200.0),
        list::Column::new(tr("Action"), 200.0),
        list::Column::new(tr("Type"), 120.0),
        list::Column::new(tr("Data"), 300.0),
    ]);
    let rows: Vec<list::Row<usize>> = shown
        .iter()
        .map(|&i| {
            let e = &doc.pol.entries[i];
            let (kind, data) = data_of(&e.action);
            let mut cells = Vec::new();
            if with_key {
                cells.push(list::Cell::new(e.key.clone()).muted());
            }
            cells.extend([
                list::Cell::new(regview_title(&e.name)),
                list::Cell::new(action_title(&e.action)),
                list::Cell::new(kind).muted(),
                list::Cell::new(data),
            ]);
            list::Row {
                key: i,
                cells,
                button: None,
            }
        })
        .collect();
    let selected = doc.entry.and_then(|i| shown.iter().position(|&s| s == i));
    let scroll = tree::Scroll::wanted(false, doc.follow_entry);
    let id = egui::Id::new(("pol-entries", doc.pol.path.display().to_string()));
    let out = list::show(ui, id, tr("Entries"), &columns, &rows, selected, scroll);
    if out.scrolled || selected.is_none() {
        doc.follow_entry = false;
    }
    if out.moved {
        doc.follow_entry = true;
    }
    for event in out.events {
        if let list::Event::Select(i) | list::Event::Open(i) = event {
            doc.entry = Some(i);
        }
    }
}

/// The name of a value as regedit shows it: `(Default)` for the empty one.
fn regview_title(name: &str) -> String {
    if name.is_empty() {
        tr("(Default)").to_owned()
    } else {
        name.to_owned()
    }
}

/// The Group Policy files of this Windows, read once.
static SYSTEM: RwLock<Option<Arc<Vec<PolFile>>>> = RwLock::new(None);

/// The Group Policy files of this Windows that are there and read, the first time they are
/// asked for; [`reload_system`] reads them again.
pub fn system() -> Arc<Vec<PolFile>> {
    if let Some(files) = SYSTEM.read().ok().and_then(|s| s.clone()) {
        return files;
    }
    reload_system()
}

/// Reads the Group Policy files of this Windows again.
pub fn reload_system() -> Arc<Vec<PolFile>> {
    let files: Vec<PolFile> = pol::system_files()
        .into_iter()
        .filter(|(_, path)| path.is_file())
        .filter_map(|(_, path)| PolFile::load(&path).ok())
        .collect();
    let files = Arc::new(files);
    if let Ok(mut s) = SYSTEM.write() {
        *s = Some(files.clone());
    }
    files
}

/// A small button beside a key or a value when Group Policy of this Windows sets or deletes it:
/// it opens the policy file there. `key` is in any form the catalog or regedit writes.
pub fn link(ui: &mut egui::Ui, key: &str, value: Option<&str>, acts: &mut Vec<Act>) {
    let Some(full) = registry::normalize(key) else {
        return;
    };
    for file in system().iter() {
        if file.find(&full, value).is_empty() {
            continue;
        }
        let hint = match file.scope {
            Scope::User => tr("Group Policy of the users sets it"),
            _ => tr("Group Policy of the computer sets it"),
        };
        if ui.small_button(icons::SCRIPT).on_hover_text(hint).clicked() {
            acts.push(Act::Pol(PolAct::OpenSystem(
                file.scope,
                Some(full.clone()),
                value.map(str::to_owned),
            )));
        }
    }
}
