//! The tools and resources of the MCP server. The names are the ones of the predecessor's server,
//! so that the instructions written for agents carry over.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize};

use serde_json::{Value, json};
use winweaver_core::admx::{self, CatalogFile, CatalogHeader};
use winweaver_core::bookmarks::{Bookmark, Bookmarks};
use winweaver_core::catalog::{Action, Catalog, Param, Rule, check};
use winweaver_core::diff::{Diff, Side};
use winweaver_core::profile::{self, Profile};
use winweaver_core::resolver::{self, State as RuleState};
use winweaver_core::{clock, render, thispc, validate};

use crate::{Mode, RegistryAccess, Server, State, WsDoc};

/// The longest text of one tool result; longer results are cut with a note.
const TEXT_LIMIT: usize = 60_000;
const LIST_LIMIT: usize = 200;
/// The most places and keys a search of the registry gives and reads.
const SEARCH_LIMIT: usize = 500;
const SEARCH_KEYS: usize = 100_000;
const PRESET_IDS: [&str; 4] = ["office", "strict", "laptop", "home"];
/// The skill for agents, served as a resource.
const SKILL: &str = include_str!("../../../agents/skills/winweaver/SKILL.md");

/// A failed tool call: the kind starts the text the client sees.
pub(crate) struct ToolError {
    pub kind: &'static str,
    pub message: String,
}

fn fail(kind: &'static str, message: impl Into<String>) -> ToolError {
    ToolError {
        kind,
        message: message.into(),
    }
}

type Outcome = Result<Value, ToolError>;

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

/// The definitions of the tools for `tools/list`.
pub(crate) fn definitions() -> Vec<Value> {
    let tool = |name: &str, description: &str, input: Value| json!({ "name": name, "description": description, "inputSchema": input });
    let s = |d: &str| json!({ "type": "string", "description": d });
    vec![
        tool(
            "get_status",
            "The program, the catalog, the mode, the option read_pc and the workspace profiles. Call it first.",
            schema(json!({}), &[]),
        ),
        tool(
            "list_groups",
            "The groups under a parent group (the roots without one), with the counts of rules and of rules in effect in the current profile.",
            schema(
                json!({ "parent": s("A group id; leave it out for the roots") }),
                &[],
            ),
        ),
        tool(
            "list_rules",
            "The rules of a group and its subgroups (all rules without one), with their state in the current profile.",
            schema(
                json!({
                    "group": s("A group id"),
                    "on": { "type": "boolean", "description": "Only the rules in effect (true) or not (false)" },
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": LIST_LIMIT },
                }),
                &[],
            ),
        ),
        tool(
            "get_rule",
            "Everything about one rule: texts, parameters with their values in the current profile, what it writes, dependencies and conflicts.",
            schema(json!({ "id": s("A rule id") }), &["id"]),
        ),
        tool(
            "search_rules",
            "Rules whose id, title, summary or tags hold every word of the query.",
            schema(
                json!({
                    "query": s("Words to look for"),
                    "limit": { "type": "integer", "minimum": 1, "maximum": LIST_LIMIT },
                }),
                &["query"],
            ),
        ),
        tool(
            "get_profile",
            "The current profile: name, forms (passwords and keys hidden), the rules on and their parameters.",
            schema(json!({}), &[]),
        ),
        tool(
            "set_rule",
            "Turns a rule of the current profile on or off. Mode edit.",
            schema(
                json!({ "id": s("A rule id"), "on": { "type": "boolean" } }),
                &["id", "on"],
            ),
        ),
        tool(
            "set_param",
            "Sets a parameter of a rule of the current profile. Mode edit.",
            schema(
                json!({
                    "id": s("A rule id"),
                    "name": s("The parameter name"),
                    "value": { "description": "An option of an enum, a whole number in range, a text, or for a list an array of texts" },
                }),
                &["id", "name", "value"],
            ),
        ),
        tool(
            "set_group",
            "Turns every rule of a group and its subgroups on or off. Mode edit.",
            schema(
                json!({ "group": s("A group id"), "on": { "type": "boolean" } }),
                &["group", "on"],
            ),
        ),
        tool(
            "set_profile_info",
            "Changes the name, author or comment of the current profile. Mode edit.",
            schema(
                json!({
                    "name": s("One line, up to 120 characters"), "author": s("One line"), "comment": s("A text"),
                }),
                &[],
            ),
        ),
        tool(
            "check_profile",
            "Checks the current profile like the window does (F7): errors, warnings and infos.",
            schema(json!({}), &[]),
        ),
        tool(
            "preview_build",
            "Builds the answer file of the current profile in memory and shows its summary and beginning, without writing anything.",
            schema(json!({}), &[]),
        ),
        tool(
            "write_answer_file",
            "Checks the current profile, builds autounattend.xml with its scripts and writes them into the output folder of the program. Mode files.",
            schema(json!({}), &[]),
        ),
        tool(
            "load_profile",
            "Makes a workspace profile current, or loads a preset or a profile file of the profiles folder into the workspace. Mode edit.",
            schema(
                json!({ "name": s("A workspace name, a preset (Office, Strict, Laptop, Home) or a file name of the profiles folder") }),
                &["name"],
            ),
        ),
        tool(
            "save_profile",
            "Saves the current profile into the profiles folder, under its name or the name given. Mode files.",
            schema(
                json!({ "name": s("The file name without the extension") }),
                &[],
            ),
        ),
        tool(
            "new_profile",
            "Adds a new profile to the workspace, empty or a copy of a preset, and makes it current. Mode edit.",
            schema(
                json!({ "name": s("The name of the profile"), "preset": s("Office, Strict, Laptop or Home") }),
                &["name"],
            ),
        ),
        tool(
            "list_profiles",
            "The presets, the profile files of the profiles folder and the workspace profiles.",
            schema(json!({}), &[]),
        ),
        tool(
            "compare_profiles",
            "The differences between two profiles: workspace names, presets or file names.",
            schema(
                json!({ "a": s("The base profile"), "b": s("The other profile") }),
                &["a", "b"],
            ),
        ),
        tool(
            "list_bookmarks",
            "The bookmarks of the user: options of profiles and keys of the registry, with their comments. The index names a bookmark for the other tools.",
            schema(json!({}), &[]),
        ),
        tool(
            "add_bookmark",
            "Adds a bookmark with a comment: of an option (kind option, item the id of a rule, path the profile file or empty) or of the registry (kind registry, path the key, item the value or empty). Needs the option edit_library.",
            schema(
                json!({
                    "kind": s("option or registry"),
                    "item": s("The id of a rule, or the value of the key"),
                    "path": s("The profile file of an option, or the key of the registry"),
                    "comment": s("A comment")
                }),
                &["kind", "item"],
            ),
        ),
        tool(
            "set_bookmark_comment",
            "Changes the comment of a bookmark. Needs the option edit_library.",
            schema(
                json!({ "index": { "type": "integer", "description": "The index of list_bookmarks" }, "comment": s("The comment") }),
                &["index", "comment"],
            ),
        ),
        tool(
            "remove_bookmark",
            "Removes a bookmark. Needs the option edit_library.",
            schema(
                json!({ "index": { "type": "integer", "description": "The index of list_bookmarks" } }),
                &["index"],
            ),
        ),
        tool(
            "list_catalogs",
            "The catalog files of the imported policy templates next to the program: id, name and size. Only the first bytes of each are read.",
            schema(json!({}), &[]),
        ),
        tool(
            "get_catalog",
            "A catalog file: without a group its groups with their counts of policies, with a group the policies of that group (id, title).",
            schema(
                json!({ "id": s("The id of list_catalogs"), "group": s("A group of the file") }),
                &["id"],
            ),
        ),
        tool(
            "get_catalog_policy",
            "A policy of a catalog file as JSON (texts, parameters, registry actions), with its Disabled rule when it has one.",
            schema(
                json!({ "id": s("The id of the catalog"), "policy": s("The id of the policy") }),
                &["id", "policy"],
            ),
        ),
        tool(
            "set_catalog_policy",
            "Writes a policy into a catalog file, a new one or in place of the one of its id, as JSON in the form of get_catalog_policy; the whole file is checked, then saved. The program reads it at its next load. Needs the option edit_library.",
            schema(
                json!({ "id": s("The id of the catalog"), "policy": { "type": "object", "description": "The policy" } }),
                &["id", "policy"],
            ),
        ),
        tool(
            "remove_catalog_policy",
            "Removes a policy with its Disabled rule from a catalog file and saves it. Needs the option edit_library.",
            schema(
                json!({ "id": s("The id of the catalog"), "policy": s("The id of the policy") }),
                &["id", "policy"],
            ),
        ),
        tool(
            "read_registry_key",
            "Reads a key of the registry of this PC: its subkeys and its values with their types (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY, REG_NONE). A path like HKEY_LOCAL_MACHINE\\Software or HKLM\\Software. Needs the option registry read or write.",
            schema(json!({ "path": s("The key") }), &["path"]),
        ),
        tool(
            "search_registry",
            "Searches a key of the registry of this PC and every key under it, as Find of regedit: the names of keys, the names of values and the data of values (texts and numbers), without regard to case. Gives the keys and values found (at most limit: 100 by default, 500 at most, within 100000 keys read) and whether every key was read. Needs the option registry read or write.",
            schema(
                json!({
                    "path": s("The key to search under"),
                    "text": s("The text looked for"),
                    "keys": { "type": "boolean", "description": "Look at the names of keys; true by default" },
                    "values": { "type": "boolean", "description": "Look at the names of values; true by default" },
                    "data": { "type": "boolean", "description": "Look at the data of values; true by default" },
                    "whole": { "type": "boolean", "description": "Match the whole name or data only; false by default" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": SEARCH_LIMIT }
                }),
                &["path", "text"],
            ),
        ),
        tool(
            "set_registry_value",
            "Writes a value of a key that exists. The data is text: lines for REG_MULTI_SZ, a decimal or 0x hexadecimal number for REG_DWORD and REG_QWORD, hexadecimal pairs for REG_BINARY. Needs the option registry write.",
            schema(
                json!({
                    "path": s("The key"),
                    "name": s("The value; empty for the default value of the key"),
                    "type": s("REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD or REG_BINARY"),
                    "data": s("The data as text")
                }),
                &["path", "name", "type", "data"],
            ),
        ),
        tool(
            "delete_registry_value",
            "Deletes a value of a key. Needs the option registry write.",
            schema(
                json!({ "path": s("The key"), "name": s("The value") }),
                &["path", "name"],
            ),
        ),
        tool(
            "create_registry_key",
            "Creates a key and the keys above it that are missing. Needs the option registry write.",
            schema(json!({ "path": s("The new key") }), &["path"]),
        ),
        tool(
            "delete_registry_key",
            "Deletes a key with all its subkeys and values; a hive is never deleted. Needs the option registry write.",
            schema(json!({ "path": s("The key") }), &["path"]),
        ),
        tool(
            "read_this_pc",
            "Reads the settings of this PC into a new workspace profile (on the base of the current one). Needs the option read_pc and mode edit; reads only.",
            schema(json!({}), &[]),
        ),
    ]
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

fn need(state: &State, mode: Mode) -> Result<(), ToolError> {
    if state.options.mode >= mode {
        Ok(())
    } else {
        Err(fail(
            "not_allowed",
            format!(
                "the server is in the mode {}; this tool needs the mode {} (the user switches it in the window or with --mode)",
                state.options.mode.name(),
                mode.name()
            ),
        ))
    }
}

fn arg<'a>(args: &'a Value, name: &str) -> Option<&'a Value> {
    args.get(name).filter(|v| !v.is_null())
}

/// The bookmarks and the catalog files: read always, changed only when the user allows it.
fn library_allowed(state: &State) -> Result<(), ToolError> {
    if state.options.edit_library {
        Ok(())
    } else {
        Err(fail(
            "not_allowed",
            "changing the bookmarks and the catalogs is off; the user allows it in the window (MCP tab) or with --edit-library",
        ))
    }
}

fn load_marks(state: &State) -> Result<Bookmarks, ToolError> {
    Bookmarks::load_or_default(&state.options.marks_path)
        .map_err(|e| fail("read_failed", e.to_string()))
}

fn save_marks(state: &State, marks: &Bookmarks) -> Result<(), ToolError> {
    marks
        .save(&state.options.marks_path)
        .map_err(|e| fail("write_failed", e.to_string()))
}

/// The comment of a bookmark, at most 4000 characters without control characters but line breaks.
fn comment_arg(args: &Value) -> Result<String, ToolError> {
    let comment = optional_text(args, "comment");
    if comment.chars().count() > 4000
        || comment
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(fail(
            "invalid",
            "a comment has at most 4000 characters and no control characters",
        ));
    }
    Ok(comment)
}

/// A catalog file of the imports folder by its id (the name of the file), read and checked; its path.
fn load_catalog(state: &State, args: &Value) -> Result<(CatalogFile, PathBuf), ToolError> {
    let id = text_arg(args, "id")?;
    if id.is_empty()
        || id.len() > 120
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || id.starts_with('.')
    {
        return Err(fail(
            "invalid",
            "id is the name of a catalog of list_catalogs",
        ));
    }
    let path = state
        .options
        .imports_dir
        .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
    let file = CatalogFile::load(&path).map_err(|e| fail("not_found", e))?;
    Ok((file, path))
}

/// A text argument that may be empty or left out.
fn optional_text(args: &Value, name: &str) -> String {
    args.get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The tools of the registry: off unless the user allows them, writing only when allowed too.
fn registry_allowed(state: &State, write: bool) -> Result<(), ToolError> {
    match (state.options.registry, write) {
        (RegistryAccess::Write, _) | (RegistryAccess::Read, false) => Ok(()),
        (RegistryAccess::Read, true) => Err(fail(
            "not_allowed",
            "writing the registry is off; the user allows it in the window (MCP tab) or with --registry write",
        )),
        (RegistryAccess::Off, _) => Err(fail(
            "not_allowed",
            "the registry is off; the user allows it in the window (MCP tab) or with --registry read or write",
        )),
    }
}

fn text_arg(args: &Value, name: &str) -> Result<String, ToolError> {
    arg(args, name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| fail("invalid", format!("the argument {name} must be a text")))
}

fn bool_arg(args: &Value, name: &str) -> Result<bool, ToolError> {
    arg(args, name).and_then(Value::as_bool).ok_or_else(|| {
        fail(
            "invalid",
            format!("the argument {name} must be true or false"),
        )
    })
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

fn state_text(state: &RuleState) -> &'static str {
    match state {
        RuleState::On => "on",
        RuleState::Off => "off",
        RuleState::Blocked { .. } => "blocked",
    }
}

fn rule_brief(rule: &Rule, resolution: &resolver::Resolution) -> Value {
    json!({
        "id": rule.id,
        "title": rule.title,
        "group": rule.group,
        "state": state_text(resolution.state(&rule.id)),
        "level": format!("{:?}", rule.level).to_lowercase(),
        "params": rule.params.keys().collect::<Vec<_>>(),
    })
}

fn describe(action: &Action) -> String {
    match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            ..
        } => format!("registry {path}\\{name} = {value} ({kind:?})"),
        Action::RegRemove { path, name, .. } => format!("registry {path}\\{name} removed"),
        Action::RegList {
            path,
            param,
            additive,
            ..
        } => {
            let how = if *additive { "added to" } else { "replacing" };
            format!("registry list {path} from the parameter {param}, {how} its values")
        }
        Action::Service { name, start, .. } => format!("service {name} start type {start}"),
        Action::Exe { file, args } => format!("runs {file} {}", args.join(" ")),
        Action::Feature { name, state, .. } => format!("optional feature {name} {state}"),
        Action::Capability { pattern } => format!("capabilities {pattern} removed"),
        Action::Appx { names } => format!("apps removed: {}", names.join(", ")),
        Action::Ps { script } => format!("a script of {} lines", script.lines().len()),
        Action::XmlSetting {
            pass,
            component,
            path,
            value,
        } => format!("answer file {pass} {component} {path} = {value}"),
        Action::XmlOobe { element, value } => format!("answer file OOBE {element} = {value}"),
        Action::XmlPeCommand { command, .. } | Action::XmlSpecializeCommand { command, .. } => {
            format!("answer file command {command}")
        }
    }
}

fn param_json(catalog: &Catalog, p: &Profile, rule: &Rule, name: &str, param: &Param) -> Value {
    let value = p.param(catalog, &rule.id, name).unwrap_or(Value::Null);
    match param {
        Param::Enum {
            title,
            default,
            values,
            ..
        } => json!({
            "name": name, "type": "enum", "title": title, "default": default, "value": value,
            "options": values.iter().map(|v| json!({ "value": v.value, "title": v.title })).collect::<Vec<_>>(),
        }),
        Param::Int {
            title,
            default,
            min,
            max,
        } => json!({
            "name": name, "type": "int", "title": title, "default": default, "value": value, "min": min, "max": max,
        }),
        Param::Text {
            title,
            default,
            required,
        } => json!({
            "name": name, "type": "string", "title": title, "default": default, "value": value, "required": required,
        }),
        Param::List { title, default } => json!({
            "name": name, "type": "list", "title": title, "default": default, "value": value,
        }),
    }
}

/// Whether a rule matches every word of a query.
fn matches(rule: &Rule, words: &[String]) -> bool {
    let hay = format!(
        "{} {} {} {}",
        rule.id,
        rule.title,
        rule.summary,
        rule.tags.join(" ")
    )
    .to_lowercase();
    words.iter().all(|w| hay.contains(w.as_str()))
}

/// A file name that stays inside the profiles folder.
fn safe_file_name(name: &str) -> Result<String, ToolError> {
    let stem = name.strip_suffix(".wwprofile").unwrap_or(name).trim();
    let ok = !stem.is_empty()
        && stem.len() <= 120
        && stem != "."
        && stem != ".."
        && !stem.chars().any(|c| {
            matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
        });
    if ok {
        Ok(stem.to_owned())
    } else {
        Err(fail(
            "invalid",
            "a profile name may not hold a path, control characters or / \\ : * ? \" < > |",
        ))
    }
}

fn profile_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".wwprofile"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A profile by name: a workspace profile, a preset or a file of the profiles folder.
fn find_profile(
    server: &Server,
    state: &State,
    name: &str,
) -> Result<(Profile, Option<PathBuf>), ToolError> {
    if let Some(d) = state.docs.get(name) {
        return Ok((d.profile.clone(), d.file.clone()));
    }
    if let Some((_, p)) = server
        .presets
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
    {
        return Ok((p.clone(), None));
    }
    let stem = safe_file_name(name)?;
    let file = state.options.profiles_dir.join(format!("{stem}.wwprofile"));
    if !file.is_file() {
        return Err(fail(
            "not_found",
            format!("no workspace profile, preset or file named {name}"),
        ));
    }
    Profile::load(&file)
        .map(|p| (p, Some(file)))
        .map_err(|e| fail("read_failed", e.to_string()))
}

fn unique_name(state: &State, name: &str) -> String {
    if !state.docs.contains_key(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} ({n})"))
        .find(|candidate| !state.docs.contains_key(candidate))
        .unwrap_or_else(|| name.to_owned())
}

fn one_line(text: &str, limit: usize) -> Result<String, ToolError> {
    if text.chars().any(char::is_control) || text.chars().count() > limit {
        return Err(fail(
            "invalid",
            format!("one line of at most {limit} characters"),
        ));
    }
    Ok(text.trim().to_owned())
}

/// Runs a tool.
pub(crate) fn call(server: &Server, state: &mut State, name: &str, args: &Value) -> Outcome {
    if !args.is_object() {
        return Err(fail("invalid", "the arguments must be an object"));
    }
    let catalog = &server.catalog;
    match name {
        "get_status" => {
            let doc = current(state);
            let resolution = resolver::resolve(catalog, &doc.profile);
            let on = catalog
                .rules()
                .iter()
                .filter(|r| resolution.in_effect(&r.id))
                .count();
            Ok(json!({
                "program": "WinWeaver",
                "version": winweaver_core::APP_VERSION,
                "commit": winweaver_core::COMMIT,
                "catalog": { "version": catalog.version(), "rules": catalog.rules().len(), "groups": catalog.groups().len() },
                "mode": state.options.mode.name(),
                "read_pc": state.options.read_pc,
                "current": state.current,
                "current_rules_in_effect": on,
                "workspace": state.docs.iter().map(|(n, d)| json!({
                    "name": n,
                    "changed": d.dirty,
                    "file": d.file.as_ref().and_then(|f| f.file_name()).map(|f| f.to_string_lossy().into_owned()),
                })).collect::<Vec<_>>(),
            }))
        }
        "list_groups" => {
            let parent = arg(args, "parent").and_then(Value::as_str);
            if let Some(p) = parent
                && catalog.group(p).is_none()
            {
                return Err(fail("not_found", format!("no group {p}")));
            }
            let resolution = resolver::resolve(catalog, &current(state).profile);
            let groups: Vec<Value> = catalog
                .child_groups(parent)
                .iter()
                .map(|g| {
                    let rules = catalog.rules_under(&g.id);
                    json!({
                        "id": g.id,
                        "title": g.title,
                        "summary": g.summary,
                        "rules": rules.len(),
                        "in_effect": rules.iter().filter(|r| resolution.in_effect(&r.id)).count(),
                        "subgroups": catalog.child_groups(Some(&g.id)).len(),
                    })
                })
                .collect();
            Ok(json!({ "parent": parent, "groups": groups }))
        }
        "list_rules" => {
            let group = arg(args, "group").and_then(Value::as_str);
            let rules: Vec<&Rule> = match group {
                Some(g) if catalog.group(g).is_none() => {
                    return Err(fail("not_found", format!("no group {g}")));
                }
                Some(g) => catalog.rules_under(g),
                None => catalog.rules().iter().collect(),
            };
            let resolution = resolver::resolve(catalog, &current(state).profile);
            let on = arg(args, "on").and_then(Value::as_bool);
            let filtered: Vec<&Rule> = rules
                .into_iter()
                .filter(|r| on.is_none_or(|want| resolution.in_effect(&r.id) == want))
                .collect();
            let offset = arg(args, "offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let limit = (arg(args, "limit").and_then(Value::as_u64).unwrap_or(50) as usize)
                .clamp(1, LIST_LIMIT);
            let page: Vec<Value> = filtered
                .iter()
                .skip(offset)
                .take(limit)
                .map(|r| rule_brief(r, &resolution))
                .collect();
            Ok(json!({ "total": filtered.len(), "offset": offset, "rules": page }))
        }
        "get_rule" => {
            let id = text_arg(args, "id")?;
            let rule = catalog
                .rule(&id)
                .ok_or_else(|| fail("not_found", format!("no rule {id}")))?;
            let doc = current(state);
            let resolution = resolver::resolve(catalog, &doc.profile);
            let blocked_by = match resolution.state(&id) {
                RuleState::Blocked { by } => Some(by.clone()),
                _ => None,
            };
            Ok(json!({
                "id": rule.id,
                "title": rule.title,
                "summary": rule.summary,
                "effect": rule.effect,
                "risk": rule.risk,
                "versions": rule.versions,
                "verify": rule.verify,
                "rollback": rule.rollback,
                "group": rule.group,
                "group_path": catalog.path_of(&rule.group),
                "phase": format!("{:?}", rule.phase),
                "level": format!("{:?}", rule.level).to_lowercase(),
                "on_by_default": rule.default,
                "mechanism": format!("{:?}", Catalog::mechanism(rule)).to_lowercase(),
                "state": state_text(resolution.state(&id)),
                "chosen_on": doc.profile.is_on(&id),
                "blocked_by": blocked_by,
                "linked_by": resolution.linked_by(&id),
                "params": rule.params.iter().map(|(n, p)| param_json(catalog, &doc.profile, rule, n, p)).collect::<Vec<_>>(),
                "requires": rule.requires,
                "conflicts": rule.conflicts,
                "writes": rule.actions.iter().map(describe).collect::<Vec<_>>(),
            }))
        }
        "search_rules" => {
            let query = text_arg(args, "query")?;
            let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
            if words.is_empty() {
                return Err(fail("invalid", "the query is empty"));
            }
            let limit = (arg(args, "limit").and_then(Value::as_u64).unwrap_or(20) as usize)
                .clamp(1, LIST_LIMIT);
            let resolution = resolver::resolve(catalog, &current(state).profile);
            let found: Vec<&Rule> = catalog
                .rules()
                .iter()
                .filter(|r| matches(r, &words))
                .collect();
            Ok(json!({
                "total": found.len(),
                "rules": found.iter().take(limit).map(|r| rule_brief(r, &resolution)).collect::<Vec<_>>(),
            }))
        }
        "get_profile" => {
            let doc = current(state);
            let resolution = resolver::resolve(catalog, &doc.profile);
            let on: Vec<&str> = catalog
                .rules()
                .iter()
                .filter(|r| resolution.in_effect(&r.id))
                .map(|r| r.id.as_str())
                .collect();
            let mut value = redacted(&doc.profile);
            if let Some(obj) = value.as_object_mut() {
                obj.remove("rules");
                obj.insert("rules_in_effect".to_owned(), json!(on));
                obj.insert(
                    "parameters".to_owned(),
                    json!(
                        doc.profile
                            .rules
                            .iter()
                            .filter(|(_, c)| !c.params.is_empty())
                            .map(|(id, c)| (id.clone(), json!(c.params)))
                            .collect::<serde_json::Map<_, _>>()
                    ),
                );
                obj.insert("changed".to_owned(), json!(doc.dirty));
                obj.insert("unknown".to_owned(), json!(doc.profile.unknown.len()));
            }
            Ok(value)
        }
        "set_rule" => {
            need(state, Mode::Edit)?;
            let id = text_arg(args, "id")?;
            let on = bool_arg(args, "on")?;
            if catalog.rule(&id).is_none() {
                return Err(fail("not_found", format!("no rule {id}")));
            }
            let doc = current_mut(state);
            if doc.profile.is_on(&id) != on {
                doc.profile.set_on(&id, on);
                doc.dirty = true;
            }
            let resolution = resolver::resolve(catalog, &doc.profile);
            let blocked: Vec<&str> = catalog
                .rules()
                .iter()
                .filter(
                    |r| matches!(resolution.state(&r.id), RuleState::Blocked { by } if by == &id),
                )
                .map(|r| r.id.as_str())
                .collect();
            Ok(
                json!({ "id": id, "state": state_text(resolution.state(&id)), "blocked_rules": blocked }),
            )
        }
        "set_param" => {
            need(state, Mode::Edit)?;
            let id = text_arg(args, "id")?;
            let pname = text_arg(args, "name")?;
            let rule = catalog
                .rule(&id)
                .ok_or_else(|| fail("not_found", format!("no rule {id}")))?;
            let param = rule.params.get(&pname).ok_or_else(|| {
                fail(
                    "not_found",
                    format!("the rule {id} has no parameter {pname}"),
                )
            })?;
            let value = arg(args, "value")
                .cloned()
                .ok_or_else(|| fail("invalid", "the argument value is missing"))?;
            let value = match param {
                Param::Enum { values, .. } => {
                    if value.is_boolean() {
                        return Err(fail(
                            "invalid",
                            "an enum takes one of its options, not true or false",
                        ));
                    }
                    values
                        .iter()
                        .find(|v| check::same_value(&v.value, &value))
                        .map(|v| v.value.clone())
                        .ok_or_else(|| {
                            fail("invalid", format!("{value} is not an option of {pname}"))
                        })?
                }
                Param::Int { min, max, .. } => {
                    let n = value
                        .as_i64()
                        .ok_or_else(|| fail("invalid", "a whole number is needed"))?;
                    if !(*min..=*max).contains(&n) {
                        return Err(fail("invalid", format!("{n} is outside {min}..{max}")));
                    }
                    Value::from(n)
                }
                Param::Text { .. } => {
                    let t = value
                        .as_str()
                        .ok_or_else(|| fail("invalid", "a text is needed"))?;
                    Value::from(one_line(t, 500)?)
                }
                Param::List { .. } => {
                    let items = value
                        .as_array()
                        .ok_or_else(|| fail("invalid", "a list of texts is needed"))?;
                    if items.len() > 1000 {
                        return Err(fail("invalid", "a list of at most 1000 items"));
                    }
                    let mut texts = Vec::new();
                    for item in items {
                        let t = item
                            .as_str()
                            .ok_or_else(|| fail("invalid", "every item of a list is a text"))?;
                        let t = one_line(t, 500)?;
                        if !t.is_empty() {
                            texts.push(Value::from(t));
                        }
                    }
                    Value::Array(texts)
                }
            };
            let default = param.default_value();
            let doc = current_mut(state);
            let changed = doc
                .profile
                .param(catalog, &id, &pname)
                .is_none_or(|v| !check::same_value(&v, &value));
            if changed {
                let choice = doc.profile.rules.entry(id.clone()).or_default();
                if check::same_value(&default, &value) {
                    choice.params.shift_remove(&pname);
                } else {
                    choice.params.insert(pname.clone(), value.clone());
                }
                doc.dirty = true;
            }
            Ok(json!({ "id": id, "name": pname, "value": value, "changed": changed }))
        }
        "set_group" => {
            need(state, Mode::Edit)?;
            let group = text_arg(args, "group")?;
            let on = bool_arg(args, "on")?;
            if catalog.group(&group).is_none() {
                return Err(fail("not_found", format!("no group {group}")));
            }
            let ids: Vec<String> = catalog
                .rules_under(&group)
                .iter()
                .map(|r| r.id.clone())
                .collect();
            let doc = current_mut(state);
            let mut changed = 0;
            for id in &ids {
                if doc.profile.is_on(id) != on {
                    doc.profile.set_on(id, on);
                    changed += 1;
                }
            }
            if changed > 0 {
                doc.dirty = true;
            }
            Ok(json!({ "group": group, "rules": ids.len(), "changed": changed }))
        }
        "set_profile_info" => {
            need(state, Mode::Edit)?;
            let name = arg(args, "name")
                .and_then(Value::as_str)
                .map(|t| one_line(t, 120))
                .transpose()?;
            let author = arg(args, "author")
                .and_then(Value::as_str)
                .map(|t| one_line(t, 120))
                .transpose()?;
            let comment = arg(args, "comment")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if name.is_none() && author.is_none() && comment.is_none() {
                return Err(fail("invalid", "give name, author or comment"));
            }
            if name.as_deref() == Some("") {
                return Err(fail("invalid", "the name may not be empty"));
            }
            let key = state.current.clone();
            let doc = current_mut(state);
            let mut changed = false;
            for (field, new) in [
                (&mut doc.profile.name, &name),
                (&mut doc.profile.author, &author),
                (&mut doc.profile.comment, &comment),
            ] {
                if let Some(v) = new
                    && field != v
                {
                    *field = v.clone();
                    changed = true;
                }
            }
            if changed {
                doc.dirty = true;
            }
            let new_name = doc.profile.name.clone();
            if new_name != key && !state.docs.contains_key(&new_name) {
                if let Some(d) = state.docs.shift_remove(&key) {
                    state.docs.insert(new_name.clone(), d);
                }
                state.current = new_name;
            }
            Ok(json!({ "current": state.current, "changed": changed }))
        }
        "check_profile" => {
            let doc = current(state);
            let resolution = resolver::resolve(catalog, &doc.profile);
            let issues = validate::check(catalog, &doc.profile, &resolution);
            Ok(json!({
                "summary": validate::summary(&issues),
                "issues": issues.iter().take(LIST_LIMIT).map(|i| json!({
                    "severity": format!("{:?}", i.severity).to_lowercase(),
                    "target": match &i.target {
                        validate::Target::Rule(id) => id.clone(),
                        other => format!("{other:?}").to_lowercase(),
                    },
                    "text": i.text,
                })).collect::<Vec<_>>(),
            }))
        }
        "preview_build" => {
            let doc = current(state);
            let build = render::build(catalog, &doc.profile, winweaver_core::BUILD)
                .map_err(|e| fail("invalid", format!("the build fails: {e}")))?;
            let mut head: String = build.xml.chars().take(6000).collect();
            head = hide_secrets(&head);
            Ok(json!({
                "rules": build.rule_ids.len(),
                "scripts": build.scripts.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
                "warnings": build.warnings,
                "characters": build.xml.len(),
                "beginning": head,
                "note": "the window also checks the syntax of the embedded scripts when it builds; this preview does not",
            }))
        }
        "write_answer_file" => {
            need(state, Mode::Files)?;
            let doc = current(state);
            let resolution = resolver::resolve(catalog, &doc.profile);
            let issues = validate::check(catalog, &doc.profile, &resolution);
            if validate::errors(&issues) > 0 {
                let first: Vec<&str> = issues
                    .iter()
                    .filter(|i| i.severity == validate::Severity::Error)
                    .take(5)
                    .map(|i| i.text.as_str())
                    .collect();
                return Err(fail(
                    "invalid",
                    format!("{}: {}", validate::summary(&issues), first.join("; ")),
                ));
            }
            let build = render::build(catalog, &doc.profile, winweaver_core::BUILD)
                .map_err(|e| fail("invalid", format!("the build fails: {e}")))?;
            let dir = state.options.output_dir.clone();
            std::fs::create_dir_all(&dir).map_err(|e| fail("write_failed", e.to_string()))?;
            write(&dir.join("autounattend.xml"), build.xml.as_bytes())?;
            for (name, text) in &build.scripts {
                write(&dir.join(name), text.replace('\n', "\r\n").as_bytes())?;
            }
            Ok(json!({
                "file": "autounattend.xml",
                "folder": "output",
                "scripts": build.scripts.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
                "rules": build.rule_ids.len(),
                "warnings": build.warnings,
            }))
        }
        "load_profile" => {
            need(state, Mode::Edit)?;
            let name = text_arg(args, "name")?;
            if state.docs.contains_key(&name) {
                state.current = name.clone();
                return Ok(json!({ "current": name, "loaded": false }));
            }
            let (mut profile, file) = find_profile(server, state, &name)?;
            let unknown = profile.split_unknown(catalog);
            let key = unique_name(state, &profile.name);
            state.docs.insert(
                key.clone(),
                WsDoc {
                    profile,
                    file,
                    dirty: false,
                },
            );
            state.current = key.clone();
            Ok(json!({ "current": key, "loaded": true, "unknown_choices": unknown.len() }))
        }
        "save_profile" => {
            need(state, Mode::Files)?;
            let name = match arg(args, "name").and_then(Value::as_str) {
                Some(n) => n.to_owned(),
                None => current(state).profile.name.clone(),
            };
            let stem = safe_file_name(&name)?;
            if PRESET_IDS.iter().any(|p| p.eq_ignore_ascii_case(&stem)) {
                return Err(fail(
                    "not_allowed",
                    "the names of the presets are kept for the presets; choose another name",
                ));
            }
            let dir = state.options.profiles_dir.clone();
            std::fs::create_dir_all(&dir).map_err(|e| fail("write_failed", e.to_string()))?;
            let file = dir.join(format!("{stem}.{}", profile::EXTENSION));
            let doc = current_mut(state);
            if doc.profile.created.is_empty() {
                doc.profile.created = clock::now_utc_iso();
            }
            doc.profile.modified = clock::now_utc_iso();
            doc.profile
                .save(&file)
                .map_err(|e| fail("write_failed", e.to_string()))?;
            doc.file = Some(file);
            doc.dirty = false;
            Ok(json!({ "file": format!("{stem}.{}", profile::EXTENSION), "folder": "profiles" }))
        }
        "new_profile" => {
            need(state, Mode::Edit)?;
            let name = one_line(&text_arg(args, "name")?, 120)?;
            if name.is_empty() {
                return Err(fail("invalid", "the name may not be empty"));
            }
            let mut profile = match arg(args, "preset").and_then(Value::as_str) {
                Some(preset) => server
                    .presets
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(preset))
                    .map(|(_, p)| p.clone())
                    .ok_or_else(|| fail("not_found", format!("no preset {preset}")))?,
                None => Profile::new(&name, catalog.version()),
            };
            profile.name = name.clone();
            let key = unique_name(state, &name);
            state.docs.insert(
                key.clone(),
                WsDoc {
                    profile,
                    file: None,
                    dirty: true,
                },
            );
            state.current = key.clone();
            Ok(json!({ "current": key }))
        }
        "list_profiles" => Ok(json!({
            "presets": server.presets.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
            "files": profile_files(&state.options.profiles_dir),
            "workspace": state.docs.keys().cloned().collect::<Vec<_>>(),
            "current": state.current,
        })),
        "compare_profiles" => {
            let (a, a_file) = find_profile(server, state, &text_arg(args, "a")?)?;
            let (b, b_file) = find_profile(server, state, &text_arg(args, "b")?)?;
            let diff = Diff::between(
                &a,
                &b,
                Side::of(&a, a_file.as_deref()),
                Side::of(&b, b_file.as_deref()),
            );
            let state_of = |c: &Option<profile::Choice>| match c {
                None => json!("off"),
                Some(c) if c.params.is_empty() => json!(if c.on { "on" } else { "off" }),
                Some(c) => json!({ "on": c.on, "params": c.params }),
            };
            Ok(json!({
                "differences": diff.count(),
                "rules": diff.rules.iter().take(LIST_LIMIT).map(|(id, d)| json!({ "id": id, "a": state_of(&d.base), "b": state_of(&d.other) })).collect::<Vec<_>>(),
                "install": diff.install.iter().map(|(k, f)| (k.clone(), json!({ "a": f.base, "b": f.other }))).collect::<serde_json::Map<_, _>>(),
                "languages": diff.languages.iter().map(|(k, f)| (k.clone(), json!({ "a": f.base, "b": f.other }))).collect::<serde_json::Map<_, _>>(),
                "accounts_differ": diff.accounts.is_some(),
            }))
        }
        "list_bookmarks" => {
            let marks = load_marks(state)?;
            let items: Vec<Value> = marks
                .items
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    let kind = if b.is_registry() {
                        "registry"
                    } else if b.is_option() {
                        "option"
                    } else {
                        b.document.as_str()
                    };
                    json!({ "index": i, "kind": kind, "item": b.item, "path": b.path, "comment": b.comment, "created": b.created })
                })
                .collect();
            Ok(json!({ "bookmarks": items }))
        }
        "add_bookmark" => {
            library_allowed(state)?;
            let item = optional_text(args, "item");
            let path = optional_text(args, "path");
            let mut mark = match text_arg(args, "kind")?.as_str() {
                "option" if !item.is_empty() => Bookmark::option(&item, &path),
                "registry" => {
                    let key = winweaver_core::registry::normalize(&path)
                        .ok_or_else(|| fail("invalid", "path is not a key of the registry"))?;
                    Bookmark::registry(&key, &item)
                }
                _ => {
                    return Err(fail("invalid", "kind is option (with an item) or registry"));
                }
            };
            mark.comment = comment_arg(args)?;
            mark.created = clock::now_utc_iso();
            let mut marks = load_marks(state)?;
            if marks.find(&mark).is_some() {
                return Err(fail("invalid", "this is bookmarked already"));
            }
            marks.items.push(mark);
            save_marks(state, &marks)?;
            Ok(json!({ "index": marks.items.len() - 1 }))
        }
        "set_bookmark_comment" | "remove_bookmark" => {
            library_allowed(state)?;
            let index = args
                .get("index")
                .and_then(Value::as_u64)
                .ok_or_else(|| fail("invalid", "index is a number of list_bookmarks"))?
                as usize;
            let mut marks = load_marks(state)?;
            if index >= marks.items.len() {
                return Err(fail("not_found", format!("no bookmark {index}")));
            }
            if name == "remove_bookmark" {
                marks.items.remove(index);
            } else {
                marks.items[index].comment = comment_arg(args)?;
            }
            save_marks(state, &marks)?;
            Ok(json!({ "index": index, "done": name }))
        }
        "list_catalogs" => {
            let mut files: Vec<Value> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&state.options.imports_dir) {
                let mut paths: Vec<PathBuf> = entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == admx::CATALOG_EXTENSION))
                    .collect();
                paths.sort();
                for p in paths {
                    let id = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let name = CatalogHeader::read(&p).map_or_else(|| id.clone(), |h| h.name);
                    let bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    files.push(json!({ "id": id, "name": name, "bytes": bytes }));
                }
            }
            Ok(json!({ "catalogs": files }))
        }
        "get_catalog" => {
            let (file, _) = load_catalog(state, args)?;
            match args.get("group").and_then(Value::as_str) {
                None => {
                    let groups: Vec<Value> = file
                        .groups
                        .iter()
                        .map(|g| {
                            let n = file
                                .rules
                                .iter()
                                .filter(|r| r.group == g.id && !r.id.ends_with(".off"))
                                .count();
                            json!({ "id": g.id, "title": g.title, "parent": g.parent, "policies": n })
                        })
                        .collect();
                    Ok(json!({ "id": file.id, "name": file.name, "groups": groups }))
                }
                Some(group) => {
                    let policies: Vec<Value> = file
                        .rules
                        .iter()
                        .filter(|r| r.group == group && !r.id.ends_with(".off"))
                        .map(|r| json!({ "id": r.id, "title": r.title }))
                        .collect();
                    Ok(json!({ "group": group, "policies": policies }))
                }
            }
        }
        "get_catalog_policy" => {
            let (file, _) = load_catalog(state, args)?;
            let policy = text_arg(args, "policy")?;
            let rule = file
                .rules
                .iter()
                .find(|r| r.id == policy)
                .ok_or_else(|| fail("not_found", format!("no policy {policy}")))?;
            let off = file.rules.iter().find(|r| r.id == format!("{policy}.off"));
            Ok(json!({ "policy": rule, "off": off }))
        }
        "set_catalog_policy" | "remove_catalog_policy" => {
            library_allowed(state)?;
            let (mut file, path) = load_catalog(state, args)?;
            if name == "remove_catalog_policy" {
                let policy = text_arg(args, "policy")?;
                file.remove_rule(&policy)
                    .map_err(|e| fail("not_found", e))?;
            } else {
                let rule: Rule =
                    serde_json::from_value(args.get("policy").cloned().unwrap_or_default())
                        .map_err(|e| fail("invalid", format!("policy: {e}")))?;
                file.put_rule(rule).map_err(|e| fail("invalid", e))?;
            }
            file.save(&path).map_err(|e| fail("write_failed", e))?;
            Ok(json!({ "saved": file.id, "policies": file.rules.len() }))
        }
        "read_registry_key" => {
            registry_allowed(state, false)?;
            let path = text_arg(args, "path")?;
            let key = server
                .registry
                .read(&path)
                .map_err(|e| fail("read_failed", e))?;
            Ok(json!(key))
        }
        "search_registry" => {
            registry_allowed(state, false)?;
            let path = text_arg(args, "path")?;
            if winweaver_core::registry::normalize(&path).is_none() {
                return Err(fail(
                    "invalid",
                    format!("not a key of the registry: {path}"),
                ));
            }
            let flag = |name: &str, default: bool| {
                args.get(name).and_then(Value::as_bool).unwrap_or(default)
            };
            let query = winweaver_core::registry::Query {
                text: text_arg(args, "text")?.trim().to_owned(),
                keys: flag("keys", true),
                values: flag("values", true),
                data: flag("data", true),
                whole: flag("whole", false),
            };
            if query.text.is_empty() {
                return Err(fail("invalid", "the argument text must not be empty"));
            }
            let limit = match args.get("limit").and_then(Value::as_u64) {
                Some(n) => n.clamp(1, SEARCH_LIMIT as u64) as usize,
                None => 100,
            };
            let (stop, read) = (AtomicBool::new(false), AtomicUsize::new(0));
            let found = winweaver_core::registry::search(
                &*server.registry,
                &path,
                &query,
                limit,
                SEARCH_KEYS,
                &stop,
                &read,
            );
            Ok(json!(found))
        }
        "set_registry_value" => {
            registry_allowed(state, true)?;
            let path = text_arg(args, "path")?;
            let name = optional_text(args, "name");
            let kind = text_arg(args, "type")?;
            let data = winweaver_core::registry::Data::parse(&kind, &optional_text(args, "data"))
                .map_err(|e| fail("invalid", e))?;
            server
                .registry
                .set(&path, &name, &data)
                .map_err(|e| fail("write_failed", e))?;
            Ok(json!({ "written": path, "name": name, "type": kind }))
        }
        "delete_registry_value" => {
            registry_allowed(state, true)?;
            let path = text_arg(args, "path")?;
            let name = optional_text(args, "name");
            server
                .registry
                .delete_value(&path, &name)
                .map_err(|e| fail("write_failed", e))?;
            Ok(json!({ "deleted": path, "name": name }))
        }
        "create_registry_key" => {
            registry_allowed(state, true)?;
            let path = text_arg(args, "path")?;
            server
                .registry
                .create_key(&path)
                .map_err(|e| fail("write_failed", e))?;
            Ok(json!({ "created": path }))
        }
        "delete_registry_key" => {
            registry_allowed(state, true)?;
            let path = text_arg(args, "path")?;
            server
                .registry
                .delete_key(&path)
                .map_err(|e| fail("write_failed", e))?;
            Ok(json!({ "deleted": path }))
        }
        "read_this_pc" => {
            if !state.options.read_pc {
                return Err(fail(
                    "not_allowed",
                    "reading this PC is off; the user allows it in the window (MCP tab) or with --read-pc",
                ));
            }
            need(state, Mode::Edit)?;
            let machine = winweaver_win::this_pc()
                .ok_or_else(|| fail("read_failed", "this PC can be read on Windows only"))?;
            let base = current(state).profile.clone();
            let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
            let plan = thispc::plan(catalog, &base, &ids);
            let report = thispc::run(&*machine, &plan, true, &clock::now_utc_iso());
            let readings = thispc::judge(catalog, &plan, &report.results);
            let (mut profile, summary) =
                thispc::capture(catalog, &base, &readings, report.system.as_ref());
            profile.name = if report.computer.is_empty() {
                "This PC".to_owned()
            } else {
                report.computer.clone()
            };
            let key = unique_name(state, &profile.name);
            state.docs.insert(
                key.clone(),
                WsDoc {
                    profile,
                    file: None,
                    dirty: true,
                },
            );
            state.current = key.clone();
            Ok(json!({
                "current": key,
                "summary": summary.line(),
                "forms": summary.forms,
                "notes": summary.notes,
                "partly": summary.partly.iter().map(|(id, differs)| json!({ "id": id, "differs": differs })).collect::<Vec<_>>(),
                "not_read": plan.excluded.len(),
            }))
        }
        _ => Err(fail("not_found", format!("no tool {name}"))),
    }
}

/// Loads a preset or a profile file into the workspace and makes it current.
pub(crate) fn select(server: &Server, state: &mut State, name: &str) -> Result<(), ToolError> {
    let (mut profile, file) = find_profile(server, state, name)?;
    profile.split_unknown(&server.catalog);
    let key = unique_name(state, &profile.name);
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

fn write(path: &Path, bytes: &[u8]) -> Result<(), ToolError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| {
        fail(
            "write_failed",
            format!(
                "{}: {e}",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
        )
    })?;
    std::fs::rename(&tmp, path).map_err(|e| fail("write_failed", e.to_string()))
}

/// The answer file text with the product key and the passwords hidden.
fn hide_secrets(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    loop {
        let next = ["<Key>", "<Value>"]
            .iter()
            .filter_map(|tag| rest.find(tag).map(|i| (i, *tag)))
            .min_by_key(|(i, _)| *i);
        let Some((i, tag)) = next else {
            out.push_str(rest);
            return out;
        };
        let start = i + tag.len();
        out.push_str(&rest[..start]);
        let close = tag.replace('<', "</");
        match rest[start..].find(&close) {
            Some(end) if end > 0 => {
                out.push_str("***");
                rest = &rest[start + end..];
            }
            _ => rest = &rest[start..],
        }
    }
}

#[cfg(test)]
mod tests {
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
}
