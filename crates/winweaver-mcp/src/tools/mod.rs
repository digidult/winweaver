//! The tools and resources of WinWeaver.
//!
//! Every tool is one entry of [`TOOLS`]: its name, its description, what it needs (the mode, the
//! registry, the library, this PC) and the function that runs it, in `profiles`, `rules`,
//! `bookmarks`, `catalogs` and `registry`. `definitions` and `call` read the table, so a tool is
//! named once; the server of W# checks the mode and the options a tool needs before it runs.

use serde_json::{Value, json};
use winweaver_core::profile::Profile;
use winweaver_core::resolver::{self, Resolution};
pub(crate) use wwmcp::{Args, Kind, ToolError, fail};
use wwmcp::{Mode, ResourceDef, ToolDef};

use crate::{Server, State, WsDoc};

mod bookmarks;
mod catalogs;
mod profiles;
mod registry;
mod rules;

const LIST_LIMIT: usize = 200;
/// The skill for agents, served as a resource.
const SKILL: &str = include_str!("../../../../agents/skills/winweaver/SKILL.md");

type Outcome = Result<Value, ToolError>;

/// What a tool needs before it runs: the mode, the registry (reading, or writing too), the
/// library (bookmarks and catalogs) and this PC; the options of the source `winweaver` that the
/// server of W# checks.
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

    /// The tool for the server: its mode and the options it needs.
    fn def(&self, tool: &Tool) -> ToolDef {
        let mut def =
            ToolDef::new(tool.name, group(tool), tool.description, (tool.schema)()).mode(self.mode);
        if let Some(write) = self.registry {
            def = def.needs("registry", if write { "write" } else { "read" });
        }
        if self.library {
            def = def.needs("edit_library", "on");
        }
        if self.read_pc {
            def = def.needs("read_pc", "on");
        }
        def
    }
}

/// The group of a tool, by the family of its name: the switches of the tab turn a group off.
fn group(tool: &Tool) -> &'static str {
    let name = tool.name;
    if name.contains("registry") {
        "registry"
    } else if name.contains("bookmark") {
        "bookmarks"
    } else if name.contains("catalog") {
        "catalogs"
    } else if name.contains("rule") || name.contains("group") || name.contains("param") {
        "rules"
    } else if name == "read_this_pc" {
        "this_pc"
    } else {
        "profiles"
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

/// The definitions of the tools for the server.
pub(crate) fn definitions() -> Vec<ToolDef> {
    TOOLS.iter().map(|t| t.needs.def(t)).collect()
}

/// The resources of WinWeaver.
pub(crate) fn resources() -> Vec<ResourceDef> {
    let resource = |uri: &str, name: &str, mime: &str| ResourceDef {
        uri: uri.to_owned(),
        name: name.to_owned(),
        mime: mime.to_owned(),
    };
    vec![
        resource("winweaver://catalog", "Rule catalog", "application/json"),
        resource("winweaver://profile", "Current profile", "application/json"),
        resource("winweaver://presets", "Presets", "application/json"),
        resource(
            "winweaver://skill",
            "How to use WinWeaver over MCP",
            "text/markdown",
        ),
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
    Some(serde_json::to_string_pretty(&value).unwrap_or_default())
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
        let defs = definitions();
        assert_eq!(defs.len(), count);
        let write = defs
            .iter()
            .find(|d| d.name == "set_registry_value")
            .unwrap();
        assert_eq!(write.needs[0].at_least, "write");
        let pc = defs.iter().find(|d| d.name == "read_this_pc").unwrap();
        assert_eq!((pc.mode, pc.group.as_str()), (Mode::Edit, "this_pc"));
    }
}
