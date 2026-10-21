//! The window over the tab that edits a key, a value, a comment or a line.

use super::*;

/// The editor of a key, a value or a comment, over the window.
pub(crate) fn editor(ctx: &egui::Context, doc: &mut RegFileDoc) {
    let Some(edit) = &mut doc.edit else {
        return;
    };
    let title = match (edit.kind, edit.line) {
        (EditKind::Key, None) => tr("New key"),
        (EditKind::Key, Some(_)) => tr("Edit the key"),
        (EditKind::Value, None) => tr("New value"),
        (EditKind::Value, Some(_)) => tr("Edit the value"),
        (EditKind::Comment, _) => tr("Edit the comment"),
        (EditKind::Raw, _) => tr("Edit the line"),
    };
    let mut close = false;
    let mut apply = false;
    widgets::Modal::new(title)
        .id("regfile-editor")
        .width(560.0)
        .show(ctx, |ui| {
            egui::Grid::new("regfile-edit")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| match edit.kind {
                    EditKind::Key => {
                        ui.label(tr("Key"));
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.name)
                                .desired_width(420.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut edit.delete, tr("Delete this key with its subkeys"));
                        ui.end_row();
                    }
                    EditKind::Value => value_fields(ui, edit),
                    EditKind::Comment | EditKind::Raw => {
                        ui.label(tr("Text"));
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.name)
                                .desired_width(420.0)
                                .font(egui::TextStyle::Monospace),
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

/// The fields of a value in the editor: the name, the deletion, the type and the data.
pub(crate) fn value_fields(ui: &mut egui::Ui, edit: &mut Edit) {
    ui.label(tr("Name"));
    ui.add(
        egui::TextEdit::singleline(&mut edit.name)
            .desired_width(420.0)
            .hint_text(tr("(Default)")),
    );
    ui.end_row();
    ui.label("");
    ui.checkbox(&mut edit.delete, tr("Delete this value"));
    ui.end_row();
    if edit.delete {
        return;
    }
    let types = value_types();
    regedit::type_and_data(
        ui,
        "regfile-type",
        Some(&types),
        &mut edit.type_name,
        &mut edit.text,
        420.0,
    );
}
