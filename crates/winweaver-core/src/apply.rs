//! The native apply (`docs/spec/04-architecture.md` section 5): the plan of operations an
//! elevated helper runs through the Windows API, without PowerShell, and the backup it writes
//! before every change. The backup has the format of the stand-alone apply script, so that the
//! program and `Undo-Apply.ps1` both restore it.
//!
//! The default user profile (`DU:` paths) is written at `HKU:\UnattendDefault`, where the helper
//! loads its hive: the key of the stand-alone apply script too. What the plan cannot do natively
//! (scripts of the rules, app and capability removals, the profiles of other users) is listed as
//! skipped: the stand-alone apply script does it.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{Action, Catalog, RegKind, Rule};
use crate::jsonfile;
use crate::profile::Profile;
use crate::render;
use crate::resolver;
use crate::thispc;

pub const PLAN_FORMAT: &str = "winweaver-apply-plan";
pub const PLAN_VERSION: u32 = 1;
const PLAN_MAX_BYTES: u64 = 16 * 1024 * 1024;
const SERVICES: &str = "HKLM:\\SYSTEM\\CurrentControlSet\\Services";

/// One change of the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Op {
    /// Writes a registry value; the key is created when missing.
    SetValue {
        path: String,
        name: String,
        kind: RegKind,
        data: Value,
    },
    /// Deletes a registry value when it exists.
    DeleteValue { path: String, name: String },
    /// Sets the start type of a service that exists (2 automatic, 3 manual, 4 disabled).
    ServiceStart { name: String, start: u32 },
    /// Runs a program of the system folder with arguments; nothing to back up.
    Run { file: String, args: Vec<String> },
}

impl Op {
    /// Whether the operation writes the default user profile, which the helper loads first.
    pub fn in_default_user(&self) -> bool {
        match self {
            Op::SetValue { path, .. } | Op::DeleteValue { path, .. } => in_default_user(path),
            Op::ServiceStart { .. } | Op::Run { .. } => false,
        }
    }
}

/// A change and the rule it belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub rule: String,
    pub op: Op,
}

/// The plan the elevated helper runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub format: String,
    pub version: u32,
    /// A random text the helper must get on its command line too.
    pub nonce: String,
    /// What the plan does, for the log: "apply", "revert" or "undo".
    pub purpose: String,
    pub steps: Vec<Step>,
    /// Rules or actions left to the stand-alone script, and why.
    #[serde(default)]
    pub skipped: Vec<(String, String)>,
}

impl Plan {
    fn new(purpose: &str, nonce: &str) -> Self {
        Self {
            format: PLAN_FORMAT.to_owned(),
            version: PLAN_VERSION,
            nonce: nonce.to_owned(),
            purpose: purpose.to_owned(),
            steps: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    /// Reads a plan file strictly.
    pub fn load(path: &Path) -> Result<Self, String> {
        let plan: Self = jsonfile::read(path, PLAN_MAX_BYTES).map_err(|e| e.to_string())?;
        if plan.format != PLAN_FORMAT || plan.version > PLAN_VERSION {
            return Err(format!("{}: not an apply plan of this version", jsonfile::display_name(path)));
        }
        Ok(plan)
    }

    /// The registry keys and services the plan touches, for the confirmation.
    pub fn changes(&self) -> usize {
        self.steps.len()
    }

    /// Whether a step writes the default user profile.
    pub fn uses_default_user(&self) -> bool {
        self.steps.iter().any(|s| s.op.in_default_user())
    }
}

/// Where the helper loads the hive of the default user profile: the key of the stand-alone apply
/// script, so that the backups of both name the same keys and `Undo-Apply.ps1` restores either.
pub const DEFAULT_USER_KEY: &str = "HKU:\\UnattendDefault";

/// Whether a registry path is in the default user profile the helper loads.
pub fn in_default_user(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let key = DEFAULT_USER_KEY.to_ascii_lowercase();
    path == key || path.starts_with(&format!("{key}\\"))
}

/// Where a registry action of the catalog is written natively: a `DU:` path in the loaded hive of
/// the default user profile; the profiles of other users are left to the stand-alone script.
fn native_path(path: &str) -> Result<String, &'static str> {
    if let Some(rest) = path.strip_prefix("DU:\\") {
        return Ok(format!("{DEFAULT_USER_KEY}\\{rest}"));
    }
    if path.starts_with("DU:") || path.to_ascii_uppercase().starts_with("HKU:") {
        return Err("the profiles of other users are written by the stand-alone apply script");
    }
    Ok(path.to_owned())
}

fn substitute_value(value: &Value, params: &std::collections::BTreeMap<String, Value>) -> Value {
    match value {
        Value::String(s) => Value::String(render::substitute(s, params)),
        Value::Array(items) => Value::Array(items.iter().map(|v| substitute_value(v, params)).collect()),
        other => other.clone(),
    }
}

/// The program a rule runs, with the switches of DISM for an optional feature.
fn feature_run(name: &str, state: &str) -> Op {
    let enable = state.eq_ignore_ascii_case("enabled");
    let mut args = vec![
        "/Online".to_owned(),
        if enable { "/Enable-Feature" } else { "/Disable-Feature" }.to_owned(),
        format!("/FeatureName:{name}"),
    ];
    if enable {
        args.push("/All".to_owned());
    }
    args.push("/NoRestart".to_owned());
    args.push("/Quiet".to_owned());
    Op::Run {
        file: "dism.exe".to_owned(),
        args,
    }
}

/// The operations that make one action of a rule true, or why it is left to the script.
fn apply_ops(action: &Action, params: &std::collections::BTreeMap<String, Value>) -> Result<Vec<Op>, String> {
    Ok(match action {
        Action::Reg {
            path, name, kind, value, ..
        } => vec![Op::SetValue {
            path: native_path(path)?,
            name: name.clone(),
            kind: *kind,
            data: substitute_value(value, params),
        }],
        Action::RegRemove { path, name, .. } => vec![Op::DeleteValue {
            path: native_path(path)?,
            name: name.clone(),
        }],
        Action::Service { name, start, .. } => vec![Op::ServiceStart {
            name: name.clone(),
            start: *start,
        }],
        Action::Exe { file, args } => vec![Op::Run {
            file: file.clone(),
            args: args.iter().map(|a| render::substitute(a, params)).collect(),
        }],
        Action::Feature { name, state, .. } => vec![feature_run(name, state)],
        Action::Ps { .. } => return Err("a script step: the stand-alone apply script runs it".to_owned()),
        Action::Appx { .. } | Action::Capability { .. } => {
            return Err("app and capability removals: the stand-alone apply script does them".to_owned());
        }
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => Vec::new(),
    })
}

/// The operations that return one action to the value of a clean Windows; `Ok(None)` when its
/// default is not known.
fn revert_ops(action: &Action) -> Result<Option<Vec<Op>>, String> {
    Ok(match action {
        Action::Reg {
            path, name, kind, default, ..
        } => {
            let path = native_path(path)?;
            match default {
                Some(Value::String(s)) if s == "absent" => Some(vec![Op::DeleteValue { path, name: name.clone() }]),
                Some(Value::String(s)) if s == "unknown" => None,
                Some(value) => Some(vec![Op::SetValue {
                    path,
                    name: name.clone(),
                    kind: *kind,
                    data: value.clone(),
                }]),
                None if path.to_ascii_lowercase().contains("\\policies\\") => {
                    Some(vec![Op::DeleteValue { path, name: name.clone() }])
                }
                None => None,
            }
        }
        Action::RegRemove { default, .. } => match default {
            None => Some(Vec::new()),
            Some(Value::String(s)) if s == "absent" => Some(Vec::new()),
            Some(_) => None,
        },
        Action::Service { name, default, .. } => default
            .as_ref()
            .and_then(Value::as_u64)
            .map(|start| vec![Op::ServiceStart { name: name.clone(), start: start as u32 }]),
        Action::Feature { name, default, .. } => default.as_ref().map(|state| vec![feature_run(name, state)]),
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => Some(Vec::new()),
        Action::Ps { .. } | Action::Exe { .. } | Action::Appx { .. } | Action::Capability { .. } => None,
    })
}

fn with_requirements(catalog: &Catalog, ids: &HashSet<String>) -> HashSet<String> {
    let mut all = ids.clone();
    let mut stack: Vec<String> = ids.iter().cloned().collect();
    while let Some(id) = stack.pop() {
        if let Some(rule) = catalog.rule(&id) {
            for r in &rule.requires {
                if all.insert(r.clone()) {
                    stack.push(r.clone());
                }
            }
        }
    }
    all
}

fn push_revert(plan: &mut Plan, rule: &Rule) {
    for action in &rule.actions {
        match revert_ops(action) {
            Ok(Some(ops)) => plan.steps.extend(ops.into_iter().map(|op| Step { rule: rule.id.clone(), op })),
            Ok(None) => plan.skipped.push((rule.id.clone(), "the Windows default of a step is not known".to_owned())),
            Err(why) => plan.skipped.push((rule.id.clone(), why)),
        }
    }
}

/// The plan that makes the selection match the profile: the rules on are applied (with the rules
/// they need), the rules off return to the Windows defaults.
pub fn plan_apply(catalog: &Catalog, profile: &Profile, rule_ids: &[String], nonce: &str) -> Plan {
    let resolution = resolver::resolve(catalog, profile);
    let selected: HashSet<String> = rule_ids.iter().cloned().collect();
    let on: HashSet<String> = selected.iter().filter(|id| resolution.in_effect(id)).cloned().collect();
    let applied = with_requirements(catalog, &on);
    let mut plan = Plan::new("apply", nonce);
    for rule in catalog.rules() {
        let is_applied = applied.contains(&rule.id);
        if !is_applied && !selected.contains(&rule.id) {
            continue;
        }
        if let Some(why) = thispc::exclusion(rule) {
            plan.skipped.push((rule.id.clone(), why.to_owned()));
            continue;
        }
        if is_applied {
            let params = render::params_of(catalog, profile, rule);
            for action in &rule.actions {
                match apply_ops(action, &params) {
                    Ok(ops) => plan.steps.extend(ops.into_iter().map(|op| Step { rule: rule.id.clone(), op })),
                    Err(why) => plan.skipped.push((rule.id.clone(), why)),
                }
            }
        } else {
            push_revert(&mut plan, rule);
        }
    }
    plan
}

/// The plan that returns the selection to the Windows defaults.
pub fn plan_revert(catalog: &Catalog, rule_ids: &[String], nonce: &str) -> Plan {
    let selected: HashSet<&str> = rule_ids.iter().map(String::as_str).collect();
    let mut plan = Plan::new("revert", nonce);
    for rule in catalog.rules() {
        if !selected.contains(rule.id.as_str()) {
            continue;
        }
        if let Some(why) = thispc::exclusion(rule) {
            plan.skipped.push((rule.id.clone(), why.to_owned()));
            continue;
        }
        push_revert(&mut plan, rule);
    }
    plan
}

/// The state of a registry value before a change, as the stand-alone apply script writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupEntry {
    pub kind: String,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub existed: bool,
    /// The kind of the value as .NET names it (DWord, QWord, String, ExpandString, MultiString,
    /// Binary).
    #[serde(default, rename = "type")]
    pub value_type: Option<String>,
    #[serde(default)]
    pub value: Value,
}

/// The entries of a backup file (a list; PowerShell writes a single entry as an object).
pub fn read_backup(path: &Path) -> Result<Vec<BackupEntry>, String> {
    let text = jsonfile::read_text_bom(path, PLAN_MAX_BYTES).map_err(|e| e.to_string())?;
    let value: Value = jsonfile::parse(&text, &jsonfile::display_name(path)).map_err(|e| e.to_string())?;
    let items = match value {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        other => vec![other],
    };
    items
        .into_iter()
        .filter(|v| v.get("kind").and_then(Value::as_str) == Some("reg"))
        .map(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        .collect()
}

fn kind_of_name(name: &str) -> Option<RegKind> {
    Some(match name {
        "DWord" => RegKind::DWord,
        "QWord" => RegKind::QWord,
        "String" => RegKind::String,
        "ExpandString" => RegKind::ExpandString,
        "MultiString" => RegKind::MultiString,
        "Binary" => RegKind::Binary,
        _ => return None,
    })
}

/// The plan that restores a backup: the values get their previous contents back, the values
/// that did not exist are deleted, the newest change first.
pub fn plan_undo(entries: &[BackupEntry], nonce: &str) -> Plan {
    let mut plan = Plan::new("undo", nonce);
    for e in entries.iter().rev() {
        let op = if e.existed {
            match e.value_type.as_deref().and_then(kind_of_name) {
                Some(kind) => Op::SetValue {
                    path: e.path.clone(),
                    name: e.name.clone(),
                    kind,
                    data: e.value.clone(),
                },
                None => {
                    plan.skipped.push((format!("{}\\{}", e.path, e.name), "a value of a kind the program does not restore".to_owned()));
                    continue;
                }
            }
        } else {
            Op::DeleteValue {
                path: e.path.clone(),
                name: e.name.clone(),
            }
        };
        plan.steps.push(Step { rule: "undo".to_owned(), op });
    }
    plan
}

/// The registry key of the start type of a service.
pub fn service_key(name: &str) -> String {
    format!("{SERVICES}\\{name}")
}

/// What one step did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepResult {
    pub rule: String,
    pub what: String,
    /// `ok`, `skipped` or `failed`.
    pub status: String,
    #[serde(default)]
    pub note: String,
}

/// The report of the helper.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Report {
    pub purpose: String,
    pub results: Vec<StepResult>,
    /// The file name of the backup in the output folder, empty when nothing was backed up.
    #[serde(default)]
    pub backup: String,
    #[serde(default)]
    pub error: String,
}

impl Report {
    pub fn count(&self, status: &str) -> usize {
        self.results.iter().filter(|r| r.status == status).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn office() -> Profile {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/presets/office.wwprofile");
        Profile::load(&path).unwrap()
    }

    #[test]
    fn plans_follow_the_profile_and_leave_the_scripts_their_part() {
        let catalog = Catalog::embedded().unwrap();
        let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
        let plan = plan_apply(&catalog, &office(), &ids, "n");
        assert!(plan.steps.len() > 100);
        assert!(plan.steps.iter().all(|s| match &s.op {
            Op::SetValue { path, .. } | Op::DeleteValue { path, .. } => !path.starts_with("DU:"),
            _ => true,
        }));
        assert!(plan.steps.iter().any(|s| matches!(&s.op, Op::ServiceStart { name, start: 2 } if name == "Spooler")));
        assert!(plan.steps.iter().any(|s| matches!(&s.op, Op::Run { file, .. } if file == "net.exe")));
        assert!(plan.skipped.iter().any(|(id, why)| id == "apps.remove-quick-assist" && why.contains("app")));
        assert!(plan.uses_default_user(), "the default user profile is written natively");
        assert!(plan.steps.iter().any(|s| matches!(&s.op,
            Op::SetValue { path, .. } if path.starts_with("HKU:\\UnattendDefault\\Software\\"))));
        assert_eq!(
            native_path("DU:\\Software\\X"),
            Ok("HKU:\\UnattendDefault\\Software\\X".to_owned())
        );
        assert!(native_path("HKU:\\.DEFAULT\\X").is_err());
        assert!(in_default_user("hku:\\unattenddefault\\x"));
        assert!(!in_default_user("HKU:\\UnattendDefaultX"));
        let text = plan.to_json();
        let back: Plan = serde_json::from_str(&text).unwrap();
        assert_eq!(back, plan);
        let revert = plan_revert(&catalog, &["lsa.protection".to_owned()], "n");
        assert_eq!(
            revert.steps,
            vec![Step {
                rule: "lsa.protection".to_owned(),
                op: Op::DeleteValue {
                    path: "HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Lsa".to_owned(),
                    name: "RunAsPPL".to_owned(),
                },
            }]
        );
    }

    #[test]
    fn a_backup_turns_into_its_undo() {
        let entries = vec![
            BackupEntry {
                kind: "reg".into(),
                path: "HKLM:\\SOFTWARE\\X".into(),
                name: "A".into(),
                existed: true,
                value_type: Some("DWord".into()),
                value: Value::from(1),
            },
            BackupEntry {
                kind: "reg".into(),
                path: "HKLM:\\SOFTWARE\\X".into(),
                name: "B".into(),
                existed: false,
                value_type: None,
                value: Value::Null,
            },
        ];
        let plan = plan_undo(&entries, "n");
        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(&plan.steps[0].op, Op::DeleteValue { name, .. } if name == "B"), "the newest change first");
        assert!(matches!(&plan.steps[1].op, Op::SetValue { name, kind: RegKind::DWord, .. } if name == "A"));
    }
}
