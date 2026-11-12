//! The native apply (`docs/spec/04-architecture.md` section 5): the plan of operations an
//! elevated helper runs through the Windows API, without PowerShell, and the backup it writes
//! before every change. The backup has the format of the stand-alone apply script, so that the
//! program and `Undo-Apply.ps1` both restore it.
//!
//! The default user profile (`DU:` paths) is written at `HKU:\UnattendDefault`, where the helper
//! loads its hive: the key of the stand-alone apply script too. Capabilities are removed through
//! DISM and apps through the package API of Windows; the backup names them, and no undo brings
//! them back. What the plan cannot do natively (scripts of the rules, the profiles of other users)
//! is listed as skipped: the stand-alone apply script does it.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::catalog::{Action, Catalog, RegKind, Rule, values};
use crate::jsonfile;
use crate::profile::Profile;
use crate::render;
use crate::resolver;
use crate::thispc;

pub const PLAN_FORMAT: &str = "winweaver-apply-plan";
pub const PLAN_VERSION: u32 = 1;
/// The largest plan or backup file the program reads.
pub const PLAN_MAX_BYTES: u64 = 16 * 1024 * 1024;
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
    /// Writes the values of a list key; without `additive` the other values of the key are
    /// deleted first. Every value it touches is backed up.
    SetList {
        path: String,
        kind: RegKind,
        names: Vec<String>,
        values: Vec<String>,
        additive: bool,
    },
    /// Sets the start type of a service that exists (2 automatic, 3 manual, 4 disabled).
    ServiceStart { name: String, start: u32 },
    /// Runs a program of the system folder with arguments; nothing to back up.
    Run { file: String, args: Vec<String> },
    /// Removes every installed capability whose name matches a pattern of
    /// `Get-WindowsCapability -Name`; the backup names each one, and no undo brings it back.
    RemoveCapability { pattern: String },
    /// Removes the provisioned packages and the packages of every user of the apps with these
    /// names; the backup names each one found, and no undo brings it back.
    RemoveApps { names: Vec<String> },
    /// Runs a program of the system folder on a folder of the installation media: the first drive
    /// but the one of Windows whose `folder` holds a file of `pattern` (`*.cab`), in its
    /// subfolders too when `deep`; `{source}` in the arguments is that folder. Skipped when no
    /// drive has one. The drive of Windows is left out on a running PC: a standard user can make
    /// a folder at its root, and what is found is installed by an administrator.
    RunFromMedia {
        folder: String,
        pattern: String,
        deep: bool,
        file: String,
        args: Vec<String>,
    },
    /// Sets the start type of a service whose start type is `from` now (a disabled update service
    /// back to manual).
    ServiceStartFrom { name: String, from: u32, start: u32 },
    /// Writes a value in every subkey of a key, such as the network interfaces of a service.
    SetValueInSubkeys {
        path: String,
        name: String,
        kind: RegKind,
        data: Value,
    },
    /// Deletes a key with everything under it; every value under it is backed up first, so an
    /// undo makes the key again with its values.
    DeleteKey { path: String },
    /// Sets "password never expires" on the local accounts of these names that exist; no undo
    /// clears it.
    PasswordNeverExpires { accounts: Vec<String> },
    /// Disables the built-in accounts Administrator and Guest (relative ids 500 and 501), never
    /// the account the helper runs as; no undo enables them.
    DisableBuiltinAccounts,
    /// Puts the lines of the error file of the installation into the report and deletes it.
    CollectErrors { file: String },
    /// Deletes the copies of the answer file that exist (they may hold passwords); no undo brings
    /// them back.
    DeleteFiles { paths: Vec<String> },
}

impl Op {
    /// Whether the operation writes the default user profile, which the helper loads first.
    pub fn in_default_user(&self) -> bool {
        match self {
            Op::SetValue { path, .. }
            | Op::DeleteValue { path, .. }
            | Op::SetList { path, .. }
            | Op::SetValueInSubkeys { path, .. }
            | Op::DeleteKey { path } => in_default_user(path),
            Op::ServiceStart { .. }
            | Op::Run { .. }
            | Op::RemoveCapability { .. }
            | Op::RemoveApps { .. }
            | Op::RunFromMedia { .. }
            | Op::ServiceStartFrom { .. }
            | Op::PasswordNeverExpires { .. }
            | Op::DisableBuiltinAccounts
            | Op::CollectErrors { .. }
            | Op::DeleteFiles { .. } => false,
        }
    }

    /// What the step does, in a line of the report and of a registry file.
    pub fn describe(&self) -> String {
        match self {
            Op::SetValue {
                path, name, data, ..
            } => format!("{path}\\{name} = {data}"),
            Op::DeleteValue { path, name } => format!("{path}\\{name} removed"),
            Op::SetList {
                path,
                names,
                additive,
                ..
            } => {
                let how = if *additive { "added to" } else { "the list of" };
                format!("{} values {how} {path}", names.len())
            }
            Op::ServiceStart { name, start } => format!("service {name} start type {start}"),
            Op::Run { file, args } => format!("{file} {}", args.join(" ")),
            Op::RemoveCapability { pattern } => format!("capabilities {pattern} removed"),
            Op::RemoveApps { names } => format!("apps {} removed", names.join(", ")),
            Op::RunFromMedia {
                folder, file, args, ..
            } => {
                let args = args.join(" ");
                format!("{file} {args} with the folder {folder} of a drive")
            }
            Op::ServiceStartFrom { name, from, start } => {
                format!("service {name} start type {start} where it is {from}")
            }
            Op::SetValueInSubkeys {
                path, name, data, ..
            } => format!("{name} = {data} in every subkey of {path}"),
            Op::DeleteKey { path } => format!("{path} deleted with its subkeys"),
            Op::PasswordNeverExpires { accounts } => {
                format!("password never expires: {}", accounts.join(", "))
            }
            Op::DisableBuiltinAccounts => {
                "the built-in Administrator and Guest disabled".to_owned()
            }
            Op::CollectErrors { file } => {
                format!("the errors of {file} read and the file deleted")
            }
            Op::DeleteFiles { paths } => format!("deleted when present: {}", paths.join(", ")),
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
    /// A random text that makes the bytes of every plan, and so its SHA-256, its own. The helper
    /// gets the SHA-256 of the file on its command line, never this text.
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

    /// Reads a plan from the bytes of its file strictly; `name` names it in the errors.
    pub fn from_bytes(bytes: &[u8], name: &str) -> Result<Self, String> {
        if bytes.len() as u64 > PLAN_MAX_BYTES {
            return Err(format!("{name}: larger than a plan can be"));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| format!("{name}: not UTF-8"))?;
        let plan: Self = jsonfile::parse(text, name).map_err(|e| e.to_string())?;
        if plan.format != PLAN_FORMAT || plan.version > PLAN_VERSION {
            return Err(format!("{name}: not an apply plan of this version"));
        }
        Ok(plan)
    }

    /// Reads a plan file strictly.
    pub fn load(path: &Path) -> Result<Self, String> {
        let plan: Self = jsonfile::read(path, PLAN_MAX_BYTES).map_err(|e| e.to_string())?;
        if plan.format != PLAN_FORMAT || plan.version > PLAN_VERSION {
            return Err(format!(
                "{}: not an apply plan of this version",
                jsonfile::display_name(path)
            ));
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

    /// The steps that remove apps or capabilities, which no undo brings back.
    pub fn removals(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| matches!(s.op, Op::RemoveCapability { .. } | Op::RemoveApps { .. }))
            .count()
    }

    /// The other steps no undo takes back: programs that run, accounts that change, files that
    /// are deleted.
    pub fn irreversible(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| {
                matches!(
                    s.op,
                    Op::Run { .. }
                        | Op::RunFromMedia { .. }
                        | Op::PasswordNeverExpires { .. }
                        | Op::DisableBuiltinAccounts
                        | Op::CollectErrors { .. }
                        | Op::DeleteFiles { .. }
                )
            })
            .count()
    }
}

/// The SHA-256 of the bytes of a plan file in lower case hexadecimal. The window passes it on the
/// command line of the elevated helper, which runs a plan only when its file has these bytes, so a
/// plan changed in the output folder after the permission is refused.
pub fn digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in Sha256::digest(bytes).iter() {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
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

/// The program a rule runs, with the switches of DISM for an optional feature.
fn feature_run(name: &str, state: &str) -> Op {
    let enable = state.eq_ignore_ascii_case("enabled");
    let mut args = vec![
        "/Online".to_owned(),
        if enable {
            "/Enable-Feature"
        } else {
            "/Disable-Feature"
        }
        .to_owned(),
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

/// Why a script step the program does not know stays with the stand-alone script.
const SCRIPT_STEP: &str = "a script step: the stand-alone apply script runs it";

/// The copies of the answer file the step of the rule `post-oobe.delete-answer-file-copies`
/// deletes, as its script names them; the executor deletes no other file.
pub const ANSWER_FILE_COPIES: [&str; 6] = [
    "C:\\Windows\\Panther\\unattend.xml",
    "C:\\Windows\\Panther\\unattend-original.xml",
    "C:\\Windows\\Panther\\Unattend\\unattend.xml",
    "C:\\Windows\\System32\\Sysprep\\unattend.xml",
    "C:\\unattend.xml",
    "C:\\autounattend.xml",
];
/// The file the commands of the specialize pass write their errors into.
pub const SETUP_ERRORS: &str = "C:\\Windows\\Temp\\ua.err";

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The script step of a rule of the catalog as native operations: what its PowerShell does, done
/// by the program without running it. `None` for a script step it does not know, which stays with
/// the stand-alone script.
pub fn script_ops(rule: &str, profile: &Profile) -> Option<Vec<Op>> {
    const NETFX: [&str; 8] = [
        "/Online",
        "/Enable-Feature",
        "/FeatureName:NetFx3",
        "/All",
        "/LimitAccess",
        "/Source:{source}",
        "/NoRestart",
        "/Quiet",
    ];
    const PNPUTIL: [&str; 4] = ["/add-driver", "{source}\\*.inf", "/subdirs", "/install"];
    const UPDATE: [&str; 5] = ["wuauserv", "UsoSvc", "BITS", "DoSvc", "WaaSMedicSvc"];
    const ROOTS: [&str; 2] = [
        "HKLM:\\SOFTWARE\\Microsoft",
        "HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft",
    ];
    const DELEGATE: &str = "Windows\\CurrentVersion\\Explorer\\Desktop\\NameSpace\\DelegateFolders";
    const DRIVES: &str = "{F5FB2C77-0E2F-4A16-A381-3E560C68BC83}";
    const INTERFACES: &str =
        "HKLM:\\SYSTEM\\CurrentControlSet\\Services\\NetBT\\Parameters\\Interfaces";
    Some(match rule {
        "install.netfx3" => vec![Op::RunFromMedia {
            folder: "sources\\sxs".to_owned(),
            pattern: "*.cab".to_owned(),
            deep: false,
            file: "dism.exe".to_owned(),
            args: strings(&NETFX),
        }],
        "install.drivers-from-media" => vec![Op::RunFromMedia {
            folder: "Drivers".to_owned(),
            pattern: "*.inf".to_owned(),
            deep: true,
            file: "pnputil.exe".to_owned(),
            args: strings(&PNPUTIL),
        }],
        "printing.spooler-automatic" => vec![Op::ServiceStart {
            name: "Spooler".to_owned(),
            start: 2,
        }],
        "update.unblock" => UPDATE
            .iter()
            .map(|name| Op::ServiceStartFrom {
                name: (*name).to_owned(),
                from: 4,
                start: 3,
            })
            .collect(),
        "network.netbios-off" => vec![Op::SetValueInSubkeys {
            path: INTERFACES.to_owned(),
            name: "NetbiosOptions".to_owned(),
            kind: RegKind::DWord,
            data: Value::from(2),
        }],
        "nav.removable-drives-once" => ROOTS
            .iter()
            .map(|root| Op::DeleteKey {
                path: format!("{root}\\{DELEGATE}\\{DRIVES}"),
            })
            .collect(),
        "post-oobe.password-never-expires" => vec![Op::PasswordNeverExpires {
            accounts: profile.accounts.iter().map(|a| a.name.clone()).collect(),
        }],
        "post-oobe.disable-builtin-accounts" => vec![Op::DisableBuiltinAccounts],
        "post-oobe.collect-specialize-errors" => vec![Op::CollectErrors {
            file: SETUP_ERRORS.to_owned(),
        }],
        "post-oobe.delete-answer-file-copies" => vec![Op::DeleteFiles {
            paths: strings(&ANSWER_FILE_COPIES),
        }],
        _ => return None,
    })
}

/// The operations that make one action of a rule true, or why it is left to the script.
fn apply_ops(
    action: &Action,
    params: &std::collections::BTreeMap<String, Value>,
) -> Result<Vec<Op>, String> {
    Ok(match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            ..
        } => vec![Op::SetValue {
            path: native_path(path)?,
            name: name.clone(),
            kind: *kind,
            data: render::fill_value(value, params),
        }],
        Action::RegRemove { path, name, .. } => vec![Op::DeleteValue {
            path: native_path(path)?,
            name: name.clone(),
        }],
        Action::RegList {
            path,
            param,
            names,
            prefix,
            additive,
            kind,
        } => {
            let items = render::list_values(params, param);
            let (names, values) = values::list_entries(*names, prefix, &items)?;
            vec![Op::SetList {
                path: native_path(path)?,
                kind: *kind,
                names,
                values,
                additive: *additive,
            }]
        }
        Action::Service { name, start, .. } => vec![Op::ServiceStart {
            name: name.clone(),
            start: *start,
        }],
        Action::Exe { file, args } => vec![Op::Run {
            file: file.clone(),
            args: args.iter().map(|a| render::substitute(a, params)).collect(),
        }],
        Action::Feature { name, state, .. } => vec![feature_run(name, state)],
        Action::Ps { .. } => return Err(SCRIPT_STEP.to_owned()),
        Action::Capability { pattern } => vec![Op::RemoveCapability {
            pattern: pattern.clone(),
        }],
        Action::Appx { names } => vec![Op::RemoveApps {
            names: names.clone(),
        }],
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
            path,
            name,
            kind,
            default,
            ..
        } => {
            let path = native_path(path)?;
            match default {
                Some(Value::String(s)) if s == "absent" => Some(vec![Op::DeleteValue {
                    path,
                    name: name.clone(),
                }]),
                Some(Value::String(s)) if s == "unknown" => None,
                Some(value) => Some(vec![Op::SetValue {
                    path,
                    name: name.clone(),
                    kind: *kind,
                    data: value.clone(),
                }]),
                None if path.to_ascii_lowercase().contains("\\policies\\") => {
                    Some(vec![Op::DeleteValue {
                        path,
                        name: name.clone(),
                    }])
                }
                None => None,
            }
        }
        Action::RegRemove { default, .. } => match default {
            None => Some(Vec::new()),
            Some(Value::String(s)) if s == "absent" => Some(Vec::new()),
            Some(_) => None,
        },
        Action::RegList {
            path,
            additive,
            kind,
            ..
        } => {
            let path = native_path(path)?;
            let policy = path.to_ascii_lowercase().contains("\\policies\\");
            (policy && !*additive).then(|| {
                vec![Op::SetList {
                    path,
                    kind: *kind,
                    names: Vec::new(),
                    values: Vec::new(),
                    additive: false,
                }]
            })
        }
        Action::Service { name, default, .. } => {
            default.as_ref().and_then(Value::as_u64).map(|start| {
                vec![Op::ServiceStart {
                    name: name.clone(),
                    start: start as u32,
                }]
            })
        }
        Action::Feature { name, default, .. } => {
            default.as_ref().map(|state| vec![feature_run(name, state)])
        }
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => Some(Vec::new()),
        Action::Ps { .. }
        | Action::Exe { .. }
        | Action::Appx { .. }
        | Action::Capability { .. } => None,
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
            Ok(Some(ops)) => plan.steps.extend(ops.into_iter().map(|op| Step {
                rule: rule.id.clone(),
                op,
            })),
            Ok(None) => plan.skipped.push((
                rule.id.clone(),
                "the Windows default of a step is not known".to_owned(),
            )),
            Err(why) => plan.skipped.push((rule.id.clone(), why)),
        }
    }
}

/// The plan that makes the selection match the profile: the rules on are applied (with the rules
/// they need), the rules off return to the Windows defaults.
pub fn plan_apply(catalog: &Catalog, profile: &Profile, rule_ids: &[String], nonce: &str) -> Plan {
    let resolution = resolver::resolve(catalog, profile);
    let selected: HashSet<String> = rule_ids.iter().cloned().collect();
    let on: HashSet<String> = selected
        .iter()
        .filter(|id| resolution.in_effect(id))
        .cloned()
        .collect();
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
            // The script step of a rule the program knows is done natively, once.
            let mut scripted = false;
            for action in &rule.actions {
                let ops = match action {
                    Action::Ps { .. } if scripted => continue,
                    Action::Ps { .. } => {
                        scripted = true;
                        script_ops(&rule.id, profile).ok_or_else(|| SCRIPT_STEP.to_owned())
                    }
                    _ => apply_ops(action, &params),
                };
                match ops {
                    Ok(ops) => plan.steps.extend(ops.into_iter().map(|op| Step {
                        rule: rule.id.clone(),
                        op,
                    })),
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
    let value: Value =
        jsonfile::parse(&text, &jsonfile::display_name(path)).map_err(|e| e.to_string())?;
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
                    plan.skipped.push((
                        format!("{}\\{}", e.path, e.name),
                        "a value of a kind the program does not restore".to_owned(),
                    ));
                    continue;
                }
            }
        } else {
            Op::DeleteValue {
                path: e.path.clone(),
                name: e.name.clone(),
            }
        };
        plan.steps.push(Step {
            rule: "undo".to_owned(),
            op,
        });
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
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/presets/office.wwprofile");
        Profile::load(&path).unwrap()
    }

    #[test]
    fn the_script_steps_of_the_catalog_go_natively() {
        let catalog = Catalog::embedded().unwrap();
        let profile = office();
        for rule in catalog.rules() {
            let scripted = rule.actions.iter().any(|a| matches!(a, Action::Ps { .. }));
            if scripted && thispc::exclusion(rule).is_none() {
                assert!(
                    script_ops(&rule.id, &profile).is_some(),
                    "{} has no native steps",
                    rule.id
                );
            }
        }
        // A plan of every rule leaves no script step it knows to the stand-alone script.
        let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
        let plan = plan_apply(&catalog, &profile, &ids, "n");
        let left = plan.skipped.iter().any(|(_, why)| why == SCRIPT_STEP);
        assert!(!left, "{:?}", plan.skipped);
        let names: Vec<String> = profile.accounts.iter().map(|a| a.name.clone()).collect();
        let ops = script_ops("post-oobe.password-never-expires", &profile).unwrap();
        assert_eq!(ops, vec![Op::PasswordNeverExpires { accounts: names }]);
        let update = script_ops("update.unblock", &profile).unwrap();
        assert_eq!(update.len(), 5);
        assert!(script_ops("no.such-rule", &profile).is_none());
        let files = Op::DeleteFiles {
            paths: strings(&ANSWER_FILE_COPIES),
        };
        let plan = Plan {
            steps: vec![Step {
                rule: "r".to_owned(),
                op: files,
            }],
            ..Plan::new("apply", "n")
        };
        assert_eq!((plan.removals(), plan.irreversible()), (0, 1));
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
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(&s.op, Op::ServiceStart { name, start: 2 } if name == "Spooler"))
        );
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(&s.op, Op::Run { file, .. } if file == "net.exe"))
        );
        // Quick Assist goes natively: its capability and its app.
        let id = "apps.remove-quick-assist";
        assert!(!plan.skipped.iter().any(|(rule, _)| rule == id));
        let capability = Op::RemoveCapability {
            pattern: "App.Support.QuickAssist*".to_owned(),
        };
        let app = Op::RemoveApps {
            names: vec!["MicrosoftCorporationII.QuickAssist".to_owned()],
        };
        for op in [&capability, &app] {
            assert!(plan.steps.iter().any(|s| s.rule == id && &s.op == op));
            assert!(!op.in_default_user());
        }
        assert!(plan.removals() >= 2);
        assert!(
            plan.uses_default_user(),
            "the default user profile is written natively"
        );
        assert!(plan.steps.iter().any(|s| matches!(&s.op,
            Op::SetValue { path, .. } if path.starts_with("HKU:\\UnattendDefault\\Software\\"))));
        assert_eq!(
            native_path("DU:\\Software\\X"),
            Ok("HKU:\\UnattendDefault\\Software\\X".to_owned())
        );
        assert!(native_path("HKU:\\.DEFAULT\\X").is_err());
        assert!(in_default_user("hku:\\unattenddefault\\x"));
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let bytes = plan.to_json().into_bytes();
        assert_eq!(Plan::from_bytes(&bytes, "plan").unwrap(), plan);
        assert!(Plan::from_bytes(b"{\"format\": \"other\"}", "plan").is_err());
        assert!(Plan::from_bytes(&[0xff, 0xfe], "plan").is_err());
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
        assert!(
            matches!(&plan.steps[0].op, Op::DeleteValue { name, .. } if name == "B"),
            "the newest change first"
        );
        assert!(
            matches!(&plan.steps[1].op, Op::SetValue { name, kind: RegKind::DWord, .. } if name == "A")
        );
    }
}
