//! The readings of the rules and the data of the forms into a profile.

use indexmap::IndexMap;

use crate::catalog::{Catalog, Param, check::same_value};
use crate::profile::{Account, Profile};
use crate::resources;

use super::edition_of;
use super::judge::{Reading, Verdict};
use super::report::SystemInfo;

/// The counts of a capture and what it did with the forms.
#[derive(Debug, Clone, Default)]
pub struct CaptureSummary {
    pub counts: IndexMap<&'static str, usize>,
    pub forms: Vec<String>,
    pub notes: Vec<String>,
    /// The rules partly in effect, with what differs: a rule that was applied and of which
    /// Windows or another program changed a value since.
    pub partly: Vec<(String, Vec<String>)>,
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
            Verdict::Partly => {
                profile.set_on(&reading.rule, false);
                summary
                    .partly
                    .push((reading.rule.clone(), reading.differing.clone()));
            }
            Verdict::NotInEffect => profile.set_on(&reading.rule, false),
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
    let mut known: Vec<String> = Vec::new();
    let mut unknown: Vec<&str> = Vec::new();
    let table = resources::locales();
    for language in &info.input {
        match table.read_items(&language.tag, &language.tips) {
            Some(items) => {
                for item in items {
                    if !known.iter().any(|k| k.eq_ignore_ascii_case(&item)) {
                        known.push(item);
                    }
                }
            }
            None => unknown.push(language.tag.as_str()),
        }
    }
    if !unknown.is_empty() {
        summary.notes.push(format!(
            "input languages Windows does not list were left out: {}",
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
