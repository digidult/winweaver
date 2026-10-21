//! Two catalog files side by side, policy by policy.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;

use crate::catalog::{ROOT_POLICIES, Rule};

use super::CatalogFile;

/// One policy against another: what differs between two catalog files (`compare`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDifference {
    pub id: String,
    pub title: String,
    /// What differs: title, description, values, values when off, parameters, category.
    pub what: Vec<&'static str>,
}

/// Two catalog files side by side, policy by policy (the rules `<id>.off` of the Disabled state
/// count as part of their policy).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comparison {
    /// The policies of the first only, then of the second only: ids and titles.
    pub only_first: Vec<(String, String)>,
    pub only_second: Vec<(String, String)>,
    pub changed: Vec<PolicyDifference>,
    pub same: usize,
}

/// Compares two catalog files by the ids of their policies.
pub fn compare(first: &CatalogFile, second: &CatalogFile) -> Comparison {
    fn policies(f: &CatalogFile) -> IndexMap<String, &Rule> {
        f.rules
            .iter()
            .filter(|r| !r.id.ends_with(".off"))
            .map(|r| (r.id.clone(), r))
            .collect()
    }
    fn rule<'a>(f: &'a CatalogFile, id: &str) -> Option<&'a Rule> {
        f.rules.iter().find(|r| r.id == id)
    }
    // The root group of an import is its name, not a category: a policy right under it has none.
    let group_title = |f: &CatalogFile, id: &str| {
        f.groups
            .iter()
            .find(|g| g.id == id && g.parent.as_deref() != Some(ROOT_POLICIES))
            .map(|g| g.title.clone())
            .unwrap_or_default()
    };
    fn json<T: Serialize>(v: &T) -> Value {
        serde_json::to_value(v).unwrap_or_default()
    }
    let (a, b) = (policies(first), policies(second));
    let mut out = Comparison::default();
    for (id, ra) in &a {
        let Some(rb) = b.get(id) else {
            out.only_first.push((id.clone(), ra.title.clone()));
            continue;
        };
        let mut what = Vec::new();
        if ra.title != rb.title {
            what.push("title");
        }
        if ra.summary != rb.summary {
            what.push("description");
        }
        if json(&ra.actions) != json(&rb.actions) {
            what.push("values");
        }
        let off = format!("{id}.off");
        let (oa, ob) = (rule(first, &off), rule(second, &off));
        if oa.map(|r| json(&r.actions)) != ob.map(|r| json(&r.actions)) {
            what.push("values when off");
        }
        if json(&ra.params) != json(&rb.params) {
            what.push("parameters");
        }
        if group_title(first, &ra.group) != group_title(second, &rb.group) {
            what.push("category");
        }
        if what.is_empty() {
            out.same += 1;
        } else {
            out.changed.push(PolicyDifference {
                id: id.clone(),
                title: ra.title.clone(),
                what,
            });
        }
    }
    for (id, rb) in &b {
        if !a.contains_key(id) {
            out.only_second.push((id.clone(), rb.title.clone()));
        }
    }
    out
}
