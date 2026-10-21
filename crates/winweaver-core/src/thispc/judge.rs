//! The verdict of each rule of a plan from the results of its checks, with the parameter
//! values read.

use indexmap::IndexMap;
use serde_json::Value;

use crate::catalog::{Catalog, Param, check::same_value, values};

use super::plan::{Check, Plan, Probe};
use super::report::{CheckResult, Status};

/// What a reading says about a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Every check matches and at least one tells the rule apart from a clean Windows.
    InEffect,
    /// Some checks match, some do not.
    Partly,
    /// No check that tells anything matches.
    NotInEffect,
    /// No check could be read.
    NotReadable,
    /// Everything matches, but a clean Windows has the same: no evidence of the rule.
    AsDefault,
}

impl Verdict {
    pub const ALL: [Verdict; 5] = [
        Verdict::InEffect,
        Verdict::Partly,
        Verdict::NotInEffect,
        Verdict::NotReadable,
        Verdict::AsDefault,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Verdict::InEffect => "in effect",
            Verdict::Partly => "partly in effect",
            Verdict::NotInEffect => "not in effect",
            Verdict::NotReadable => "not readable",
            Verdict::AsDefault => "as on a clean Windows",
        }
    }
}

/// The reading of one rule.
#[derive(Debug, Clone)]
pub struct Reading {
    pub rule: String,
    pub verdict: Verdict,
    /// Parameter values read, by name.
    pub params: IndexMap<String, Value>,
    pub ok: usize,
    pub differs: usize,
    pub unknown: usize,
    /// The checks that differ: what was read where, and what the rule wants.
    pub differing: Vec<String>,
}

/// The value read for a parameter, when it is one the parameter accepts.
fn param_value(param: &Param, current: &str) -> Option<Value> {
    let current = current.trim();
    if current.is_empty() {
        return None;
    }
    match param {
        Param::Enum { values, .. } => values.iter().find_map(|v| {
            let matches = match &v.value {
                Value::Number(n) => current.parse::<i64>().ok() == n.as_i64(),
                Value::String(s) => s.eq_ignore_ascii_case(current),
                other => values::text(other) == current,
            };
            matches.then(|| v.value.clone())
        }),
        Param::Int { min, max, .. } => current
            .parse::<i64>()
            .ok()
            .filter(|n| (*min..=*max).contains(n))
            .map(Value::from),
        Param::Text { required, .. } => {
            (!*required || !current.is_empty()).then(|| Value::from(current))
        }
        Param::List { .. } => None,
    }
}

/// Whether the Windows default of an action is a value (not absent, not unknown).
fn concrete(default: &Option<Value>) -> Option<&Value> {
    default
        .as_ref()
        .filter(|d| !matches!(d.as_str(), Some("absent") | Some("unknown")))
}

/// Whether a check that matches tells the rule apart from a clean Windows.
fn evidence(probe: &Probe, value: Option<&Value>) -> bool {
    match probe {
        Probe::Reg {
            expected, default, ..
        } => {
            let value = value.unwrap_or(expected);
            match default {
                Some(d) if d.as_str() == Some("absent") => true,
                Some(d) if d.as_str() == Some("unknown") => true,
                Some(d) => !same_value(d, value),
                None => true,
            }
        }
        Probe::RegAbsent { default, .. } => {
            matches!(default, Some(d) if d.as_str() != Some("absent"))
        }
        Probe::RegList { names, .. } => !names.is_empty(),
        Probe::Service { start, default, .. } => match concrete(default) {
            Some(d) => d.as_u64() != Some(u64::from(*start)),
            None => true,
        },
        Probe::Feature { state, default, .. } => match default {
            Some(d) => !d.eq_ignore_ascii_case(state),
            None => true,
        },
        Probe::CapabilityAbsent { .. } | Probe::AppAbsent { .. } => true,
        Probe::Unknown { .. } => false,
    }
}

/// Judges every rule of a plan by the results of its checks. A result belongs to the check of
/// the same rule at the same position, so a report of the audit script pairs as a native read.
pub fn judge(catalog: &Catalog, plan: &Plan, results: &[CheckResult]) -> Vec<Reading> {
    let mut by_rule: IndexMap<&str, Vec<&CheckResult>> = IndexMap::new();
    for r in results {
        by_rule.entry(r.rule.as_str()).or_default().push(r);
    }
    let mut checks_by_rule: IndexMap<&str, Vec<&Check>> = IndexMap::new();
    for c in &plan.checks {
        checks_by_rule.entry(c.rule.as_str()).or_default().push(c);
    }
    let mut out = Vec::new();
    for id in &plan.rules {
        let Some(rule) = catalog.rule(id) else {
            continue;
        };
        let checks = checks_by_rule.get(id.as_str()).cloned().unwrap_or_default();
        let found = by_rule.get(id.as_str()).cloned().unwrap_or_default();
        let mut reading = Reading {
            rule: id.clone(),
            verdict: Verdict::NotReadable,
            params: IndexMap::new(),
            ok: 0,
            differs: 0,
            unknown: 0,
            differing: Vec::new(),
        };
        let mut any_evidence = false;
        let mut ok_evidence = 0;
        for (k, check) in checks.iter().enumerate() {
            let Some(res) = found.get(k) else {
                reading.unknown += 1;
                continue;
            };
            let mut status = res.status;
            let mut read = None;
            if let Probe::Reg { param: Some(p), .. } = &check.probe
                && status != Status::Unknown
                && let Some(param) = rule.params.get(p)
                && let Some(v) = param_value(param, &res.current)
            {
                status = Status::Ok;
                reading.params.insert(p.clone(), v.clone());
                read = Some(v);
            }
            match status {
                Status::Ok => {
                    reading.ok += 1;
                    if evidence(&check.probe, read.as_ref()) {
                        any_evidence = true;
                        ok_evidence += 1;
                    }
                }
                Status::Differs => {
                    reading.differs += 1;
                    let current = if res.current.is_empty() {
                        "(absent)"
                    } else {
                        res.current.as_str()
                    };
                    reading.differing.push(format!(
                        "{} = {current}, the rule wants {}",
                        res.check, res.expected
                    ));
                }
                Status::Unknown => reading.unknown += 1,
            }
        }
        reading.verdict = if reading.ok + reading.differs == 0 {
            Verdict::NotReadable
        } else if reading.differs == 0 {
            if any_evidence {
                Verdict::InEffect
            } else {
                Verdict::AsDefault
            }
        } else if ok_evidence > 0 {
            Verdict::Partly
        } else {
            Verdict::NotInEffect
        };
        out.push(reading);
    }
    out
}
