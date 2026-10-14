//! Stand-alone scripts for a PC without WinWeaver: Apply, Revert (return to Windows defaults),
//! Audit and Undo, filled from the runtime templates carried over as data. With the answer file
//! they are the only PowerShell the program writes (principle P2); the program itself never runs
//! them.

use std::collections::HashSet;

use serde_json::Value;

use crate::catalog::{Action, Catalog, Phase, Rule};
use crate::profile::Profile;
use crate::render::{self, RenderError};
use crate::resolver;
use crate::thispc::{self, Probe};

const APPLY: &str = include_str!("../../../data/templates/Apply.runtime.ps1");
const AUDIT: &str = include_str!("../../../data/templates/Audit.runtime.ps1");
const UNDO: &str = include_str!("../../../data/templates/Undo.runtime.ps1");
const READ_SYSTEM: &str = include_str!("../../../data/templates/section-read-system.ps1");

/// The name of the undo script, which the apply scripts name in their help.
pub const UNDO_NAME: &str = "Undo-Apply.ps1";

/// What a script does with the selection.
#[derive(Debug, Clone, Default)]
pub struct ScriptPlan {
    /// Rules applied (on in the profile).
    pub applied: Vec<String>,
    /// Rules returned to the Windows defaults.
    pub reverted: Vec<String>,
    /// Rules added because a selected rule needs them (applied) or needs one returned (reverted).
    pub added: Vec<String>,
    /// Rules left out and why.
    pub excluded: Vec<(String, &'static str)>,
    /// Rules with actions that cannot return to a default automatically, and how many.
    pub partial: Vec<(String, usize)>,
}

impl ScriptPlan {
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.applied.is_empty() {
            parts.push(format!("{} rules applied", self.applied.len()));
        }
        if !self.reverted.is_empty() {
            parts.push(format!("{} returned to Windows defaults", self.reverted.len()));
        }
        if !self.added.is_empty() {
            parts.push(format!("{} added as dependencies", self.added.len()));
        }
        if !self.excluded.is_empty() {
            parts.push(format!("{} left out", self.excluded.len()));
        }
        if !self.partial.is_empty() {
            parts.push(format!(
                "{} with steps that cannot be returned automatically",
                self.partial.len()
            ));
        }
        if parts.is_empty() {
            "nothing to do".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

/// The first comment line of a script: the program, the profile and what the script does.
fn label(profile: &Profile, app_version: &str, what: &str) -> String {
    let name: String = profile
        .name
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '?'
            }
        })
        .collect();
    format!("# WinWeaver {app_version}, profile: {name}{what}")
}

/// The lines that return one action to the value of a clean Windows; `None` when that is not
/// known or not possible, an empty list when nothing needs to be done.
fn revert_action(rule: &Rule, action: &Action) -> Result<Option<Vec<String>>, RenderError> {
    let quote = |s: &str| render::ps_quote(s);
    Ok(match action {
        Action::Reg {
            path,
            name,
            kind,
            default,
            ..
        } => {
            let remove = format!(
                "Remove-Reg -Path {} -Name {}",
                render::reg_path(path),
                quote(name)
            );
            match default {
                Some(Value::String(s)) if s == "absent" => Some(vec![remove]),
                Some(Value::String(s)) if s == "unknown" => None,
                Some(value) => Some(vec![format!(
                    "Set-Reg -Path {} -Name {} -Type {kind:?} -Value {} -Why 'Windows default'",
                    render::reg_path(path),
                    quote(name),
                    render::reg_value(rule, *kind, value)?
                )]),
                None if path.to_ascii_lowercase().contains("\\policies\\") => Some(vec![remove]),
                None => None,
            }
        }
        Action::RegRemove { default, .. } => match default {
            None => Some(Vec::new()),
            Some(Value::String(s)) if s == "absent" => Some(Vec::new()),
            Some(_) => None,
        },
        Action::Service { name, default, .. } => {
            default.as_ref().and_then(Value::as_u64).map(|start| {
                vec![format!("Set-ServiceStart -Name {} -Start {start}", quote(name))]
            })
        }
        Action::Feature { name, default, .. } => default
            .as_ref()
            .map(|state| vec![format!("Set-Feature -Name {} -State {state}", quote(name))]),
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => Some(Vec::new()),
        Action::Ps { .. } | Action::Exe { .. } | Action::Capability { .. } | Action::Appx { .. } => {
            None
        }
    })
}

/// The lines that return a rule to the Windows defaults and how many actions could not be.
fn revert_lines(rule: &Rule) -> Result<(Vec<String>, usize), RenderError> {
    let mut lines = Vec::new();
    let mut skipped = 0;
    for action in &rule.actions {
        match revert_action(rule, action)? {
            Some(l) => lines.extend(l),
            None => skipped += 1,
        }
    }
    Ok((lines, skipped))
}

fn block(marker: &str, lines: &[String]) -> String {
    let mut text = format!("# [{marker}]\n");
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    text
}

/// The block that returns a rule to the Windows defaults.
fn revert_block(id: &str, lines: &[String]) -> String {
    let mut text = format!("# [{id}] Windows defaults\n");
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    text
}

/// Blocks of the specialize and post-OOBE phases and of the default user hive, assembled in that
/// order; the default user blocks run inside `Mount-DefaultUser`.
#[derive(Default)]
struct Blocks {
    system: Vec<String>,
    default_user: Vec<String>,
    post: Vec<String>,
}

impl Blocks {
    fn push(&mut self, phase: Phase, text: String) {
        match phase {
            Phase::DefaultUser => self.default_user.push(text),
            Phase::PostOobe => self.post.push(text),
            _ => self.system.push(text),
        }
    }

    fn assemble(&self, empty: &str) -> String {
        let mut parts = Vec::new();
        if !self.system.is_empty() {
            parts.push(render::join_blocks(&self.system, 0));
        }
        if !self.default_user.is_empty() {
            parts.push(format!(
                "if (Mount-DefaultUser) {{\n{}\n    Dismount-DefaultUser\n}}",
                render::join_blocks(&self.default_user, 4)
            ));
        }
        if !self.post.is_empty() {
            parts.push(render::join_blocks(&self.post, 0));
        }
        if parts.is_empty() {
            empty.to_owned()
        } else {
            parts.join("\n\n")
        }
    }
}

/// The rule ids of the selection with the rules they need, transitively.
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

/// The rule ids of the selection with the rules that need them, transitively.
fn with_dependents(catalog: &Catalog, ids: &HashSet<String>) -> HashSet<String> {
    let mut all = ids.clone();
    loop {
        let more: Vec<String> = catalog
            .rules()
            .iter()
            .filter(|r| !all.contains(&r.id) && r.requires.iter().any(|q| all.contains(q)))
            .map(|r| r.id.clone())
            .collect();
        if more.is_empty() {
            return all;
        }
        all.extend(more);
    }
}

fn accounts(profile: &Profile) -> String {
    let names: Vec<String> = if profile.install.account_mode == "file" {
        profile.accounts.iter().map(|a| a.name.clone()).collect()
    } else {
        Vec::new()
    };
    render::ps_list(&names)
}

/// The apply script of a selection: the rules on in the profile are applied (with the rules they
/// need), the rules off return to the Windows defaults.
pub fn apply_script(
    catalog: &Catalog,
    profile: &Profile,
    rule_ids: &[String],
    app_version: &str,
) -> Result<(String, ScriptPlan), RenderError> {
    let resolution = resolver::resolve(catalog, profile);
    let selected: HashSet<String> = rule_ids.iter().cloned().collect();
    let on: HashSet<String> = selected
        .iter()
        .filter(|id| resolution.in_effect(id))
        .cloned()
        .collect();
    let applied_set = with_requirements(catalog, &on);
    let mut plan = ScriptPlan::default();
    let mut blocks = Blocks::default();
    for rule in catalog.rules() {
        let is_applied = applied_set.contains(&rule.id);
        if !is_applied && !selected.contains(&rule.id) {
            continue;
        }
        if let Some(why) = thispc::exclusion(rule) {
            plan.excluded.push((rule.id.clone(), why));
            continue;
        }
        if is_applied {
            let params = render::params_of(catalog, profile, rule);
            let mut lines = Vec::new();
            for action in &rule.actions {
                lines.extend(render::script_lines(rule, action, &params, rule.phase)?);
            }
            if !selected.contains(&rule.id) {
                plan.added.push(rule.id.clone());
            }
            plan.applied.push(rule.id.clone());
            if !lines.is_empty() {
                blocks.push(rule.phase, block(&rule.id, &lines));
            }
        } else {
            let (lines, skipped) = revert_lines(rule)?;
            if skipped > 0 {
                plan.partial.push((rule.id.clone(), skipped));
            }
            if !lines.is_empty() {
                plan.reverted.push(rule.id.clone());
                blocks.push(rule.phase, revert_block(&rule.id, &lines));
            }
        }
    }
    let text = render::fill(
        APPLY,
        &[
            (
                "build_label",
                label(profile, app_version, " (rules that are off return to Windows defaults)").as_str(),
            ),
            ("accounts", accounts(profile).as_str()),
            ("blocks", blocks.assemble("# (nothing to apply)").as_str()),
        ],
    )?;
    Ok((text, plan))
}

/// The script that returns a selection, and the rules that need it, to the Windows defaults.
pub fn revert_script(
    catalog: &Catalog,
    profile: &Profile,
    rule_ids: &[String],
    app_version: &str,
) -> Result<(String, ScriptPlan), RenderError> {
    let selected: HashSet<String> = rule_ids.iter().cloned().collect();
    let all = with_dependents(catalog, &selected);
    let mut plan = ScriptPlan::default();
    let mut blocks = Blocks::default();
    for rule in catalog.rules() {
        if !all.contains(&rule.id) {
            continue;
        }
        if let Some(why) = thispc::exclusion(rule) {
            plan.excluded.push((rule.id.clone(), why));
            continue;
        }
        let (lines, skipped) = revert_lines(rule)?;
        if skipped > 0 {
            plan.partial.push((rule.id.clone(), skipped));
        }
        if lines.is_empty() {
            continue;
        }
        if !selected.contains(&rule.id) {
            plan.added.push(rule.id.clone());
        }
        plan.reverted.push(rule.id.clone());
        blocks.push(rule.phase, revert_block(&rule.id, &lines));
    }
    let text = render::fill(
        APPLY,
        &[
            (
                "build_label",
                label(profile, app_version, " (return to Windows defaults)").as_str(),
            ),
            ("accounts", accounts(profile).as_str()),
            ("blocks", blocks.assemble("# (nothing to return)").as_str()),
        ],
    )?;
    Ok((text, plan))
}

/// The `Test-*` line of one check of the audit script.
fn audit_line(rule: &Rule, check: &thispc::Check) -> Result<String, RenderError> {
    let q = |s: &str| render::ps_quote(s);
    let r = q(&check.rule);
    Ok(match &check.probe {
        Probe::Reg {
            path,
            name,
            kind,
            expected,
            ..
        } => format!(
            "Test-Reg -Rule {r} -Path {} -Name {} -Kind {kind:?} -Expected {} -Note {}",
            q(path),
            q(name),
            render::reg_value(rule, *kind, expected)?,
            q(&check.note)
        ),
        Probe::RegAbsent { path, name, .. } => format!(
            "Test-RegAbsent -Rule {r} -Path {} -Name {} -Note {}",
            q(path),
            q(name),
            q(&check.note)
        ),
        Probe::Service { name, start, .. } => {
            format!("Test-ServiceStart -Rule {r} -Name {} -Start {start}", q(name))
        }
        Probe::Feature { name, state, .. } => {
            format!("Test-Feature -Rule {r} -Name {} -State {state}", q(name))
        }
        Probe::CapabilityAbsent { pattern } => {
            format!("Test-CapabilityAbsent -Rule {r} -Pattern {}", q(pattern))
        }
        Probe::AppAbsent { name } => format!("Test-AppAbsent -Rule {r} -Name {}", q(name)),
        Probe::Unknown { what } => format!("Add-Unknown -Rule {r} -What {}", q(what)),
    })
}

/// The audit script of a selection: it only reads and writes a JSON report that WinWeaver opens
/// and turns into a profile; it reads the data of the forms too.
pub fn audit_script(
    catalog: &Catalog,
    profile: &Profile,
    rule_ids: &[String],
    app_version: &str,
) -> Result<(String, thispc::Plan), RenderError> {
    let plan = thispc::plan(catalog, profile, rule_ids);
    let mut blocks = Vec::new();
    for id in &plan.rules {
        let Some(rule) = catalog.rule(id) else {
            continue;
        };
        let mut lines = Vec::new();
        for check in plan.checks.iter().filter(|c| &c.rule == id) {
            lines.push(audit_line(rule, check)?);
        }
        if !lines.is_empty() {
            blocks.push(block(id, &lines));
        }
    }
    let blocks = if blocks.is_empty() {
        "# (nothing to check)".to_owned()
    } else {
        render::join_blocks(&blocks, 0)
    };
    let text = render::fill(
        AUDIT,
        &[
            ("build_label", label(profile, app_version, "").as_str()),
            ("blocks", blocks.as_str()),
            ("system_section", READ_SYSTEM.replace("\r\n", "\n").as_str()),
        ],
    )?;
    Ok((text, plan))
}

/// The undo script that restores the backup an apply or revert script saved next to itself.
pub fn undo_script(profile: &Profile, app_version: &str) -> Result<String, RenderError> {
    render::fill(UNDO, &[("build_label", label(profile, app_version, "").as_str())])
}

/// A script as a file: CRLF line endings and a UTF-8 byte order mark, which Windows PowerShell
/// 5.1 needs to read anything outside ASCII correctly.
pub fn file_bytes(text: &str) -> Vec<u8> {
    let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}
