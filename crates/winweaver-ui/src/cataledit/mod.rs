//! The editor of a catalog file: its groups and policies as a tree on the left, the selected one
//! as a form on the right. A policy is edited whole, as a rule of the built-in catalog: its texts,
//! its place in the tree and in the installation, its parameters, its actions of every kind (the
//! answer file, the registry, scripts, services, features, capabilities, apps and programs) and its
//! translations; a group its texts, its place and its translations; the file its name. A change of
//! a form reaches the file with Apply, which checks the whole file with the checks of the built-in
//! catalog (`CatalogFile::put_rule_with_texts`, `CatalogFile::put_group`), and the disk with Save;
//! a loaded import is read again after it is saved.

mod action;
mod group;
mod param;
mod rule;
mod texts;

use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui;
use indexmap::IndexMap;
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{Group, ROOT_POLICIES, Rule};
use winweaver_core::i18n::{GroupText, RuleText};

use crate::catalogview;
use crate::i18n::{tr, trf};
use crate::{Act, TreeKey, style, tree, widgets};

pub use action::ActionRow;
pub use param::ParamRow;

/// The selected policy as the form edits it.
pub struct Form {
    /// The id of the policy in the file, and the new id typed for it.
    pub id: String,
    pub new_id: String,
    /// The policy: its texts and choices are edited in place; its parameters, actions, search
    /// words and lists of rules in the fields below, which Apply puts back.
    pub rule: Rule,
    /// The search words, separated by commas.
    pub tags: String,
    /// The rules it needs and the rules it conflicts with, separated by spaces, commas or lines.
    pub requires: String,
    pub conflicts: String,
    pub params: Vec<ParamRow>,
    pub actions: Vec<ActionRow>,
    /// The translations of the policy by language code.
    pub texts: IndexMap<String, RuleText>,
}

/// The selected group as the form edits it.
pub struct GroupForm {
    pub group: Group,
    /// The new id typed for the group.
    pub new_id: String,
    /// The translations of the group by language code.
    pub texts: IndexMap<String, GroupText>,
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
    /// The form of the selected group when no policy is selected.
    pub group_form: Option<GroupForm>,
    pub expanded: HashSet<String>,
    pub error: Option<String>,
    /// The title of a new group or policy, typed in the bar of the form.
    pub new_name: String,
    /// The name of the file as the form of the file edits it, and the new id typed for it.
    pub name: String,
    pub new_file_id: String,
    /// The id the file has on the disk: a save of an import whose id changed writes it under its
    /// new name and removes the old file.
    pub saved_id: String,
    /// The rules renamed since the last save, old and new id, for the open profiles and the
    /// bookmarks at the save.
    pub renamed: Vec<(String, String)>,
    /// The language of the translations the forms show, and the languages offered: the ones of
    /// the interface and the ones the file has texts in, by code with their native names.
    pub lang: String,
    pub languages: Vec<(String, String)>,
    pub draft: ww::DraftState,
}

/// The languages a translation can be written in: the ones of the interface but English, and the
/// ones the file holds texts in.
fn languages(file: &CatalogFile) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = crate::i18n::languages(&crate::lang_dir())
        .into_iter()
        .filter(|l| l.code != "en")
        .map(|l| (l.code, l.name))
        .collect();
    for code in file.texts.keys() {
        if !out.iter().any(|(c, _)| c == code) {
            out.push((code.clone(), code.clone()));
        }
    }
    out
}

impl Form {
    fn of(rule: &Rule, file: &CatalogFile) -> Self {
        let texts = file
            .texts
            .iter()
            .filter_map(|(code, lang)| Some((code.clone(), lang.rules.get(&rule.id)?.clone())))
            .collect();
        Self {
            id: rule.id.clone(),
            new_id: rule.id.clone(),
            tags: rule.tags.join(", "),
            requires: rule.requires.join(" "),
            conflicts: rule.conflicts.join(" "),
            params: rule
                .params
                .iter()
                .map(|(name, p)| ParamRow::of(name, p))
                .collect(),
            actions: rule.actions.iter().map(ActionRow::of).collect(),
            rule: rule.clone(),
            texts,
        }
    }

    /// The policy of the form, or why it cannot be one.
    fn rule(&self) -> Result<Rule, String> {
        let mut rule = self.rule.clone();
        for text in [
            &mut rule.title,
            &mut rule.summary,
            &mut rule.effect,
            &mut rule.risk,
            &mut rule.versions,
            &mut rule.rollback,
            &mut rule.verify,
            &mut rule.note,
        ] {
            *text = text.trim().to_owned();
        }
        if rule.title.is_empty() {
            return Err(tr("A policy needs a title").to_owned());
        }
        rule.tags = words(&self.tags, &[',']);
        rule.requires = words(&self.requires, &[',', ' ', '\n']);
        rule.conflicts = words(&self.conflicts, &[',', ' ', '\n']);
        let mut params = IndexMap::new();
        for row in &self.params {
            let (name, param) = row.param()?;
            if params.insert(name.clone(), param).is_some() {
                return Err(trf("Two parameters are named {name}", &[("name", &name)]));
            }
        }
        rule.params = params;
        rule.actions = self
            .actions
            .iter()
            .map(|a| a.action(rule.phase))
            .collect::<Result<_, _>>()?;
        Ok(rule)
    }

    /// The translations of the form without the texts left empty.
    fn clean_texts(&self) -> IndexMap<String, RuleText> {
        self.texts
            .iter()
            .map(|(code, t)| (code.clone(), texts::clean_rule(t)))
            .filter(|(_, t)| !t.is_empty())
            .collect()
    }
}

/// The words of a text split at the separators, trimmed, without the empty ones.
fn words(text: &str, separators: &[char]) -> Vec<String> {
    text.split(separators)
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

impl EditorDoc {
    pub fn new(file: CatalogFile, path: PathBuf, import: bool) -> Self {
        let expanded = file.groups.iter().map(|g| g.id.clone()).collect();
        let draft = ww::DraftState::new(Some(&path));
        let languages = languages(&file);
        let current = crate::i18n::current();
        let lang = languages
            .iter()
            .find(|(c, _)| *c == current)
            .or_else(|| languages.first())
            .map(|(c, _)| c.clone())
            .unwrap_or_default();
        Self {
            name: file.name.clone(),
            new_file_id: file.id.clone(),
            saved_id: file.id.clone(),
            renamed: Vec::new(),
            file,
            path,
            import,
            dirty: false,
            group: None,
            form: None,
            group_form: None,
            expanded,
            error: None,
            new_name: String::new(),
            lang,
            languages,
            draft,
        }
    }

    pub fn title(&self) -> String {
        let star = if self.dirty { "*" } else { "" };
        trf("{name}: editor", &[("name", &self.file.name)]) + star
    }

    /// Selects a group or a policy, whose form starts from the file.
    pub fn select(&mut self, key: &TreeKey) {
        self.error = None;
        match key {
            TreeKey::Group(g) => {
                self.group = Some(g.clone());
                self.form = None;
                self.group_form = self.file.groups.iter().find(|x| x.id == *g).map(|group| {
                    let texts = self
                        .file
                        .texts
                        .iter()
                        .filter_map(|(code, lang)| {
                            Some((code.clone(), lang.groups.get(&group.id)?.clone()))
                        })
                        .collect();
                    GroupForm {
                        group: group.clone(),
                        new_id: group.id.clone(),
                        texts,
                    }
                });
            }
            TreeKey::Rule(id) => {
                let Some(rule) = self.file.rules.iter().find(|r| r.id == *id) else {
                    return;
                };
                self.group = Some(rule.group.clone());
                self.group_form = None;
                self.form = Some(Form::of(rule, &self.file));
            }
        }
    }

    /// Puts the form in front into the file; the whole file is checked, and an error leaves it
    /// unchanged.
    pub fn apply(&mut self) {
        let result = if let Some(form) = &self.form {
            form.rule()
                .and_then(|rule| self.file.put_rule_with_texts(rule, form.clean_texts()))
        } else if let Some(form) = &self.group_form {
            let mut group = form.group.clone();
            group.title = group.title.trim().to_owned();
            group.summary = group.summary.trim().to_owned();
            if group.title.is_empty() {
                Err(tr("A group needs a title").to_owned())
            } else {
                let texts = form
                    .texts
                    .iter()
                    .map(|(code, t)| (code.clone(), texts::clean_group(t)))
                    .collect();
                self.file.put_group(group, texts)
            }
        } else {
            let name = self.name.trim();
            if name.is_empty() {
                Err(tr("A catalog needs a name").to_owned())
            } else {
                self.file.name = name.to_owned();
                Ok(())
            }
        };
        match result {
            Ok(()) => {
                self.dirty = true;
                self.error = None;
                // The form starts again from the file, as the check left it.
                let key = match (&self.form, &self.group_form) {
                    (Some(f), _) => Some(TreeKey::Rule(f.id.clone())),
                    (None, Some(g)) => Some(TreeKey::Group(g.group.id.clone())),
                    _ => None,
                };
                if let Some(key) = key {
                    self.select(&key);
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Gives the selected policy (and its Disabled rule) the id typed in its form, after the
    /// form is applied: the file changes at once, and the open profiles and the bookmarks at the
    /// save.
    pub fn rename_rule(&mut self) {
        let Some(form) = &self.form else {
            return;
        };
        let (old, new) = (form.id.clone(), form.new_id.trim().to_owned());
        self.apply();
        if self.error.is_some() {
            return;
        }
        match self.file.rename_rule(&old, &new) {
            Ok(pairs) => {
                let selected = pairs
                    .iter()
                    .find(|(o, _)| *o == old)
                    .map_or(old, |(_, n)| n.clone());
                self.note_renames(&pairs);
                self.select(&TreeKey::Rule(selected));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Gives the selected group the id typed in its form, after the form is applied.
    pub fn rename_group(&mut self) {
        let Some(form) = &self.group_form else {
            return;
        };
        let (old, new) = (form.group.id.clone(), form.new_id.trim().to_owned());
        self.apply();
        if self.error.is_some() {
            return;
        }
        match self.file.rename_group(&old, &new) {
            Ok(()) => {
                self.dirty |= old != new;
                if self.expanded.remove(&old) {
                    self.expanded.insert(new.clone());
                }
                self.select(&TreeKey::Group(new));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Gives the file the id typed in its form, and its policies and groups named after the old
    /// one the new one.
    pub fn rename_file(&mut self) {
        let new = self.new_file_id.trim().to_owned();
        match self.file.rename_id(&new) {
            Ok(pairs) => {
                self.dirty |= self.file.id != self.saved_id || !pairs.is_empty();
                self.note_renames(&pairs);
                self.expanded = self.file.groups.iter().map(|g| g.id.clone()).collect();
                self.form = None;
                self.group_form = None;
                self.group = None;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Adds renames to the ones since the last save: a rule renamed twice is one rename from its
    /// first id, and a rule named back is no rename.
    fn note_renames(&mut self, pairs: &[(String, String)]) {
        for (old, new) in pairs {
            self.dirty = true;
            match self.renamed.iter_mut().find(|(_, n)| n == old) {
                Some(pair) => pair.1.clone_from(new),
                None => self.renamed.push((old.clone(), new.clone())),
            }
        }
        self.renamed.retain(|(o, n)| o != n);
    }

    /// A new policy in the selected group (or the first one) with the typed title.
    pub fn new_policy(&mut self) {
        let Some(group) = self
            .group
            .clone()
            .or_else(|| self.file.groups.first().map(|g| g.id.clone()))
        else {
            return;
        };
        let name = self.new_name.trim().to_owned();
        if name.is_empty() {
            self.error = Some(tr("Type the title of the new policy first").to_owned());
            return;
        }
        let result = self.file.new_policy(&group, &name).and_then(|rule| {
            let id = rule.id.clone();
            self.file.put_rule(rule).map(|()| id)
        });
        match result {
            Ok(id) => {
                self.dirty = true;
                self.new_name.clear();
                self.expanded.insert(group);
                self.select(&TreeKey::Rule(id));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// A new group under the selected group (or the top one of the file) with the typed title.
    pub fn new_group(&mut self) {
        let Some(parent) = self
            .group
            .clone()
            .or_else(|| self.file.groups.first().map(|g| g.id.clone()))
        else {
            return;
        };
        match self.file.new_group(&parent, &self.new_name) {
            Ok(id) => {
                self.dirty = true;
                self.new_name.clear();
                self.expanded.insert(parent);
                self.select(&TreeKey::Group(id));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Removes a group with every group and policy under it; the group above it is selected.
    pub fn remove_group(&mut self, id: &str) {
        let parent = self
            .file
            .groups
            .iter()
            .find(|g| g.id == id)
            .and_then(|g| g.parent.clone());
        match self.file.remove_group(id) {
            Ok(_) => {
                self.dirty = true;
                self.form = None;
                self.group_form = None;
                self.group = parent.filter(|p| self.file.groups.iter().any(|g| g.id == *p));
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

/// The tab: the tree on the left, the form on the right.
pub fn editor_view(ui: &mut egui::Ui, doc: &mut EditorDoc, acts: &mut Vec<Act>) {
    let p = style::palette(ui);
    widgets::side_panel(ui, "catalog-editor-tree", 360.0, |ui| {
        let mut out = Vec::new();
        let style = catalogview::RowStyle {
            lang: "",
            filter: None,
            counts: false,
            mute_off: true,
        };
        // Every group whose parent is outside the file: under the policies, the installation
        // or the rest.
        let tops: Vec<&Group> = doc
            .file
            .groups
            .iter()
            .filter(|g| {
                !doc.file
                    .groups
                    .iter()
                    .any(|x| g.parent.as_deref() == Some(x.id.as_str()))
            })
            .collect();
        for g in tops {
            catalogview::catalog_rows(&mut out, &doc.file, &doc.expanded, &style, g, 0);
        }
        let selected_key = match (&doc.form, &doc.group) {
            (Some(f), _) => Some(TreeKey::Rule(f.id.clone())),
            (None, Some(g)) => Some(TreeKey::Group(g.clone())),
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
                tree::Event::Expand(TreeKey::Group(g), open) => {
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
        .frame(
            egui::Frame::new()
                .fill(p.bg)
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| form_view(ui, doc, acts));
        });
}

fn form_view(ui: &mut egui::Ui, doc: &mut EditorDoc, acts: &mut Vec<Act>) {
    ui.horizontal_wrapped(|ui| {
        let width = (ui.available_width() * 0.5).clamp(280.0, 560.0);
        ui.add(
            egui::TextEdit::singleline(&mut doc.new_name)
                .hint_text(tr("Title of a new policy or group"))
                .desired_width(width),
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
    let mut apply = false;
    if doc.form.is_some() {
        rule::rule_form(ui, doc, &mut apply, acts);
    } else if doc.group_form.is_some() {
        group::group_form(ui, doc, &mut apply, acts);
    } else {
        file_form(ui, doc, &mut apply);
    }
    if apply {
        doc.apply();
    }
}

/// The form of the file, when nothing is selected: its name.
fn file_form(ui: &mut egui::Ui, doc: &mut EditorDoc, apply: &mut bool) {
    let mut rename = false;
    ui.label(style::strong(ui, &doc.file.name, 18.0));
    crate::views::id_line(ui, &doc.file.id);
    ui.add_space(6.0);
    egui::Grid::new("catalog-editor-file")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("Name"));
            ui.add(egui::TextEdit::singleline(&mut doc.name).desired_width(f32::INFINITY));
            ui.end_row();
            let id = doc.file.id.clone();
            if id_row(ui, &mut doc.new_file_id, &id) {
                rename = true;
            }
        });
    ui.weak(tr(
        "A new id of the catalog renames its file at the save and every policy and group whose id starts with admx. and the old id. The open profiles and the bookmarks follow the renamed policies at the save; a profile file that is not open keeps the old ids.",
    ));
    ui.add_space(8.0);
    ui.weak(tr(
        "Select a policy in the tree to edit it, or a group to add one to it.",
    ));
    ui.add_space(8.0);
    if ui.button(tr("Apply")).clicked() {
        *apply = true;
    }
    if rename {
        doc.rename_file();
    }
}

/// The line of the grid with the id of a policy, a group or the file and the button that gives it
/// the id typed; whether the button was pressed (or Enter in the field).
fn id_row(ui: &mut egui::Ui, new_id: &mut String, id: &str) -> bool {
    ui.label(tr("Id"));
    let mut pressed = false;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 110.0).max(200.0);
        let field = ui.add(
            egui::TextEdit::singleline(new_id)
                .font(egui::TextStyle::Monospace)
                .desired_width(width),
        );
        let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let changed = new_id.trim() != id;
        let button = ui
            .add_enabled(changed, egui::Button::new(tr("Rename")))
            .on_hover_text(tr("Renames it in the whole catalog: the rules that name it and its translations follow"));
        pressed = changed && (enter || button.clicked());
    });
    ui.end_row();
    pressed
}

/// The buttons at the end of a form: Apply, and the removal of what the form edits.
fn form_buttons(
    ui: &mut egui::Ui,
    apply: &mut bool,
    remove: Option<(&str, Act)>,
    acts: &mut Vec<Act>,
) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui
            .button(tr("Apply"))
            .on_hover_text(tr(
                "Puts the form into the file after a check of the whole file",
            ))
            .clicked()
        {
            *apply = true;
        }
        if let Some((label, act)) = remove
            && ui.button(label).clicked()
        {
            acts.push(act);
        }
    });
}

/// Whether a group sits right under a root of the tree (the top group of the file under the
/// policies stays).
fn is_top(group: &Group) -> bool {
    group.parent.as_deref() == Some(ROOT_POLICIES)
}
