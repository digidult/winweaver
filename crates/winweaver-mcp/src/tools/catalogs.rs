//! The tools of the catalog files of the imported policy templates: read always, changed when
//! the user allows it.

use std::path::PathBuf;

use serde_json::{Value, json};
use winweaver_core::admx::{self, CatalogFile, CatalogHeader};
use winweaver_core::catalog::Rule;

use super::{Args, Kind, Needs, Outcome, Tool, ToolError, fail, s, schema};
use crate::{Server, State};

pub(super) const LIST_CATALOGS: Tool = Tool {
    name: "list_catalogs",
    description: "The catalog files of the imported policy templates next to the program: id, name and size. Only the first bytes of each are read.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: list_catalogs,
};

pub(super) const GET_CATALOG: Tool = Tool {
    name: "get_catalog",
    description: "A catalog file: without a group its groups with their counts of policies, with a group the policies of that group (id, title).",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({ "id": s("The id of list_catalogs"), "group": s("A group of the file") }),
            &["id"],
        )
    },
    run: get_catalog,
};

pub(super) const GET_CATALOG_POLICY: Tool = Tool {
    name: "get_catalog_policy",
    description: "A policy of a catalog file as JSON (texts, parameters, registry actions), with its Disabled rule when it has one.",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({ "id": s("The id of the catalog"), "policy": s("The id of the policy") }),
            &["id", "policy"],
        )
    },
    run: get_catalog_policy,
};

pub(super) const SET_CATALOG_POLICY: Tool = Tool {
    name: "set_catalog_policy",
    description: "Writes a policy into a catalog file, a new one or in place of the one of its id, as JSON in the form of get_catalog_policy; the whole file is checked, then saved. The program reads it at its next load. Needs the option edit_library.",
    needs: Needs::LIBRARY,
    schema: || {
        schema(
            json!({ "id": s("The id of the catalog"), "policy": { "type": "object", "description": "The policy" } }),
            &["id", "policy"],
        )
    },
    run: set_catalog_policy,
};

pub(super) const REMOVE_CATALOG_POLICY: Tool = Tool {
    name: "remove_catalog_policy",
    description: "Removes a policy with its Disabled rule from a catalog file and saves it. Needs the option edit_library.",
    needs: Needs::LIBRARY,
    schema: || {
        schema(
            json!({ "id": s("The id of the catalog"), "policy": s("The id of the policy") }),
            &["id", "policy"],
        )
    },
    run: remove_catalog_policy,
};

/// A catalog file of the imports folder by its id (the name of the file), read and checked; its path.
fn load_catalog(state: &State, args: &Args<'_>) -> Result<(CatalogFile, PathBuf), ToolError> {
    let id = args.text("id")?;
    if id.is_empty()
        || id.len() > 120
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || id.starts_with('.')
    {
        return Err(fail(
            Kind::Invalid,
            "id is the name of a catalog of list_catalogs",
        ));
    }
    let path = state
        .options
        .imports_dir
        .join(format!("{id}.{}", admx::CATALOG_EXTENSION));
    let file = CatalogFile::load(&path).map_err(|e| fail(Kind::NotFound, e))?;
    Ok((file, path))
}

fn list_catalogs(_server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
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

fn get_catalog(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let (file, _) = load_catalog(state, args)?;
    match args.str("group") {
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

fn get_catalog_policy(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let (file, _) = load_catalog(state, args)?;
    let policy = args.text("policy")?;
    let rule = file
        .rules
        .iter()
        .find(|r| r.id == policy)
        .ok_or_else(|| fail(Kind::NotFound, format!("no policy {policy}")))?;
    let off = file.rules.iter().find(|r| r.id == format!("{policy}.off"));
    Ok(json!({ "policy": rule, "off": off }))
}

fn set_catalog_policy(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let (mut file, path) = load_catalog(state, args)?;
    let rule: Rule = serde_json::from_value(args.get("policy").cloned().unwrap_or_default())
        .map_err(|e| fail(Kind::Invalid, format!("policy: {e}")))?;
    file.put_rule(rule).map_err(|e| fail(Kind::Invalid, e))?;
    file.save(&path).map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "saved": file.id, "policies": file.rules.len() }))
}

fn remove_catalog_policy(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let (mut file, path) = load_catalog(state, args)?;
    let policy = args.text("policy")?;
    file.remove_rule(&policy)
        .map_err(|e| fail(Kind::NotFound, e))?;
    file.save(&path).map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "saved": file.id, "policies": file.rules.len() }))
}
