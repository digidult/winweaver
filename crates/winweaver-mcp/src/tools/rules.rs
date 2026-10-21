//! The tools of the catalog and of the choices of the current profile: groups, rules, their
//! parameters.

use serde_json::{Value, json};
use winweaver_core::catalog::{Action, Catalog, Param, Rule, check};
use winweaver_core::resolver::{self, State as RuleState};

use super::{Args, Kind, LIST_LIMIT, Needs, Outcome, Tool, current, current_mut, fail, resolution, s, schema};
use crate::{Server, State};

pub(super) const LIST_GROUPS: Tool = Tool {
    name: "list_groups",
    description: "The groups under a parent group (the roots without one), with the counts of rules and of rules in effect in the current profile.",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({ "parent": s("A group id; leave it out for the roots") }),
            &[],
        )
    },
    run: list_groups,
};

pub(super) const LIST_RULES: Tool = Tool {
    name: "list_rules",
    description: "The rules of a group and its subgroups (all rules without one), with their state in the current profile.",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({
                "group": s("A group id"),
                "on": { "type": "boolean", "description": "Only the rules in effect (true) or not (false)" },
                "offset": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": LIST_LIMIT },
            }),
            &[],
        )
    },
    run: list_rules,
};

pub(super) const GET_RULE: Tool = Tool {
    name: "get_rule",
    description: "Everything about one rule: texts, parameters with their values in the current profile, what it writes, dependencies and conflicts.",
    needs: Needs::READ,
    schema: || schema(json!({ "id": s("A rule id") }), &["id"]),
    run: get_rule,
};

pub(super) const SEARCH_RULES: Tool = Tool {
    name: "search_rules",
    description: "Rules whose id, title, summary or tags hold every word of the query.",
    needs: Needs::READ,
    schema: || {
        schema(
            json!({
                "query": s("Words to look for"),
                "limit": { "type": "integer", "minimum": 1, "maximum": LIST_LIMIT },
            }),
            &["query"],
        )
    },
    run: search_rules,
};

pub(super) const SET_RULE: Tool = Tool {
    name: "set_rule",
    description: "Turns a rule of the current profile on or off. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({ "id": s("A rule id"), "on": { "type": "boolean" } }),
            &["id", "on"],
        )
    },
    run: set_rule,
};

pub(super) const SET_PARAM: Tool = Tool {
    name: "set_param",
    description: "Sets a parameter of a rule of the current profile. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({
                "id": s("A rule id"),
                "name": s("The parameter name"),
                "value": { "description": "An option of an enum, a whole number in range, a text, or for a list an array of texts" },
            }),
            &["id", "name", "value"],
        )
    },
    run: set_param,
};

pub(super) const SET_GROUP: Tool = Tool {
    name: "set_group",
    description: "Turns every rule of a group and its subgroups on or off; turning on leaves the imported policies off, as the window does. Mode edit.",
    needs: Needs::EDIT,
    schema: || {
        schema(
            json!({ "group": s("A group id"), "on": { "type": "boolean" } }),
            &["group", "on"],
        )
    },
    run: set_group,
};

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

fn param_json(catalog: &Catalog, p: &winweaver_core::profile::Profile, rule: &Rule, name: &str, param: &Param) -> Value {
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

fn list_groups(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let parent = args.str("parent");
    if let Some(p) = parent
        && catalog.group(p).is_none()
    {
        return Err(fail(Kind::NotFound, format!("no group {p}")));
    }
    let resolution = resolution(server, state);
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

fn list_rules(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let group = args.str("group");
    let rules: Vec<&Rule> = match group {
        Some(g) if catalog.group(g).is_none() => {
            return Err(fail(Kind::NotFound, format!("no group {g}")));
        }
        Some(g) => catalog.rules_under(g),
        None => catalog.rules().iter().collect(),
    };
    let resolution = resolution(server, state);
    let on = args.get("on").and_then(Value::as_bool);
    let filtered: Vec<&Rule> = rules
        .into_iter()
        .filter(|r| on.is_none_or(|want| resolution.in_effect(&r.id) == want))
        .collect();
    let offset = args.number("offset").unwrap_or(0) as usize;
    let limit = args.limit("limit", 50, LIST_LIMIT);
    let page: Vec<Value> = filtered
        .iter()
        .skip(offset)
        .take(limit)
        .map(|r| rule_brief(r, &resolution))
        .collect();
    Ok(json!({ "total": filtered.len(), "offset": offset, "rules": page }))
}

fn get_rule(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let id = args.text("id")?;
    let rule = catalog
        .rule(&id)
        .ok_or_else(|| fail(Kind::NotFound, format!("no rule {id}")))?;
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

fn search_rules(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let query = args.text("query")?;
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Err(fail(Kind::Invalid, "the query is empty"));
    }
    let limit = args.limit("limit", 20, LIST_LIMIT);
    let resolution = resolution(server, state);
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

fn set_rule(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let id = args.text("id")?;
    let on = args.flag("on")?;
    if catalog.rule(&id).is_none() {
        return Err(fail(Kind::NotFound, format!("no rule {id}")));
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
        .filter(|r| matches!(resolution.state(&r.id), RuleState::Blocked { by } if by == &id))
        .map(|r| r.id.as_str())
        .collect();
    Ok(
        json!({ "id": id, "state": state_text(resolution.state(&id)), "blocked_rules": blocked }),
    )
}

/// The value of a parameter as the tool takes it: an option of an enum, a number in range, one
/// line of text, or a list of lines.
fn param_value(param: &Param, pname: &str, value: Value) -> Result<Value, ToolError> {
    Ok(match param {
        Param::Enum { values, .. } => {
            if value.is_boolean() {
                return Err(fail(
                    Kind::Invalid,
                    "an enum takes one of its options, not true or false",
                ));
            }
            values
                .iter()
                .find(|v| check::same_value(&v.value, &value))
                .map(|v| v.value.clone())
                .ok_or_else(|| {
                    fail(
                        Kind::Invalid,
                        format!("{value} is not an option of {pname}"),
                    )
                })?
        }
        Param::Int { min, max, .. } => {
            let n = value
                .as_i64()
                .ok_or_else(|| fail(Kind::Invalid, "a whole number is needed"))?;
            if !(*min..=*max).contains(&n) {
                return Err(fail(
                    Kind::Invalid,
                    format!("{n} is outside {min}..{max}"),
                ));
            }
            Value::from(n)
        }
        Param::Text { .. } => {
            let t = value
                .as_str()
                .ok_or_else(|| fail(Kind::Invalid, "a text is needed"))?;
            Value::from(super::profiles::one_line(t, 500)?)
        }
        Param::List { .. } => {
            let items = value
                .as_array()
                .ok_or_else(|| fail(Kind::Invalid, "a list of texts is needed"))?;
            if items.len() > 1000 {
                return Err(fail(Kind::Invalid, "a list of at most 1000 items"));
            }
            let mut texts = Vec::new();
            for item in items {
                let t = item.as_str().ok_or_else(|| {
                    fail(Kind::Invalid, "every item of a list is a text")
                })?;
                let t = super::profiles::one_line(t, 500)?;
                if !t.is_empty() {
                    texts.push(Value::from(t));
                }
            }
            Value::Array(texts)
        }
    })
}

use super::ToolError;

fn set_param(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let id = args.text("id")?;
    let pname = args.text("name")?;
    let rule = catalog
        .rule(&id)
        .ok_or_else(|| fail(Kind::NotFound, format!("no rule {id}")))?;
    let param = rule.params.get(&pname).ok_or_else(|| {
        fail(
            Kind::NotFound,
            format!("the rule {id} has no parameter {pname}"),
        )
    })?;
    let value = args
        .get("value")
        .cloned()
        .ok_or_else(|| fail(Kind::Invalid, "the argument value is missing"))?;
    let value = param_value(param, &pname, value)?;
    let doc = current_mut(state);
    let changed = doc.profile.set_param(catalog, &id, &pname, value.clone());
    if changed {
        doc.dirty = true;
    }
    Ok(json!({ "id": id, "name": pname, "value": value, "changed": changed }))
}

fn set_group(server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let catalog = &server.catalog;
    let group = args.text("group")?;
    let on = args.flag("on")?;
    if catalog.group(&group).is_none() {
        return Err(fail(Kind::NotFound, format!("no group {group}")));
    }
    let rules = catalog.rules_under(&group).len();
    let doc = current_mut(state);
    let changed = doc.profile.set_group(catalog, &group, on);
    if changed > 0 {
        doc.dirty = true;
    }
    Ok(json!({ "group": group, "rules": rules, "changed": changed }))
}
