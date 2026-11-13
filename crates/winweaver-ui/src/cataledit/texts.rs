//! The translations of a policy and of a group in the form: one language at a time, every text the
//! translation files of the built-in catalog hold, with the English text as the hint of an empty
//! field (an empty field is the English text).

use eframe::egui;
use winweaver_core::catalog::Param;
use winweaver_core::i18n::{GroupText, RuleText, value_key};

use super::param::value_of;
use super::{Form, GroupForm};
use crate::i18n::tr;

/// The choice of the language of the translations.
fn language_choice(
    ui: &mut egui::Ui,
    salt: &str,
    languages: &[(String, String)],
    lang: &mut String,
) {
    ui.horizontal(|ui| {
        ui.label(tr("Language"));
        let shown = languages
            .iter()
            .find(|(c, _)| c == lang)
            .map_or(lang.clone(), |(c, n)| format!("{n} ({c})"));
        egui::ComboBox::from_id_salt(("catalog-editor-lang", salt))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (code, name) in languages {
                    ui.selectable_value(lang, code.clone(), format!("{name} ({code})"));
                }
            });
    });
}

/// A line of the grid with an optional text: empty is none, and the hint is the English text.
fn opt_line(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<String>,
    english: &str,
    rows: usize,
) {
    ui.label(label);
    let mut text = value.clone().unwrap_or_default();
    let edit = if rows > 1 {
        egui::TextEdit::multiline(&mut text).desired_rows(rows)
    } else {
        egui::TextEdit::singleline(&mut text)
    };
    let hint = english.lines().next().unwrap_or_default();
    if ui
        .add(edit.hint_text(hint).desired_width(f32::INFINITY))
        .changed()
    {
        *value = (!text.is_empty()).then_some(text);
    }
    ui.end_row();
}

/// A line of the grid with a text kept in a map under a key: empty takes the key out.
fn map_line(
    ui: &mut egui::Ui,
    label: &str,
    map: &mut indexmap::IndexMap<String, String>,
    key: &str,
    english: &str,
) {
    let mut value = map.get(key).cloned();
    opt_line(ui, label, &mut value, english, 1);
    match value {
        Some(v) => {
            map.insert(key.to_owned(), v);
        }
        None => {
            map.shift_remove(key);
        }
    }
}

pub(super) fn rule_texts_ui(
    ui: &mut egui::Ui,
    languages: &[(String, String)],
    lang: &mut String,
    form: &mut Form,
) {
    if languages.is_empty() {
        ui.weak(tr("The program has no language but English."));
        return;
    }
    language_choice(ui, &form.id, languages, lang);
    ui.add_space(4.0);
    let rule = &form.rule;
    let text = form.texts.entry(lang.clone()).or_default();
    egui::Grid::new(("catalog-editor-rule-texts", form.id.as_str(), lang.as_str()))
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            opt_line(ui, tr("Title"), &mut text.title, &rule.title, 1);
            opt_line(ui, tr("Description"), &mut text.summary, &rule.summary, 3);
            opt_line(ui, tr("What it does"), &mut text.effect, &rule.effect, 2);
            opt_line(ui, tr("Risk"), &mut text.risk, &rule.risk, 2);
            opt_line(ui, tr("Versions"), &mut text.versions, &rule.versions, 1);
            opt_line(ui, tr("How to check"), &mut text.verify, &rule.verify, 2);
            opt_line(ui, tr("Rollback"), &mut text.rollback, &rule.rollback, 2);
            opt_line(ui, tr("Note"), &mut text.note, &rule.note, 2);
            let mut tags = text.tags.as_ref().map(|t| t.join(", "));
            opt_line(ui, tr("Search words"), &mut tags, &form.tags, 1);
            text.tags = tags.map(|t| super::words(&t, &[',']));
            for row in &form.params {
                let name = row.name.trim();
                if name.is_empty() {
                    continue;
                }
                map_line(ui, name, &mut text.params, name, row.param.title());
                if let Param::Enum { values, .. } = &row.param {
                    let titles = text.values.entry(name.to_owned()).or_default();
                    for (v, value_text) in values.iter().zip(&row.values) {
                        let key = value_key(&value_of(value_text));
                        let label = format!("{name} = {key}");
                        map_line(ui, &label, titles, &key, &v.title);
                    }
                    if titles.is_empty() {
                        text.values.shift_remove(name);
                    }
                }
            }
        });
}

pub(super) fn group_texts_ui(
    ui: &mut egui::Ui,
    languages: &[(String, String)],
    lang: &mut String,
    form: &mut GroupForm,
) {
    if languages.is_empty() {
        ui.weak(tr("The program has no language but English."));
        return;
    }
    language_choice(ui, &form.group.id, languages, lang);
    ui.add_space(4.0);
    let group = &form.group;
    let text = form.texts.entry(lang.clone()).or_default();
    egui::Grid::new((
        "catalog-editor-group-texts",
        group.id.as_str(),
        lang.as_str(),
    ))
    .num_columns(2)
    .spacing([12.0, 6.0])
    .show(ui, |ui| {
        opt_line(ui, tr("Title"), &mut text.title, &group.title, 1);
        opt_line(ui, tr("Description"), &mut text.summary, &group.summary, 3);
    });
}

/// A text without its spaces at the ends, none when nothing is left.
fn clean(text: &Option<String>) -> Option<String> {
    text.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

/// The translation of a policy without the texts left empty.
pub(super) fn clean_rule(t: &RuleText) -> RuleText {
    RuleText {
        title: clean(&t.title),
        summary: clean(&t.summary),
        effect: clean(&t.effect),
        risk: clean(&t.risk),
        versions: clean(&t.versions),
        verify: clean(&t.verify),
        rollback: clean(&t.rollback),
        note: clean(&t.note),
        tags: t.tags.clone().filter(|t| !t.is_empty()),
        params: t
            .params
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .map(|(k, v)| (k.clone(), v.trim().to_owned()))
            .collect(),
        values: t
            .values
            .iter()
            .map(|(k, v)| {
                let v = v
                    .iter()
                    .filter(|(_, t)| !t.trim().is_empty())
                    .map(|(a, b)| (a.clone(), b.trim().to_owned()))
                    .collect::<indexmap::IndexMap<_, _>>();
                (k.clone(), v)
            })
            .filter(|(_, v)| !v.is_empty())
            .collect(),
    }
}

/// The translation of a group without the texts left empty.
pub(super) fn clean_group(t: &GroupText) -> GroupText {
    GroupText {
        title: clean(&t.title),
        summary: clean(&t.summary),
    }
}
