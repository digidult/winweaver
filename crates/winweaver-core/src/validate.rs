//! The check of a profile (F7): errors, warnings and infos, each with the item it belongs to.
//!
//! An error stops the build; a warning names something the user should look at; an info is a
//! fact worth knowing. The checks follow `docs/spec/03-features.md` section 3.

use crate::catalog::{Catalog, Level, Param, check::same_value};
use crate::profile::Profile;
use crate::resolver::{Resolution, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// The item an issue belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Profile,
    Install,
    Accounts,
    Languages,
    Rule(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub target: Target,
    pub text: String,
}

/// The rule that hides the online account screen of OOBE.
pub const RULE_HIDE_ONLINE_ACCOUNT: &str = "oobe.hide-online-account";
/// The rule that lets OOBE finish without a network connection.
pub const RULE_BYPASS_NRO: &str = "install.bypass-nro";
/// The automatic sign-in rule; its parameter `account` must name an account without a password.
pub const RULE_AUTOLOGON: &str = "accounts.autologon";

const ACCOUNT_NAME_MAX: usize = 20;
const NAME_MAX: usize = 15;

/// Checks the profile; the issues are sorted by severity, errors first.
pub fn check(catalog: &Catalog, profile: &Profile, resolution: &Resolution) -> Vec<Issue> {
    let mut issues = Vec::new();
    check_install(profile, &mut issues);
    check_accounts(catalog, profile, resolution, &mut issues);
    check_languages(profile, &mut issues);
    check_rules(catalog, profile, resolution, &mut issues);
    if !profile.unknown.is_empty() {
        push(
            &mut issues,
            Severity::Info,
            Target::Profile,
            format!(
                "{} choices of rules this catalog does not have are kept in the profile and not built",
                profile.unknown.len()
            ),
        );
    }
    issues.sort_by_key(|i| i.severity);
    issues
}

/// One line with the counts: "Check: 1 error, 2 warnings" or "Check: no problems".
pub fn summary(issues: &[Issue]) -> String {
    let count = |s: Severity| issues.iter().filter(|i| i.severity == s).count();
    let (e, w, n) = (
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info),
    );
    if e + w + n == 0 {
        return "Check: no problems".to_owned();
    }
    let mut parts = Vec::new();
    for (k, one, many) in [(e, "error", "errors"), (w, "warning", "warnings"), (n, "info", "infos")]
    {
        if k == 1 {
            parts.push(format!("1 {one}"));
        } else if k > 1 {
            parts.push(format!("{k} {many}"));
        }
    }
    format!("Check: {}", parts.join(", "))
}

pub fn errors(issues: &[Issue]) -> usize {
    issues.iter().filter(|i| i.severity == Severity::Error).count()
}

fn push(issues: &mut Vec<Issue>, severity: Severity, target: Target, text: impl Into<String>) {
    issues.push(Issue {
        severity,
        target,
        text: text.into(),
    });
}

fn check_install(profile: &Profile, issues: &mut Vec<Issue>) {
    let i = &profile.install;
    let err = |issues: &mut Vec<Issue>, text: String| push(issues, Severity::Error, Target::Install, text);
    if i.edition.trim().is_empty() {
        err(issues, "The edition is empty".into());
    }
    match i.product_key_mode.as_str() {
        "generic" | "ask" => {}
        "key" => {
            if !is_product_key(&i.product_key) {
                err(
                    issues,
                    "The product key must be five groups of five letters or digits separated by hyphens"
                        .into(),
                );
            }
        }
        other => err(issues, format!("Unknown product key mode {other}")),
    }
    if i.time_zone.trim().is_empty() {
        err(issues, "The time zone is empty".into());
    }
    if !matches!(i.account_mode.as_str(), "file" | "ask") {
        err(issues, format!("Unknown account mode {}", i.account_mode));
    }
    match i.computer_name_mode.as_str() {
        "random" => {}
        "fixed" => {
            if !is_computer_name(&i.computer_name) {
                err(
                    issues,
                    "A computer name is 1 to 15 Latin letters, digits and hyphens, not only digits"
                        .into(),
                );
            }
        }
        "template" => {
            if !is_name_template(&i.computer_name) {
                err(
                    issues,
                    "A computer name template holds Latin letters, digits, hyphens and the placeholders {serial}, {mac} and {random}, up to 15 characters when expanded"
                        .into(),
                );
            }
        }
        other => err(issues, format!("Unknown computer name mode {other}")),
    }
    if !i.workgroup.is_empty() && !is_workgroup(&i.workgroup) {
        err(
            issues,
            "A workgroup name is 1 to 15 Latin letters, digits and hyphens".into(),
        );
    }
    match i.disk_mode.as_str() {
        "ask" => {}
        "partition" => {
            if !i.disk_id.parse::<u32>().is_ok_and(|d| d <= 99) {
                err(issues, "The disk number must be 0 to 99".into());
            }
            if !i
                .partition_id
                .parse::<u32>()
                .is_ok_and(|p| (1..=128).contains(&p))
            {
                err(issues, "The partition number must be 1 to 128".into());
            }
            push(
                issues,
                Severity::Warning,
                Target::Install,
                "In the disk mode partition nothing asks during Setup: the chosen partition is installed over",
            );
        }
        other => err(issues, format!("Unknown disk mode {other}")),
    }
}

fn check_accounts(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
    issues: &mut Vec<Issue>,
) {
    let mode = profile.install.account_mode.as_str();
    if mode == "file" {
        if profile.accounts.is_empty() {
            push(
                issues,
                Severity::Error,
                Target::Accounts,
                "No account: Windows Setup needs at least one, or choose the mode ask",
            );
        }
        let mut seen: Vec<String> = Vec::new();
        for a in &profile.accounts {
            let name = a.name.trim();
            if name.is_empty() {
                push(issues, Severity::Error, Target::Accounts, "An account has no name");
                continue;
            }
            if !name.is_ascii() || !a.password.is_ascii() {
                push(
                    issues,
                    Severity::Error,
                    Target::Accounts,
                    format!(
                        "Account {name}: the name and the password must be ASCII, Windows Setup turns other characters into question marks"
                    ),
                );
            }
            if name.chars().count() > ACCOUNT_NAME_MAX {
                push(
                    issues,
                    Severity::Error,
                    Target::Accounts,
                    format!("Account {name}: the name is longer than {ACCOUNT_NAME_MAX} characters"),
                );
            }
            if !a.display_name.is_ascii() {
                push(
                    issues,
                    Severity::Info,
                    Target::Accounts,
                    format!("Account {name}: the display name outside ASCII is set after OOBE"),
                );
            }
            if !matches!(a.group.as_str(), "Administrators" | "Users") {
                push(
                    issues,
                    Severity::Error,
                    Target::Accounts,
                    format!("Account {name}: the group must be Administrators or Users"),
                );
            }
            let lower = name.to_lowercase();
            if seen.contains(&lower) {
                push(
                    issues,
                    Severity::Error,
                    Target::Accounts,
                    format!("Account {name} is listed twice"),
                );
            }
            seen.push(lower);
        }
    } else if mode == "ask" {
        for id in [RULE_HIDE_ONLINE_ACCOUNT, RULE_BYPASS_NRO] {
            if catalog.rule(id).is_some() && !resolution.in_effect(id) {
                push(
                    issues,
                    Severity::Warning,
                    Target::Accounts,
                    format!(
                        "In the account mode ask the rule {id} should be on, so that Setup asks for a local account"
                    ),
                );
            }
        }
    }
    if resolution.in_effect(RULE_AUTOLOGON) {
        let account = profile
            .param(catalog, RULE_AUTOLOGON, "account")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let target = Target::Rule(RULE_AUTOLOGON.to_owned());
        match profile
            .accounts
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(&account))
        {
            Some(a) if !a.password.is_empty() => push(
                issues,
                Severity::Error,
                target,
                format!("The automatic sign-in needs an account without a password; {account} has one"),
            ),
            Some(_) => {
                if mode == "ask" {
                    push(
                        issues,
                        Severity::Warning,
                        target,
                        "The automatic sign-in names an account of the profile, but in the mode ask Windows Setup creates the account",
                    );
                }
            }
            None => push(
                issues,
                Severity::Error,
                target,
                format!("The automatic sign-in names the account {account}, which the profile does not list"),
            ),
        }
    }
}

fn check_languages(profile: &Profile, issues: &mut Vec<Issue>) {
    let l = &profile.languages;
    for (what, value) in [
        ("display language", &l.ui_language),
        ("system locale", &l.system_locale),
        ("user locale", &l.user_locale),
    ] {
        if !is_culture(value) {
            push(
                issues,
                Severity::Error,
                Target::Languages,
                format!("The {what} {value} is not a language tag such as uk-UA"),
            );
        }
    }
    if l.input.is_empty() {
        push(
            issues,
            Severity::Error,
            Target::Languages,
            "No input language: at least one is needed",
        );
    }
    for item in &l.input {
        if !is_culture(item) && !is_layout(item) {
            push(
                issues,
                Severity::Error,
                Target::Languages,
                format!("The input language {item} is not a language tag or a layout id such as 0409:00000409"),
            );
        }
    }
}

fn check_rules(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
    issues: &mut Vec<Issue>,
) {
    for rule in catalog.rules() {
        let target = || Target::Rule(rule.id.clone());
        match resolution.state(&rule.id) {
            State::Blocked { by } => push(
                issues,
                Severity::Warning,
                target(),
                format!("Chosen on but blocked: it needs {by}, which is off"),
            ),
            State::Off => {
                if rule.level == Level::Baseline {
                    push(
                        issues,
                        Severity::Warning,
                        target(),
                        "A baseline rule of the protection is off",
                    );
                }
                continue;
            }
            State::On => {}
        }
        if !resolution.in_effect(&rule.id) {
            continue;
        }
        for (name, param) in &rule.params {
            let Some(value) = profile.param(catalog, &rule.id, name) else {
                continue;
            };
            match param {
                Param::Enum {
                    values,
                    differs_from,
                    same_allowed,
                    ..
                } => {
                    if !values.iter().any(|v| same_value(&v.value, &value)) {
                        push(
                            issues,
                            Severity::Error,
                            target(),
                            format!("Parameter {name}: {value} is not one of its options"),
                        );
                    }
                    if let Some(other) = differs_from
                        && let Some(other_value) = profile.param(catalog, &rule.id, other)
                        && same_value(&other_value, &value)
                        && !same_allowed.iter().any(|v| same_value(v, &value))
                    {
                        push(
                            issues,
                            Severity::Error,
                            target(),
                            format!("Parameters {name} and {other} must differ"),
                        );
                    }
                }
                Param::Int { min, max, .. } => {
                    if !value.as_i64().is_some_and(|v| (*min..=*max).contains(&v)) {
                        push(
                            issues,
                            Severity::Error,
                            target(),
                            format!("Parameter {name}: {value} is outside {min}..{max}"),
                        );
                    }
                }
                Param::Text { required, .. } => {
                    let text = value.as_str().unwrap_or_default();
                    if *required && text.trim().is_empty() {
                        push(
                            issues,
                            Severity::Error,
                            target(),
                            format!("Parameter {name} is empty"),
                        );
                    }
                    if !text.is_ascii() {
                        push(
                            issues,
                            Severity::Error,
                            target(),
                            format!(
                                "Parameter {name}: the text must be ASCII, the embedded scripts are read without a code page"
                            ),
                        );
                    }
                }
            }
        }
    }
    for (a, b) in &resolution.conflicts {
        push(
            issues,
            Severity::Error,
            Target::Rule(a.clone()),
            format!("Conflicts with {b}: turn one of them off"),
        );
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// A workgroup name: 1 to 15 Latin letters, digits and hyphens.
pub fn is_workgroup(name: &str) -> bool {
    let n = name.chars().count();
    (1..=NAME_MAX).contains(&n) && name.chars().all(is_name_char)
}

/// A computer name: like a workgroup name, and not digits only.
pub fn is_computer_name(name: &str) -> bool {
    is_workgroup(name) && !name.bytes().all(|b| b.is_ascii_digit())
}

/// A computer name template: name characters and the placeholders `{serial}`, `{mac}`,
/// `{random}` (with an optional length such as `{random:6}`); the expanded name fits 15.
pub fn is_name_template(template: &str) -> bool {
    let mut rest = template;
    let mut literal = 0usize;
    let mut expanded = 0usize;
    let mut placeholders = 0usize;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('{') {
            let Some(end) = after.find('}') else {
                return false;
            };
            let (name, length) = match after[..end].split_once(':') {
                Some((n, l)) => (n, l.parse::<usize>().ok()),
                None => (&after[..end], None),
            };
            let default_len = match name {
                "serial" => 15,
                "mac" => 12,
                "random" => 6,
                _ => return false,
            };
            let len = length.unwrap_or(default_len);
            if len == 0 || len > NAME_MAX {
                return false;
            }
            expanded += len;
            placeholders += 1;
            rest = &after[end + 1..];
        } else {
            let c = rest.chars().next().unwrap_or('?');
            if !is_name_char(c) {
                return false;
            }
            literal += 1;
            rest = &rest[c.len_utf8()..];
        }
    }
    (placeholders > 0 || literal > 0) && literal + expanded <= NAME_MAX
}

/// A product key: five groups of five letters or digits separated by hyphens.
pub fn is_product_key(key: &str) -> bool {
    let groups: Vec<&str> = key.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .all(|g| g.len() == 5 && g.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// A language tag such as `uk-UA` or `ru-UA`: a language of 2 or 3 letters, up to three
/// subtags of 2 to 8 letters or digits.
pub fn is_culture(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let Some(language) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_lowercase()) {
        return false;
    }
    let mut count = 0;
    for part in parts {
        count += 1;
        if count > 3
            || !(2..=8).contains(&part.len())
            || !part.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return false;
        }
    }
    true
}

/// An input layout id such as `0409:00000409`.
pub fn is_layout(id: &str) -> bool {
    let Some((a, b)) = id.split_once(':') else {
        return false;
    };
    a.len() == 4
        && b.len() == 8
        && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(is_workgroup("WORKGROUP") && is_workgroup("a-1"));
        assert!(!is_workgroup("") && !is_workgroup("sixteen-chars-xx") && !is_workgroup("a b"));
        assert!(is_computer_name("PC-01") && !is_computer_name("12345"));
        assert!(is_name_template("PC-{random}") && is_name_template("{serial}"));
        assert!(is_name_template("W-{random:8}") && !is_name_template("W-{random:14}"));
        assert!(!is_name_template("PC-{user}") && !is_name_template("PC-{random") && !is_name_template(""));
        assert!(is_product_key("VK7JG-NPHTM-C97JM-9MPGT-3V66T") && !is_product_key("VK7JG-NPHTM"));
        assert!(is_culture("uk-UA") && is_culture("en") && is_culture("sr-Latn-RS"));
        assert!(!is_culture("UK-ua") && !is_culture("u") && !is_culture("uk-"));
        assert!(is_layout("0409:00000409") && !is_layout("0409:0409") && !is_layout("en-US"));
    }

    #[test]
    fn summary_counts() {
        assert_eq!(summary(&[]), "Check: no problems");
        let issues = vec![
            Issue {
                severity: Severity::Warning,
                target: Target::Profile,
                text: String::new(),
            },
            Issue {
                severity: Severity::Error,
                target: Target::Profile,
                text: String::new(),
            },
            Issue {
                severity: Severity::Warning,
                target: Target::Profile,
                text: String::new(),
            },
        ];
        assert_eq!(summary(&issues), "Check: 1 error, 2 warnings");
    }
}
