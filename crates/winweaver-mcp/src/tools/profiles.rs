//! The tools of the profiles: the status, the workspace, the check and the build, the files of
//! the profiles folder, this PC.

use std::path::{Path, PathBuf};

use serde_json::json;
use winweaver_core::diff::{Diff, Side};
use winweaver_core::profile::{self, Profile};
use winweaver_core::resolver;
use winweaver_core::{clock, render, thispc, validate};

use super::{
    Args, Kind, LIST_LIMIT, Needs, Outcome, Tool, ToolError, current, current_mut, fail, redacted,
    resolution, s, schema,
};
use crate::{Server, State, WsDoc};

const PRESET_IDS: [&str; 4] = ["office", "strict", "laptop", "home"];
/// How much of the answer file `preview_build` shows.
const PREVIEW_CHARS: usize = 6000;

pub(super) const GET_STATUS: Tool = Tool {
    name: "get_status",
    description: "The program, the catalog, the mode, the option read_pc and the workspace profiles. Call it first.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: get_status,
};

pub(super) const GET_PROFILE: Tool = Tool {
    name: "get_profile",
    description: "The current profile: name, forms (passwords and keys hidden), the rules on and their parameters.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: get_profile,
};

pub(super) const SET_PROFILE_INFO: Tool = Tool {
    name: "set_profile_info",
    description: "Changes the name, author or comment of the current profile. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({
                "name": s("One line, up to 120 characters"), "author": s("One line"), "comment": s("A text"),
            }),
            &[],
        )
    },
    run: set_profile_info,
};

pub(super) const CHECK_PROFILE: Tool = Tool {
    name: "check_profile",
    description: "Checks the current profile like the window does (F7): errors, warnings and infos.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: check_profile,
};

pub(super) const PREVIEW_BUILD: Tool = Tool {
    name: "preview_build",
    description: "Builds the answer file of the current profile in memory and shows its summary and beginning, without writing anything.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: preview_build,
};

pub(super) const WRITE_ANSWER_FILE: Tool = Tool {
    name: "write_answer_file",
    description: "Checks the current profile, builds autounattend.xml with its scripts and writes them into the output folder of the program. Mode files.",
    needs: Needs::FILES,
    schema: || schema(json!({}), &[]),
    run: write_answer_file,
};

pub(super) const LOAD_PROFILE: Tool = Tool {
    name: "load_profile",
    description: "Makes a workspace profile current, or loads a preset or a profile file of the profiles folder into the workspace. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({ "name": s("A workspace name, a preset (Office, Strict, Laptop, Home) or a file name of the profiles folder") }),
            &["name"],
        )
    },
    run: load_profile,
};

pub(super) const SAVE_PROFILE: Tool = Tool {
    name: "save_profile",
    description: "Saves the current profile into the profiles folder, under its name or the name given. Mode files.",
    needs: Needs::FILES,
    schema: || {
        schema(
            json!({ "name": s("The file name without the extension") }),
            &[],
        )
    },
    run: save_profile,
};

pub(super) const NEW_PROFILE: Tool = Tool {
    name: "new_profile",
    description: "Adds a new profile to the workspace, empty or a copy of a preset, and makes it current. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({ "name": s("The name of the profile"), "preset": s("Office, Strict, Laptop or Home") }),
            &["name"],
        )
    },
    run: new_profile,
};

pub(super) const LIST_PROFILES: Tool = Tool {
    name: "list_profiles",
    description: "The presets, the profile files of the profiles folder and the workspace profiles.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: list_profiles,
};

pub(super) const COMPARE_PROFILES: Tool = Tool {
    name: "compare_profiles",
    description: "The differences between two profiles: workspace names, presets or file names. A product key shows as ***, the accounts only as differing.",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({ "a": s("The base profile"), "b": s("The other profile") }),
            &["a", "b"],
        )
    },
    run: compare_profiles,
};

pub(super) const READ_THIS_PC: Tool = Tool {
    name: "read_this_pc",
    description: "Reads the settings of this PC into a new workspace profile (on the base of the current one). Needs the option read_pc and mode edit; reads only.",
    needs: Needs::THIS_PC,
    schema: || schema(json!({}), &[]),
    run: read_this_pc,
};

/// A file name that stays inside the profiles folder.
pub(super) fn safe_file_name(name: &str) -> Result<String, ToolError> {
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
            Kind::Invalid,
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
pub(super) fn find_profile(
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
            Kind::NotFound,
            format!("no workspace profile, preset or file named {name}"),
        ));
    }
    Profile::load(&file)
        .map(|p| (p, Some(file)))
        .map_err(|e| fail(Kind::ReadFailed, e.to_string()))
}

pub(super) fn unique_name(state: &State, name: &str) -> String {
    if !state.docs.contains_key(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} ({n})"))
        .find(|candidate| !state.docs.contains_key(candidate))
        .unwrap_or_else(|| name.to_owned())
}

pub(super) fn one_line(text: &str, limit: usize) -> Result<String, ToolError> {
    if text.chars().any(char::is_control) || text.chars().count() > limit {
        return Err(fail(
            Kind::Invalid,
            format!("one line of at most {limit} characters"),
        ));
    }
    Ok(text.trim().to_owned())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), ToolError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| {
        fail(
            Kind::WriteFailed,
            format!(
                "{}: {e}",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
        )
    })?;
    std::fs::rename(&tmp, path).map_err(|e| fail(Kind::WriteFailed, e.to_string()))
}

/// The answer file text with the product key and the passwords hidden. A value without its
/// closing tag (a text that was cut) is hidden to its end: no part of a secret is ever shown.
pub(super) fn hide_secrets(xml: &str) -> String {
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
            Some(0) => rest = &rest[start..],
            Some(end) => {
                out.push_str("***");
                rest = &rest[start + end..];
            }
            None => {
                if start < rest.len() {
                    out.push_str("***");
                }
                return out;
            }
        }
    }
}

/// The first `limit` characters of an answer file for a preview: the secrets of the whole text
/// are hidden first and the text is cut after, so the cut never falls inside a secret.
pub(super) fn beginning(xml: &str, limit: usize) -> String {
    hide_secrets(xml).chars().take(limit).collect()
}

/// A new workspace profile, made current.
fn add_doc(state: &mut State, profile: Profile, file: Option<PathBuf>, dirty: bool) -> String {
    let key = unique_name(state, &profile.name);
    state.docs.insert(
        key.clone(),
        WsDoc {
            profile,
            file,
            dirty,
        },
    );
    state.current = key.clone();
    key
}

fn get_status(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let resolution = resolution(server, state);
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

fn get_profile(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
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

fn set_profile_info(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let name = args.str("name").map(|t| one_line(t, 120)).transpose()?;
    let author = args.str("author").map(|t| one_line(t, 120)).transpose()?;
    let comment = args.str("comment").map(str::to_owned);
    if name.is_none() && author.is_none() && comment.is_none() {
        return Err(fail(Kind::Invalid, "give name, author or comment"));
    }
    if name.as_deref() == Some("") {
        return Err(fail(Kind::Invalid, "the name may not be empty"));
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

fn check_profile(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
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

fn preview_build(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let doc = current(state);
    let build = render::build(&server.catalog, &doc.profile, winweaver_core::BUILD)
        .map_err(|e| fail(Kind::Invalid, format!("the build fails: {e}")))?;
    let head = beginning(&build.xml, PREVIEW_CHARS);
    Ok(json!({
        "rules": build.rule_ids.len(),
        "scripts": build.scripts.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        "warnings": build.warnings,
        "characters": build.xml.len(),
        "beginning": head,
        "note": "the window also checks the syntax of the embedded scripts when it builds; this preview does not",
    }))
}

fn write_answer_file(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
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
            Kind::Invalid,
            format!("{}: {}", validate::summary(&issues), first.join("; ")),
        ));
    }
    let build = render::build(catalog, &doc.profile, winweaver_core::BUILD)
        .map_err(|e| fail(Kind::Invalid, format!("the build fails: {e}")))?;
    let dir = state.options.output_dir.clone();
    std::fs::create_dir_all(&dir).map_err(|e| fail(Kind::WriteFailed, e.to_string()))?;
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

fn load_profile(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let name = args.text("name")?;
    if state.docs.contains_key(&name) {
        state.current = name.clone();
        return Ok(json!({ "current": name, "loaded": false }));
    }
    let (mut profile, file) = find_profile(server, state, &name)?;
    let unknown = profile.split_unknown(&server.catalog);
    let key = add_doc(state, profile, file, false);
    Ok(json!({ "current": key, "loaded": true, "unknown_choices": unknown.len() }))
}

fn save_profile(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let name = match args.str("name") {
        Some(n) => n.to_owned(),
        None => current(state).profile.name.clone(),
    };
    let stem = safe_file_name(&name)?;
    if PRESET_IDS.iter().any(|p| p.eq_ignore_ascii_case(&stem)) {
        return Err(fail(
            Kind::NotAllowed,
            "the names of the presets are kept for the presets; choose another name",
        ));
    }
    let dir = state.options.profiles_dir.clone();
    std::fs::create_dir_all(&dir).map_err(|e| fail(Kind::WriteFailed, e.to_string()))?;
    let file = dir.join(format!("{stem}.{}", profile::EXTENSION));
    let doc = current_mut(state);
    if doc.profile.created.is_empty() {
        doc.profile.created = clock::now_utc_iso();
    }
    doc.profile.modified = clock::now_utc_iso();
    doc.profile
        .save(&file)
        .map_err(|e| fail(Kind::WriteFailed, e.to_string()))?;
    doc.file = Some(file);
    doc.dirty = false;
    Ok(json!({ "file": format!("{stem}.{}", profile::EXTENSION), "folder": "profiles" }))
}

fn new_profile(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let name = one_line(&args.text("name")?, 120)?;
    if name.is_empty() {
        return Err(fail(Kind::Invalid, "the name may not be empty"));
    }
    let mut profile = match args.str("preset") {
        Some(preset) => server
            .presets
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(preset))
            .map(|(_, p)| p.clone())
            .ok_or_else(|| fail(Kind::NotFound, format!("no preset {preset}")))?,
        None => Profile::new(&name, server.catalog.version()),
    };
    profile.name = name;
    let key = add_doc(state, profile, None, true);
    Ok(json!({ "current": key }))
}

fn list_profiles(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    Ok(json!({
        "presets": server.presets.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        "files": profile_files(&state.options.profiles_dir),
        "workspace": state.docs.keys().cloned().collect::<Vec<_>>(),
        "current": state.current,
    }))
}

/// A secret of a comparison: `***` when it is set, as `redacted` shows it in a profile.
fn hidden(value: &serde_json::Value) -> serde_json::Value {
    match value.as_str() {
        Some("") | None => value.clone(),
        Some(_) => json!("***"),
    }
}

fn compare_profiles(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let (a, a_file) = find_profile(server, state, &args.text("a")?)?;
    let (b, b_file) = find_profile(server, state, &args.text("b")?)?;
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
        "install": diff.install.iter().map(|(k, f)| {
            // The product key is a secret: the answer says that it differs, never what it is.
            let (a, b) = if k == "product_key" {
                (hidden(&f.base), hidden(&f.other))
            } else {
                (f.base.clone(), f.other.clone())
            };
            (k.clone(), json!({ "a": a, "b": b }))
        }).collect::<serde_json::Map<_, _>>(),
        "languages": diff.languages.iter().map(|(k, f)| (k.clone(), json!({ "a": f.base, "b": f.other }))).collect::<serde_json::Map<_, _>>(),
        "accounts_differ": diff.accounts.is_some(),
    }))
}

fn read_this_pc(server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let machine = winweaver_win::this_pc()
        .ok_or_else(|| fail(Kind::ReadFailed, "this PC can be read on Windows only"))?;
    let base = current(state).profile.clone();
    let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
    let plan = thispc::plan(catalog, &base, &ids);
    let report = thispc::run(&*machine, &plan, true, &clock::now_utc_iso());
    let readings = thispc::judge(catalog, &plan, &report.results);
    let (mut profile, summary) = thispc::capture(catalog, &base, &readings, report.system.as_ref());
    profile.name = if report.computer.is_empty() {
        "This PC".to_owned()
    } else {
        report.computer.clone()
    };
    let key = add_doc(state, profile, None, true);
    Ok(json!({
        "current": key,
        "summary": summary.line(),
        "forms": summary.forms,
        "notes": summary.notes,
        "partly": summary.partly.iter().map(|(id, differs)| json!({ "id": id, "differs": differs })).collect::<Vec<_>>(),
        "not_read": plan.excluded.len(),
    }))
}
