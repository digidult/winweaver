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

/// The problems of some rules of a catalog: the checks of `check` for each of them. A catalog
/// file checks its own rules so, in a catalog of the roots and its groups.
pub fn check_rules<'a>(
    catalog: &Catalog,
    rules: impl IntoIterator<Item = &'a Rule>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for rule in rules {
        check_rule(catalog, rule, &mut problems);
    }
    problems
}

fn check_rule(catalog: &Catalog, rule: &Rule, problems: &mut Vec<String>) {
    let id = &rule.id;
    if !is_safe_id(id) {
        problems.push(format!(
            "rule {id}: the id has characters outside a-z, 0-9, dot, hyphen, underscore"
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
        for (field, text) in action.code_fields() {
            for name in placeholders(&text) {
                if rule.params.contains_key(name) {
                    problems.push(format!(
                        "rule {id}: {}.{field} holds the parameter {{{name}}}: it is code, and a value of a profile would be run as code",
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
        Some(root) if mechanism.roots().contains(&root) => {}
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
            pick,
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
            // A region is chosen into two parameters at once, so a rule has both picks.
            if let Some(pick) = pick
                && rule.param_picked(pick.partner()).is_none()
            {
                problems.push(format!(
                    "rule {id}: parameter {name} picks {pick:?}, but no parameter of the rule picks {:?}",
                    pick.partner()
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

/// Identifiers of rules and parameters: ASCII lower case letters, digits, dots, hyphens and
/// underscores.
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

    /// A line of a script and a command of the answer file are code: a `{name}` of a parameter
    /// of the rule in one is a problem of the catalog (the value of a profile would be run),
    /// while the braces of PowerShell and a parameter in a value that is quoted are none.
    #[test]
    fn a_parameter_in_code_is_refused() {
        let catalog = Catalog::embedded().unwrap();
        let rule = catalog
            .rules()
            .iter()
            .find(|r| !r.params.is_empty())
            .expect("a rule with a parameter");
        let name = rule.params.keys().next().unwrap().clone();
        // The problems of code alone: an action more may also move the rule to another root.
        let problems_of = |action: Action| -> Vec<String> {
            let mut changed = rule.clone();
            changed.actions.push(action);
            let mut problems = Vec::new();
            check_rule(&catalog, &changed, &mut problems);
            problems.retain(|p| p.contains("holds the parameter"));
            problems
        };
        let script = |text: String| Action::Ps {
            script: crate::catalog::Lines::Many(vec!["$x = 1".to_owned(), text]),
        };
        let sound = "if ($x) {return} else { '{0}' -f $x; & {exit} }".to_owned();
        assert!(problems_of(script(sound)).is_empty());
        let found = problems_of(script(format!("Write-Host {{{name}}}")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("ps.script holds the parameter"),
            "{found:?}"
        );
        let pe = problems_of(Action::XmlPeCommand {
            command: format!("cmd.exe /c echo {{{name}}}"),
            description: String::new(),
        });
        assert_eq!(pe.len(), 1, "{pe:?}");
        assert!(pe[0].contains("xml-pe-command.command holds"), "{pe:?}");
        let specialize = problems_of(Action::XmlSpecializeCommand {
            command: format!("cmd.exe /c echo {{{name}}}"),
            description: String::new(),
        });
        assert_eq!(specialize.len(), 1, "{specialize:?}");
        // The whole embedded catalog has none, and its scripts are written as they are.
        for rule in catalog.rules() {
            for action in &rule.actions {
                for (field, text) in action.code_fields() {
                    let named: Vec<&str> = placeholders(&text)
                        .into_iter()
                        .filter(|n| rule.params.contains_key(*n))
                        .collect();
                    assert!(named.is_empty(), "{}: {field} {named:?}", rule.id);
                }
            }
        }
    }
}
