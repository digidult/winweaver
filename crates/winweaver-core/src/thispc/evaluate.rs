//! The checks of a plan run on a machine: one result per check, as the audit script would
//! write it.

use serde_json::Value;

use crate::catalog::{RegKind, values};

use super::plan::{Check, Plan, Probe};
use super::report::{CheckResult, Report, Status};
use super::{
    Components, Machine, NOTE_ADMIN, NOTE_NOT_ON_BUILD, PROVISIONED_PACKAGES, RegData, SERVICES,
    USER_PACKAGES,
};

fn expected_text(expected: &Value) -> String {
    match expected {
        Value::Array(list) => list.iter().map(values::text).collect::<Vec<_>>().join(","),
        other => values::text(other),
    }
}

/// `name=value` for every item, joined by commas as the audit script writes them.
fn pairs_text(names: &[String], values: &[String]) -> String {
    names
        .iter()
        .zip(values)
        .map(|(n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn result(check: &Check, status: Status, current: String, note: &str) -> CheckResult {
    let expected = match &check.probe {
        Probe::Reg { expected, .. } => expected_text(expected),
        Probe::RegAbsent { .. } | Probe::AppAbsent { .. } => "(absent)".to_owned(),
        Probe::RegList { names, values, .. } => pairs_text(names, values),
        Probe::Service { start, .. } => start.to_string(),
        Probe::Feature { state, .. } => state.clone(),
        Probe::CapabilityAbsent { .. } => "NotPresent".to_owned(),
        Probe::Unknown { .. } => String::new(),
    };
    let note = if note.is_empty() {
        check.note.clone()
    } else if check.note.is_empty() {
        note.to_owned()
    } else {
        format!("{}; {note}", check.note)
    };
    CheckResult {
        rule: check.rule.clone(),
        check: check.label(),
        status,
        current,
        expected,
        note,
    }
}

/// Whether a value read equals the expected one, as the audit script compares (PowerShell
/// compares texts without regard to case).
fn matches_expected(kind: RegKind, data: &RegData, expected: &Value) -> bool {
    match kind {
        RegKind::DWord | RegKind::QWord => {
            let want = match expected {
                Value::Number(n) => n.as_i64(),
                Value::String(s) => s.trim().parse().ok(),
                Value::Bool(b) => Some(i64::from(*b)),
                _ => None,
            };
            want.is_some_and(|w| data.numbers().contains(&w))
        }
        _ => data.text().to_lowercase() == expected_text(expected).to_lowercase(),
    }
}

/// The current text and whether a feature is in the state a rule wants; the audit script
/// compares with `-like "<state>*"`, so `Disabled` also takes `DisabledWithPayloadRemoved`.
fn feature_state(list: &[(String, String)], name: &str, state: &str) -> Option<(String, bool)> {
    let (_, current) = list.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
    let ok = current
        .to_ascii_lowercase()
        .starts_with(&state.to_ascii_lowercase());
    Some((current.clone(), ok))
}

fn evaluate(machine: &dyn Machine, components: Option<&Components>, check: &Check) -> CheckResult {
    match &check.probe {
        Probe::Reg {
            path,
            name,
            kind,
            expected,
            ..
        } => match machine.value(path, name) {
            Err(e) => result(check, Status::Unknown, String::new(), &e),
            Ok(None) => result(check, Status::Differs, String::new(), ""),
            Ok(Some(data)) => {
                let status = if matches_expected(*kind, &data, expected) {
                    Status::Ok
                } else {
                    Status::Differs
                };
                result(check, status, data.text(), "")
            }
        },
        Probe::RegAbsent { path, name, .. } => match machine.value(path, name) {
            Err(e) => result(check, Status::Unknown, String::new(), &e),
            Ok(None) => result(check, Status::Ok, String::new(), ""),
            Ok(Some(data)) => result(check, Status::Differs, data.text(), ""),
        },
        Probe::RegList {
            path,
            names,
            values,
            additive,
            ..
        } => match machine.values(path) {
            Err(e) => result(check, Status::Unknown, String::new(), &e),
            Ok(present) => {
                let present = present.unwrap_or_default();
                let read = |n: &str| {
                    present
                        .iter()
                        .find(|(p, _)| p.eq_ignore_ascii_case(n))
                        .map(|(_, d)| d.text())
                };
                let listed = names
                    .iter()
                    .zip(values)
                    .all(|(n, v)| read(n).is_some_and(|t| t.eq_ignore_ascii_case(v)));
                let extra = present
                    .iter()
                    .any(|(p, _)| !names.iter().any(|n| n.eq_ignore_ascii_case(p)));
                let ok = listed && (*additive || !extra);
                let current = present
                    .iter()
                    .map(|(n, d)| format!("{n}={}", d.text()))
                    .collect::<Vec<_>>()
                    .join(",");
                let status = if ok { Status::Ok } else { Status::Differs };
                result(check, status, current, "")
            }
        },
        Probe::Service { name, start, .. } => {
            match machine.value(&format!("{SERVICES}\\{name}"), "Start") {
                Err(e) => result(check, Status::Unknown, String::new(), &e),
                Ok(None) => result(check, Status::Unknown, String::new(), "service not present"),
                Ok(Some(data)) => {
                    let status = if data.numbers().contains(&i64::from(*start)) {
                        Status::Ok
                    } else {
                        Status::Differs
                    };
                    result(check, status, data.text(), "")
                }
            }
        }
        Probe::AppAbsent { name } => {
            let prefix = format!("{}_", name.to_lowercase());
            let mut found = Vec::new();
            let mut readable = 0;
            let mut notes = Vec::new();
            for (where_, path) in [
                ("current user", USER_PACKAGES),
                ("provisioned", PROVISIONED_PACKAGES),
            ] {
                match machine.subkeys(path) {
                    Ok(keys) => {
                        readable += 1;
                        found.extend(
                            keys.unwrap_or_default()
                                .into_iter()
                                .filter(|k| k.to_lowercase().starts_with(&prefix)),
                        );
                    }
                    Err(_) => notes.push(format!("the {where_} packages cannot be read")),
                }
            }
            if readable == 0 {
                result(check, Status::Unknown, String::new(), &notes.join("; "))
            } else if found.is_empty() {
                result(check, Status::Ok, String::new(), &notes.join("; "))
            } else {
                result(check, Status::Differs, found.join(","), &notes.join("; "))
            }
        }
        Probe::Feature { name, state, .. } => match components.map(|c| &c.features) {
            None => result(check, Status::Unknown, String::new(), NOTE_ADMIN),
            Some(Err(e)) => result(check, Status::Unknown, String::new(), e),
            Some(Ok(list)) => match feature_state(list, name, state) {
                None => result(check, Status::Ok, "absent".to_owned(), NOTE_NOT_ON_BUILD),
                Some((current, ok)) => {
                    let status = if ok { Status::Ok } else { Status::Differs };
                    result(check, status, current, "")
                }
            },
        },
        Probe::CapabilityAbsent { pattern } => match components.map(|c| &c.capabilities) {
            None => result(check, Status::Unknown, String::new(), NOTE_ADMIN),
            Some(Err(e)) => result(check, Status::Unknown, String::new(), e),
            Some(Ok(list)) => {
                let installed: Vec<&str> = list
                    .iter()
                    .filter(|(n, s)| s.eq_ignore_ascii_case("Installed") && wildcard(pattern, n))
                    .map(|(n, _)| n.as_str())
                    .collect();
                let status = if installed.is_empty() {
                    Status::Ok
                } else {
                    Status::Differs
                };
                result(check, status, installed.join(","), "")
            }
        },
        Probe::Unknown { .. } => result(
            check,
            Status::Unknown,
            String::new(),
            "cannot be checked automatically",
        ),
    }
}

/// Runs the checks of a plan on a machine, with the data of the forms when asked. The features
/// and capabilities are read once, and only by an administrator.
pub fn run(machine: &dyn Machine, plan: &Plan, read_system: bool, time: &str) -> Report {
    let admin = machine.is_admin();
    let components = if admin && plan.admin_checks() > 0 {
        Some(machine.components())
    } else {
        None
    };
    Report {
        computer: machine.computer_name(),
        admin,
        time: time.to_owned(),
        system: read_system.then(|| machine.system_info()),
        results: plan
            .checks
            .iter()
            .map(|c| evaluate(machine, components.as_ref(), c))
            .collect(),
    }
}
