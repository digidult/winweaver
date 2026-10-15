//! Tests of the plan, the judgement and the capture of This PC on a machine kept in memory.

use std::path::Path;

use serde_json::Value;
use winweaver_core::catalog::{Action, Catalog, Param, Phase, Rule};
use winweaver_core::profile::Profile;
use winweaver_core::thispc::{
    self, FIRST_SIGN_IN, InputLanguage, MemoryMachine, NOTE_CURRENT_USER, ONLY_DURING_INSTALL,
    Probe, ReadAccount, RegData, Report, SIGN_IN_SCREEN, Status, SystemInfo, Verdict,
};

fn office() -> Profile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/presets/office.wwprofile");
    Profile::load(&path).unwrap()
}

fn all_ids(catalog: &Catalog) -> Vec<String> {
    catalog.rules().iter().map(|r| r.id.clone()).collect()
}

fn reading(catalog: &Catalog, machine: &MemoryMachine, id: &str) -> thispc::Reading {
    let plan = thispc::plan(catalog, &office(), &[id.to_owned()]);
    let report = thispc::run(machine, &plan, false, "t");
    thispc::judge(catalog, &plan, &report.results)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{id} was not judged"))
}

/// The registry key and value of every reg action of a rule, with the value it writes.
fn reg_actions(rule: &Rule) -> Vec<(String, String, Value)> {
    rule.actions
        .iter()
        .filter_map(|a| match a {
            Action::Reg {
                path, name, value, ..
            } => Some((path.replace("DU:", "HKCU:"), name.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

fn data_of(value: &Value) -> RegData {
    match value {
        Value::Number(n) => RegData::Dword(n.as_u64().unwrap_or(0) as u32),
        Value::String(s) => RegData::Str(s.clone()),
        Value::Array(items) => RegData::Multi(
            items
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect(),
        ),
        other => RegData::Str(other.to_string()),
    }
}

#[test]
fn exclusions_follow_the_phase_and_the_sign_in_screen() {
    let catalog = Catalog::embedded().unwrap();
    let plan = thispc::plan(&catalog, &office(), &all_ids(&catalog));
    assert_eq!(
        plan.rules.len() + plan.excluded.len(),
        catalog.rules().len()
    );
    let reason = |id: &str| plan.excluded.iter().find(|(r, _)| r == id).map(|(_, w)| *w);
    assert_eq!(reason("install.bypass-tpm"), Some(ONLY_DURING_INSTALL));
    let first_logon = catalog
        .rules()
        .iter()
        .find(|r| r.phase == Phase::UserFirstLogon)
        .unwrap();
    assert_eq!(reason(&first_logon.id), Some(FIRST_SIGN_IN));
    let sign_in = catalog.rules().iter().find(|r| {
        r.actions.iter().any(|a| {
            a.registry_path()
                .is_some_and(|p| p.to_uppercase().starts_with("HKU:\\.DEFAULT"))
        })
    });
    if let Some(r) = sign_in {
        assert_eq!(reason(&r.id), Some(SIGN_IN_SCREEN));
    }
    assert!(
        plan.rules.contains(&"install.netfx3".to_owned()),
        "a script rule is checked as unknown"
    );
    let du = plan
        .checks
        .iter()
        .find(|c| matches!(&c.probe, Probe::Reg { path, .. } if path.starts_with("HKCU:")))
        .unwrap();
    assert_eq!(du.note, NOTE_CURRENT_USER);
}

#[test]
fn a_rule_is_judged_by_its_values() {
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog.rule("defender.pua").unwrap();
    let (path, name, value) = reg_actions(rule).remove(0);
    let profile_value = office()
        .param(&catalog, "defender.pua", "action")
        .unwrap_or(value.clone());
    let wanted = match &value {
        Value::String(s) if s.starts_with('{') => profile_value,
        other => other.clone(),
    };
    let mut m = MemoryMachine::default();
    m.set(&path, &name, data_of(&wanted));
    assert_eq!(
        reading(&catalog, &m, "defender.pua").verdict,
        Verdict::InEffect
    );
    let mut other = MemoryMachine::default();
    other.set(&path, &name, RegData::Dword(77));
    assert_eq!(
        reading(&catalog, &other, "defender.pua").verdict,
        Verdict::NotInEffect
    );
    let empty = MemoryMachine::default();
    assert_eq!(
        reading(&catalog, &empty, "defender.pua").verdict,
        Verdict::NotInEffect
    );
    let denied = MemoryMachine {
        denied: vec![path.to_lowercase()],
        ..MemoryMachine::default()
    };
    assert_eq!(
        reading(&catalog, &denied, "defender.pua").verdict,
        Verdict::NotReadable
    );
}

#[test]
fn a_partly_applied_rule_is_partly_in_effect() {
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog
        .rules()
        .iter()
        .filter(|r| thispc::exclusion(r).is_none())
        .find(|r| {
            let regs = reg_actions(r);
            regs.len() >= 2
                && r.actions.len() == regs.len()
                && regs
                    .iter()
                    .all(|(p, _, v)| p.to_lowercase().contains("\\policies\\") && !v.is_string())
        })
        .expect("a rule with two policy values");
    let regs = reg_actions(rule);
    let mut m = MemoryMachine::default();
    m.set(&regs[0].0, &regs[0].1, data_of(&regs[0].2));
    m.set(&regs[1].0, &regs[1].1, RegData::Dword(4242));
    assert_eq!(reading(&catalog, &m, &rule.id).verdict, Verdict::Partly);
}

#[test]
fn a_value_a_clean_windows_has_is_no_evidence() {
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog.rule("printing.spooler-automatic").unwrap();
    let mut m = MemoryMachine::default();
    for a in &rule.actions {
        if let Action::Service { name, start, .. } = a {
            m.set(
                &format!("HKLM:\\SYSTEM\\CurrentControlSet\\Services\\{name}"),
                "Start",
                RegData::Dword(*start),
            );
        }
    }
    let r = reading(&catalog, &m, "printing.spooler-automatic");
    assert_eq!(r.verdict, Verdict::AsDefault, "{r:?}");
    assert_eq!(r.unknown, 1, "the script step cannot be read");
}

#[test]
fn a_parameter_is_read_from_its_value() {
    let catalog = Catalog::embedded().unwrap();
    let (rule, path, name, pname, value) = catalog
        .rules()
        .iter()
        .filter(|r| thispc::exclusion(r).is_none())
        .find_map(|r| {
            r.actions.iter().find_map(|a| match a {
                Action::Reg {
                    path, name, value, ..
                } => {
                    let inner = value.as_str()?.strip_prefix('{')?.strip_suffix('}')?;
                    let Param::Enum {
                        values, default, ..
                    } = r.params.get(inner)?
                    else {
                        return None;
                    };
                    let other = values.iter().find(|v| &v.value != default)?;
                    (r.actions.len() == 1).then(|| {
                        (
                            r,
                            path.replace("DU:", "HKCU:"),
                            name.clone(),
                            inner.to_owned(),
                            other.value.clone(),
                        )
                    })
                }
                _ => None,
            })
        })
        .expect("a rule with one value that is an enum parameter");
    let mut m = MemoryMachine::default();
    m.set(&path, &name, data_of(&value));
    let r = reading(&catalog, &m, &rule.id);
    assert_eq!(r.verdict, Verdict::InEffect, "{r:?}");
    assert_eq!(r.params.get(&pname), Some(&value));
    let mut base = office();
    base.set_on(&rule.id, false);
    let (captured, summary) = thispc::capture(&catalog, &base, &[r], None);
    assert!(captured.is_on(&rule.id));
    assert_eq!(captured.rules[&rule.id].params.get(&pname), Some(&value));
    assert_eq!(summary.counts.get("in effect"), Some(&1));
}

#[test]
fn apps_are_read_from_the_package_lists() {
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog
        .rules()
        .iter()
        .find(|r| {
            r.actions.len() == 1
                && matches!(&r.actions[0], Action::Appx { names } if names.len() == 1)
        })
        .expect("a rule that removes one app");
    let Action::Appx { names } = &rule.actions[0] else {
        unreachable!()
    };
    let mut present = MemoryMachine::default();
    present.keys.insert(
        "hkcu:\\software\\classes\\local settings\\software\\microsoft\\windows\\currentversion\\appmodel\\repository\\packages".to_owned(),
        vec![format!("{}_1.0.0.0_x64__8wekyb3d8bbwe", names[0])],
    );
    assert_eq!(
        reading(&catalog, &present, &rule.id).verdict,
        Verdict::NotInEffect
    );
    let absent = MemoryMachine::default();
    assert_eq!(
        reading(&catalog, &absent, &rule.id).verdict,
        Verdict::InEffect
    );
}

#[test]
fn the_forms_take_what_a_profile_can_hold() {
    let catalog = Catalog::embedded().unwrap();
    let info = SystemInfo {
        edition_id: "Professional".into(),
        time_zone: "GTB Standard Time".into(),
        ui_language: "uk-UA".into(),
        system_locale: "uk-UA".into(),
        user_locale: "uk-UA".into(),
        input: vec![
            InputLanguage {
                tag: "uk-UA".into(),
                tips: vec![],
            },
            InputLanguage {
                tag: "xx-XX".into(),
                tips: vec![],
            },
        ],
        accounts: vec![ReadAccount {
            name: "Office".into(),
            full_name: "Office PC".into(),
            description: String::new(),
            group: "Users".into(),
        }],
        ..SystemInfo::default()
    };
    let (p, summary) = thispc::capture(&catalog, &office(), &[], Some(&info));
    assert_eq!(p.install.edition, "Pro");
    assert_eq!(p.install.time_zone, "GTB Standard Time");
    assert_eq!(p.languages.input, vec!["uk-UA".to_owned()]);
    assert_eq!(p.accounts.len(), 1);
    assert!(p.accounts[0].password.is_empty());
    assert!(summary.notes.iter().any(|n| n.contains("xx-XX")));
    let home = SystemInfo {
        edition_id: "Core".into(),
        ..SystemInfo::default()
    };
    let (p, summary) = thispc::capture(&catalog, &office(), &[], Some(&home));
    assert_eq!(p.install.edition, office().install.edition);
    assert!(summary.notes.iter().any(|n| n.contains("Core")));
}

#[test]
fn accounts_outside_the_two_groups_are_left_out_and_a_failure_is_told() {
    let catalog = Catalog::embedded().unwrap();
    let info = SystemInfo {
        accounts: vec![
            ReadAccount {
                name: "Admin".into(),
                group: "Administrators".into(),
                ..ReadAccount::default()
            },
            ReadAccount {
                name: "Service".into(),
                ..ReadAccount::default()
            },
        ],
        ..SystemInfo::default()
    };
    let (p, _) = thispc::capture(&catalog, &office(), &[], Some(&info));
    let names: Vec<&str> = p.accounts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Admin"]);
    let failed = SystemInfo {
        accounts_error: "error 5".into(),
        ..SystemInfo::default()
    };
    let (p, summary) = thispc::capture(&catalog, &office(), &[], Some(&failed));
    assert_eq!(p.accounts, office().accounts);
    assert!(
        summary.notes.iter().any(|n| n.contains("error 5")),
        "{:?}",
        summary.notes
    );
}

#[test]
fn a_report_of_the_audit_script_is_judged_like_a_native_read() {
    let catalog = Catalog::embedded().unwrap();
    let ids = vec![
        "defender.pua".to_owned(),
        "printing.spooler-automatic".to_owned(),
    ];
    let plan = thispc::plan(&catalog, &office(), &ids);
    let mut rows = Vec::new();
    for c in &plan.checks {
        let status = if c.rule == "defender.pua" {
            "ok"
        } else {
            "unknown"
        };
        rows.push(format!(
            r#"{{"rule": "{}", "check": "x", "status": "{status}", "current": "1", "expected": "1", "note": ""}}"#,
            c.rule
        ));
    }
    let text = format!(
        "\u{feff}{{\"computer\": \"PC1\", \"admin\": true, \"time\": \"2026-10-14T10:00:00\", \"system\": {{\"edition_id\": \"Professional\", \"input\": {{\"tag\": \"en-US\", \"tips\": \"0409:00000409\"}}, \"accounts\": null}}, \"results\": [{}]}}",
        rows.join(", ")
    );
    let report = Report::parse(&text, "report.json").unwrap();
    assert_eq!(report.computer, "PC1");
    assert!(report.admin);
    let system = report.system.as_ref().unwrap();
    assert_eq!(system.input.len(), 1);
    assert_eq!(system.input[0].tips, vec!["0409:00000409".to_owned()]);
    assert_eq!(report.results.len(), plan.checks.len());
    assert!(report.results.iter().any(|r| r.status == Status::Unknown));
    let readings = thispc::judge(&catalog, &plan, &report.results);
    let verdict = |id: &str| readings.iter().find(|r| r.rule == id).unwrap().verdict;
    assert_eq!(verdict("printing.spooler-automatic"), Verdict::NotReadable);
    assert!(matches!(
        verdict("defender.pua"),
        Verdict::InEffect | Verdict::AsDefault
    ));
    assert!(Report::parse("{\"a\": 1}", "x.json").is_err());
}

/// Every rule of plain registry values reads as in effect (or as on a clean Windows) on a PC
/// where its actions ran in their order: a value removed for a clean start and written again,
/// or written twice, is judged by what the rule leaves.
#[test]
fn every_rule_of_registry_values_reads_as_applied() {
    let catalog = Catalog::embedded().unwrap();
    let mut wrong = Vec::new();
    for rule in catalog.rules() {
        if thispc::exclusion(rule).is_some() || !rule.params.is_empty() || rule.actions.is_empty() {
            continue;
        }
        let plain = rule.actions.iter().all(|a| match a {
            Action::Reg { kind, .. } => !matches!(kind, winweaver_core::catalog::RegKind::Binary),
            Action::RegRemove { .. } => true,
            _ => false,
        });
        if !plain {
            continue;
        }
        let mut m = MemoryMachine::default();
        for action in &rule.actions {
            match action {
                Action::Reg { path, name, value, .. } => {
                    m.set(&path.replace("DU:", "HKCU:"), name, data_of(value))
                }
                Action::RegRemove { path, name, .. } => {
                    m.values.remove(&(
                        path.replace("DU:", "HKCU:").to_lowercase(),
                        name.to_lowercase(),
                    ));
                }
                _ => {}
            }
        }
        let verdict = reading(&catalog, &m, &rule.id).verdict;
        if !matches!(verdict, Verdict::InEffect | Verdict::AsDefault) {
            wrong.push(format!("{} {verdict:?}", rule.id));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
    let plan = thispc::plan(&catalog, &office(), &["defender.realtime".to_owned()]);
    let realtime = |absent: bool| {
        plan.checks
            .iter()
            .filter(|c| match &c.probe {
                Probe::RegAbsent { name, .. } => absent && name == "DisableRealtimeMonitoring",
                Probe::Reg { name, .. } => !absent && name == "DisableRealtimeMonitoring",
                _ => false,
            })
            .count()
    };
    assert_eq!(realtime(true), 0, "the removal before the write is not checked");
    assert_eq!(realtime(false), 1);
}

/// Windows shortens uk-UA to uk in the list of the user; the layouts of the tips pick the
/// keyboard, and a language read twice is kept once.
#[test]
fn input_languages_are_read_as_windows_lists_them() {
    let catalog = Catalog::embedded().unwrap();
    let info = SystemInfo {
        input: vec![
            InputLanguage {
                tag: "en-US".into(),
                tips: vec!["0409:00000409".into()],
            },
            InputLanguage {
                tag: "uk".into(),
                tips: vec!["0422:00020422".into()],
            },
            InputLanguage {
                tag: "ru-UA".into(),
                tips: vec!["2000:00000419".into()],
            },
            InputLanguage {
                tag: "uk-UA".into(),
                tips: vec![],
            },
        ],
        ..SystemInfo::default()
    };
    let (profile, summary) = thispc::capture(&catalog, &office(), &[], Some(&info));
    assert_eq!(profile.languages.input, vec!["en-US", "uk-UA", "ru-UA"]);
    assert!(
        !summary.notes.iter().any(|n| n.contains("input")),
        "{:?}",
        summary.notes
    );
}

/// A rule of which one value was changed after the installation is partly in effect: the
/// capture turns it off and names the value that differs.
#[test]
fn a_rule_partly_in_effect_names_what_differs() {
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog.rule("default-user.widgets-button-off").unwrap();
    let mut m = MemoryMachine::default();
    for (path, name, value) in reg_actions(rule) {
        m.set(&path, &name, data_of(&value));
    }
    let id = "default-user.no-consumer-content";
    let cdm = catalog.rule(id).unwrap();
    let actions = reg_actions(cdm);
    for (path, name, value) in &actions {
        m.set(path, name, data_of(value));
    }
    let (path, name, _) = &actions[0];
    m.set(path, name, RegData::Dword(1));
    let r = reading(&catalog, &m, id);
    assert_eq!(r.verdict, Verdict::Partly);
    assert_eq!(r.differing.len(), 1, "{:?}", r.differing);
    assert!(r.differing[0].contains(name.as_str()), "{:?}", r.differing);
    let (profile, summary) = thispc::capture(&catalog, &office(), &[r], None);
    assert!(!profile.is_on(id));
    assert_eq!(summary.partly.len(), 1);
    assert_eq!(summary.partly[0].0, id);
}
