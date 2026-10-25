//! The rules of a set of templates: one rule per policy that enables it, one `<id>.off` that
//! disables it, the groups of the categories, the texts of the other cultures.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use serde_json::Value;

use crate::catalog::{
    Action, EnumValue, Group, Level, ListNames, Param, Phase, ROOT_POLICIES, RegKind, Rule,
};

use super::file::{CatalogFile, LangTexts};
use super::templates::{Element, Policy, Templates, Texts, Val};
use super::{CATALOG_FORMAT, CATALOG_VERSION, CULTURES, NAME_MAX, SUMMARY_MAX};

pub(super) fn slug(text: &str) -> String {
    let s: String = text
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_owned();
    if s.is_empty() {
        "x".to_owned()
    } else {
        s.chars().take(80).collect()
    }
}

pub(super) fn en(texts: &Texts) -> String {
    texts
        .get("en-US")
        .or_else(|| texts.values().next())
        .cloned()
        .unwrap_or_default()
}

/// Whether a text can go into a key, a value name or a value: no control characters, nothing that
/// closes a CDATA section, no non-characters, within the length limit.
pub(super) fn safe_text(text: &str, limit: usize) -> bool {
    text.chars().count() <= limit
        && !text.contains("]]>")
        && !text.chars().any(|c| {
            c.is_control()
                || c == '\u{FFFE}'
                || c == '\u{FFFF}'
                || ('\u{80}'..='\u{9F}').contains(&c)
        })
}

/// A registry key of a template: segments that are not empty, `.` or `..`.
pub(super) fn safe_key(key: &str) -> bool {
    safe_text(key, 1024)
        && !key.is_empty()
        && key
            .split(['\\', '/'])
            .all(|s| !s.is_empty() && s != "." && s != ".." && s.len() <= NAME_MAX)
}

pub(super) fn cut(text: &str, limit: usize) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    if clean.chars().count() <= limit {
        clean
    } else {
        clean.chars().take(limit).collect::<String>() + "..."
    }
}

fn reg_of(path: &str, name: &str, val: &Val) -> Result<Action, String> {
    if !safe_text(name, 16383) {
        return Err("a value name with unsafe characters".to_owned());
    }
    Ok(match val {
        Val::Delete => Action::RegRemove {
            path: path.to_owned(),
            name: name.to_owned(),
            default: None,
        },
        Val::Decimal(n) => Action::Reg {
            path: path.to_owned(),
            name: name.to_owned(),
            kind: RegKind::DWord,
            value: Value::from(*n),
            default: None,
            why: String::new(),
        },
        Val::LongDecimal(n) => Action::Reg {
            path: path.to_owned(),
            name: name.to_owned(),
            kind: RegKind::QWord,
            value: Value::from(*n),
            default: None,
            why: String::new(),
        },
        Val::Text(t) => {
            if !safe_text(t, 16383) || t.contains('{') {
                return Err("a text value with unsafe characters or braces".to_owned());
            }
            Action::Reg {
                path: path.to_owned(),
                name: name.to_owned(),
                kind: RegKind::String,
                value: Value::from(t.as_str()),
                default: None,
                why: String::new(),
            }
        }
    })
}

/// The rules of one policy: the rule that enables it and the rule `<id>.off` that disables it.
fn policy_rules(p: &Policy, id: &str, group: &str) -> Result<(Rule, Option<Rule>), String> {
    if !safe_key(&p.key) {
        return Err("an unsafe registry key".to_owned());
    }
    let root = if p.user { "DU:\\" } else { "HKLM:\\" };
    let path_of = |key: &Option<String>| -> Result<String, String> {
        let k = key.as_deref().unwrap_or(&p.key);
        if !safe_key(k) {
            return Err("an unsafe registry key".to_owned());
        }
        Ok(format!("{root}{k}"))
    };
    let main_path = format!("{root}{}", p.key);
    let mut on_actions = Vec::new();
    let mut off_actions = Vec::new();
    if let Some(name) = &p.value_name {
        on_actions.push(reg_of(
            &main_path,
            name,
            p.enabled.as_ref().unwrap_or(&Val::Decimal(1)),
        )?);
        off_actions.push(reg_of(
            &main_path,
            name,
            p.disabled.as_ref().unwrap_or(&Val::Decimal(0)),
        )?);
    }
    for item in &p.enabled_list {
        on_actions.push(reg_of(&path_of(&item.key)?, &item.value_name, &item.value)?);
    }
    for item in &p.disabled_list {
        off_actions.push(reg_of(&path_of(&item.key)?, &item.value_name, &item.value)?);
    }
    let mut params = IndexMap::new();
    // A list that is not additive deletes the other values of its key, so the lists come first.
    let mut list_on = Vec::new();
    let mut list_off = Vec::new();
    for (n, e) in p.elements.iter().enumerate() {
        let pname = format!("e{}", n + 1);
        let placeholder = Value::from(format!("{{{pname}}}"));
        let (key, eid) = match e {
            Element::Enum { key, id, .. }
            | Element::Decimal { key, id, .. }
            | Element::Boolean { key, id, .. }
            | Element::Text { key, id, .. }
            | Element::List { key, id, .. }
            | Element::MultiText { key, id, .. } => (key, id),
        };
        let path = path_of(key)?;
        let label = p
            .labels
            .get(eid)
            .map(en)
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| eid.clone());
        let value_name = match e {
            Element::List {
                prefix,
                additive,
                expandable,
                explicit,
                ..
            } => {
                if prefix
                    .as_deref()
                    .is_some_and(|x| !safe_text(x, NAME_MAX) || x.contains('{'))
                {
                    return Err("a list prefix with unsafe characters".to_owned());
                }
                let names = if *explicit {
                    ListNames::Pairs
                } else if prefix.is_some() {
                    ListNames::Numbered
                } else {
                    ListNames::Items
                };
                let kind = if *expandable {
                    RegKind::ExpandString
                } else {
                    RegKind::String
                };
                let prefix = prefix.clone().unwrap_or_default();
                if !*additive {
                    list_off.push(Action::RegList {
                        path: path.clone(),
                        param: String::new(),
                        names,
                        prefix: prefix.clone(),
                        additive: false,
                        kind,
                    });
                }
                list_on.push(Action::RegList {
                    path,
                    param: pname.clone(),
                    names,
                    prefix,
                    additive: *additive,
                    kind,
                });
                params.insert(
                    pname,
                    Param::List {
                        title: cut(&label, 200),
                        default: Vec::new(),
                    },
                );
                continue;
            }
            Element::Enum { value_name, .. }
            | Element::Decimal { value_name, .. }
            | Element::Boolean { value_name, .. }
            | Element::Text { value_name, .. }
            | Element::MultiText { value_name, .. } => value_name,
        };
        if !safe_text(value_name, 16383) {
            return Err("a value name with unsafe characters".to_owned());
        }
        let (kind, param) = match e {
            Element::Enum { items, .. } => {
                let decimal = items.iter().all(|(_, v)| matches!(v, Val::Decimal(_)));
                let text = items.iter().all(|(_, v)| matches!(v, Val::Text(_)));
                if items.is_empty() || !(decimal || text) {
                    return Err("an enum of mixed or deleted values".to_owned());
                }
                let values: Vec<EnumValue> = items
                    .iter()
                    .map(|(t, v)| EnumValue {
                        value: match v {
                            Val::Decimal(n) => Value::from(*n),
                            Val::Text(s) => Value::from(s.as_str()),
                            _ => Value::Null,
                        },
                        title: cut(&en(t), 200),
                    })
                    .collect();
                if values.iter().any(|v| {
                    v.value
                        .as_str()
                        .is_some_and(|s| !safe_text(s, 1000) || s.contains('{'))
                }) {
                    return Err("an enum value with unsafe characters".to_owned());
                }
                let default = p
                    .defaults
                    .get(eid)
                    .and_then(|d| d.parse::<usize>().ok())
                    .and_then(|i| values.get(i))
                    .map_or_else(|| values[0].value.clone(), |v| v.value.clone());
                (
                    if decimal {
                        RegKind::DWord
                    } else {
                        RegKind::String
                    },
                    Param::Enum {
                        title: cut(&label, 200),
                        default,
                        values,
                        differs_from: None,
                        same_allowed: Vec::new(),
                    },
                )
            }
            Element::Boolean { on, off, .. } => {
                let pair = [on, off];
                let decimal = pair.iter().all(|v| matches!(v, Val::Decimal(_)));
                if !decimal {
                    return Err("a check box that writes texts or deletes".to_owned());
                }
                let num = |v: &Val| {
                    if let Val::Decimal(n) = v {
                        Value::from(*n)
                    } else {
                        Value::Null
                    }
                };
                let checked = p.defaults.get(eid).is_some_and(|d| d == "true");
                (
                    RegKind::DWord,
                    Param::Enum {
                        title: cut(&label, 200),
                        default: if checked { num(on) } else { num(off) },
                        values: vec![
                            EnumValue {
                                value: num(on),
                                title: "On".to_owned(),
                            },
                            EnumValue {
                                value: num(off),
                                title: "Off".to_owned(),
                            },
                        ],
                        differs_from: None,
                        same_allowed: Vec::new(),
                    },
                )
            }
            Element::Decimal {
                min,
                max,
                text,
                long,
                ..
            } => {
                if min > max || *max > i64::MAX as u64 {
                    return Err("a number with a wrong range".to_owned());
                }
                let default = p
                    .defaults
                    .get(eid)
                    .and_then(|d| d.parse::<i64>().ok())
                    .filter(|d| (*min as i64..=*max as i64).contains(d))
                    .unwrap_or(*min as i64);
                (
                    if *text {
                        RegKind::String
                    } else if *long {
                        RegKind::QWord
                    } else {
                        RegKind::DWord
                    },
                    Param::Int {
                        title: cut(&label, 200),
                        default,
                        min: *min as i64,
                        max: *max as i64,
                    },
                )
            }
            Element::Text {
                expandable,
                required,
                max_length,
                ..
            } => {
                let default = p.defaults.get(eid).cloned().unwrap_or_default();
                if !safe_text(&default, 1000) || default.contains('{') {
                    return Err("a default text with unsafe characters".to_owned());
                }
                (
                    if *expandable {
                        RegKind::ExpandString
                    } else {
                        RegKind::String
                    },
                    Param::Text {
                        title: cut(&label, 200),
                        default: default.clone(),
                        required: *required && !default.is_empty(),
                        max_length: max_length.filter(|&m| default.chars().count() <= m),
                    },
                )
            }
            Element::MultiText { .. } => (
                RegKind::MultiString,
                Param::List {
                    title: cut(&label, 200),
                    default: Vec::new(),
                },
            ),
            Element::List { .. } => continue,
        };
        on_actions.push(Action::Reg {
            path: path.clone(),
            name: value_name.clone(),
            kind,
            value: placeholder,
            default: None,
            why: String::new(),
        });
        off_actions.push(Action::RegRemove {
            path,
            name: value_name.clone(),
            default: None,
        });
        params.insert(pname, param);
    }
    list_on.append(&mut on_actions);
    on_actions = list_on;
    list_off.append(&mut off_actions);
    off_actions = list_off;
    if on_actions.is_empty() {
        return Err("the policy writes nothing".to_owned());
    }
    let title = cut(&en(&p.title), 300);
    let summary = cut(&en(&p.explain), SUMMARY_MAX);
    let phase = if p.user {
        Phase::DefaultUser
    } else {
        Phase::Specialize
    };
    let off_id = format!("{id}.off");
    let has_off = !off_actions.is_empty();
    let on = Rule {
        id: id.to_owned(),
        group: group.to_owned(),
        phase,
        title: title.clone(),
        level: Level::Optional,
        default: false,
        tags: vec![p.prefix.clone(), p.name.clone()],
        doc: String::new(),
        summary: summary.clone(),
        effect: String::new(),
        risk: String::new(),
        versions: String::new(),
        rollback: String::new(),
        verify: String::new(),
        note: String::new(),
        params,
        requires: Vec::new(),
        conflicts: if has_off {
            vec![off_id.clone()]
        } else {
            Vec::new()
        },
        actions: on_actions,
    };
    let off = has_off.then(|| Rule {
        id: off_id,
        title: format!("{title}: Disabled"),
        params: IndexMap::new(),
        conflicts: vec![id.to_owned()],
        actions: off_actions,
        ..on.clone()
    });
    Ok((on, off))
}

/// The catalog file of a set of templates.
pub fn to_catalog_file(
    t: &Templates,
    id: &str,
    name: &str,
    windows: &str,
    created: &str,
) -> CatalogFile {
    let root_id = format!("admx.{}", slug(id));
    let mut groups = vec![Group {
        id: root_id.clone(),
        title: name.to_owned(),
        order: 900,
        summary: String::new(),
        parent: Some(ROOT_POLICIES.to_owned()),
    }];
    let mut texts: IndexMap<String, LangTexts> = IndexMap::new();
    let mut group_of: HashMap<(String, String), String> = HashMap::new();
    let mut used_groups: HashSet<String> = HashSet::new();
    used_groups.insert(root_id.clone());
    for c in &t.categories {
        if group_of.contains_key(&(c.ns.clone(), c.name.clone())) {
            continue;
        }
        let mut gid = format!(
            "{root_id}.{}.{}",
            slug(c.ns.rsplit('.').next().unwrap_or("x")),
            slug(&c.name)
        );
        let mut n = 2;
        while used_groups.contains(&gid) {
            gid = format!("{root_id}.{}-{n}", slug(&c.name));
            n += 1;
        }
        used_groups.insert(gid.clone());
        group_of.insert((c.ns.clone(), c.name.clone()), gid);
    }
    // Categories of two namespaces with one title (Zoom has one for its user and one for its machine
    // templates) are told apart by the end of their namespace.
    let mut namespaces_of_title: HashMap<String, HashSet<&str>> = HashMap::new();
    for c in &t.categories {
        namespaces_of_title
            .entry(en(&c.title))
            .or_default()
            .insert(c.ns.as_str());
    }
    let mut placed: HashSet<(String, String)> = HashSet::new();
    for (order, c) in t.categories.iter().enumerate() {
        if !placed.insert((c.ns.clone(), c.name.clone())) {
            continue;
        }
        let gid = group_of[&(c.ns.clone(), c.name.clone())].clone();
        let parent = c
            .parent
            .as_ref()
            .and_then(|p| group_of.get(p))
            .cloned()
            .unwrap_or_else(|| root_id.clone());
        let title = en(&c.title);
        let mark = if namespaces_of_title.get(&title).is_some_and(|n| n.len() > 1) {
            format!(" ({})", c.ns.rsplit('.').next().unwrap_or(&c.ns))
        } else {
            String::new()
        };
        groups.push(Group {
            id: gid.clone(),
            title: cut(&format!("{title}{mark}"), 200),
            order: 1000 + order as u32,
            summary: String::new(),
            parent: Some(parent),
        });
        for (culture, code) in CULTURES.iter().skip(1) {
            if let Some(text) = c.title.get(*culture) {
                texts
                    .entry((*code).to_owned())
                    .or_default()
                    .groups
                    .insert(gid.clone(), cut(&format!("{text}{mark}"), 200));
            }
        }
    }
    // A cycle of parents would hang the tree: a group whose chain does not reach the root goes
    // under the root.
    let parents: HashMap<String, String> = groups
        .iter()
        .filter_map(|g| g.parent.clone().map(|p| (g.id.clone(), p)))
        .collect();
    for g in groups.iter_mut().skip(1) {
        let mut seen = HashSet::new();
        let mut cur = g.id.clone();
        let mut ok = false;
        while let Some(p) = parents.get(&cur) {
            if p == ROOT_POLICIES {
                ok = true;
                break;
            }
            if !seen.insert(p.clone()) {
                break;
            }
            cur = p.clone();
        }
        if !ok {
            g.parent = Some(root_id.clone());
        }
    }
    let mut rules = Vec::new();
    let mut skipped = t.skipped.clone();
    let mut used: HashSet<String> = HashSet::new();
    // Two namespaces with one prefix and one policy name (Zoom gives its user and its machine
    // templates the prefix fullarmor) are two policies: they take the namespace into their ids,
    // whatever the order of the files. A second copy of a policy of the same namespace (the x64 and
    // x86 templates of Office) is left out.
    let mut namespaces: HashMap<(String, String), HashSet<&str>> = HashMap::new();
    for p in &t.policies {
        namespaces
            .entry((slug(&p.prefix), slug(&p.name)))
            .or_default()
            .insert(p.ns.as_str());
    }
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    for p in &t.policies {
        if !seen.insert((p.ns.as_str(), p.name.as_str())) {
            skipped.push((
                p.name.clone(),
                format!("a second copy of a policy of {}", p.ns),
            ));
            continue;
        }
        let key = (slug(&p.prefix), slug(&p.name));
        let base = if namespaces.get(&key).is_some_and(|n| n.len() > 1) {
            format!("admx.{}.{}", slug(&p.ns), key.1)
        } else {
            format!("admx.{}.{}", key.0, key.1)
        };
        let mut rid = base.clone();
        let mut n = 2;
        while used.contains(&rid) || used.contains(&format!("{rid}.off")) {
            rid = format!("{base}-{n}");
            n += 1;
        }
        let group = p
            .category
            .as_ref()
            .and_then(|c| group_of.get(c))
            .cloned()
            .unwrap_or_else(|| root_id.clone());
        match policy_rules(p, &rid, &group) {
            Ok((on, off)) => {
                used.insert(rid.clone());
                used.insert(format!("{rid}.off"));
                for (culture, code) in CULTURES.iter().skip(1) {
                    if let Some(title) = p.title.get(*culture) {
                        let summary = p
                            .explain
                            .get(*culture)
                            .map(|s| cut(s, SUMMARY_MAX))
                            .unwrap_or_default();
                        let lang = texts.entry((*code).to_owned()).or_default();
                        lang.rules.insert(rid.clone(), (cut(title, 300), summary));
                    }
                }
                rules.push(on);
                if let Some(off) = off {
                    rules.push(off);
                }
            }
            Err(why) => skipped.push((format!("{}:{}", p.prefix, p.name), why)),
        }
    }
    CatalogFile {
        format: CATALOG_FORMAT.to_owned(),
        version: CATALOG_VERSION,
        kind: "admx".to_owned(),
        id: slug(id),
        name: name.to_owned(),
        windows: windows.to_owned(),
        created: created.to_owned(),
        groups,
        rules,
        texts,
        skipped,
    }
}
