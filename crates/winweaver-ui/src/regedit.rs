//! The fields of a registry value shared by the editors of the registry tab and of a registry
//! file: the type and the data, with the hint and the lines the type wants, and the width of
//! the windows over a tab that edit a value, a key or an entry.

use eframe::egui;

use crate::i18n::tr;

/// Whether the data of a type is edited on several lines.
pub fn multiline(type_name: &str) -> bool {
    matches!(type_name, "REG_MULTI_SZ" | "REG_BINARY" | "REG_NONE")
}

/// The hint of the data field of a type.
pub fn hint(type_name: &str) -> &'static str {
    match type_name {
        "REG_DWORD" | "REG_QWORD" => tr("A number: decimal, or hexadecimal after 0x"),
        "REG_MULTI_SZ" => tr("One string per line"),
        "REG_BINARY" | "REG_NONE" => tr("Bytes as hexadecimal pairs"),
        _ => "",
    }
}

/// The width a window over the tab opens with: half of the window for data of one line, three
/// quarters for data of several lines (a list of strings, bytes), never wider than the window
/// and never narrower than 560 points where the window allows it. The window keeps the size
/// the user drags it to.
pub fn modal_width(ctx: &egui::Context, long: bool) -> f32 {
    let room = (ctx.content_rect().width() - 80.0).max(400.0);
    let wanted = if long { room * 0.75 } else { room * 0.5 };
    wanted.clamp(560.0_f32.min(room), room)
}

/// The rows Type and Data of a two-column grid: the type as a list of `types` when it may
/// change, else as text; the data on one line or several, by the type, in the width of the
/// window.
pub fn type_and_data(
    ui: &mut egui::Ui,
    salt: &str,
    types: Option<&[&str]>,
    type_name: &mut String,
    text: &mut String,
) {
    ui.label(tr("Type"));
    match types {
        Some(types) => {
            egui::ComboBox::from_id_salt(salt)
                .selected_text(type_name.as_str())
                .show_ui(ui, |ui| {
                    for t in types {
                        ui.selectable_value(type_name, (*t).to_owned(), *t);
                    }
                });
        }
        None => {
            ui.monospace(type_name.as_str());
        }
    }
    ui.end_row();
    ui.label(tr("Data"));
    let hint = hint(type_name);
    let field = if multiline(type_name) {
        egui::TextEdit::multiline(text).desired_rows(8)
    } else {
        egui::TextEdit::singleline(text)
    };
    ui.add(
        field
            .desired_width(f32::INFINITY)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace),
    );
    ui.end_row();
}
