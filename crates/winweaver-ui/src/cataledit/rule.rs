//! The form of a policy: its texts, its place, its parameters, its actions and its translations.

use eframe::egui;
use winweaver_core::catalog::{Level, Param, Phase};

use super::{EditorDoc, action, form_buttons, param, texts};
use crate::i18n::{tr, trf};
use crate::{Act, CatalogAct, ProfileAct, style};

/// The phases of a rule in their order, for the choice of the form.
const PHASES: [Phase; 7] = [
    Phase::Windowspe,
    Phase::SpecializeXml,
    Phase::Specialize,
    Phase::OobeXml,
    Phase::DefaultUser,
    Phase::UserFirstLogon,
    Phase::PostOobe,
];

const LEVELS: [Level; 4] = [
    Level::Optional,
    Level::Baseline,
    Level::Recommended,
    Level::Risky,
];

/// The phase of a rule in words: when its actions run and what carries them.
pub(super) fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Windowspe => tr("windowsPE: a setting or a command of the answer file"),
        Phase::SpecializeXml => tr("specialize: a setting or a command of the answer file"),
        Phase::Specialize => tr("specialize: the script of the installation"),
        Phase::OobeXml => tr("oobeSystem: a setting of the answer file"),
        Phase::DefaultUser => tr("The default user profile"),
        Phase::UserFirstLogon => tr("The first sign-in of every user"),
        Phase::PostOobe => tr("After OOBE, as SYSTEM"),
    }
}

/// A line of the grid of a form: a label and a text that takes the width of the panel.
pub(super) fn text_row(ui: &mut egui::Ui, label: &str, text: &mut String, rows: usize) {
    ui.label(label);
    let edit = if rows > 1 {
        egui::TextEdit::multiline(text).desired_rows(rows)
    } else {
        egui::TextEdit::singleline(text)
    };
    ui.add(edit.desired_width(f32::INFINITY));
    ui.end_row();
}

pub(super) fn rule_form(
    ui: &mut egui::Ui,
    doc: &mut EditorDoc,
    apply: &mut bool,
    acts: &mut Vec<Act>,
) {
    let import = doc
        .import
        .then(|| (doc.file.id.clone(), doc.file.name.clone()));
    let groups: Vec<(String, String)> = doc
        .file
        .groups
        .iter()
        .map(|g| (g.id.clone(), g.title.clone()))
        .collect();
    let languages = doc.languages.clone();
    let Some(form) = &mut doc.form else {
        return;
    };
    ui.label(style::strong(ui, &form.rule.title, 18.0));
    crate::views::id_line(ui, &form.id);
    if let Some((id, name)) = import {
        ui.horizontal_wrapped(|ui| {
            let text = trf("Show in the catalog {name}", &[("name", &name)]);
            if ui.link(text).clicked() {
                acts.push(Act::Catalog(CatalogAct::OpenAt(
                    id.clone(),
                    form.id.clone(),
                )));
            }
            if ui.link(tr("Show in the profile")).clicked() {
                acts.push(Act::Profile(ProfileAct::ShowInProfile(
                    id.clone(),
                    form.id.clone(),
                )));
            }
        });
    }
    if form.rule.runs_code() {
        ui.add_space(4.0);
        ui.colored_label(
            style::palette(ui).warn,
            tr("The policy runs code of its own: an import of the file asks first."),
        );
    }
    ui.add_space(6.0);
    let id = form.id.clone();
    egui::Grid::new(("catalog-editor-form", id.as_str()))
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            text_row(ui, tr("Title"), &mut form.rule.title, 1);
            text_row(ui, tr("Description"), &mut form.rule.summary, 4);
            ui.label(tr("Group"));
            let shown = groups
                .iter()
                .find(|(g, _)| *g == form.rule.group)
                .map_or(form.rule.group.as_str(), |(_, t)| t.as_str());
            egui::ComboBox::from_id_salt(("catalog-editor-group", id.as_str()))
                .selected_text(shown)
                .width(320.0)
                .show_ui(ui, |ui| {
                    for (g, title) in &groups {
                        ui.selectable_value(&mut form.rule.group, g.clone(), title);
                    }
                });
            ui.end_row();
            ui.label(tr("Runs"));
            egui::ComboBox::from_id_salt(("catalog-editor-phase", id.as_str()))
                .selected_text(phase_label(form.rule.phase))
                .width(320.0)
                .show_ui(ui, |ui| {
                    for phase in PHASES {
                        ui.selectable_value(&mut form.rule.phase, phase, phase_label(phase));
                    }
                });
            ui.end_row();
            ui.label(tr("Level"));
            egui::ComboBox::from_id_salt(("catalog-editor-level", id.as_str()))
                .selected_text(tr(crate::views::level_title(form.rule.level)))
                .show_ui(ui, |ui| {
                    for level in LEVELS {
                        let title = tr(crate::views::level_title(level));
                        ui.selectable_value(&mut form.rule.level, level, title);
                    }
                });
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut form.rule.default, tr("on by default"));
            ui.end_row();
        });
    ui.add_space(4.0);
    egui::CollapsingHeader::new(tr("Details"))
        .id_salt(("catalog-editor-details", id.as_str()))
        .show(ui, |ui| {
            egui::Grid::new(("catalog-editor-details-grid", id.as_str()))
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    text_row(ui, tr("What it does"), &mut form.rule.effect, 3);
                    text_row(ui, tr("Risk"), &mut form.rule.risk, 2);
                    text_row(ui, tr("Versions"), &mut form.rule.versions, 1);
                    text_row(ui, tr("How to check"), &mut form.rule.verify, 2);
                    text_row(ui, tr("Rollback"), &mut form.rule.rollback, 2);
                    text_row(ui, tr("Note"), &mut form.rule.note, 2);
                    text_row(ui, tr("Search words"), &mut form.tags, 1);
                    text_row(ui, tr("Needs"), &mut form.requires, 1);
                    text_row(ui, tr("Conflicts with"), &mut form.conflicts, 1);
                });
            ui.weak(tr(
                "Search words are separated by commas; the rules it needs and the ones it conflicts with are ids of this file or of the built-in catalog, separated by spaces.",
            ));
        });
    ui.add_space(4.0);
    egui::CollapsingHeader::new(trf("Parameters ({n})", &[("n", &form.params.len())]))
        .id_salt(("catalog-editor-params", id.as_str()))
        .default_open(!form.params.is_empty())
        .show(ui, |ui| param::params_ui(ui, &id, &mut form.params));
    ui.add_space(4.0);
    let lists: Vec<String> = form
        .params
        .iter()
        .filter(|p| matches!(p.param, Param::List { .. }))
        .map(|p| p.name.trim().to_owned())
        .collect();
    egui::CollapsingHeader::new(trf("Actions ({n})", &[("n", &form.actions.len())]))
        .id_salt(("catalog-editor-actions", id.as_str()))
        .default_open(true)
        .show(ui, |ui| {
            action::actions_ui(ui, &id, &lists, &mut form.actions, acts);
        });
    ui.add_space(4.0);
    egui::CollapsingHeader::new(tr("Translations"))
        .id_salt(("catalog-editor-texts", id.as_str()))
        .show(ui, |ui| {
            texts::rule_texts_ui(ui, &languages, &mut doc.lang, form);
        });
    let remove = (
        tr("Remove the policy"),
        crate::question::remove_policy(form.id.clone()).into(),
    );
    form_buttons(ui, apply, Some(remove), acts);
}
