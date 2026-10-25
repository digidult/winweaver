//! The tab of a Group Policy file (`Registry.pol`): its keys as a tree on the left, the entries of
//! the selected key on the right with what each does, the way to the registry tab and to the
//! rules that write a value. An entry is edited in a window over the tab, like a value of a
//! registry file (the same fields of type and data), and removed after a question; the root a
//! file outside Group Policy applies to is chosen in the bar. The files of this Windows are read
//! once and kept for the links of the other tabs ([`system`], [`link`]); Save writes a copy of
//! them elsewhere.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use eframe::egui;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::pol::{self, Action, Entry, PolFile, Scope};
use winweaver_core::registry::{self, Data};

use crate::i18n::{tr, trf};
use crate::{
    Act, PolAct, ProfileAct, Question, RegistryAct, Search, Tab, icons, list, matches_words,
    regedit, style, tree, views, widgets,
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
    pub dirty: bool,
    /// The editor over the tab.
    pub edit: Option<PolEdit>,
    pub error: Option<String>,
    pub draft: ww::DraftState,
}

/// What the editor makes an entry do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Set,
    Soft,
    Delete,
    DeleteAll,
    DeleteValues,
    DeleteKeys,
    SecureOn,
    SecureOff,
    CreateKey,
}

impl EntryKind {
    const ALL: [EntryKind; 9] = [
        EntryKind::Set,
        EntryKind::Soft,
        EntryKind::Delete,
        EntryKind::DeleteAll,
        EntryKind::DeleteValues,
        EntryKind::DeleteKeys,
        EntryKind::SecureOn,
        EntryKind::SecureOff,
        EntryKind::CreateKey,
    ];

    fn of(action: &Action) -> Self {
        match action {
            Action::Set(_) => EntryKind::Set,
            Action::Soft(_) => EntryKind::Soft,
            Action::Delete => EntryKind::Delete,
            Action::DeleteAll => EntryKind::DeleteAll,
            Action::DeleteValues(_) => EntryKind::DeleteValues,
            Action::DeleteKeys(_) => EntryKind::DeleteKeys,
            Action::SecureKey(true) => EntryKind::SecureOn,
            Action::SecureKey(false) => EntryKind::SecureOff,
            Action::CreateKey => EntryKind::CreateKey,
        }
    }

    fn title(self) -> String {
        match self {
            EntryKind::DeleteValues => tr("Deletes the values named").to_owned(),
            EntryKind::DeleteKeys => tr("Deletes the keys named").to_owned(),
            other => action_title(&other.sample()),
        }
    }

    /// An action of the kind, for its title.
    fn sample(self) -> Action {
        match self {
            EntryKind::Set => Action::Set(Data::Dword(0)),
            EntryKind::Soft => Action::Soft(Data::Dword(0)),
            EntryKind::Delete => Action::Delete,
            EntryKind::DeleteAll => Action::DeleteAll,
            EntryKind::DeleteValues => Action::DeleteValues(Vec::new()),
            EntryKind::DeleteKeys => Action::DeleteKeys(Vec::new()),
            EntryKind::SecureOn => Action::SecureKey(true),
            EntryKind::SecureOff => Action::SecureKey(false),
            EntryKind::CreateKey => Action::CreateKey,
        }
    }

    /// The kinds that name one value.
    fn named(self) -> bool {
        matches!(self, EntryKind::Set | EntryKind::Soft | EntryKind::Delete)
    }

    fn with_data(self) -> bool {
        matches!(self, EntryKind::Set | EntryKind::Soft)
    }

    fn with_list(self) -> bool {
        matches!(self, EntryKind::DeleteValues | EntryKind::DeleteKeys)
    }
}

/// The editor of an entry over the tab.
pub struct PolEdit {
    /// The entry it changes; `None` for a new one.
    pub index: Option<usize>,
    /// The key without its hive.
    pub key: String,
    pub kind: EntryKind,
    pub name: String,
    pub type_name: String,
    pub text: String,
    /// The names a list deletes, one a line.
    pub names: String,
    pub error: Option<String>,
}

/// The short names of the hives a key typed with its hive may start with.
const HIVES: [(&str, &str); 2] = [
    ("HKEY_LOCAL_MACHINE", "HKLM"),
    ("HKEY_CURRENT_USER", "HKCU"),
];

impl PolDoc {
    pub fn new(pol: PolFile) -> Self {
        let draft = ww::DraftState::new(Some(&pol.path));
        Self {
            pol,
            sel: None,
            entry: None,
            closed: HashSet::new(),
            search: Search::default(),
            follow: false,
            follow_entry: false,
            dirty: false,
            edit: None,
            error: None,
            draft,
        }
    }

    /// Whether the file is one of the two files of Group Policy of this Windows, whose root
    /// their folder gives and which Save does not change.
    pub fn is_system(&self) -> bool {
        pol::system_files().iter().any(|(_, p)| {
            p.to_string_lossy()
                .eq_ignore_ascii_case(&self.pol.path.to_string_lossy())
        })
    }

    /// The editor of the selected entry.
    pub fn start_edit(&mut self) {
        let Some(e) = self.entry.and_then(|i| self.pol.entries.get(i)) else {
            self.error = Some(tr("Select an entry first.").to_owned());
            return;
        };
        let mut edit = PolEdit {
            index: self.entry,
            key: e.key.clone(),
            kind: EntryKind::of(&e.action),
            name: e.name.clone(),
            type_name: "REG_DWORD".to_owned(),
            text: String::new(),
            names: String::new(),
            error: None,
        };
        match &e.action {
            Action::Set(d) | Action::Soft(d) => {
                if !d.editable() {
                    self.error = Some(tr("This type of value is shown, not edited.").to_owned());
                    return;
                }
                edit.type_name = d.type_name();
                edit.text = d.edit_text();
            }
            Action::DeleteValues(list) | Action::DeleteKeys(list) => edit.names = list.join("\n"),
            _ => {}
        }
        self.error = None;
        self.edit = Some(edit);
    }

    /// The editor of a new key under the selected one, or of a new value of the selected key.
    pub fn start_new(&mut self, key: bool) {
        let base = self.sel.clone().unwrap_or_default();
        if !key && base.is_empty() {
            let text = tr("Select a key first: a new value goes under it.");
            self.error = Some(text.to_owned());
            return;
        }
        let (kind, key) = if key {
            let under = if base.is_empty() {
                String::new()
            } else {
                format!("{base}\\")
            };
            (EntryKind::CreateKey, under)
        } else {
            (EntryKind::Set, base)
        };
        self.error = None;
        self.edit = Some(PolEdit {
            index: None,
            key,
            kind,
            name: String::new(),
            type_name: "REG_DWORD".to_owned(),
            text: String::new(),
            names: String::new(),
            error: None,
        });
    }

    /// The key of the editor without the hive of the file it may be typed with; the error for a
    /// key that is empty, has a control character or an empty part, or is in another hive.
    fn edited_key(&self, text: &str) -> Result<String, String> {
        let text = text.trim().trim_matches('\\');
        let (head, rest) = text.split_once('\\').unwrap_or((text, ""));
        let hive = HIVES.iter().find(|(long, short)| {
            head.eq_ignore_ascii_case(long) || head.eq_ignore_ascii_case(short)
        });
        let key = match hive {
            Some((long, _)) if self.pol.scope.hive() == Some(*long) => rest,
            Some(_) => return Err(tr("The key is in another hive than the file.").to_owned()),
            None => text,
        };
        let bad = key.is_empty()
            || key.chars().any(char::is_control)
            || key.split('\\').any(str::is_empty);
        if bad {
            let text = tr("A key is needed: its parts without the hive, none of them empty.");
            return Err(text.to_owned());
        }
        Ok(key.to_owned())
    }

    /// Puts the entry of the editor into the file; false, with the error in the editor, for a
    /// key, a name or data Group Policy does not take.
    pub fn apply_edit(&mut self) -> bool {
        let Some(edit) = &self.edit else {
            return false;
        };
        let result = self.edited_key(&edit.key).and_then(|key| {
            let name = if edit.kind.named() {
                edit.name.clone()
            } else {
                String::new()
            };
            let long = name.chars().count() > registry::NAME_MAX;
            if long || name.chars().any(char::is_control) {
                return Err(tr("The name is too long or has a control character.").to_owned());
            }
            let names: Vec<String> = edit
                .names
                .split(['\n', ';'])
                .map(|n| n.trim().to_owned())
                .filter(|n| !n.is_empty())
                .collect();
            if edit.kind.with_list() && names.is_empty() {
                return Err(tr("Name at least one, one a line.").to_owned());
            }
            let data = || Data::parse(&edit.type_name, &edit.text);
            let action = match edit.kind {
                EntryKind::Set => Action::Set(data()?),
                EntryKind::Soft => Action::Soft(data()?),
                EntryKind::Delete => Action::Delete,
                EntryKind::DeleteAll => Action::DeleteAll,
                EntryKind::DeleteValues => Action::DeleteValues(names),
                EntryKind::DeleteKeys => Action::DeleteKeys(names),
                EntryKind::SecureOn => Action::SecureKey(true),
                EntryKind::SecureOff => Action::SecureKey(false),
                EntryKind::CreateKey => Action::CreateKey,
            };
            Ok(Entry { key, name, action })
        });
        let entry = match result {
            Ok(entry) => entry,
            Err(e) => {
                if let Some(edit) = &mut self.edit {
                    edit.error = Some(e);
                }
                return false;
            }
        };
        let index = self.edit.as_ref().and_then(|e| e.index);
        let key = entry.key.clone();
        let i = match index {
            Some(i) if i < self.pol.entries.len() => {
                self.pol.entries[i] = entry;
                i
            }
            _ => self.pol.put(entry),
        };
        self.edit = None;
        self.dirty = true;
        self.error = None;
        let name = self.pol.entries[i].name.clone();
        self.select(&key, Some(&name));
        self.entry = Some(i);
        true
    }

    /// The question before the selected entry, or the selected key with every entry of it and
    /// of the keys under it, leaves the file.
    pub fn remove_question(&self) -> Option<Question> {
        let (list, text) = if let Some(i) = self.entry {
            let e = self.pol.entries.get(i)?;
            let text = trf(
                "Remove the entry {what} of {key} from the file?",
                &[("what", &action_title(&e.action)), ("key", &e.key)],
            );
            (vec![i], text)
        } else {
            let key = self.sel.as_ref()?;
            let lower = key.to_lowercase();
            let list: Vec<usize> = self
                .pol
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    let k = e.key.to_lowercase();
                    k == lower || k.starts_with(&format!("{lower}\\"))
                })
                .map(|(i, _)| i)
                .collect();
            if list.is_empty() {
                return None;
            }
            let text = trf(
                "Remove the key {key} with its {n} entries and the keys under it from the file?",
                &[("key", key), ("n", &list.len())],
            );
            (list, text)
        };
        Some(Question::new(
            tr("Remove from the file"),
            text,
            tr("Remove"),
            PolAct::Remove(list),
        ))
    }

    /// Takes entries out of the file.
    pub fn remove_entries(&mut self, list: &[usize]) {
        let mut sorted = list.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for &i in sorted.iter().rev() {
            if i < self.pol.entries.len() {
                self.pol.entries.remove(i);
            }
        }
        self.entry = None;
        self.dirty = true;
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

    fn dirty(&self) -> bool {
        self.dirty
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
            if let Some(e) = &doc.error {
                ui.colored_label(p.warn, e);
            }
            if let Some(act) = entries_list(ui, doc, &shown, full.is_none() || !words.is_empty()) {
                acts.push(Act::Pol(act));
            }
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
    editor(ui.ctx(), doc);
}

/// The editor of an entry over the window: the key, what the entry does, the name of a value,
/// its type and data in the fields of a value of a registry file, or the names a list deletes.
fn editor(ctx: &egui::Context, doc: &mut PolDoc) {
    let hive = doc.pol.scope.hive();
    let Some(edit) = &mut doc.edit else {
        return;
    };
    let title = if edit.index.is_some() {
        tr("Edit the entry")
    } else {
        tr("New entry")
    };
    let mut close = false;
    let mut apply = false;
    widgets::Modal::new(title)
        .id("pol-editor")
        .width(560.0)
        .show(ctx, |ui| {
            egui::Grid::new("pol-edit")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(tr("Key"));
                    ui.vertical(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.key)
                                .desired_width(420.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        let note = match hive {
                            Some(h) => trf("in {hive}", &[("hive", &h)]),
                            None => {
                                tr("without a hive: the root of the file is not chosen").to_owned()
                            }
                        };
                        ui.label(style::muted(ui, note, 11.0));
                    });
                    ui.end_row();
                    ui.label(tr("Action"));
                    egui::ComboBox::from_id_salt("pol-kind")
                        .selected_text(edit.kind.title())
                        .width(420.0)
                        .show_ui(ui, |ui| {
                            for kind in EntryKind::ALL {
                                ui.selectable_value(&mut edit.kind, kind, kind.title());
                            }
                        });
                    ui.end_row();
                    if edit.kind.named() {
                        ui.label(tr("Name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.name)
                                .desired_width(420.0)
                                .hint_text(tr("(Default)")),
                        );
                        ui.end_row();
                    }
                    if edit.kind.with_data() {
                        let types: Vec<&str> = registry::NEW_TYPES.to_vec();
                        regedit::type_and_data(
                            ui,
                            "pol-type",
                            Some(&types),
                            &mut edit.type_name,
                            &mut edit.text,
                            420.0,
                        );
                    }
                    if edit.kind.with_list() {
                        ui.label(tr("Names"));
                        ui.add(
                            egui::TextEdit::multiline(&mut edit.names)
                                .desired_width(420.0)
                                .desired_rows(4)
                                .hint_text(tr("One name a line")),
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

/// The entries `shown` as a list; `with_key` adds the column of the key. The act of Enter or a
/// double click (edit) or of Delete (the question of the removal) on an entry.
fn entries_list(
    ui: &mut egui::Ui,
    doc: &mut PolDoc,
    shown: &[usize],
    with_key: bool,
) -> Option<PolAct> {
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
    let mut act = None;
    for event in out.events {
        match event {
            list::Event::Select(i) => doc.entry = Some(i),
            list::Event::Open(i) => {
                doc.entry = Some(i);
                act = Some(PolAct::Edit);
            }
            list::Event::Delete(i) => {
                doc.entry = Some(i);
                act = Some(PolAct::AskRemove);
            }
            list::Event::Button(_) => {}
        }
    }
    act
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
