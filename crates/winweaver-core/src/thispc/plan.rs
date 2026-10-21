//! The plan of checks of a selection of rules: what each action of a rule reads.

use serde_json::Value;

use crate::catalog::{Action, Catalog, RegKind, Rule, values};
use crate::profile::Profile;
use crate::render;

use super::{NOTE_CURRENT_USER, exclusion};

/// What one check reads.
#[derive(Debug, Clone, PartialEq)]
pub enum Probe {
    Reg {
        path: String,
        name: String,
        kind: RegKind,
        expected: Value,
        /// The parameter whose value the registry value is, when the value is `{name}` alone.
        param: Option<String>,
        default: Option<Value>,
    },
    RegAbsent {
        path: String,
        name: String,
        default: Option<Value>,
    },
    /// A key that holds a list of values: every name holds its value, and without `additive`
    /// the key holds nothing else.
    RegList {
        path: String,
        kind: RegKind,
        names: Vec<String>,
        values: Vec<String>,
        additive: bool,
    },
    Service {
        name: String,
        start: u32,
        default: Option<Value>,
    },
    Feature {
        name: String,
        state: String,
        default: Option<String>,
    },
    CapabilityAbsent {
        pattern: String,
    },
    AppAbsent {
        name: String,
    },
    Unknown {
        what: &'static str,
    },
}

/// One check of one rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub rule: String,
    pub probe: Probe,
    pub note: String,
}

impl Check {
    /// The text the audit script reports as `check`.
    pub fn label(&self) -> String {
        match &self.probe {
            Probe::Reg { path, name, .. } | Probe::RegAbsent { path, name, .. } => {
                format!("{path}\\{name}")
            }
            Probe::RegList { path, additive, .. } => {
                if *additive {
                    format!("{path} (list, other values allowed)")
                } else {
                    format!("{path} (list)")
                }
            }
            Probe::Service { name, .. } => format!("service {name}"),
            Probe::Feature { name, .. } => format!("feature {name}"),
            Probe::CapabilityAbsent { pattern } => format!("capability {pattern}"),
            Probe::AppAbsent { name } => format!("app {name}"),
            Probe::Unknown { what } => (*what).to_owned(),
        }
    }
}

/// The checks of a selection of rules.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// The rules checked, in catalog order.
    pub rules: Vec<String>,
    pub checks: Vec<Check>,
    /// The rules left out and why.
    pub excluded: Vec<(String, &'static str)>,
}

impl Plan {
    /// The checks of features and capabilities, which Windows shows to an administrator only.
    pub fn admin_checks(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| {
                matches!(
                    c.probe,
                    Probe::Feature { .. } | Probe::CapabilityAbsent { .. }
                )
            })
            .count()
    }
}

/// Whether a name matches a pattern of `Get-WindowsCapability -Name`: `*` is any text, `?` one
/// character, and letters match without regard to case.
pub fn wildcard(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let name: Vec<char> = name.to_lowercase().chars().collect();
    let (mut p, mut n) = (0, 0);
    // The position after the last `*` and the character of the name it covers up to.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p + 1, n));
                p += 1;
            }
            Some(&c) if c == '?' || c == name[n] => {
                p += 1;
                n += 1;
            }
            _ => match star {
                Some((after, covered)) => {
                    p = after;
                    n = covered + 1;
                    star = Some((after, covered + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// The path a check reads: the default user profile is read in HKCU of the current user.
pub fn audit_path(path: &str) -> (String, String) {
    match path.strip_prefix("DU:") {
        Some(rest) => (format!("HKCU:{rest}"), NOTE_CURRENT_USER.to_owned()),
        None => (path.to_owned(), String::new()),
    }
}

/// The parameter name of a value that is a placeholder alone, such as `{level}`.
fn single_placeholder(rule: &Rule, value: &Value) -> Option<String> {
    let text = value.as_str()?;
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    rule.params.contains_key(inner).then(|| inner.to_owned())
}

fn checks_of(
    rule: &Rule,
    action: &Action,
    params: &std::collections::BTreeMap<String, Value>,
) -> Vec<Check> {
    let check = |probe: Probe, note: String| Check {
        rule: rule.id.clone(),
        probe,
        note,
    };
    match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            default,
            ..
        } => {
            let (path, note) = audit_path(path);
            vec![check(
                Probe::Reg {
                    path,
                    name: name.clone(),
                    kind: *kind,
                    expected: render::fill_value(value, params),
                    param: single_placeholder(rule, value),
                    default: default.clone(),
                },
                note,
            )]
        }
        Action::RegRemove {
            path,
            name,
            default,
        } => {
            let (path, note) = audit_path(path);
            vec![check(
                Probe::RegAbsent {
                    path,
                    name: name.clone(),
                    default: default.clone(),
                },
                note,
            )]
        }
        Action::RegList {
            path,
            param,
            names,
            prefix,
            additive,
            kind,
        } => {
            let (path, note) = audit_path(path);
            let items = render::list_values(params, param);
            match values::list_entries(*names, prefix, &items) {
                Ok((names, values)) => vec![check(
                    Probe::RegList {
                        path,
                        kind: *kind,
                        names,
                        values,
                        additive: *additive,
                    },
                    note,
                )],
                Err(_) => vec![check(Probe::Unknown { what: "reg-list" }, note)],
            }
        }
        Action::Service {
            name,
            start,
            default,
        } => vec![check(
            Probe::Service {
                name: name.clone(),
                start: *start,
                default: default.clone(),
            },
            String::new(),
        )],
        Action::Feature {
            name,
            state,
            default,
        } => vec![check(
            Probe::Feature {
                name: name.clone(),
                state: state.clone(),
                default: default.clone(),
            },
            String::new(),
        )],
        Action::Capability { pattern } => vec![check(
            Probe::CapabilityAbsent {
                pattern: pattern.clone(),
            },
            String::new(),
        )],
        Action::Appx { names } => names
            .iter()
            .map(|n| check(Probe::AppAbsent { name: n.clone() }, String::new()))
            .collect(),
        Action::Ps { .. } => vec![check(Probe::Unknown { what: "ps" }, String::new())],
        Action::Exe { .. } => vec![check(Probe::Unknown { what: "exe" }, String::new())],
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => Vec::new(),
    }
}

/// The checks of the rules `rule_ids` (any order; the plan follows the catalog), with the
/// parameter values of the profile.
pub fn plan(catalog: &Catalog, profile: &Profile, rule_ids: &[String]) -> Plan {
    let wanted: std::collections::HashSet<&str> = rule_ids.iter().map(String::as_str).collect();
    let mut out = Plan::default();
    for rule in catalog.rules() {
        if !wanted.contains(rule.id.as_str()) {
            continue;
        }
        if let Some(why) = exclusion(rule) {
            out.excluded.push((rule.id.clone(), why));
            continue;
        }
        out.rules.push(rule.id.clone());
        let params = render::params_of(catalog, profile, rule);
        let mut checks: Vec<Check> = rule
            .actions
            .iter()
            .flat_map(|action| checks_of(rule, action, &params))
            .collect();
        // Of the actions of a rule on one value (a removal for a clean start, then the value)
        // only the last is what the rule leaves, so only the last is checked.
        let value_of = |c: &Check| match &c.probe {
            Probe::Reg { path, name, .. } | Probe::RegAbsent { path, name, .. } => {
                Some((path.to_lowercase(), name.to_lowercase()))
            }
            _ => None,
        };
        let values: Vec<Option<(String, String)>> = checks.iter().map(value_of).collect();
        let mut k = 0;
        checks.retain(|_| {
            let i = k;
            k += 1;
            values[i]
                .as_ref()
                .is_none_or(|v| !values[i + 1..].iter().any(|w| w.as_ref() == Some(v)))
        });
        out.checks.extend(checks);
    }
    out
}
