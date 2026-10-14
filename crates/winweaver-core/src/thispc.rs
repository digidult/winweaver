//! This PC: how the rules of the catalog look on a running Windows (`docs/spec/03-features.md`
//! section 5). The plan of checks, the judgement of the rules and the capture into a profile are
//! pure; the machine is reached through [`Machine`], implemented natively by `winweaver-win` and
//! in memory by the tests. Nothing in this module writes anything.
//!
//! The checks are the ones of the audit script (`data/templates/Audit.runtime.ps1`), so a report
//! of that script, made on a PC without WinWeaver, is judged the same way as a native read.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{Action, Catalog, Param, Phase, RegKind, Rule, check::same_value};
use crate::jsonfile;
use crate::profile::{Account, Profile};
use crate::render;
use crate::resources;

/// Why a rule is left out of the work on a running Windows.
pub const ONLY_DURING_INSTALL: &str = "takes effect only during Windows installation";
pub const FIRST_SIGN_IN: &str =
    "runs at the first sign-in of each user; not applied to a running system";
pub const SIGN_IN_SCREEN: &str = "changes the sign-in screen; not applied to a running system";
/// The note of a check of the default user profile, made in HKCU of the current user.
pub const NOTE_CURRENT_USER: &str = "current user instead of the default profile";
/// The note of a check the native read cannot make yet; the audit script can.
pub const NOTE_NOT_NATIVE: &str =
    "not readable by the program yet; the audit script reads it as administrator";
const SERVICES: &str = "HKLM:\\SYSTEM\\CurrentControlSet\\Services";
const USER_PACKAGES: &str = "HKCU:\\Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages";
const PROVISIONED_PACKAGES: &str =
    "HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\Applications";
/// The largest audit report the program reads.
pub const REPORT_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The edition ids of Windows (`EditionID`) and the edition names of the profile.
pub const EDITION_IDS: [(&str, &str); 14] = [
    ("Professional", "Pro"),
    ("ProfessionalN", "Pro N"),
    ("ProfessionalWorkstation", "Pro for Workstations"),
    ("ProfessionalWorkstationN", "Pro for Workstations N"),
    ("ProfessionalEducation", "Pro Education"),
    ("ProfessionalEducationN", "Pro Education N"),
    ("Education", "Education"),
    ("EducationN", "Education N"),
    ("Enterprise", "Enterprise"),
    ("EnterpriseN", "Enterprise N"),
    ("EnterpriseG", "Enterprise G"),
    ("EnterpriseGN", "Enterprise G N"),
    ("EnterpriseS", "Enterprise LTSC 2024 / 2021 / 2019"),
    ("EnterpriseSN", "Enterprise N LTSC"),
];

/// The reason a rule is not checked or applied on a running Windows, if any.
pub fn exclusion(rule: &Rule) -> Option<&'static str> {
    if matches!(
        rule.phase,
        Phase::Windowspe | Phase::SpecializeXml | Phase::OobeXml
    ) || rule.actions.iter().all(Action::is_xml)
    {
        return Some(ONLY_DURING_INSTALL);
    }
    if rule.phase == Phase::UserFirstLogon {
        return Some(FIRST_SIGN_IN);
    }
    let sign_in_screen = rule.actions.iter().any(|a| {
        a.registry_path()
            .is_some_and(|p| p.to_ascii_uppercase().starts_with("HKU:\\.DEFAULT"))
    });
    if sign_in_screen {
        return Some(SIGN_IN_SCREEN);
    }
    None
}

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
            match render::list_entries(*names, prefix, &items) {
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
        for action in &rule.actions {
            out.checks.extend(checks_of(rule, action, &params));
        }
    }
    out
}

/// A registry value as read.
#[derive(Debug, Clone, PartialEq)]
pub enum RegData {
    Dword(u32),
    Qword(u64),
    Str(String),
    ExpandStr(String),
    Multi(Vec<String>),
    Binary(Vec<u8>),
    Other,
}

impl RegData {
    /// The value as the audit script reports it.
    pub fn text(&self) -> String {
        match self {
            RegData::Dword(v) => (*v as i32).to_string(),
            RegData::Qword(v) => v.to_string(),
            RegData::Str(s) | RegData::ExpandStr(s) => s.clone(),
            RegData::Multi(items) => items.join(","),
            RegData::Binary(bytes) => bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            RegData::Other => "(another kind of value)".to_owned(),
        }
    }

    fn numbers(&self) -> Vec<i64> {
        match self {
            RegData::Dword(v) => vec![i64::from(*v), i64::from(*v as i32)],
            RegData::Qword(v) => vec![*v as i64],
            RegData::Str(s) | RegData::ExpandStr(s) => s.trim().parse().into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// What the program reads of a running Windows. Paths are written as in the catalog
/// (`HKLM:\...`, `HKCU:\...`, `HKU:\...`); the name `(Default)` is the default value of a key.
pub trait Machine {
    /// The value, `None` when the key or the value does not exist, an error when it cannot be read.
    fn value(&self, path: &str, name: &str) -> Result<Option<RegData>, String>;
    /// The names of the subkeys, `None` when the key does not exist.
    fn subkeys(&self, path: &str) -> Result<Option<Vec<String>>, String>;
    /// The values of a key with their names, `None` when the key does not exist.
    fn values(&self, path: &str) -> Result<Option<Vec<(String, RegData)>>, String>;
    fn is_admin(&self) -> bool;
    fn computer_name(&self) -> String;
    /// The data of the forms (edition, time zone, languages).
    fn system_info(&self) -> SystemInfo;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Differs,
    Unknown,
}

/// The result of one check, as the audit script writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub rule: String,
    pub check: String,
    pub status: Status,
    #[serde(default)]
    pub current: String,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub note: String,
}

/// An input language read from Windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputLanguage {
    pub tag: String,
    #[serde(default)]
    pub tips: Vec<String>,
}

/// A local account read from Windows (never a password).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReadAccount {
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub group: String,
}

/// The data of the forms read from Windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub computer_name: String,
    pub edition_id: String,
    pub build: String,
    pub display_version: String,
    pub time_zone: String,
    pub ui_language: String,
    pub system_locale: String,
    pub user_locale: String,
    pub input: Vec<InputLanguage>,
    pub accounts: Vec<ReadAccount>,
    /// Why the accounts are missing, when they are.
    pub accounts_error: String,
}

/// An audit: the results of the checks and, optionally, the data of the forms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub computer: String,
    pub admin: bool,
    pub time: String,
    pub system: Option<SystemInfo>,
    pub results: Vec<CheckResult>,
}

fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn field(value: &Value, key: &str) -> String {
    value.get(key).map(text_of).unwrap_or_default()
}

/// A list, also when PowerShell wrote a single object or nothing.
fn items(value: &Value, key: &str) -> Vec<Value> {
    match value.get(key) {
        Some(Value::Array(list)) => list.clone(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => vec![other.clone()],
    }
}

impl SystemInfo {
    fn from_value(v: &Value) -> Self {
        Self {
            computer_name: field(v, "computer_name"),
            edition_id: field(v, "edition_id"),
            build: field(v, "build"),
            display_version: field(v, "display_version"),
            time_zone: field(v, "time_zone"),
            ui_language: field(v, "ui_language"),
            system_locale: field(v, "system_locale"),
            user_locale: field(v, "user_locale"),
            input: items(v, "input")
                .iter()
                .map(|i| match i {
                    Value::String(tag) => InputLanguage {
                        tag: tag.clone(),
                        tips: Vec::new(),
                    },
                    other => InputLanguage {
                        tag: field(other, "tag"),
                        tips: items(other, "tips").iter().map(text_of).collect(),
                    },
                })
                .filter(|i| !i.tag.is_empty())
                .collect(),
            accounts: items(v, "accounts")
                .iter()
                .map(|a| ReadAccount {
                    name: field(a, "name"),
                    full_name: field(a, "full_name"),
                    description: field(a, "description"),
                    group: field(a, "group"),
                })
                .filter(|a| !a.name.is_empty())
                .collect(),
            accounts_error: field(v, "accounts_error"),
        }
    }
}

impl Report {
    /// Parses the JSON report of the audit script (PowerShell writes it with a byte order mark).
    pub fn parse(text: &str, name: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let value: Value = jsonfile::parse(text, name).map_err(|e| e.to_string())?;
        if !value.is_object() || value.get("results").is_none() {
            return Err(format!("{name}: not an audit report of WinWeaver"));
        }
        let mut results = Vec::new();
        for item in items(&value, "results") {
            let status = match field(&item, "status").as_str() {
                "ok" => Status::Ok,
                "differs" => Status::Differs,
                _ => Status::Unknown,
            };
            results.push(CheckResult {
                rule: field(&item, "rule"),
                check: field(&item, "check"),
                status,
                current: field(&item, "current"),
                expected: field(&item, "expected"),
                note: field(&item, "note"),
            });
        }
        Ok(Self {
            computer: field(&value, "computer"),
            admin: value.get("admin").and_then(Value::as_bool).unwrap_or(false),
            time: field(&value, "time"),
            system: value
                .get("system")
                .filter(|s| s.is_object())
                .map(SystemInfo::from_value),
            results,
        })
    }

    /// Reads a report file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = jsonfile::read_text_bom(path, REPORT_MAX_BYTES).map_err(|e| e.to_string())?;
        Self::parse(&text, &jsonfile::display_name(path))
    }
}

fn expected_text(expected: &Value) -> String {
    match expected {
        Value::Array(list) => list.iter().map(text_of).collect::<Vec<_>>().join(","),
        other => text_of(other),
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

fn evaluate(machine: &dyn Machine, check: &Check) -> CheckResult {
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
        Probe::Feature { .. } | Probe::CapabilityAbsent { .. } => {
            result(check, Status::Unknown, String::new(), NOTE_NOT_NATIVE)
        }
        Probe::Unknown { .. } => result(
            check,
            Status::Unknown,
            String::new(),
            "cannot be checked automatically",
        ),
    }
}

/// Runs the checks of a plan on a machine, with the data of the forms when asked.
pub fn run(machine: &dyn Machine, plan: &Plan, read_system: bool, time: &str) -> Report {
    Report {
        computer: machine.computer_name(),
        admin: machine.is_admin(),
        time: time.to_owned(),
        system: read_system.then(|| machine.system_info()),
        results: plan.checks.iter().map(|c| evaluate(machine, c)).collect(),
    }
}

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
                other => text_of(other) == current,
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
                Status::Differs => reading.differs += 1,
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

/// The counts of a capture and what it did with the forms.
#[derive(Debug, Clone, Default)]
pub struct CaptureSummary {
    pub counts: IndexMap<&'static str, usize>,
    pub forms: Vec<String>,
    pub notes: Vec<String>,
}

impl CaptureSummary {
    /// One line: the counts by verdict.
    pub fn line(&self) -> String {
        let parts: Vec<String> = Verdict::ALL
            .iter()
            .filter_map(|v| {
                let n = self.counts.get(v.title()).copied().unwrap_or(0);
                (n > 0).then(|| format!("{n} {}", v.title()))
            })
            .collect();
        if parts.is_empty() {
            "nothing read".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

/// The edition name of a Windows edition id.
pub fn edition_of(edition_id: &str) -> Option<&'static str> {
    EDITION_IDS
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(edition_id))
        .map(|(_, name)| *name)
}

/// The profile `base` with the readings applied: a rule in effect is on with the parameters read,
/// partly or not in effect is off, not readable and as on a clean Windows keep their state. The
/// forms take the edition, time zone, languages and the accounts when the reading has them, never
/// the computer name or a password.
pub fn capture(
    catalog: &Catalog,
    base: &Profile,
    readings: &[Reading],
    system: Option<&SystemInfo>,
) -> (Profile, CaptureSummary) {
    let mut profile = base.clone();
    let mut summary = CaptureSummary::default();
    for reading in readings {
        *summary.counts.entry(reading.verdict.title()).or_default() += 1;
        match reading.verdict {
            Verdict::InEffect => {
                profile.set_on(&reading.rule, true);
                let rule = catalog.rule(&reading.rule);
                let choice = profile.rules.entry(reading.rule.clone()).or_default();
                for (name, value) in &reading.params {
                    let default = rule
                        .and_then(|r| r.params.get(name))
                        .map(Param::default_value);
                    if default.as_ref().is_some_and(|d| same_value(d, value)) {
                        choice.params.shift_remove(name);
                    } else {
                        choice.params.insert(name.clone(), value.clone());
                    }
                }
            }
            Verdict::Partly | Verdict::NotInEffect => profile.set_on(&reading.rule, false),
            Verdict::NotReadable | Verdict::AsDefault => {}
        }
    }
    if let Some(info) = system {
        capture_forms(&mut profile, info, &mut summary);
    }
    (profile, summary)
}

fn capture_forms(profile: &mut Profile, info: &SystemInfo, summary: &mut CaptureSummary) {
    match edition_of(&info.edition_id) {
        Some(edition) => {
            profile.install.edition = edition.to_owned();
            summary.forms.push(format!("edition {edition}"));
        }
        None if !info.edition_id.is_empty() => summary.notes.push(format!(
            "the edition {} is not one a profile can name; the edition was kept",
            info.edition_id
        )),
        None => {}
    }
    if !info.time_zone.is_empty() {
        profile.install.time_zone = info.time_zone.clone();
        summary.forms.push(format!("time zone {}", info.time_zone));
    }
    for (value, target, what) in [
        (
            &info.ui_language,
            &mut profile.languages.ui_language,
            "display language",
        ),
        (
            &info.system_locale,
            &mut profile.languages.system_locale,
            "system locale",
        ),
        (
            &info.user_locale,
            &mut profile.languages.user_locale,
            "user locale",
        ),
    ] {
        if crate::validate::is_culture(value) {
            *target = value.clone();
            summary.forms.push(format!("{what} {value}"));
        }
    }
    let known: Vec<String> = info
        .input
        .iter()
        .map(|i| i.tag.clone())
        .filter(|t| resources::keyboard(t).is_some())
        .collect();
    let unknown: Vec<&str> = info
        .input
        .iter()
        .map(|i| i.tag.as_str())
        .filter(|t| resources::keyboard(t).is_none())
        .collect();
    if !unknown.is_empty() {
        summary.notes.push(format!(
            "input languages the catalog has no layout for were left out: {}",
            unknown.join(", ")
        ));
    }
    if !known.is_empty() {
        summary
            .forms
            .push(format!("input languages {}", known.join(", ")));
        profile.languages.input = known;
    }
    let accounts: Vec<Account> = info
        .accounts
        .iter()
        .filter(|a| matches!(a.group.as_str(), "Administrators" | "Users"))
        .map(|a| Account {
            name: a.name.clone(),
            display_name: a.full_name.clone(),
            group: a.group.clone(),
            description: a.description.clone(),
            password: String::new(),
        })
        .collect();
    if accounts.is_empty() {
        let note = if info.accounts_error.is_empty() {
            "the accounts of the profile were kept".to_owned()
        } else {
            format!(
                "the accounts of the profile were kept, the local accounts could not be read: {}",
                info.accounts_error
            )
        };
        summary.notes.push(note);
    } else {
        summary
            .forms
            .push(format!("{} accounts (no passwords)", accounts.len()));
        profile.accounts = accounts;
    }
}

/// A machine kept in memory, for the tests and for a look at a report without a PC.
#[derive(Debug, Clone, Default)]
pub struct MemoryMachine {
    /// Values by `(path, name)`, both in lower case.
    pub values: std::collections::HashMap<(String, String), RegData>,
    /// Keys that exist, in lower case, with their subkey names.
    pub keys: std::collections::HashMap<String, Vec<String>>,
    /// Keys that cannot be read, in lower case.
    pub denied: Vec<String>,
    pub admin: bool,
    pub system: SystemInfo,
}

impl MemoryMachine {
    pub fn set(&mut self, path: &str, name: &str, data: RegData) {
        self.values
            .insert((path.to_lowercase(), name.to_lowercase()), data);
    }
}

impl Machine for MemoryMachine {
    fn value(&self, path: &str, name: &str) -> Result<Option<RegData>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        Ok(self.values.get(&(p, name.to_lowercase())).cloned())
    }

    fn subkeys(&self, path: &str) -> Result<Option<Vec<String>>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        Ok(self.keys.get(&p).cloned())
    }

    fn values(&self, path: &str) -> Result<Option<Vec<(String, RegData)>>, String> {
        let p = path.to_lowercase();
        if self.denied.iter().any(|d| p.starts_with(d)) {
            return Err("access is denied".to_owned());
        }
        let mut found: Vec<(String, RegData)> = self
            .values
            .iter()
            .filter(|((key, _), _)| *key == p)
            .map(|((_, name), data)| (name.clone(), data.clone()))
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        let exists = !found.is_empty() || self.keys.contains_key(&p);
        Ok(exists.then_some(found))
    }

    fn is_admin(&self) -> bool {
        self.admin
    }

    fn computer_name(&self) -> String {
        "MEMORY".to_owned()
    }

    fn system_info(&self) -> SystemInfo {
        self.system.clone()
    }
}
