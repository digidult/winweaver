//! The editor of a catalog file (customer list of 18.10.2026, item 11): its groups and policies as a
//! tree on the left, the selected policy as a form on the right. A change of the form reaches the
//! file with Apply, which checks the whole file (`CatalogFile::put_rule`), and the file with Save;
//! a loaded import is read again after it is saved.

use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui;
use serde_json::Value;
use winweaver_core::admx::{self, CatalogFile};
use winweaver_core::catalog::{Action, Phase, ROOT_POLICIES, RegKind, Rule};
use winweaver_core::registry;

use crate::i18n::{tr, trf};
use crate::{Act, style, tree};

/// The kinds of values the form writes.
const KINDS: [(RegKind, &str); 4] = [
    (RegKind::DWord, "REG_DWORD"),
    (RegKind::QWord, "REG_QWORD"),
    (RegKind::String, "REG_SZ"),
    (RegKind::ExpandString, "REG_EXPAND_SZ"),
];

/// A registry action of the form: a value written or removed, or another action kept as it is.
pub struct ActionRow {
    pub remove: bool,
    /// The key in the form of regedit.
    pub key: String,
    pub name: String,
    pub kind: RegKind,
    pub value: String,
    /// An action the form does not edit (a list of values), kept unchanged.
    pub keep: Option<Action>,
}

/// The selected policy as the form edits it.
pub struct Form {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub user: bool,
    pub actions: Vec<ActionRow>,
}

pub struct EditorDoc {
    pub file: CatalogFile,
    /// Where Save writes the file.
    pub path: PathBuf,
    /// Whether the file is an import of the program, read again after a save.
    pub import: bool,
    pub dirty: bool,
    /// The selected group, and the selected policy with its form.
    pub group: Option<String>,
    pub form: Option<Form>,
    pub expanded: HashSet<String>,
    pub error: Option<String>,
    /// The title of a new group or policy, typed in the bar of the form.
    pub new_name: String,
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

impl ActionRow {
    fn of(action: &Action) -> Self {
        let shown = |path: &str| registry::normalize(path).unwrap_or_else(|| path.to_owned());
        match action {
            Action::Reg {
                path,
                name,
                kind,
                value,
                ..
            } if KINDS.iter().any(|(k, _)| k == kind) => Self {
                remove: false,
                key: shown(path),
                name: name.clone(),
                kind: *kind,
                value: value_text(value),
                keep: None,
            },
            Action::RegRemove { path, name, .. } => Self {
                remove: true,
                key: shown(path),
                name: name.clone(),
                kind: RegKind::DWord,
                value: String::new(),
                keep: None,
            },
            other => Self {
                remove: false,
                key: String::new(),
                name: crate::views::describe(other),
                kind: RegKind::DWord,
                value: String::new(),
                keep: Some(other.clone()),
            },
        }
    }

    /// The action of the row, or why it cannot be one.
    fn action(&self) -> Result<Action, String> {
        if let Some(a) = &self.keep {
            return Ok(a.clone());
        }
        let path = admx::catalog_path(&self.key).ok_or_else(|| {
            trf(
                "{key} is not a key of the machine or of the user",
                &[("key", &self.key)],
            )
        })?;
        if self.remove {
            return Ok(Action::RegRemove {
                path,
                name: self.name.clone(),
                default: None,
            });
        }
        let value = match self.kind {
            RegKind::DWord | RegKind::QWord => {
                let t = self.value.trim();
                let n = match t.strip_prefix("0x") {
                    Some(h) => u64::from_str_radix(h, 16),
                    None => t.parse::<u64>(),
                }
                .map_err(|_| trf("{value} is not a number", &[("value", &t)]))?;
                if self.kind == RegKind::DWord && n > u64::from(u32::MAX) {
                    return Err(trf("{value} is not a number", &[("value", &t)]));
                }
                Value::from(n)
            }
            _ => Value::from(self.value.clone()),
        };
        Ok(Action::Reg {
            path,
            name: self.name.clone(),
            kind: self.kind,
            value,
            default: None,
            why: String::new(),
        })
    }
}

impl EditorDoc {
    pub fn new(file: CatalogFile, path: PathBuf, import: bool) -> Self {
        let expanded = file.groups.iter().map(|g| g.id.clone()).collect();
        Self {
            file,
            path,
            import,
            dirty: false,
            group: None,
            form: None,
            expanded,
            error: None,
            new_name: String::new(),
        }
    }

    pub fn title(&self) -> String {
        let star = if self.dirty { "*" } else { "" };
        trf("{name}: editor", &[("name", &self.file.name)]) + star
    }

    /// Selects a group (`g:<id>`) or a policy, whose form starts from the file.
    pub fn select(&mut self, key: &str) {
        self.error = None;
        if let Some(g) = key.strip_prefix("g:") {
            self.group = Some(g.to_owned());
            self.form = None;
            return;
        }
        let Some(rule) = self.file.rules.iter().find(|r| r.id == key) else {
            return;
        };
        self.group = Some(rule.group.clone());
        self.form = Some(Form {
            id: rule.id.clone(),
            title: rule.title.clone(),
            summary: rule.summary.clone(),
            user: rule.phase == Phase::DefaultUser,
            actions: rule.actions.iter().map(ActionRow::of).collect(),
        });
    }

    /// Puts the form into the file; the whole file is checked, and an error leaves it unchanged.
    pub fn apply(&mut self) {
        let Some(form) = &self.form else {
            return;
        };
        let Some(old) = self.file.rules.iter().find(|r| r.id == form.id) else {
            return;
        };
        let actions: Result<Vec<Action>, String> =
            form.actions.iter().map(ActionRow::action).collect();
        let actions = match actions {
            Ok(a) => a,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let rule = Rule {
            title: form.title.trim().to_owned(),
            summary: form.summary.trim().to_owned(),
            phase: if form.user {
                Phase::DefaultUser
            } else {
                Phase::Specialize
            },
            actions,
            ..old.clone()
        };
        match self.file.put_rule(rule) {
            Ok(()) => {
                self.dirty = true;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// A new policy in the selected group (or the first one) with the typed title.
    pub fn new_policy(&mut self) {
        let Some(group) = self.group.clone().or_else(|| self.file.groups.first().map(|g| g.id.clone())) else {
            return;
        };
        let name = self.new_name.trim().to_owned();
        if name.is_empty() {
            self.error = Some(tr("Type the title of the new policy first").to_owned());
            return;
        }
        let result = self
            .file
            .new_policy(&group, &name)
            .and_then(|rule| {
                let id = rule.id.clone();
                self.file.put_rule(rule).map(|()| id)
            });
        match result {
            Ok(id) => {
                self.dirty = true;
                self.new_name.clear();
                self.expanded.insert(group);
                self.select(&id);
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// A new group under the selected group (or the root of the file) with the typed title.
    pub fn new_group(&mut self) {
        let Some(parent) = self.group.clone().or_else(|| self.file.groups.first().map(|g| g.id.clone())) else {
            return;
        };
        match self.file.new_group(&parent, &self.new_name) {
            Ok(id) => {
                self.dirty = true;
                self.new_name.clear();
                self.expanded.insert(parent);
                self.select(&format!("g:{id}"));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Removes a policy with its Disabled rule.
    pub fn remove(&mut self, id: &str) {
        match self.file.remove_rule(id) {
            Ok(()) => {
                self.dirty = true;
                self.form = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
}

/// The rows of the tree under a group of the file.
fn rows(doc: &EditorDoc, out: &mut Vec<tree::Row<String>>, group: &str, depth: usize) {
    let Some(g) = doc.file.groups.iter().find(|g| g.id == group) else {
        return;
    };
    let open = doc.expanded.contains(group);
    out.push(tree::Row {
        key: format!("g:{group}"),
        depth,
        title: g.title.clone(),
        right: String::new(),
        check: tree::Check::None,
        open: Some(open),
        muted: false,
        icon: None,
        switch: None,
    });
    if !open {
        return;
    }
    let children: Vec<String> = doc
        .file
        .groups
        .iter()
        .filter(|c| c.parent.as_deref() == Some(group))
        .map(|c| c.id.clone())
        .collect();
    for child in children {
        rows(doc, out, &child, depth + 1);
    }
    for r in doc.file.rules.iter().filter(|r| r.group == group) {
        out.push(tree::Row {
            key: r.id.clone(),
            depth: depth + 1,
            title: r.title.clone(),
            right: String::new(),
            check: tree::Check::None,
            open: None,
            muted: r.id.ends_with(".off"),
            icon: None,
            switch: None,
        });
    }
}

/// The tab: the tree on the left, the form on the right.
pub fn editor_view(ui: &mut egui::Ui, doc: &mut EditorDoc, acts: &mut Vec<Act>) {
    let p = style::palette(ui);
    egui::Panel::left("catalog-editor-tree")
        .resizable(true)
        .default_size(360.0)
        .frame(egui::Frame::new().fill(p.card).inner_margin(egui::Margin::symmetric(0, 4)))
        .show(ui, |ui| {
            let mut out = Vec::new();
            let roots: Vec<String> = doc
                .file
                .groups
                .iter()
                .filter(|g| g.parent.as_deref() == Some(ROOT_POLICIES))
                .map(|g| g.id.clone())
                .collect();
            for root in roots {
                rows(doc, &mut out, &root, 0);
            }
            let selected_key = match (&doc.form, &doc.group) {
                (Some(f), _) => Some(f.id.clone()),
                (None, Some(g)) => Some(format!("g:{g}")),
                _ => None,
            };
            let selected = selected_key.and_then(|k| out.iter().position(|r| r.key == k));
            let shown = tree::show(
                ui,
                egui::Id::new(("catalog-editor", doc.path.display().to_string())),
                &out,
                selected,
                tree::Scroll::None,
            );
            for event in shown.events {
                match event {
                    tree::Event::Select(key) => doc.select(&key),
                    tree::Event::Expand(key, open) => {
                        let g = key.trim_start_matches("g:").to_owned();
                        if open {
                            doc.expanded.insert(g);
                        } else {
                            doc.expanded.remove(&g);
                        }
                    }
                    _ => {}
                }
            }
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::same(14)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| form_view(ui, doc, acts));
        });
}

fn form_view(ui: &mut egui::Ui, doc: &mut EditorDoc, acts: &mut Vec<Act>) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut doc.new_name)
                .hint_text(tr("Title of a new policy or group"))
                .desired_width(280.0),
        );
        if ui.button(tr("New policy")).clicked() {
            doc.new_policy();
        }
        if ui.button(tr("New group")).clicked() {
            doc.new_group();
        }
    });
    ui.add_space(8.0);
    if let Some(e) = &doc.error {
        ui.colored_label(style::palette(ui).warn, e);
        ui.add_space(6.0);
    }
    let Some(form) = &mut doc.form else {
        ui.weak(tr("Select a policy in the tree to edit it, or a group to add one to it."));
        return;
    };
    let mut apply = false;
    ui.label(style::strong(ui, &form.title, 18.0));
    crate::views::id_line(ui, &form.id);
    ui.add_space(6.0);
    egui::Grid::new(("catalog-editor-form", form.id.as_str()))
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("Title"));
            ui.add(egui::TextEdit::singleline(&mut form.title).desired_width(520.0));
            ui.end_row();
            ui.label(tr("Description"));
            ui.add(
                egui::TextEdit::multiline(&mut form.summary)
                    .desired_rows(4)
                    .desired_width(520.0),
            );
            ui.end_row();
            ui.label(tr("Applies to"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut form.user, false, tr("the computer"));
                ui.radio_value(&mut form.user, true, tr("the default user profile"));
            });
            ui.end_row();
        });
    ui.add_space(8.0);
    style::section_title(ui, tr("What it writes"));
    ui.add_space(4.0);
    let mut gone = None;
    egui::Grid::new(("catalog-editor-actions", form.id.as_str()))
        .num_columns(6)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            for (i, row) in form.actions.iter_mut().enumerate() {
                if row.keep.is_some() {
                    ui.label(tr("kept"));
                    ui.monospace(&row.name);
                    ui.label("");
                    ui.label("");
                    ui.label("");
                } else {
                    ui.checkbox(&mut row.remove, tr("remove"));
                    ui.add(
                        egui::TextEdit::singleline(&mut row.key)
                            .hint_text(tr("Key"))
                            .desired_width(300.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut row.name)
                            .hint_text(tr("Name"))
                            .desired_width(140.0),
                    );
                    if row.remove {
                        ui.label("");
                        ui.label("");
                    } else {
                        egui::ComboBox::from_id_salt(("catalog-editor-kind", form.id.as_str(), i))
                            .selected_text(KINDS.iter().find(|(k, _)| *k == row.kind).map_or("", |(_, n)| *n))
                            .show_ui(ui, |ui| {
                                for (k, n) in KINDS {
                                    ui.selectable_value(&mut row.kind, k, n);
                                }
                            });
                        ui.add(
                            egui::TextEdit::singleline(&mut row.value)
                                .hint_text(tr("Value"))
                                .desired_width(160.0),
                        );
                    }
                }
                if ui.small_button("\u{2715}").on_hover_text(tr("Remove the line")).clicked() {
                    gone = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = gone {
        form.actions.remove(i);
    }
    if ui.button(tr("Add a value")).clicked() {
        form.actions.push(ActionRow {
            remove: false,
            key: "HKEY_LOCAL_MACHINE\\Software\\Policies\\".to_owned(),
            name: String::new(),
            kind: RegKind::DWord,
            value: "1".to_owned(),
            keep: None,
        });
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui
            .button(tr("Apply"))
            .on_hover_text(tr("Puts the form into the file after a check of the whole file"))
            .clicked()
        {
            apply = true;
        }
        if ui.button(tr("Remove the policy")).clicked() {
            acts.push(Act::AskCatalogEditRemove(form.id.clone()));
        }
    });
    if apply {
        doc.apply();
    }
}
