//! The data model of the rule catalog: groups, rules, parameters and actions.
//!
//! The JSON layout is the one of the catalog data in `data/catalog/`: a file `groups.json` with the
//! group tree and files `NN-<topic>.json` with rules. Rules contain no program logic.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A node of the group tree. Groups with a `parent` are nested; `order` sorts siblings.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub order: u32,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// The pass or the moment at which the actions of a rule run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Windowspe,
    SpecializeXml,
    Specialize,
    OobeXml,
    DefaultUser,
    UserFirstLogon,
    PostOobe,
}

/// How important a rule is for the protection of the PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Optional,
    Baseline,
    Recommended,
    Risky,
}

/// One rule of the catalog. Texts are English; translations live in `lang/<code>.json`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub group: String,
    pub phase: Phase,
    pub title: String,
    pub level: Level,
    /// Whether the rule is on in a new profile (the Office preset).
    pub default: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The documentation page of the rule, relative to the documentation folder.
    #[serde(default)]
    pub doc: String,
    pub summary: String,
    #[serde(default)]
    pub effect: String,
    #[serde(default)]
    pub risk: String,
    #[serde(default)]
    pub versions: String,
    #[serde(default)]
    pub rollback: String,
    #[serde(default)]
    pub verify: String,
    #[serde(default)]
    pub note: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Param>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    pub actions: Vec<Action>,
}

/// One option of an enum parameter.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnumValue {
    pub value: Value,
    pub title: String,
}

/// A parameter of a rule, filled into the `{name}` placeholders of its actions.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Param {
    #[serde(rename = "enum")]
    Enum {
        title: String,
        default: Value,
        values: Vec<EnumValue>,
        /// Another enum parameter of the rule whose value must differ, except for `same_allowed`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        differs_from: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        same_allowed: Vec<Value>,
    },
    #[serde(rename = "int")]
    Int {
        title: String,
        default: i64,
        min: i64,
        max: i64,
    },
    #[serde(rename = "string")]
    Text {
        title: String,
        default: String,
        /// `false` lets the text be empty.
        #[serde(default = "yes")]
        required: bool,
    },
}

fn yes() -> bool {
    true
}

impl Param {
    pub fn title(&self) -> &str {
        match self {
            Param::Enum { title, .. } | Param::Int { title, .. } | Param::Text { title, .. } => {
                title
            }
        }
    }

    /// The default value as JSON.
    pub fn default_value(&self) -> Value {
        match self {
            Param::Enum { default, .. } => default.clone(),
            Param::Int { default, .. } => Value::from(*default),
            Param::Text { default, .. } => Value::from(default.as_str()),
        }
    }
}

/// The kind of a registry value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum RegKind {
    DWord,
    QWord,
    String,
    ExpandString,
    MultiString,
    Binary,
}

/// A script or a comment: one line or a list of lines.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Lines {
    One(String),
    Many(Vec<String>),
}

impl Lines {
    pub fn lines(&self) -> Vec<&str> {
        match self {
            Lines::One(s) => vec![s.as_str()],
            Lines::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

/// What a rule does when it is on. The `default` fields hold the Windows default for the return
/// to defaults: a value, the string `"absent"` (the value does not exist) or `"unknown"`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[allow(clippy::large_enum_variant)]
pub enum Action {
    /// A command of the windowsPE pass of the answer file.
    XmlPeCommand {
        command: String,
        #[serde(default)]
        description: String,
    },
    /// A command of the specialize pass of the answer file.
    XmlSpecializeCommand {
        command: String,
        #[serde(default)]
        description: String,
    },
    /// A native setting of the answer file: a pass, a component and a path of element names.
    XmlSetting {
        pass: String,
        component: String,
        path: String,
        value: String,
    },
    /// An element of the OOBE section of the answer file.
    XmlOobe { element: String, value: String },
    /// Lines of the embedded script of the phase.
    Ps { script: Lines },
    /// A registry value.
    Reg {
        path: String,
        name: String,
        kind: RegKind,
        value: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<Value>,
        #[serde(default)]
        why: String,
    },
    /// A registry value removed.
    RegRemove {
        path: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<Value>,
    },
    /// The start type of a service (2 automatic, 3 manual, 4 disabled).
    Service {
        name: String,
        start: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<Value>,
    },
    /// A program run with arguments.
    Exe {
        file: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// A Windows optional feature.
    Feature {
        name: String,
        state: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },
    /// Windows capabilities removed by a name pattern.
    Capability { pattern: String },
    /// Store apps removed by package name.
    Appx { names: Vec<String> },
}

impl Action {
    /// The kind of the action as it is written in the catalog.
    pub fn kind(&self) -> &'static str {
        match self {
            Action::XmlPeCommand { .. } => "xml-pe-command",
            Action::XmlSpecializeCommand { .. } => "xml-specialize-command",
            Action::XmlSetting { .. } => "xml-setting",
            Action::XmlOobe { .. } => "xml-oobe",
            Action::Ps { .. } => "ps",
            Action::Reg { .. } => "reg",
            Action::RegRemove { .. } => "reg-remove",
            Action::Service { .. } => "service",
            Action::Exe { .. } => "exe",
            Action::Feature { .. } => "feature",
            Action::Capability { .. } => "capability",
            Action::Appx { .. } => "appx",
        }
    }

    /// Whether the action is a setting or a command of the answer file itself.
    pub fn is_xml(&self) -> bool {
        self.kind().starts_with("xml-")
    }

    /// The registry key the action writes or removes, if any.
    pub fn registry_path(&self) -> Option<&str> {
        match self {
            Action::Reg { path, .. } | Action::RegRemove { path, .. } => Some(path),
            _ => None,
        }
    }

    /// The fields in which `{name}` placeholders are filled in; every other field is literal.
    pub fn placeholder_fields(&self) -> Vec<(&'static str, String)> {
        match self {
            Action::XmlPeCommand { command, .. } | Action::XmlSpecializeCommand { command, .. } => {
                vec![("command", command.clone())]
            }
            Action::XmlSetting { value, .. } | Action::XmlOobe { value, .. } => {
                vec![("value", value.clone())]
            }
            Action::Ps { script } => vec![("script", script.lines().join("\n"))],
            Action::Reg { value, .. } => match value {
                Value::String(s) => vec![("value", s.clone())],
                Value::Array(items) => items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| ("value", s.to_owned()))
                    .collect(),
                _ => Vec::new(),
            },
            Action::Exe { args, .. } => args.iter().map(|a| ("args", a.clone())).collect(),
            _ => Vec::new(),
        }
    }

    /// The literal fields that must never hold a placeholder.
    pub fn literal_fields(&self) -> Vec<(&'static str, &str)> {
        match self {
            Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. } => {
                vec![("path", path.as_str()), ("name", name.as_str())]
            }
            Action::XmlSetting {
                pass,
                component,
                path,
                ..
            } => vec![("pass", pass), ("component", component), ("path", path)],
            Action::XmlOobe { element, .. } => vec![("element", element.as_str())],
            Action::Service { name, .. } | Action::Feature { name, .. } => {
                vec![("name", name.as_str())]
            }
            Action::Exe { file, .. } => vec![("file", file.as_str())],
            Action::Capability { pattern } => vec![("pattern", pattern.as_str())],
            Action::Appx { names } => names.iter().map(|n| ("names", n.as_str())).collect(),
            _ => Vec::new(),
        }
    }
}

/// A file of rules: a comment and the rules.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RulesFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<Lines>,
    pub rules: Vec<Rule>,
}

/// The file `groups.json`: a comment and the groups.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GroupsFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<Lines>,
    pub groups: Vec<Group>,
}
