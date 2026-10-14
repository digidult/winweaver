//! The resolver: the state of every rule after dependencies and conflicts are applied.
//!
//! A rule is in effect when it is chosen on and every rule it requires is in effect; otherwise
//! it is blocked by the first required rule that is not. Two conflicting rules in effect are
//! reported, not changed: the window asks the user which one to keep. An imported policy whose
//! registry values a built-in rule in effect writes is linked to that rule: it is shown on and
//! stays off, since the built-in rule writes the values.

use std::collections::HashMap;

use crate::admx::RULE_PREFIX;
use crate::catalog::{Action, Catalog};
use crate::profile::Profile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    On,
    Off,
    /// Chosen on, but a required rule is not in effect.
    Blocked {
        by: String,
    },
}

impl State {
    pub fn in_effect(&self) -> bool {
        matches!(self, State::On)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Resolution {
    pub states: HashMap<String, State>,
    /// Pairs of conflicting rules both in effect, each pair once.
    pub conflicts: Vec<(String, String)>,
    /// Imported policies and the built-in rule in effect that writes their values.
    pub linked: HashMap<String, String>,
}

impl Resolution {
    pub fn state(&self, id: &str) -> &State {
        self.states.get(id).unwrap_or(&State::Off)
    }

    pub fn in_effect(&self, id: &str) -> bool {
        self.state(id).in_effect()
    }

    /// The built-in rule that writes the values of an imported policy, when one does.
    pub fn linked_by(&self, id: &str) -> Option<&str> {
        self.linked.get(id).map(String::as_str)
    }

    /// Whether a rule is shown on: in effect, or an imported policy a built-in rule writes.
    pub fn shown_on(&self, id: &str) -> bool {
        self.in_effect(id) || self.linked.contains_key(id)
    }
}

/// Resolves the states of every rule of the catalog for the profile.
pub fn resolve(catalog: &Catalog, profile: &Profile) -> Resolution {
    let mut resolution = Resolution::default();
    for rule in catalog.rules() {
        let mut stack = Vec::new();
        let state = state_of(catalog, profile, &rule.id, &mut stack);
        resolution.states.insert(rule.id.clone(), state);
    }
    for rule in catalog.rules() {
        if !resolution.in_effect(&rule.id) {
            continue;
        }
        for other in &rule.conflicts {
            if resolution.in_effect(other) && rule.id.as_str() < other.as_str() {
                resolution.conflicts.push((rule.id.clone(), other.clone()));
            }
        }
    }
    resolution.linked = links(catalog, &resolution);
    resolution
}

/// The path and the value name of a registry write in lower case, as Windows compares them.
fn write_key(path: &str, name: &str) -> (String, String) {
    (path.to_ascii_lowercase(), name.to_ascii_lowercase())
}

/// Whether two registry actions write the same: the same kind and value, or both a removal.
fn same_write(a: &Action, b: &Action) -> bool {
    match (a, b) {
        (
            Action::Reg {
                kind: ka,
                value: va,
                ..
            },
            Action::Reg {
                kind: kb,
                value: vb,
                ..
            },
        ) => ka == kb && va == vb,
        (Action::RegRemove { .. }, Action::RegRemove { .. }) => true,
        _ => false,
    }
}

/// The imported policies without parameters whose every registry action one built-in rule in
/// effect has too, with that rule.
fn links(catalog: &Catalog, resolution: &Resolution) -> HashMap<String, String> {
    let mut written: HashMap<(String, String), Vec<(&str, &Action)>> = HashMap::new();
    for rule in catalog.rules() {
        if rule.id.starts_with(RULE_PREFIX) || !resolution.in_effect(&rule.id) {
            continue;
        }
        for action in &rule.actions {
            if let Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. } = action {
                written
                    .entry(write_key(path, name))
                    .or_default()
                    .push((rule.id.as_str(), action));
            }
        }
    }
    let mut links = HashMap::new();
    if written.is_empty() {
        return links;
    }
    for rule in catalog.rules() {
        if !rule.id.starts_with(RULE_PREFIX) || !rule.params.is_empty() || rule.actions.is_empty() {
            continue;
        }
        let mut owner: Option<&str> = None;
        let covered = rule.actions.iter().all(|a| {
            let (Action::Reg { path, name, .. } | Action::RegRemove { path, name, .. }) = a else {
                return false;
            };
            let found = written.get(&write_key(path, name)).and_then(|list| {
                list.iter()
                    .find(|(id, b)| owner.is_none_or(|o| o == *id) && same_write(a, b))
            });
            match found {
                Some((id, _)) => {
                    owner = Some(*id);
                    true
                }
                None => false,
            }
        });
        if covered && let Some(id) = owner {
            links.insert(rule.id.clone(), id.to_owned());
        }
    }
    links
}

fn state_of(catalog: &Catalog, profile: &Profile, id: &str, stack: &mut Vec<String>) -> State {
    if !profile.is_on(id) {
        return State::Off;
    }
    let Some(rule) = catalog.rule(id) else {
        return State::Off;
    };
    if stack.iter().any(|s| s == id) {
        return State::Off;
    }
    stack.push(id.to_owned());
    let mut result = State::On;
    for required in &rule.requires {
        if !state_of(catalog, profile, required, stack).in_effect() {
            result = State::Blocked {
                by: required.clone(),
            };
            break;
        }
    }
    stack.pop();
    result
}
