//! The tools and resources of the MCP server.
//!
//! Every tool is one entry of [`TOOLS`]: its name, its description, what it needs (the mode, the
//! registry, the library, this PC) and the function that runs it, in `profiles`, `rules`,
//! `bookmarks`, `catalogs` and `registry`. `definitions` and `call` read the table, so a tool is
//! named once.

use serde_json::{Value, json};
use winweaver_core::profile::Profile;
use winweaver_core::resolver::{self, Resolution};

use crate::{Mode, RegistryAccess, Server, State, WsDoc};

mod bookmarks;
mod catalogs;
mod profiles;
mod registry;
mod rules;

/// The longest text of one tool result; longer results are cut with a note.
const TEXT_LIMIT: usize = 60_000;
const LIST_LIMIT: usize = 200;
/// The skill for agents, served as a resource.
const SKILL: &str = include_str!("../../../../agents/skills/winweaver/SKILL.md");

/// Why a tool call failed; its name starts the text the client sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Invalid,
    NotFound,
    NotAllowed,
    ReadFailed,
    WriteFailed,
}

impl Kind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Invalid => "invalid",
            Kind::NotFound => "not_found",
            Kind::NotAllowed => "not_allowed",
            Kind::ReadFailed => "read_failed",
            Kind::WriteFailed => "write_failed",
        }
    }
}

/// A failed tool call.
pub(crate) struct ToolError {
    pub kind: Kind,
    pub message: String,
}

fn fail(kind: Kind, message: impl Into<String>) -> ToolError {
    ToolError {
        kind,
        message: message.into(),
    }
}

type Outcome = Result<Value, ToolError>;

/// The arguments of a call: a JSON object, a field that is null as good as absent.
pub(crate) struct Args<'a>(&'a Value);

impl<'a> Args<'a> {
    fn get(&self, name: &str) -> Option<&'a Value> {
        self.0.get(name).filter(|v| !v.is_null())
    }

    /// A text that must be there.
    fn text(&self, name: &str) -> Result<String, ToolError> {
        self.str(name)
            .map(str::to_owned)
            .ok_or_else(|| fail(Kind::Invalid, format!("the argument {name} must be a text")))
    }

    /// A text that may be left out.
    fn str(&self, name: &str) -> Option<&'a str> {
        self.get(name).and_then(Value::as_str)
    }

    /// A text, empty when left out.
    fn optional_text(&self, name: &str) -> String {
        self.str(name).unwrap_or_default().to_owned()
    }

    /// A switch that must be there.
    fn flag(&self, name: &str) -> Result<bool, ToolError> {
        self.get(name).and_then(Value::as_bool).ok_or_else(|| {
            fail(
                Kind::Invalid,
                format!("the argument {name} must be true or false"),
            )
        })
    }

    /// A switch with its default.
    fn flag_or(&self, name: &str, default: bool) -> bool {
        self.get(name).and_then(Value::as_bool).unwrap_or(default)
    }

    fn number(&self, name: &str) -> Option<u64> {
        self.get(name).and_then(Value::as_u64)
    }

    /// A count between 1 and `max`, `default` when left out.
    fn limit(&self, name: &str, default: usize, max: usize) -> usize {
        match self.number(name) {
            Some(n) => n.clamp(1, max as u64) as usize,
            None => default,
        }
    }

    fn list(&self, name: &str) -> Option<&'a Vec<Value>> {
        self.get(name).and_then(Value::as_array)
    }
}

/// What a tool needs before it runs: the mode, the registry (reading, or writing too), the
/// library (bookmarks and catalogs) and this PC.
#[derive(Clone, Copy)]
pub(crate) struct Needs {
    mode: Mode,
    registry: Option<bool>,
    library: bool,
    read_pc: bool,
}

impl Needs {
    const READ: Self = Self {
        mode: Mode::Read,
        registry: None,
        library: false,
        read_pc: false,
    };
    const EDIT: Self = Self {
        mode: Mode::Edit,
        ..Self::READ
    };
    const FILES: Self = Self {
        mode: Mode::Files,
        ..Self::READ
    };
    const LIBRARY: Self = Self {
        library: true,
        ..Self::READ
    };
    const REGISTRY_READ: Self = Self {
        registry: Some(false),
        ..Self::READ
    };
    const REGISTRY_WRITE: Self = Self {
        registry: Some(true),
        ..Self::READ
    };
    const THIS_PC: Self = Self {
        mode: Mode::Edit,
        read_pc: true,
        ..Self::READ
    };

    fn check(&self, state: &State) -> Result<(), ToolError> {
        if self.read_pc && !state.options.read_pc {
            return Err(fail(
                Kind::NotAllowed,
                "reading this PC is off; the user allows it in the window (MCP tab) or with --read-pc",
            ));
        }
        if let Some(write) = self.registry {
            registry_allowed(state, write)?;
        }
        if self.library {
            library_allowed(state)?;
        }
        need(state, self.mode)
    }
}

/// One tool: named once, with its schema and the function that runs it.
pub(crate) struct Tool {
    name: &'static str,
    description: &'static str,
    needs: Needs,
    schema: fn() -> Value,
    run: fn(&Server, &mut State, &Args<'_>) -> Outcome,
}

/// The tools, in the order `tools/list` shows them.
pub(crate) const TOOLS: &[Tool] = &[
    profiles::GET_STATUS,
    rules::LIST_GROUPS,
    rules::LIST_RULES,
    rules::GET_RULE,
    rules::SEARCH_RULES,
    profiles::GET_PROFILE,
    rules::SET_RULE,
    rules::SET_PARAM,
    rules::SET_GROUP,
    profiles::SET_PROFILE_INFO,
    profiles::CHECK_PROFILE,
    profiles::PREVIEW_BUILD,
    profiles::WRITE_ANSWER_FILE,
    profiles::LOAD_PROFILE,
    profiles::SAVE_PROFILE,
    profiles::NEW_PROFILE,
    profiles::LIST_PROFILES,
    profiles::COMPARE_PROFILES,
    bookmarks::LIST_BOOKMARKS,
    bookmarks::ADD_BOOKMARK,
    bookmarks::SET_BOOKMARK_COMMENT,
    bookmarks::REMOVE_BOOKMARK,
    catalogs::LIST_CATALOGS,
    catalogs::GET_CATALOG,
    catalogs::GET_CATALOG_POLICY,
    catalogs::SET_CATALOG_POLICY,
    catalogs::REMOVE_CATALOG_POLICY,
    registry::READ_REGISTRY_KEY,
    registry::SEARCH_REGISTRY,
    registry::SET_REGISTRY_VALUE,
    registry::DELETE_REGISTRY_VALUE,
    registry::CREATE_REGISTRY_KEY,
    registry::DELETE_REGISTRY_KEY,
    registry::READ_REGISTRY_RIGHTS,
    registry::SET_REGISTRY_RIGHTS,
    profiles::READ_THIS_PC,
];

/// The schema of the arguments of a tool: an object of these properties, these required.
fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

/// A text property with its description.
fn s(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

/// The definitions of the tools for `tools/list`.
pub(crate) fn definitions() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|t| {
            json!({ "name": t.name, "description": t.description, "inputSchema": (t.schema)() })
        })
        .collect()
}

/// The resources for `resources/list`.
pub(crate) fn resources() -> Vec<Value> {
    vec![
        json!({ "uri": "winweaver://catalog", "name": "Rule catalog", "mimeType": "application/json" }),
        json!({ "uri": "winweaver://profile", "name": "Current profile", "mimeType": "application/json" }),
        json!({ "uri": "winweaver://presets", "name": "Presets", "mimeType": "application/json" }),
        json!({ "uri": "winweaver://skill", "name": "How to use WinWeaver over MCP", "mimeType": "text/markdown" }),
    ]
}

/// The text of a resource.
pub(crate) fn resource(server: &Server, state: &State, uri: &str) -> Option<String> {
    let value = match uri {
        "winweaver://catalog" => json!({
            "version": server.catalog.version(),
            "groups": server.catalog.groups().iter().map(|g| json!({ "id": g.id, "title": g.title, "parent": g.parent })).collect::<Vec<_>>(),
            "rules": server.catalog.rules().iter().map(|r| json!({ "id": r.id, "title": r.title, "group": r.group })).collect::<Vec<_>>(),
        }),
        "winweaver://profile" => redacted(&current(state).profile),
        "winweaver://presets" => {
            json!({ "presets": server.presets.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>() })
        }
        "winweaver://skill" => return Some(SKILL.to_owned()),
        _ => return None,
    };
    Some(fit(
        &serde_json::to_string_pretty(&value).unwrap_or_default()
    ))
}

/// The arguments of a call for the journal: identifiers, numbers and switches; other texts are
/// replaced by their kind.
pub(crate) fn journal_detail(args: &Value) -> String {
    let Some(map) = args.as_object() else {
        return String::new();
    };
    map.iter()
        .map(|(k, v)| {
            let shown = match v {
                Value::Bool(_) | Value::Number(_) => v.to_string(),
                Value::String(s)
                    if s.len() <= 80
                        && s.chars()
                            .all(|c| c.is_ascii_alphanumeric() || "._- ".contains(c)) =>
                {
                    s.clone()
                }
                Value::String(_) => "<text>".to_owned(),
                _ => "<value>".to_owned(),
            };
            format!("{}={shown}", k.chars().take(40).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A text cut to the limit of one result.
pub(crate) fn fit(text: &str) -> String {
    if text.len() <= TEXT_LIMIT {
        return text.to_owned();
    }
    let mut end = TEXT_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n... (cut: {} more characters)",
        &text[..end],
        text.len() - end
    )
}

fn current(state: &State) -> &WsDoc {
    state
        .docs
        .get(&state.current)
        .or_else(|| state.docs.values().next())
        .expect("the workspace always holds a profile")
}

fn current_mut(state: &mut State) -> &mut WsDoc {
    let key = if state.docs.contains_key(&state.current) {
        state.current.clone()
    } else {
        state.docs.keys().next().cloned().unwrap_or_default()
    };
    state
        .docs
        .get_mut(&key)
        .expect("the workspace always holds a profile")
}

/// The resolution of the current profile.
fn resolution(server: &Server, state: &State) -> Resolution {
    resolver::resolve(&server.catalog, &current(state).profile)
}

fn need(state: &State, mode: Mode) -> Result<(), ToolError> {
    if state.options.mode >= mode {
        Ok(())
    } else {
        Err(fail(
            Kind::NotAllowed,
            format!(
                "the server is in the mode {}; this tool needs the mode {} (the user switches it in the window or with --mode)",
                state.options.mode.name(),
                mode.name()
            ),
        ))
    }
}

/// The bookmarks and the catalog files: read always, changed only when the user allows it.
fn library_allowed(state: &State) -> Result<(), ToolError> {
    if state.options.edit_library {
        Ok(())
    } else {
        Err(fail(
            Kind::NotAllowed,
            "changing the bookmarks and the catalogs is off; the user allows it in the window (MCP tab) or with --edit-library",
        ))
    }
}

/// The tools of the registry: off unless the user allows them, writing only when allowed too.
fn registry_allowed(state: &State, write: bool) -> Result<(), ToolError> {
    match (state.options.registry, write) {
        (RegistryAccess::Write, _) | (RegistryAccess::Read, false) => Ok(()),
        (RegistryAccess::Read, true) => Err(fail(
            Kind::NotAllowed,
            "writing the registry is off; the user allows it in the window (MCP tab) or with --registry write",
        )),
        (RegistryAccess::Off, _) => Err(fail(
            Kind::NotAllowed,
            "the registry is off; the user allows it in the window (MCP tab) or with --registry read or write",
        )),
    }
}

/// A profile without its passwords and product key.
pub(crate) fn redacted(p: &Profile) -> Value {
    let mut value = serde_json::to_value(p).unwrap_or(Value::Null);
    if let Some(key) = value.pointer_mut("/install/product_key")
        && key.as_str().is_some_and(|k| !k.is_empty())
    {
        *key = json!("***");
    }
    if let Some(Value::Array(accounts)) = value.get_mut("accounts") {
        for a in accounts {
            if let Some(pw) = a.get_mut("password")
                && pw.as_str().is_some_and(|k| !k.is_empty())
            {
                *pw = json!("***");
            }
        }
    }
    value
}

/// Runs a tool: the one of the table with this name, after what it needs is checked.
pub(crate) fn call(server: &Server, state: &mut State, name: &str, args: &Value) -> Outcome {
    if !args.is_object() {
        return Err(fail(Kind::Invalid, "the arguments must be an object"));
    }
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return Err(fail(Kind::NotFound, format!("no tool {name}")));
    };
    tool.needs.check(state)?;
    (tool.run)(server, state, &Args(args))
}

/// Loads a preset or a profile file into the workspace and makes it current.
pub(crate) fn select(server: &Server, state: &mut State, name: &str) -> Result<(), ToolError> {
    let (mut profile, file) = profiles::find_profile(server, state, name)?;
    profile.split_unknown(&server.catalog);
    let key = profiles::unique_name(state, &profile.name);
    state.docs.insert(
        key.clone(),
        WsDoc {
            profile,
            file,
            dirty: false,
        },
    );
    state.current = key;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::profiles::{hide_secrets, safe_file_name};
    use super::*;

    #[test]
    fn secrets_are_hidden() {
        let xml = "<Key>VK7JG-NPHTM</Key><Password><Value>secret</Value><PlainText>true</PlainText></Password><Value></Value>";
        assert_eq!(
            hide_secrets(xml),
            "<Key>***</Key><Password><Value>***</Value><PlainText>true</PlainText></Password><Value></Value>"
        );
        assert!(
            safe_file_name("..").is_err()
                && safe_file_name("a/b").is_err()
                && safe_file_name("C:x").is_err()
        );
        assert_eq!(safe_file_name("Office 2").ok().as_deref(), Some("Office 2"));
        assert_eq!(
            journal_detail(
                &json!({ "id": "defender.pua", "on": true, "comment": "long free text, with commas" })
            ),
            "id=defender.pua on=true comment=<text>"
        );
    }

    #[test]
    fn every_tool_is_named_once_with_a_schema() {
        let mut names: Vec<&str> = TOOLS.iter().map(|t| t.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        for t in TOOLS {
            let schema = (t.schema)();
            assert_eq!(schema["type"], "object", "{}", t.name);
            assert!(!t.description.is_empty(), "{}", t.name);
        }
        assert_eq!(definitions().len(), count);
    }
}
