//! The parameters of a policy in the form: a row a parameter, its kind and the fields of the kind.

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::{EnumValue, Param, Pick};

use crate::i18n::tr;
use crate::{icons, style};

/// A parameter of the form: its name, the parameter with the fields edited in place, and the
/// fields that are values of JSON as text.
pub struct ParamRow {
    pub name: String,
    pub param: Param,
    /// The values of an enum as text, beside the titles in `param`.
    pub values: Vec<String>,
    /// The default of an enum as text, or the items of the default of a list, a line an item.
    pub default: String,
}

/// The kinds of parameters, by the name of the catalog.
const KINDS: [&str; 4] = ["enum", "int", "string", "list"];

fn kind_of(param: &Param) -> &'static str {
    match param {
        Param::Enum { .. } => "enum",
        Param::Int { .. } => "int",
        Param::Text { .. } => "string",
        Param::List { .. } => "list",
    }
}

fn kind_title(kind: &str) -> &'static str {
    match kind {
        "enum" => tr("Choice"),
        "int" => tr("Number"),
        "string" => tr("Text"),
        _ => tr("List"),
    }
}

/// A value of an enum or of a field typed as text: a whole number is a number, anything else a
/// text.
pub(super) fn value_of(text: &str) -> Value {
    let t = text.trim();
    match t.parse::<i64>() {
        Ok(n) => Value::from(n),
        Err(_) => Value::from(t),
    }
}

/// A value of JSON as the form shows it.
pub(super) fn text_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

impl ParamRow {
    pub fn of(name: &str, param: &Param) -> Self {
        let (values, default) = match param {
            Param::Enum {
                values, default, ..
            } => (
                values.iter().map(|v| text_of(&v.value)).collect(),
                text_of(default),
            ),
            Param::List { default, .. } => (Vec::new(), default.join("\n")),
            _ => (Vec::new(), String::new()),
        };
        Self {
            name: name.to_owned(),
            param: param.clone(),
            values,
            default,
        }
    }

    /// A new parameter of a kind with the title of the old one.
    fn with_kind(&self, kind: &str) -> Self {
        let title = self.param.title().to_owned();
        let param = match kind {
            "enum" => Param::Enum {
                title,
                default: Value::from(1),
                values: vec![
                    EnumValue {
                        value: Value::from(1),
                        title: tr("On").to_owned(),
                    },
                    EnumValue {
                        value: Value::from(0),
                        title: tr("Off").to_owned(),
                    },
                ],
                differs_from: None,
                same_allowed: Vec::new(),
            },
            "int" => Param::Int {
                title,
                default: 0,
                min: 0,
                max: 100,
            },
            "string" => Param::Text {
                title,
                default: String::new(),
                required: false,
                max_length: None,
                pick: None,
            },
            _ => Param::List {
                title,
                default: Vec::new(),
            },
        };
        Self::of(&self.name, &param)
    }

    /// The parameter of the row with its name, or why it cannot be one.
    pub fn param(&self) -> Result<(String, Param), String> {
        let name = self.name.trim().to_owned();
        if name.is_empty() {
            return Err(tr("A parameter needs a name").to_owned());
        }
        let mut param = self.param.clone();
        match &mut param {
            Param::Enum {
                values, default, ..
            } => {
                for (v, text) in values.iter_mut().zip(&self.values) {
                    v.value = value_of(text);
                    v.title = v.title.trim().to_owned();
                }
                *default = value_of(&self.default);
            }
            Param::List { default, .. } => {
                *default = self
                    .default
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            Param::Text { title, .. } | Param::Int { title, .. } => {
                *title = title.trim().to_owned();
            }
        }
        Ok((name, param))
    }
}

/// The rows of the parameters, with the buttons that add, move and remove them.
pub(super) fn params_ui(ui: &mut egui::Ui, id: &str, rows: &mut Vec<ParamRow>) {
    let mut gone = None;
    let mut up = None;
    let count = rows.len();
    for (i, row) in rows.iter_mut().enumerate() {
        let salt = (id, "param", i);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut row.name)
                        .hint_text(tr("Name"))
                        .desired_width(140.0),
                );
                let kind = kind_of(&row.param);
                let mut chosen = kind;
                egui::ComboBox::from_id_salt((salt, "kind"))
                    .selected_text(kind_title(kind))
                    .show_ui(ui, |ui| {
                        for k in KINDS {
                            ui.selectable_value(&mut chosen, k, kind_title(k));
                        }
                    });
                if chosen != kind {
                    *row = row.with_kind(chosen);
                }
                if i > 0 && style::icon_button(ui, icons::UP, tr("Up"), true, None).clicked() {
                    up = Some(i);
                }
                let last = i + 1 == count;
                if !last && style::icon_button(ui, icons::DOWN, tr("Down"), true, None).clicked() {
                    up = Some(i + 1);
                }
                if style::icon_button(ui, icons::CLOSE, tr("Remove the parameter"), true, None)
                    .clicked()
                {
                    gone = Some(i);
                }
            });
            fields(ui, salt, row);
        });
        ui.add_space(4.0);
    }
    if let Some(i) = gone {
        rows.remove(i);
    } else if let Some(i) = up {
        rows.swap(i - 1, i);
    }
    if ui.button(tr("Add a parameter")).clicked() {
        let n = rows.len() + 1;
        let row = ParamRow {
            name: format!("value{n}"),
            param: Param::Int {
                title: String::new(),
                default: 0,
                min: 0,
                max: 100,
            },
            values: Vec::new(),
            default: String::new(),
        };
        rows.push(row.with_kind("string"));
    }
    ui.weak(tr(
        "An action takes a parameter as {name} in its value; a list of values takes a list parameter by its name.",
    ));
}

/// The fields of a parameter by its kind.
fn fields(ui: &mut egui::Ui, salt: (&str, &str, usize), row: &mut ParamRow) {
    match &mut row.param {
        Param::Enum { title, values, .. } => {
            ui.add(
                egui::TextEdit::singleline(title)
                    .hint_text(tr("Title"))
                    .desired_width(f32::INFINITY),
            );
            let mut gone = None;
            egui::Grid::new((salt, "values"))
                .num_columns(4)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (j, (v, text)) in values.iter_mut().zip(row.values.iter_mut()).enumerate() {
                        ui.radio_value(&mut row.default, text.clone(), "")
                            .on_hover_text(tr("The default"));
                        ui.add(
                            egui::TextEdit::singleline(text)
                                .hint_text(tr("Value"))
                                .desired_width(120.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut v.title)
                                .hint_text(tr("Title"))
                                .desired_width(260.0),
                        );
                        if style::icon_button(ui, icons::CLOSE, tr("Remove the value"), true, None)
                            .clicked()
                        {
                            gone = Some(j);
                        }
                        ui.end_row();
                    }
                });
            if let Some(j) = gone {
                values.remove(j);
                row.values.remove(j);
            }
            if ui.button(tr("Add a value")).clicked() {
                let n = values.len();
                values.push(EnumValue {
                    value: Value::from(n as i64),
                    title: String::new(),
                });
                row.values.push(n.to_string());
            }
        }
        Param::Int {
            title,
            default,
            min,
            max,
        } => {
            ui.add(
                egui::TextEdit::singleline(title)
                    .hint_text(tr("Title"))
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Default"));
                ui.add(egui::DragValue::new(default));
                ui.label(tr("Minimum"));
                ui.add(egui::DragValue::new(min));
                ui.label(tr("Maximum"));
                ui.add(egui::DragValue::new(max));
            });
        }
        Param::Text {
            title,
            default,
            required,
            max_length,
            pick,
        } => {
            ui.add(
                egui::TextEdit::singleline(title)
                    .hint_text(tr("Title"))
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Default"));
                ui.add(egui::TextEdit::singleline(default).desired_width(220.0));
                ui.checkbox(required, tr("Required"));
                let mut limit = max_length.unwrap_or(0);
                ui.label(tr("Most characters"));
                if ui
                    .add(egui::DragValue::new(&mut limit).range(0..=16383))
                    .on_hover_text(tr("0 for no limit of its own"))
                    .changed()
                {
                    *max_length = (limit > 0).then_some(limit);
                }
                let shown = match pick {
                    None => tr("a free text"),
                    Some(Pick::RegionId) => tr("the GeoID of a region"),
                    Some(Pick::Region) => tr("the code of a region"),
                };
                egui::ComboBox::from_id_salt((salt, "pick"))
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(pick, None, tr("a free text"));
                        ui.selectable_value(
                            pick,
                            Some(Pick::RegionId),
                            tr("the GeoID of a region"),
                        );
                        ui.selectable_value(pick, Some(Pick::Region), tr("the code of a region"));
                    });
            });
        }
        Param::List { title, .. } => {
            ui.add(
                egui::TextEdit::singleline(title)
                    .hint_text(tr("Title"))
                    .desired_width(f32::INFINITY),
            );
            ui.label(tr("The default items, one a line"));
            ui.add(
                egui::TextEdit::multiline(&mut row.default)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        }
    }
}
