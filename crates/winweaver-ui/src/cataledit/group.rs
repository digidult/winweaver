//! The form of a group: its texts, the group or the root of the tree it sits in, and its
//! translations.

use eframe::egui;
use winweaver_core::catalog::{ROOT_OTHER, ROOT_POLICIES, ROOT_UNATTEND};

use super::rule::text_row;
use super::{EditorDoc, form_buttons, is_top, texts};
use crate::i18n::{tr, trf};
use crate::{Act, style};

/// A root of the tree in words.
fn root_title(root: &str) -> &'static str {
    match root {
        ROOT_UNATTEND => tr("Installation"),
        ROOT_POLICIES => tr("Policies"),
        _ => tr("Other"),
    }
}

pub(super) fn group_form(
    ui: &mut egui::Ui,
    doc: &mut EditorDoc,
    apply: &mut bool,
    acts: &mut Vec<Act>,
) {
    let languages = doc.languages.clone();
    let Some(form) = &mut doc.group_form else {
        return;
    };
    let id = form.group.id.clone();
    // A group goes under a root of the tree or under a group of the file but itself and the
    // groups inside it.
    let mut inside = vec![id.clone()];
    let mut i = 0;
    while i < inside.len() {
        let parent = inside[i].clone();
        inside.extend(
            doc.file
                .groups
                .iter()
                .filter(|g| g.parent.as_deref() == Some(parent.as_str()))
                .map(|g| g.id.clone()),
        );
        i += 1;
    }
    let mut parents: Vec<(String, String)> = [ROOT_UNATTEND, ROOT_POLICIES, ROOT_OTHER]
        .iter()
        .map(|r| {
            (
                (*r).to_owned(),
                trf("The root {root}", &[("root", &root_title(r))]),
            )
        })
        .collect();
    parents.extend(
        doc.file
            .groups
            .iter()
            .filter(|g| !inside.contains(&g.id))
            .map(|g| (g.id.clone(), g.title.clone())),
    );
    ui.label(style::strong(ui, &form.group.title, 18.0));
    crate::views::id_line(ui, &id);
    let mut rename = false;
    ui.weak(trf(
        "{n} policies",
        &[(
            "n",
            &doc.file.rules.iter().filter(|r| r.group == id).count(),
        )],
    ));
    ui.add_space(6.0);
    egui::Grid::new(("catalog-editor-group", id.as_str()))
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            text_row(ui, tr("Title"), &mut form.group.title, 1);
            text_row(ui, tr("Description"), &mut form.group.summary, 3);
            rename = super::id_row(ui, &mut form.new_id, &id);
            ui.label(tr("In"));
            let parent = form.group.parent.clone().unwrap_or_default();
            let shown = parents
                .iter()
                .find(|(p, _)| *p == parent)
                .map_or(parent.clone(), |(_, t)| t.clone());
            egui::ComboBox::from_id_salt(("catalog-editor-parent", id.as_str()))
                .selected_text(shown)
                .width(320.0)
                .show_ui(ui, |ui| {
                    for (p, title) in &parents {
                        ui.selectable_value(&mut form.group.parent, Some(p.clone()), title);
                    }
                });
            ui.end_row();
        });
    ui.weak(tr(
        "A policy of the answer file lives under the root Installation, one that only writes the registry under Policies, the rest under Other; a script under Installation or Other.",
    ));
    ui.add_space(4.0);
    egui::CollapsingHeader::new(tr("Translations"))
        .id_salt(("catalog-editor-group-texts", id.as_str()))
        .show(ui, |ui| {
            texts::group_texts_ui(ui, &languages, &mut doc.lang, form);
        });
    // The top group of the file under the policies stays: the file is the import.
    let remove = (!is_top(&form.group)).then(|| {
        (
            tr("Remove the group"),
            crate::question::remove_group(&form.group.title, id.clone()).into(),
        )
    });
    form_buttons(ui, apply, remove, acts);
    if rename {
        doc.rename_group();
    }
}
