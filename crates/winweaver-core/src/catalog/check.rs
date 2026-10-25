//! Integrity checks of a catalog beyond what loading guarantees: placeholders, parameter
//! defaults, the root of every rule by its mechanism. The tests require an empty list.

use serde_json::Value;

use super::{Action, Catalog, Param, RegKind, Rule};

/// The problems found in the catalog, one line each; empty when the catalog is sound.
pub fn check(catalog: &Catalog) -> Vec<String> {
    let mut problems = Vec::new();
    for rule in catalog.rules() {
        check_rule(catalog, rule, &mut problems);
    }
    for group in catalog.groups() {
        if catalog.root_of(&group.id).is_none() {
            problems.push(format!("group {}: not under a root", group.id));
        }
    }
    problems
}

fn check_rule(catalog: &Catalog, rule: &Rule, problems: &mut Vec<String>) {
    let id = &rule.id;
    if !is_safe_id(id) {
        problems.push(format!(
            "rule {id}: the id has characters outside a-z, 0-9, dot, hyphen"
        ));
    }
    if rule.actions.is_empty() {
        problems.push(format!("rule {id}: no actions"));
    }
    for (name, param) in &rule.params {
        check_param(id, name, param, rule, problems);
    }
    for action in &rule.actions {
        for (field, text) in action.placeholder_fields() {
            for name in placeholders(&text) {
                if !rule.params.contains_key(name) {
                    problems.push(format!(
                        "rule {id}: {}.{field} uses the unknown parameter {{{name}}}",
                        action.kind()
                    ));
                }
            }
        }
        for (field, text) in action.literal_fields() {
            if !placeholders(text).is_empty() {
                problems.push(format!(
                    "rule {id}: {}.{field} is literal and may not hold a placeholder",
                    action.kind()
                ));
            }
        }
        if let Action::RegList { param, kind, .. } = action {
            if !param.is_empty() && !matches!(rule.params.get(param), Some(Param::List { .. })) {
                problems.push(format!(
                    "rule {id}: reg-list takes {param}, which is not a list parameter of the rule"
                ));
            }
            if !matches!(kind, RegKind::String | RegKind::ExpandString) {
                problems.push(format!(
                    "rule {id}: reg-list writes {kind:?}, not String or ExpandString"
                ));
            }
        }
        if let Action::XmlSetting { pass, .. } = action
            && !matches!(pass.as_str(), "windowsPE" | "specialize" | "oobeSystem")
        {
            problems.push(format!("rule {id}: xml-setting pass {pass} is unknown"));
        }
    }
    let mechanism = Catalog::mechanism(rule);
    match catalog.root_of(&rule.group) {
        Some(root) if root == mechanism.root() => {}
        Some(root) => problems.push(format!(
            "rule {id}: mechanism {mechanism:?} but the group {} is under the root {root}",
            rule.group
        )),
        None => problems.push(format!("rule {id}: the group {} has no root", rule.group)),
    }
}

fn check_param(id: &str, name: &str, param: &Param, rule: &Rule, problems: &mut Vec<String>) {
    if !is_safe_id(name) {
        problems.push(format!("rule {id}: parameter {name} has an unsafe name"));
    }
    match param {
        Param::Enum {
            default,
            values,
            differs_from,
            ..
        } => {
            if values.is_empty() {
                problems.push(format!("rule {id}: parameter {name} has no values"));
            }
            if !values.iter().any(|v| &v.value == default) {
                problems.push(format!(
                    "rule {id}: parameter {name}: the default {default} is not among the values"
                ));
            }
            if let Some(other) = differs_from {
                match rule.params.get(other) {
                    Some(Param::Enum { .. }) => {}
                    _ => problems.push(format!(
                        "rule {id}: parameter {name} differs_from {other}, which is not an enum of the rule"
                    )),
                }
            }
        }
        Param::Int {
            default, min, max, ..
        } => {
            if min > max || default < min || default > max {
                problems.push(format!(
                    "rule {id}: parameter {name}: default {default} outside {min}..{max}"
                ));
            }
        }
        Param::Text {
            default,
            required,
            max_length,
            ..
        } => {
            if *required && default.is_empty() {
                problems.push(format!(
                    "rule {id}: parameter {name} is required but its default is empty"
                ));
            }
            if let Some(max) = max_length
                && (*max == 0 || default.chars().count() > *max)
            {
                problems.push(format!(
                    "rule {id}: parameter {name}: the default is longer than {max} characters"
                ));
            }
        }
        Param::List { .. } => {}
    }
    let used = rule.actions.iter().any(|a| match a {
        Action::RegList { param, .. } => param == name,
        other => other
            .placeholder_fields()
            .iter()
            .any(|(_, text)| placeholders(text).contains(&name)),
    });
    if !used {
        problems.push(format!(
            "rule {id}: parameter {name} is not used by any action"
        ));
    }
}

/// The parameter names inside `{...}` of a text, in order of appearance. A name starts with a
/// lower case letter, so a GUID such as `{B4BFCC3A-...}` or a format index `{1}` is literal text.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                if name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                    && is_safe_id(name)
                    && !is_guid(name)
                {
                    names.push(name);
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    names
}

/// A GUID in the form 8-4-4-4-12 of hex digits, as registry keys of shell folders hold them.
pub fn is_guid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// Identifiers of rules and parameters: ASCII lower case letters, digits, dots and hyphens.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'_'
        })
}

/// Whether a JSON value of an enum parameter equals another (numbers compare by value).
pub fn same_value(a: &Value, b: &Value) -> bool {
    match (a.as_i64(), b.as_i64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_found() {
        assert_eq!(
            placeholders("/lockoutthreshold:{threshold} x {y}"),
            vec!["threshold", "y"]
        );
        assert!(placeholders("no braces").is_empty());
        assert!(placeholders("{ not an id }").is_empty());
        assert!(placeholders("{d3162b92-9365-467a-956b-92703aca08af} {1} {B4BF}").is_empty());
        assert!(is_guid("d3162b92-9365-467a-956b-92703aca08af"));
    }
}
