//! The diff tab: the differences between two profiles side by side (`docs/spec/01-ui.md`
//! section 7, as a list grouped by kind for now).

use std::path::PathBuf;

use eframe::egui::{self, RichText};
use serde_json::Value;
use winweaver_core::catalog::Catalog;
use winweaver_core::diff::{self, Diff};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::profile::Choice;

use crate::views::rtitle;
use crate::i18n::{tr, trf};
use crate::{Act, Sel};

pub struct DiffDoc {
    pub diff: Diff,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

impl DiffDoc {
    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => format!(
                "{} vs {}.{}",
                self.diff.base.name,
                self.diff.other.name,
                diff::EXTENSION
            ),
        }
    }
}

/// The state of a rule in one profile as text.
fn choice_text(choice: &Option<Choice>) -> String {
    match choice {
        None => tr("off").to_owned(),
        Some(c) => {
            let mut text = if c.on { tr("on") } else { tr("off") }.to_owned();
            for (name, value) in &c.params {
                text.push_str(&format!(", {name} = {value}"));
            }
            text
        }
    }
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|v| match v {
                Value::Object(o) => o
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_owned(),
                other => value_text(other),
            })
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

pub fn diff_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &DiffDoc,
    acts: &mut Vec<Act>,
) {
    let d = &doc.diff;
    ui.heading(format!("{} vs {}", d.base.name, d.other.name));
    ui.weak(trf(
        "{n} differences; left: {left}, right: {right}",
        &[
            ("n", &d.count()),
            ("left", &if d.base.path.is_empty() { tr("not saved") } else { d.base.path.as_str() }),
            ("right", &if d.other.path.is_empty() { tr("not saved") } else { d.other.path.as_str() }),
        ],
    ));
    ui.horizontal(|ui| {
        if ui
            .button(tr("Apply the right side to the open profile"))
            .on_hover_text(tr("Sets every difference to the value of the right side in the profile tab the diff was made from"))
            .clicked()
        {
            acts.push(Act::ApplyDiff);
        }
        if ui.button(tr("Save as a diff file...")).clicked() {
            acts.push(Act::SaveDiffAs);
        }
    });
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Grid::new("diff-grid")
                .num_columns(3)
                .striped(true)
                .spacing([18.0, 4.0])
                .show(ui, |ui| {
                    ui.strong(tr("Item"));
                    ui.strong(&d.base.name);
                    ui.strong(&d.other.name);
                    ui.end_row();
                    for (k, f) in &d.install {
                        ui.label(trf("Installation: {k}", &[("k", k)]));
                        ui.label(value_text(&f.base));
                        ui.label(value_text(&f.other));
                        ui.end_row();
                    }
                    for (k, f) in &d.languages {
                        ui.label(trf("Languages: {k}", &[("k", k)]));
                        ui.label(value_text(&f.base));
                        ui.label(value_text(&f.other));
                        ui.end_row();
                    }
                    if let Some(f) = &d.accounts {
                        ui.label(tr("Accounts"));
                        ui.label(value_text(&f.base));
                        ui.label(value_text(&f.other));
                        ui.end_row();
                    }
                    for rule in catalog.rules() {
                        let Some(r) = d.rules.get(&rule.id) else {
                            continue;
                        };
                        if ui.link(rtitle(texts, rule)).clicked() {
                            acts.push(Act::Select(Sel::Rule(rule.id.clone())));
                        }
                        ui.label(choice_text(&r.base));
                        ui.label(RichText::new(choice_text(&r.other)).strong());
                        ui.end_row();
                    }
                    for (id, r) in &d.rules {
                        if catalog.rule(id).is_some() {
                            continue;
                        }
                        ui.weak(trf("{id} (not in this catalog)", &[("id", id)]));
                        ui.label(choice_text(&r.base));
                        ui.label(choice_text(&r.other));
                        ui.end_row();
                    }
                });
        });
}
