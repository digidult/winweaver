//! The tab of an import of policy templates (`docs/spec/01-ui.md` section 8): its tree with the
//! rows of a profile but without check boxes, the detail of a policy, its parameters and registry
//! values, and a button that shows the policy in the tree of a profile.

use std::collections::HashSet;

use eframe::egui::{self, RichText};
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{Group, Rule};

use crate::Act;
use crate::i18n::{tr, trf};

/// An open tab of an import.
pub struct CatalogDoc {
    /// The id of the import (its catalog file).
    pub id: String,
    /// The name of the import when the tab was opened.
    pub name: String,
    /// The selected rule.
    pub sel: Option<String>,
    pub expanded: HashSet<String>,
}

impl CatalogDoc {
    pub fn new(file: &CatalogFile) -> Self {
        Self {
            id: file.id.clone(),
            name: file.name.clone(),
            sel: None,
            expanded: roots(file).map(|g| g.id.clone()).collect(),
        }
    }
}

/// The groups of a file whose parent is outside it.
fn roots(file: &CatalogFile) -> impl Iterator<Item = &Group> {
    file.groups.iter().filter(|g| {
        !file
            .groups
            .iter()
            .any(|p| g.parent.as_deref() == Some(p.id.as_str()))
    })
}

fn group_title<'a>(file: &'a CatalogFile, lang: &str, group: &'a Group) -> &'a str {
    file.texts
        .get(lang)
        .and_then(|t| t.groups.get(&group.id))
        .map_or(group.title.as_str(), String::as_str)
}

fn rule_texts<'a>(file: &'a CatalogFile, lang: &str, rule: &'a Rule) -> (&'a str, &'a str) {
    match file.texts.get(lang).and_then(|t| t.rules.get(&rule.id)) {
        Some((title, summary)) => (title.as_str(), summary.as_str()),
        None => (rule.title.as_str(), rule.summary.as_str()),
    }
}

/// The tab: the tree on the left, the selected policy on the right.
pub fn catalog_view(ui: &mut egui::Ui, file: &CatalogFile, doc: &CatalogDoc, acts: &mut Vec<Act>) {
    let lang = crate::i18n::current();
    egui::Panel::left("catalog-tree")
        .resizable(true)
        .default_size(380.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    for root in roots(file) {
                        group_rows(ui, file, &lang, doc, root, acts);
                    }
                });
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| detail(ui, file, &lang, doc, acts));
    });
}

fn group_rows(
    ui: &mut egui::Ui,
    file: &CatalogFile,
    lang: &str,
    doc: &CatalogDoc,
    group: &Group,
    acts: &mut Vec<Act>,
) {
    let open = doc.expanded.contains(&group.id);
    let rules: Vec<&Rule> = file.rules.iter().filter(|r| r.group == group.id).collect();
    ui.horizontal(|ui| {
        let twisty = if open { "v" } else { ">" };
        if ui.small_button(twisty).clicked() {
            acts.push(Act::CatalogExpand(group.id.clone(), !open));
        }
        ui.label(group_title(file, lang, group));
        ui.weak(rules.len().to_string());
    });
    if !open {
        return;
    }
    ui.indent(&group.id, |ui| {
        for child in file
            .groups
            .iter()
            .filter(|g| g.parent.as_deref() == Some(group.id.as_str()))
        {
            group_rows(ui, file, lang, doc, child, acts);
        }
        for rule in rules {
            let selected = doc.sel.as_deref() == Some(rule.id.as_str());
            let (title, _) = rule_texts(file, lang, rule);
            if ui.selectable_label(selected, title).clicked() {
                acts.push(Act::CatalogSelect(rule.id.clone()));
            }
        }
    });
}

fn detail(ui: &mut egui::Ui, file: &CatalogFile, lang: &str, doc: &CatalogDoc, acts: &mut Vec<Act>) {
    let Some(rule) = doc
        .sel
        .as_deref()
        .and_then(|id| file.rules.iter().find(|r| r.id == id))
    else {
        ui.heading(&file.name);
        ui.weak(trf(
            "{rules} rules, {skipped} skipped",
            &[("rules", &file.rules.len()), ("skipped", &file.skipped.len())],
        ));
        ui.add_space(6.0);
        ui.label(tr("Select a policy in the tree."));
        return;
    };
    let (title, summary) = rule_texts(file, lang, rule);
    ui.label(RichText::new(title).size(18.0).strong());
    ui.monospace(&rule.id);
    ui.add_space(6.0);
    if ui.button(tr("Show in the profile tree")).clicked() {
        acts.push(Act::ShowInProfile(file.id.clone(), rule.id.clone()));
    }
    if !summary.is_empty() {
        ui.add_space(6.0);
        ui.label(summary);
    }
    if !rule.params.is_empty() {
        ui.add_space(6.0);
        ui.strong(tr("Parameters"));
        for (name, param) in &rule.params {
            ui.label(format!("{name}: {}", param.title()));
        }
    }
    ui.add_space(6.0);
    ui.strong(tr("What it writes"));
    for action in &rule.actions {
        ui.monospace(crate::views::describe(action));
    }
}
