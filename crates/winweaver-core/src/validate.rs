//! The check of a profile (F7): errors, warnings and infos, each with the item it belongs to.
//!
//! An error stops the build; a warning names something the user should look at; an info is a
//! fact worth knowing. The checks follow `docs/spec/03-features.md` section 3.
//!
//! Every message is made with `tr` or `trf` of [`crate::message`]: its English text is the key
//! of its translation and its values are kept apart. [`check`] gives the messages in English
//! (the command line, the MCP tools, the tests); the window gives [`check_shown`] its
//! translation, and the test of its translations reads this file, so a new message needs its
//! translations as a text of the window does.

use serde_json::Value;

use crate::catalog::{Action, Catalog, Level, Param, Pick, Rule, check::same_value};
use crate::message::{Message, tr, trf};
use crate::profile::{Account, Profile};
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
    /// The options of catalogs that are not loaded (`Profile::unknown`).
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub target: Target,
    pub text: String,
}

/// What a check found, before its message is put into a language.
type Found = (Severity, Target, Message);

/// The rule that hides the online account screen of OOBE.
pub const RULE_HIDE_ONLINE_ACCOUNT: &str = "oobe.hide-online-account";
/// The rule that lets OOBE finish without a network connection.
pub const RULE_BYPASS_NRO: &str = "install.bypass-nro";
/// The automatic sign-in rule; its parameter `account` must name an account without a password.
pub const RULE_AUTOLOGON: &str = "accounts.autologon";

const ACCOUNT_NAME_MAX: usize = 20;
const NAME_MAX: usize = 15;

/// Checks the profile; the issues are sorted by severity, errors first. The messages are English.
pub fn check(catalog: &Catalog, profile: &Profile, resolution: &Resolution) -> Vec<Issue> {
    check_shown(catalog, profile, resolution, &Message::english)
}

/// [`check`] with every message put into words by `show`: the window translates the key of the
/// message and fills its values (`Message::shown`).
pub fn check_shown(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
    show: &dyn Fn(&Message) -> String,
) -> Vec<Issue> {
    let mut found = Vec::new();
    check_install(profile, &mut found);
    check_accounts(catalog, profile, resolution, &mut found);
    check_languages(profile, &mut found);
    check_rules(catalog, profile, resolution, &mut found);
    found.extend(control_found(catalog, profile, resolution));
    if !profile.unknown.is_empty() {
        let n = profile.unknown.len();
        found.push((
            Severity::Warning,
            Target::Unknown,
            trf(
                "{n} options of catalogs that are not loaded are kept in the profile and left out of the build",
                &[("n", &n)],
            ),
        ));
    }
    found.sort_by_key(|(severity, _, _)| *severity);
    issues_of(found, show)
}

fn issues_of(found: Vec<Found>, show: &dyn Fn(&Message) -> String) -> Vec<Issue> {
    found
        .into_iter()
        .map(|(severity, target, message)| Issue {
            severity,
            target,
            text: show(&message),
        })
        .collect()
}

/// One line with the counts: "Check: 1 error, 2 warnings" or "Check: no problems".
pub fn summary(issues: &[Issue]) -> String {
    let (e, w, n) = counts(issues);
    if e + w + n == 0 {
        return "Check: no problems".to_owned();
    }
    let mut parts = Vec::new();
    for (k, one, many) in [
        (e, "error", "errors"),
        (w, "warning", "warnings"),
        (n, "info", "infos"),
    ] {
        if k == 1 {
            parts.push(format!("1 {one}"));
        } else if k > 1 {
            parts.push(format!("{k} {many}"));
        }
    }
    format!("Check: {}", parts.join(", "))
}

/// The numbers of the errors, the warnings and the infos of a check.
pub fn counts(issues: &[Issue]) -> (usize, usize, usize) {
    let count = |s: Severity| issues.iter().filter(|i| i.severity == s).count();
    (
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info),
    )
}

pub fn errors(issues: &[Issue]) -> usize {
    issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count()
}

/// Whether a text holds a control character the answer file cannot take: every one of them, but
/// the tab where `tab` lets a free text keep it. XML 1.0 has no control character but the tab
/// and the line breaks, Windows Setup refuses a file that is not XML, and a line break inside a
/// value ends a line of an embedded script.
fn has_control(text: &str, tab: bool) -> bool {
    text.chars().any(|c| c.is_control() && !(tab && c == '\t'))
}

/// A name for a message: its control characters shown as `?`, so the message stays one line.
fn shown(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// The texts of a profile that reach the answer file with a control character, as errors in
/// English: the texts of the forms that are written in the modes chosen, the accounts and the
/// text parameters of the rules in effect. The check reports them and the build refuses them
/// (`render::build`), so none is ever written into the XML.
pub fn control_problems(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
) -> Vec<Issue> {
    let found = control_found(catalog, profile, resolution);
    issues_of(found, &Message::english)
}

fn control_found(catalog: &Catalog, profile: &Profile, resolution: &Resolution) -> Vec<Found> {
    let mut found = Vec::new();
    let i = &profile.install;
    let mut install = vec![&i.time_zone];
    if i.product_key_mode == "key" {
        install.push(&i.product_key);
    }
    if matches!(i.computer_name_mode.as_str(), "fixed" | "template") {
        install.push(&i.computer_name);
    }
    install.push(&i.workgroup);
    if i.disk_mode == "partition" {
        install.extend([&i.disk_id, &i.partition_id]);
    }
    if install.iter().any(|t| has_control(t, false)) {
        found.push((
            Severity::Error,
            Target::Install,
            tr("A text of the installation holds a control character, a line break for one"),
        ));
    }
    let l = &profile.languages;
    let languages = [&l.ui_language, &l.system_locale, &l.user_locale];
    if languages
        .into_iter()
        .chain(&l.input)
        .any(|t| has_control(t, false))
    {
        found.push((
            Severity::Error,
            Target::Languages,
            tr("A language holds a control character, a line break for one"),
        ));
    }
    if i.account_mode == "file" {
        for a in &profile.accounts {
            let strict = [&a.name, &a.group];
            let free = [&a.password, &a.display_name, &a.description];
            if strict.iter().any(|t| has_control(t, false))
                || free.iter().any(|t| has_control(t, true))
            {
                let name = shown(a.name.trim());
                found.push((
                    Severity::Error,
                    Target::Accounts,
                    trf(
                        "Account {name}: a text holds a control character, a line break for one",
                        &[("name", &name)],
                    ),
                ));
            }
        }
    }
    for rule in catalog.rules() {
        if !resolution.in_effect(&rule.id) {
            continue;
        }
        for name in rule.params.keys() {
            // A text may keep a tab; an item of a list is a line of its own and may not.
            let bad = match profile.param(catalog, &rule.id, name) {
                Some(Value::String(text)) => has_control(&text, true),
                Some(Value::Array(items)) => items
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|t| has_control(t.trim(), false)),
                _ => false,
            };
            if bad {
                found.push((
                    Severity::Error,
                    Target::Rule(rule.id.clone()),
                    trf(
                        "Parameter {name}: the text holds a control character, a line break for one",
                        &[("name", name)],
                    ),
                ));
            }
        }
    }
    found
}

fn check_install(profile: &Profile, found: &mut Vec<Found>) {
    let i = &profile.install;
    let mut err = |message: Message| found.push((Severity::Error, Target::Install, message));
    if i.edition.trim().is_empty() {
        err(tr("The edition is empty"));
    }
    match i.product_key_mode.as_str() {
        "generic" | "ask" => {}
        "key" => {
            if !is_product_key(&i.product_key) {
                err(tr(
                    "The product key must be five groups of five letters or digits separated by hyphens",
                ));
            }
        }
        other => err(trf("Unknown product key mode {mode}", &[("mode", &other)])),
    }
    if i.time_zone.trim().is_empty() {
        err(tr("The time zone is empty"));
    }
    if !matches!(i.account_mode.as_str(), "file" | "ask") {
        err(trf(
            "Unknown account mode {mode}",
            &[("mode", &i.account_mode)],
        ));
    }
    match i.computer_name_mode.as_str() {
        "random" => {}
        "fixed" => {
            if !is_computer_name(&i.computer_name) {
                err(tr(
                    "A computer name is 1 to 15 Latin letters, digits and hyphens, not only digits",
                ));
            }
        }
        "template" => {
            if !is_name_template(&i.computer_name) {
                err(tr(
                    "A computer name template holds Latin letters, digits, hyphens and the placeholders {serial}, {mac} and {random}, each with an optional length from 1 to 15 such as {serial:6}; the name with the full length of each placeholder fits 15 characters",
                ));
            }
        }
        other => err(trf(
            "Unknown computer name mode {mode}",
            &[("mode", &other)],
        )),
    }
    if !i.workgroup.is_empty() && !is_workgroup(&i.workgroup) {
        err(tr(
            "A workgroup name is 1 to 15 Latin letters, digits and hyphens",
        ));
    }
    match i.disk_mode.as_str() {
        "ask" => {}
        "partition" => {
            if !i.disk_id.parse::<u32>().is_ok_and(|d| d <= 99) {
                err(tr("The disk number must be 0 to 99"));
            }
            if !i
                .partition_id
                .parse::<u32>()
                .is_ok_and(|p| (1..=128).contains(&p))
            {
                err(tr("The partition number must be 1 to 128"));
            }
            found.push((
                Severity::Warning,
                Target::Install,
                tr(
                    "In the disk mode partition nothing asks during Setup: the chosen partition is installed over",
                ),
            ));
        }
        other => err(trf("Unknown disk mode {mode}", &[("mode", &other)])),
    }
}

fn check_accounts(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
    found: &mut Vec<Found>,
) {
    let mode = profile.install.account_mode.as_str();
    if mode == "file" {
        if profile.accounts.is_empty() {
            found.push((
                Severity::Error,
                Target::Accounts,
                tr("No account: Windows Setup needs at least one, or choose the mode ask"),
            ));
        }
        let mut seen: Vec<String> = Vec::new();
        for a in &profile.accounts {
            check_account(a, &mut seen, found);
        }
    } else if mode == "ask" {
        for id in [RULE_HIDE_ONLINE_ACCOUNT, RULE_BYPASS_NRO] {
            if catalog.rule(id).is_some() && !resolution.in_effect(id) {
                found.push((
                    Severity::Warning,
                    Target::Accounts,
                    trf(
                        "In the account mode ask the rule {id} should be on, so that Setup asks for a local account",
                        &[("id", &id)],
                    ),
                ));
            }
        }
    }
    if resolution.in_effect(RULE_AUTOLOGON) {
        check_autologon(catalog, profile, mode, found);
    }
}

/// One account of the profile: its name, its password, its texts, its group, and the names seen
/// so far (lower case) for a name listed twice.
fn check_account(a: &Account, seen: &mut Vec<String>, found: &mut Vec<Found>) {
    let mut err = |message: Message| found.push((Severity::Error, Target::Accounts, message));
    let name = &shown(a.name.trim());
    if name.is_empty() {
        err(tr("An account has no name"));
        return;
    }
    if !name.is_ascii() || !a.password.is_ascii() {
        err(trf(
            "Account {name}: the name and the password must be ASCII, Windows Setup turns other characters into question marks",
            &[("name", name)],
        ));
    }
    if name.chars().count() > ACCOUNT_NAME_MAX {
        err(trf(
            "Account {name}: the name is longer than {max} characters",
            &[("name", name), ("max", &ACCOUNT_NAME_MAX)],
        ));
    }
    if !matches!(a.group.as_str(), "Administrators" | "Users") {
        err(trf(
            "Account {name}: the group must be Administrators or Users",
            &[("name", name)],
        ));
    }
    let lower = name.to_lowercase();
    if seen.contains(&lower) {
        err(trf("Account {name} is listed twice", &[("name", name)]));
    }
    seen.push(lower);
    if !a.display_name.is_ascii() {
        found.push((
            Severity::Info,
            Target::Accounts,
            trf(
                "Account {name}: the display name outside ASCII is set after OOBE",
                &[("name", name)],
            ),
        ));
    }
}

/// The automatic sign-in names an account of the profile without a password.
fn check_autologon(catalog: &Catalog, profile: &Profile, mode: &str, found: &mut Vec<Found>) {
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
        Some(a) if !a.password.is_empty() => found.push((
            Severity::Error,
            target,
            trf(
                "The automatic sign-in needs an account without a password; {account} has one",
                &[("account", &account)],
            ),
        )),
        Some(_) => {
            if mode == "ask" {
                found.push((
                    Severity::Warning,
                    target,
                    tr(
                        "The automatic sign-in names an account of the profile, but in the mode ask Windows Setup creates the account",
                    ),
                ));
            }
        }
        None => found.push((
            Severity::Error,
            target,
            trf(
                "The automatic sign-in names the account {account}, which the profile does not list",
                &[("account", &account)],
            ),
        )),
    }
}

fn check_languages(profile: &Profile, found: &mut Vec<Found>) {
    let l = &profile.languages;
    let mut err = |message: Message| found.push((Severity::Error, Target::Languages, message));
    if !is_culture(&l.ui_language) {
        err(trf(
            "The display language {value} is not a language tag such as uk-UA",
            &[("value", &l.ui_language)],
        ));
    }
    if !is_culture(&l.system_locale) {
        err(trf(
            "The system locale {value} is not a language tag such as uk-UA",
            &[("value", &l.system_locale)],
        ));
    }
    if !is_culture(&l.user_locale) {
        err(trf(
            "The user locale {value} is not a language tag such as uk-UA",
            &[("value", &l.user_locale)],
        ));
    }
    if l.input.is_empty() {
        err(tr("No input language: at least one is needed"));
    }
    let table = crate::resources::locales();
    for item in &l.input {
        if !is_culture(item) && !is_layout(item) {
            err(trf(
                "The input language {item} is not a language tag or a layout id such as 0409:00000409",
                &[("item", item)],
            ));
        } else if !table.languages.is_empty() && table.input(item).is_none() {
            err(trf(
                "Windows does not know the input language {item}",
                &[("item", item)],
            ));
        }
    }
}

fn check_rules(
    catalog: &Catalog,
    profile: &Profile,
    resolution: &Resolution,
    found: &mut Vec<Found>,
) {
    for rule in catalog.rules() {
        let target = || Target::Rule(rule.id.clone());
        match resolution.state(&rule.id) {
            State::Blocked { by } => found.push((
                Severity::Warning,
                target(),
                trf(
                    "Chosen on but blocked: it needs {by}, which is off",
                    &[("by", &by)],
                ),
            )),
            State::Off => {
                if rule.level == Level::Baseline {
                    found.push((
                        Severity::Warning,
                        target(),
                        tr("A baseline rule of the protection is off"),
                    ));
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
            for message in param_problems(catalog, profile, rule, name, param, &value) {
                found.push((Severity::Error, target(), message));
            }
        }
        if let Some(message) = region_problem(catalog, profile, rule) {
            found.push((Severity::Warning, target(), message));
        }
    }
    for (a, b) in &resolution.conflicts {
        found.push((
            Severity::Error,
            Target::Rule(a.clone()),
            trf(
                "Conflicts with {other}: turn one of them off",
                &[("other", b)],
            ),
        ));
    }
}

/// What is wrong with the value of a parameter of a rule in effect, as messages; none when it
/// fits.
fn param_problems(
    catalog: &Catalog,
    profile: &Profile,
    rule: &Rule,
    name: &str,
    param: &Param,
    value: &Value,
) -> Vec<Message> {
    match param {
        Param::Enum {
            values,
            differs_from,
            same_allowed,
            ..
        } => {
            let mut out = Vec::new();
            if !values.iter().any(|v| same_value(&v.value, value)) {
                out.push(trf(
                    "Parameter {name}: {value} is not one of its options",
                    &[("name", &name), ("value", value)],
                ));
            }
            if let Some(other) = differs_from
                && let Some(other_value) = profile.param(catalog, &rule.id, other)
                && same_value(&other_value, value)
                && !same_allowed.iter().any(|v| same_value(v, value))
            {
                out.push(trf(
                    "Parameters {name} and {other} must differ",
                    &[("name", &name), ("other", other)],
                ));
            }
            out
        }
        Param::Int { min, max, .. } => {
            if value.as_i64().is_some_and(|v| (*min..=*max).contains(&v)) {
                Vec::new()
            } else {
                vec![trf(
                    "Parameter {name}: {value} is outside {min}..{max}",
                    &[
                        ("name", &name),
                        ("value", value),
                        ("min", min),
                        ("max", max),
                    ],
                )]
            }
        }
        Param::List { .. } => list_problems(rule, name, value),
        Param::Text {
            required,
            max_length,
            ..
        } => {
            let text = value.as_str().unwrap_or_default();
            let mut out = Vec::new();
            if *required && text.trim().is_empty() {
                out.push(trf("Parameter {name} is empty", &[("name", &name)]));
            }
            if let Some(max) = max_length
                && text.chars().count() > *max
            {
                out.push(trf(
                    "Parameter {name}: at most {max} characters, the text has {n}",
                    &[("name", &name), ("max", max), ("n", &text.chars().count())],
                ));
            }
            if !text.is_ascii() {
                out.push(trf(
                    "Parameter {name}: the text must be ASCII, the embedded scripts are read without a code page",
                    &[("name", &name)],
                ));
            }
            out
        }
    }
}

/// What is wrong with the region of a rule whose parameters pick one: the GeoID and the code do
/// not name one region Windows lists. Nothing where the list is empty (Windows is not there), so
/// it is a warning of the check, never an error.
fn region_problem(catalog: &Catalog, profile: &Profile, rule: &Rule) -> Option<Message> {
    let id_name = rule.param_picked(Pick::RegionId)?;
    let code_name = rule.param_picked(Pick::Region)?;
    let table = crate::regions::regions();
    if table.is_empty() {
        return None;
    }
    let text = |name: &str| {
        profile
            .param(catalog, &rule.id, name)
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    let (id, code) = (text(id_name), text(code_name));
    match (table.by_id(&id), table.by_code(&code)) {
        (Some(a), Some(b)) if a.code == b.code => None,
        (Some(a), Some(b)) => Some(trf(
            "Parameters {id_name} and {code_name} name different regions: {id} is {a} and {code} is {b}",
            &[
                ("id_name", &id_name),
                ("code_name", &code_name),
                ("id", &id),
                ("a", &a.name),
                ("code", &code),
                ("b", &b.name),
            ],
        )),
        (None, _) => Some(trf(
            "Parameter {name}: Windows lists no region with the GeoID {id}",
            &[("name", &id_name), ("id", &id)],
        )),
        (_, None) => Some(trf(
            "Parameter {name}: Windows lists no region with the code {code}",
            &[("name", &code_name), ("code", &code)],
        )),
    }
}

/// The problems of a list parameter: texts only, ASCII, and the names of the values of every
/// `reg-list` that writes it.
fn list_problems(rule: &Rule, name: &str, value: &Value) -> Vec<Message> {
    let texts = value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_string));
    if !texts {
        return vec![trf(
            "Parameter {name}: a list of texts is needed",
            &[("name", &name)],
        )];
    }
    let mut out = Vec::new();
    let items = crate::catalog::values::list_items(Some(value));
    if items.iter().any(|t| !t.is_ascii()) {
        out.push(trf(
            "Parameter {name}: every item must be ASCII text, the embedded scripts are read without a code page",
            &[("name", &name)],
        ));
    }
    for action in &rule.actions {
        if let Action::RegList {
            param,
            names,
            prefix,
            ..
        } = action
            && param == name
            && let Err(e) = crate::catalog::values::list_entries(*names, prefix, &items)
        {
            // The reason is the one of the list itself, in English as the logs have it.
            out.push(trf(
                "Parameter {name}: {error}",
                &[("name", &name), ("error", &e)],
            ));
        }
    }
    out
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
    a.len() == 4 && b.len() == 8 && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_hexdigit())
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
        assert!(
            !is_name_template("PC-{user}")
                && !is_name_template("PC-{random")
                && !is_name_template("")
        );
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
