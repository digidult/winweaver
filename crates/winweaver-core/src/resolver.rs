//! The resolver: the state of every rule after dependencies and conflicts are applied.
//!
//! A rule is in effect when it is chosen on and every rule it requires is in effect; otherwise
//! it is blocked by the first required rule that is not. Two conflicting rules in effect are
//! reported, not changed: the window asks the user which one to keep.

use std::collections::HashMap;

use crate::catalog::Catalog;
use crate::profile::Profile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    On,
    Off,
    /// Chosen on, but a required rule is not in effect.
    Blocked { by: String },
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
}

impl Resolution {
    pub fn state(&self, id: &str) -> &State {
        self.states.get(id).unwrap_or(&State::Off)
    }

    pub fn in_effect(&self, id: &str) -> bool {
        self.state(id).in_effect()
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
    resolution
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
