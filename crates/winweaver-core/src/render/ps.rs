//! The PowerShell lines of the actions of a rule, as the embedded and the stand-alone
//! scripts write them.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::catalog::{Action, Phase, RegKind, Rule, values};

use super::params::{fill_value, list_values, substitute};
use super::text::{ps_list, ps_quote};
use super::{RenderError, err};

pub(crate) fn reg_path(path: &str) -> String {
    match path.strip_prefix("DU:") {
        Some(rest) => format!("($du + {})", ps_quote(rest)),
        None => ps_quote(path),
    }
}

pub(crate) fn reg_value(rule: &Rule, kind: RegKind, value: &Value) -> Result<String, RenderError> {
    let number = |v: &Value| -> Result<String, RenderError> {
        values::number(v)
            .map(|n| n.to_string())
            .ok_or_else(|| err(rule, format!("the value {v} is not a number")))
    };
    match kind {
        RegKind::DWord | RegKind::QWord => number(value),
        RegKind::String | RegKind::ExpandString => Ok(ps_quote(&values::text(value))),
        RegKind::MultiString => match value {
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().map(|v| ps_quote(&values::text(v))).collect();
                Ok(format!("@({})", parts.join(",")))
            }
            Value::String(s) if s.is_empty() => Ok("@()".to_owned()),
            Value::String(s) => Ok(format!("@({})", ps_quote(s))),
            other => Err(err(
                rule,
                format!("the value {other} is not a list of texts"),
            )),
        },
        RegKind::Binary => match value {
            Value::Array(items) => {
                let parts: Result<Vec<String>, RenderError> = items.iter().map(number).collect();
                Ok(format!("([byte[]]@({}))", parts?.join(",")))
            }
            other => Err(err(
                rule,
                format!("the value {other} is not a list of bytes"),
            )),
        },
    }
}

/// The script lines of one action.
pub(crate) fn script_lines(
    rule: &Rule,
    action: &Action,
    params: &BTreeMap<String, Value>,
    phase: Phase,
) -> Result<Vec<String>, RenderError> {
    let limited = matches!(phase, Phase::UserFirstLogon | Phase::PostOobe);
    let s = |text: &str| substitute(text, params);
    let line = match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            why,
            ..
        } => {
            let value = fill_value(value, params);
            let mut text = format!(
                "Set-Reg -Path {} -Name {} -Type {kind:?} -Value {}",
                reg_path(path),
                ps_quote(name),
                reg_value(rule, *kind, &value)?
            );
            if !why.is_empty() {
                text.push_str(&format!(" -Why {}", ps_quote(why)));
            }
            text
        }
        Action::RegRemove { path, name, .. } => {
            format!(
                "Remove-Reg -Path {} -Name {}",
                reg_path(path),
                ps_quote(name)
            )
        }
        // The lines of a script are code: written as they are, never filled with a value.
        Action::Ps { script } => {
            return Ok(script.lines().iter().map(|l| (*l).to_owned()).collect());
        }
        _ if limited => {
            return Err(err(
                rule,
                format!(
                    "the phase {phase:?} allows only reg, reg-remove and ps actions, not {}",
                    action.kind()
                ),
            ));
        }
        Action::RegList {
            path,
            param,
            names,
            prefix,
            additive,
            kind,
        } => {
            let items = list_values(params, param);
            let (n, v) = values::list_entries(*names, prefix, &items).map_err(|e| err(rule, e))?;
            reg_list_line(path, *kind, &n, &v, *additive)
        }
        Action::Service { name, start, .. } => {
            format!("Set-ServiceStart -Name {} -Start {start}", ps_quote(name))
        }
        Action::Exe { file, args } => {
            let args: Vec<String> = args.iter().map(|a| ps_quote(&s(a))).collect();
            format!("Invoke-Exe {} @({})", ps_quote(file), args.join(","))
        }
        Action::Feature { name, state, .. } => {
            format!("Set-Feature -Name {} -State {state}", ps_quote(name))
        }
        Action::Capability { pattern } => {
            format!("Remove-Capability -Pattern {}", ps_quote(pattern))
        }
        Action::Appx { names } => {
            let names: Vec<String> = names.iter().map(|n| ps_quote(n)).collect();
            format!("Remove-Apps @({})", names.join(","))
        }
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => return Ok(Vec::new()),
    };
    Ok(vec![line])
}

/// The `Set-RegList` line of a list of values.
pub(crate) fn reg_list_line(
    path: &str,
    kind: RegKind,
    names: &[String],
    values: &[String],
    additive: bool,
) -> String {
    format!(
        "Set-RegList -Path {} -Type {kind:?} -Names @({}) -Values @({}){}",
        reg_path(path),
        ps_list(names),
        ps_list(values),
        if additive { " -Additive" } else { "" }
    )
}

/// The block of a rule: the marker and the lines of its script actions.
pub(super) fn block(
    rule: &Rule,
    params: &BTreeMap<String, Value>,
) -> Result<Option<String>, RenderError> {
    let mut lines = Vec::new();
    for action in &rule.actions {
        lines.extend(script_lines(rule, action, params, rule.phase)?);
    }
    if lines.is_empty() {
        return Ok(None);
    }
    let mut text = format!("# [{}]\n", rule.id);
    for l in lines {
        text.push_str(&l);
        text.push('\n');
    }
    Ok(Some(text))
}

pub(crate) fn join_blocks(blocks: &[String], indent: usize) -> String {
    let pad = " ".repeat(indent);
    blocks
        .iter()
        .map(|b| {
            b.lines()
                .map(|l| {
                    if l.is_empty() {
                        String::new()
                    } else {
                        format!("{pad}{l}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
