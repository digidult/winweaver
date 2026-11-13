//! The actions of a policy in the form: a frame an action, its kind and the fields of the kind.
//! Every kind of the built-in catalog is here: the settings and commands of the answer file, the
//! lines of a script, the registry values, their removals and lists, services, programs, features,
//! capabilities and apps.

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::{Action, Lines, ListNames, Phase, RegKind, check};
use winweaver_core::registry;

use super::param::{text_of, value_of};
#[cfg(feature = "pc")]
use crate::RegistryAct;
use crate::i18n::{tr, trf};
use crate::{Act, icons, style};

/// An action of the form: the action with the fields edited in place, and the fields that are
/// text in the form and something else in the catalog.
pub struct ActionRow {
    pub action: Action,
    /// The key of a registry action in the form of regedit.
    pub key: String,
    /// The value of a registry value as text: a number (decimal or `0x` hex), a text, the lines of
    /// a multi-string, the hex bytes of a binary value, or `{name}` for a parameter.
    pub value: String,
    /// The Windows default as text: empty for none, `absent`, `unknown` or a value.
    pub default: String,
    /// The lines of a script, the arguments of a program or the names of apps, a line each.
    pub lines: String,
}

/// The kinds of actions in the order of the choice.
const KINDS: [&str; 13] = [
    "reg",
    "reg-remove",
    "reg-list",
    "xml-setting",
    "xml-oobe",
    "xml-pe-command",
    "xml-specialize-command",
    "ps",
    "service",
    "feature",
    "capability",
    "appx",
    "exe",
];

const REG_KINDS: [RegKind; 6] = [
    RegKind::DWord,
    RegKind::QWord,
    RegKind::String,
    RegKind::ExpandString,
    RegKind::MultiString,
    RegKind::Binary,
];

fn kind_title(kind: &str) -> &'static str {
    match kind {
        "reg" => tr("A registry value"),
        "reg-remove" => tr("A registry value removed"),
        "reg-list" => tr("The values of a key from a list"),
        "xml-setting" => tr("A setting of the answer file"),
        "xml-oobe" => tr("An element of OOBE in the answer file"),
        "xml-pe-command" => tr("A command of windowsPE"),
        "xml-specialize-command" => tr("A command of specialize"),
        "ps" => tr("Lines of the script"),
        "service" => tr("The start of a service"),
        "feature" => tr("An optional feature of Windows"),
        "capability" => tr("Capabilities removed"),
        "appx" => tr("Apps removed"),
        _ => tr("A program run"),
    }
}

/// A new action of a kind, with the fields a user changes first left empty.
fn new_action(kind: &str) -> Action {
    match kind {
        "reg-remove" => Action::RegRemove {
            path: "HKLM:\\SOFTWARE\\Policies".to_owned(),
            name: String::new(),
            default: None,
        },
        "reg-list" => Action::RegList {
            path: "HKLM:\\SOFTWARE\\Policies".to_owned(),
            param: String::new(),
            names: ListNames::Items,
            prefix: String::new(),
            additive: false,
            kind: RegKind::String,
        },
        "xml-setting" => Action::XmlSetting {
            pass: "specialize".to_owned(),
            component: "Microsoft-Windows-Shell-Setup".to_owned(),
            path: String::new(),
            value: String::new(),
        },
        "xml-oobe" => Action::XmlOobe {
            element: String::new(),
            value: String::new(),
        },
        "xml-pe-command" => Action::XmlPeCommand {
            command: String::new(),
            description: String::new(),
        },
        "xml-specialize-command" => Action::XmlSpecializeCommand {
            command: String::new(),
            description: String::new(),
        },
        "ps" => Action::Ps {
            script: Lines::Many(Vec::new()),
        },
        "service" => Action::Service {
            name: String::new(),
            start: 4,
            default: None,
        },
        "feature" => Action::Feature {
            name: String::new(),
            state: "Disabled".to_owned(),
            default: None,
        },
        "capability" => Action::Capability {
            pattern: String::new(),
        },
        "appx" => Action::Appx { names: Vec::new() },
        "exe" => Action::Exe {
            file: String::new(),
            args: Vec::new(),
        },
        _ => Action::Reg {
            path: "HKLM:\\SOFTWARE\\Policies".to_owned(),
            name: String::new(),
            kind: RegKind::DWord,
            value: Value::from(1),
            default: None,
            why: String::new(),
        },
    }
}

/// A key of a catalog in the form of regedit.
fn shown_key(path: &str) -> String {
    registry::normalize(path).unwrap_or_else(|| path.to_owned())
}

/// A key typed in the form of regedit (or of a catalog) in the form of a catalog: the current
/// user is the default user profile in its phase and the user who signs in otherwise.
pub(super) fn catalog_key(key: &str, phase: Phase) -> Result<String, String> {
    let wrong = || {
        trf(
            "{key} is not a key of the machine or of the users",
            &[("key", &key)],
        )
    };
    let full = registry::normalize(key).ok_or_else(wrong)?;
    let (hive, rest) = full.split_once('\\').ok_or_else(wrong)?;
    let root = match hive {
        "HKEY_LOCAL_MACHINE" => "HKLM:",
        "HKEY_CURRENT_USER" if phase == Phase::DefaultUser => "DU:",
        "HKEY_CURRENT_USER" => "HKCU:",
        "HKEY_USERS" => "HKU:",
        _ => return Err(wrong()),
    };
    Ok(format!("{root}\\{rest}"))
}

/// Whether a text is a parameter alone, `{name}`, whose value the action writes.
fn is_placeholder(text: &str) -> bool {
    text.starts_with('{')
        && text.ends_with('}')
        && check::placeholders(text).len() == 1
        && check::placeholders(text)[0].len() + 2 == text.len()
}

/// The value of a registry action as the form shows it.
fn value_text(kind: RegKind, value: &Value) -> String {
    match (kind, value) {
        (_, Value::String(s)) => s.clone(),
        (RegKind::Binary, Value::Array(items)) => items
            .iter()
            .map(|b| format!("{:02x}", b.as_u64().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join(","),
        (_, Value::Array(items)) => items.iter().map(text_of).collect::<Vec<_>>().join("\n"),
        (_, other) => other.to_string(),
    }
}

/// The value of a registry action from the text of the form, or why it cannot be one.
fn reg_value(kind: RegKind, text: &str) -> Result<Value, String> {
    let t = text.trim();
    if is_placeholder(t) {
        return Ok(Value::from(t));
    }
    let not_number = || trf("{value} is not a number", &[("value", &t)]);
    match kind {
        RegKind::DWord | RegKind::QWord => {
            let n = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                Some(h) => u64::from_str_radix(h, 16),
                None => t.parse::<u64>(),
            }
            .map_err(|_| not_number())?;
            if kind == RegKind::DWord && n > u64::from(u32::MAX) {
                return Err(not_number());
            }
            Ok(Value::from(n))
        }
        RegKind::String | RegKind::ExpandString => Ok(Value::from(text)),
        RegKind::MultiString => Ok(Value::from(
            text.lines()
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )),
        RegKind::Binary => {
            let bytes: Result<Vec<Value>, String> = t
                .split([',', ' ', '\n'])
                .filter(|b| !b.is_empty())
                .map(|b| {
                    u8::from_str_radix(b.trim(), 16)
                        .map(Value::from)
                        .map_err(|_| trf("{value} is not a byte in hex", &[("value", &b)]))
                })
                .collect();
            Ok(Value::from(bytes?))
        }
    }
}

/// The Windows default of the form: none, `absent`, `unknown` or a value made by `value`.
fn default_of(
    text: &str,
    value: impl Fn(&str) -> Result<Value, String>,
) -> Result<Option<Value>, String> {
    match text.trim() {
        "" => Ok(None),
        t @ ("absent" | "unknown") => Ok(Some(Value::from(t))),
        t => value(t).map(Some),
    }
}

/// The lines of a text without the empty ones.
fn lines_of(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

impl ActionRow {
    pub fn of(action: &Action) -> Self {
        let key = action.registry_path().map(shown_key).unwrap_or_default();
        let value = match action {
            Action::Reg { kind, value, .. } => value_text(*kind, value),
            _ => String::new(),
        };
        let default = match action {
            Action::Reg {
                kind,
                default: Some(d),
                ..
            } => value_text(*kind, d),
            Action::RegRemove {
                default: Some(d), ..
            }
            | Action::Service {
                default: Some(d), ..
            } => text_of(d),
            Action::Feature {
                default: Some(d), ..
            } => d.clone(),
            _ => String::new(),
        };
        let lines = match action {
            Action::Ps { script } => script.lines().join("\n"),
            Action::Exe { args, .. } => args.join("\n"),
            Action::Appx { names } => names.join("\n"),
            _ => String::new(),
        };
        Self {
            action: action.clone(),
            key,
            value,
            default,
            lines,
        }
    }

    /// The action of the row for a policy of a phase, or why it cannot be one.
    pub fn action(&self, phase: Phase) -> Result<Action, String> {
        let mut action = self.action.clone();
        match &mut action {
            Action::Reg {
                path,
                kind,
                value,
                default,
                ..
            } => {
                *path = catalog_key(&self.key, phase)?;
                *value = reg_value(*kind, &self.value)?;
                *default = default_of(&self.default, |t| reg_value(*kind, t))?;
            }
            Action::RegRemove { path, default, .. } => {
                *path = catalog_key(&self.key, phase)?;
                *default = default_of(&self.default, |t| Ok(value_of(t)))?;
            }
            Action::RegList { path, .. } => *path = catalog_key(&self.key, phase)?,
            Action::Service { default, .. } => {
                *default = default_of(&self.default, |t| Ok(value_of(t)))?;
            }
            Action::Feature { default, .. } => {
                let t = self.default.trim();
                *default = (!t.is_empty()).then(|| t.to_owned());
            }
            Action::Ps { script } => {
                let lines: Vec<String> = self.lines.lines().map(str::to_owned).collect();
                *script = Lines::Many(lines);
            }
            Action::Exe { args, .. } => *args = lines_of(&self.lines),
            Action::Appx { names } => *names = lines_of(&self.lines),
            _ => {}
        }
        Ok(action)
    }
}

/// The frames of the actions, with the buttons that add, move and remove them. `lists` are the
/// names of the list parameters of the policy, for a list of values.
pub(super) fn actions_ui(
    ui: &mut egui::Ui,
    id: &str,
    lists: &[String],
    rows: &mut Vec<ActionRow>,
    acts: &mut Vec<Act>,
) {
    let mut gone = None;
    let mut up = None;
    let count = rows.len();
    for (i, row) in rows.iter_mut().enumerate() {
        let salt = (id, "action", i);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                let kind = row.action.kind();
                let mut chosen = kind;
                egui::ComboBox::from_id_salt((salt, "kind"))
                    .selected_text(kind_title(kind))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for k in KINDS {
                            ui.selectable_value(&mut chosen, k, kind_title(k));
                        }
                    });
                if chosen != kind {
                    *row = ActionRow::of(&new_action(chosen));
                }
                if i > 0 && style::icon_button(ui, icons::UP, tr("Up"), true, None).clicked() {
                    up = Some(i);
                }
                if i + 1 < count
                    && style::icon_button(ui, icons::DOWN, tr("Down"), true, None).clicked()
                {
                    up = Some(i + 1);
                }
                if style::icon_button(ui, icons::CLOSE, tr("Remove the action"), true, None)
                    .clicked()
                {
                    gone = Some(i);
                }
            });
            egui::Grid::new((salt, "fields"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| fields(ui, salt, lists, row, acts));
        });
        ui.add_space(4.0);
    }
    if let Some(i) = gone {
        rows.remove(i);
    } else if let Some(i) = up {
        rows.swap(i - 1, i);
    }
    if ui.button(tr("Add an action")).clicked() {
        rows.push(ActionRow::of(&new_action("reg")));
    }
}

/// A line of the grid with a text of one line in the width of the panel.
fn line(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    ui.end_row();
}

/// A line of the grid with a text of several lines in the width of the panel.
fn lines(ui: &mut egui::Ui, label: &str, text: &mut String, code: bool) {
    ui.label(label);
    let edit = if code {
        egui::TextEdit::multiline(text).code_editor()
    } else {
        egui::TextEdit::multiline(text)
    };
    ui.add(edit.desired_rows(3).desired_width(f32::INFINITY));
    ui.end_row();
}

/// The key of a registry action, with the button that opens it in the registry tab.
fn key_line(ui: &mut egui::Ui, key: &mut String, name: Option<&str>, acts: &mut Vec<Act>) {
    ui.label(tr("Key"));
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 40.0).max(200.0);
        ui.add(egui::TextEdit::singleline(key).desired_width(width));
        #[cfg(feature = "pc")]
        if ui
            .small_button(icons::REGISTRY)
            .on_hover_text(tr("Open in the registry"))
            .clicked()
        {
            let value = name.filter(|n| !n.is_empty()).map(str::to_owned);
            acts.push(Act::Registry(RegistryAct::Open(key.clone(), value)));
        }
        #[cfg(not(feature = "pc"))]
        let _ = (name, &acts);
    });
    ui.end_row();
}

/// The fields of an action by its kind.
fn fields(
    ui: &mut egui::Ui,
    salt: (&str, &str, usize),
    lists: &[String],
    row: &mut ActionRow,
    acts: &mut Vec<Act>,
) {
    let default_hint = tr("absent, unknown or a value; empty when it is not known");
    match &mut row.action {
        Action::Reg {
            name, kind, why, ..
        } => {
            key_line(ui, &mut row.key, Some(name), acts);
            line(ui, tr("Name"), name);
            ui.label(tr("Type"));
            egui::ComboBox::from_id_salt((salt, "type"))
                .selected_text(kind.type_name())
                .show_ui(ui, |ui| {
                    for k in REG_KINDS {
                        ui.selectable_value(kind, k, k.type_name());
                    }
                });
            ui.end_row();
            if *kind == RegKind::MultiString {
                lines(ui, tr("Value"), &mut row.value, false);
            } else {
                line(ui, tr("Value"), &mut row.value);
            }
            ui.label(tr("Windows default"));
            ui.add(
                egui::TextEdit::singleline(&mut row.default)
                    .hint_text(default_hint)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();
            line(ui, tr("Why"), why);
        }
        Action::RegRemove { name, .. } => {
            key_line(ui, &mut row.key, Some(name), acts);
            line(ui, tr("Name"), name);
            ui.label(tr("Windows default"));
            ui.add(
                egui::TextEdit::singleline(&mut row.default)
                    .hint_text(default_hint)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();
        }
        Action::RegList {
            param,
            names,
            prefix,
            additive,
            kind,
            ..
        } => {
            key_line(ui, &mut row.key, None, acts);
            ui.label(tr("List parameter"));
            let shown = if param.is_empty() {
                tr("none: the key is emptied")
            } else {
                param.as_str()
            };
            egui::ComboBox::from_id_salt((salt, "param"))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(param, String::new(), tr("none: the key is emptied"));
                    for l in lists {
                        ui.selectable_value(param, l.clone(), l);
                    }
                });
            ui.end_row();
            ui.label(tr("Value names"));
            let title = |n: ListNames| match n {
                ListNames::Items => tr("every item is the name and the data"),
                ListNames::Numbered => tr("a prefix and a number from 1"),
                ListNames::Pairs => tr("every item is name=value"),
            };
            egui::ComboBox::from_id_salt((salt, "names"))
                .selected_text(title(*names))
                .show_ui(ui, |ui| {
                    for n in [ListNames::Items, ListNames::Numbered, ListNames::Pairs] {
                        ui.selectable_value(names, n, title(n));
                    }
                });
            ui.end_row();
            if *names == ListNames::Numbered {
                line(ui, tr("Prefix"), prefix);
            }
            ui.label(tr("Type"));
            egui::ComboBox::from_id_salt((salt, "list-type"))
                .selected_text(kind.type_name())
                .show_ui(ui, |ui| {
                    for k in [RegKind::String, RegKind::ExpandString] {
                        ui.selectable_value(kind, k, k.type_name());
                    }
                });
            ui.end_row();
            ui.label("");
            ui.checkbox(additive, tr("Keep the other values of the key"));
            ui.end_row();
        }
        Action::XmlSetting {
            pass,
            component,
            path,
            value,
        } => {
            ui.label(tr("Pass"));
            egui::ComboBox::from_id_salt((salt, "pass"))
                .selected_text(pass.as_str())
                .show_ui(ui, |ui| {
                    for p in ["windowsPE", "specialize", "oobeSystem"] {
                        ui.selectable_value(pass, p.to_owned(), p);
                    }
                });
            ui.end_row();
            line(ui, tr("Component"), component);
            line(ui, tr("Path of the elements"), path);
            line(ui, tr("Value"), value);
        }
        Action::XmlOobe { element, value } => {
            line(ui, tr("Element"), element);
            line(ui, tr("Value"), value);
        }
        Action::XmlPeCommand {
            command,
            description,
        }
        | Action::XmlSpecializeCommand {
            command,
            description,
        } => {
            line(ui, tr("Command"), command);
            line(ui, tr("Description"), description);
        }
        Action::Ps { .. } => lines(ui, tr("Script"), &mut row.lines, true),
        Action::Service { name, start, .. } => {
            line(ui, tr("Service"), name);
            ui.label(tr("Start"));
            let title = |s: u32| match s {
                2 => tr("Automatic"),
                3 => tr("Manual"),
                _ => tr("Disabled"),
            };
            egui::ComboBox::from_id_salt((salt, "start"))
                .selected_text(title(*start))
                .show_ui(ui, |ui| {
                    for s in [2, 3, 4] {
                        ui.selectable_value(start, s, title(s));
                    }
                });
            ui.end_row();
            ui.label(tr("Windows default"));
            ui.add(
                egui::TextEdit::singleline(&mut row.default)
                    .hint_text(default_hint)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();
        }
        Action::Feature { name, state, .. } => {
            line(ui, tr("Feature"), name);
            ui.label(tr("State"));
            egui::ComboBox::from_id_salt((salt, "state"))
                .selected_text(state.as_str())
                .show_ui(ui, |ui| {
                    for s in ["Enabled", "Disabled"] {
                        ui.selectable_value(state, s.to_owned(), s);
                    }
                });
            ui.end_row();
            line(ui, tr("Windows default"), &mut row.default);
        }
        Action::Capability { pattern } => line(ui, tr("Name pattern"), pattern),
        Action::Appx { .. } => lines(ui, tr("Package names"), &mut row.lines, false),
        Action::Exe { file, .. } => {
            line(ui, tr("Program"), file);
            lines(ui, tr("Arguments, one a line"), &mut row.lines, false);
        }
    }
}
