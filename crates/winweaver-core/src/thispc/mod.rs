//! This PC: how the rules of the catalog look on a running Windows (`docs/spec/03-features.md`
//! section 5). The plan of checks, the judgement of the rules and the capture into a profile are
//! pure; the machine is reached through [`Machine`], implemented natively by `winweaver-win` and
//! in memory by the tests. Nothing in this module writes anything.
//!
//! The checks are the ones of the audit script (`data/templates/Audit.runtime.ps1`), so a report
//! of that script, made on a PC without WinWeaver, is judged the same way as a native read.
//!
//! The folder: `plan` (the checks of a selection of rules), `report` (the results and the data
//! of the forms, as the audit script writes them), `evaluate` (the checks run on a machine),
//! `judge` (the verdict of each rule), `capture` (the readings into a profile) and `memory`
//! (a machine in memory for the tests).

use crate::catalog::{Action, Catalog, Phase, Rule};
use crate::registry::Data;

mod capture;
mod evaluate;
mod judge;
mod memory;
mod plan;
mod report;

pub use capture::{CaptureSummary, capture};
pub use evaluate::run;
pub use judge::{Reading, Verdict, judge};
pub use memory::MemoryMachine;
pub use plan::{Check, Plan, Probe, audit_path, plan, wildcard};
pub use report::{CheckResult, InputLanguage, ReadAccount, Report, Status, SystemInfo};

/// Why a rule is left out of the work on a running Windows.
pub const ONLY_DURING_INSTALL: &str = "takes effect only during Windows installation";
pub const FIRST_SIGN_IN: &str =
    "runs at the first sign-in of each user; not applied to a running system";
pub const SIGN_IN_SCREEN: &str = "changes the sign-in screen; not applied to a running system";
/// The note of a check of the default user profile, made in HKCU of the current user.
pub const NOTE_CURRENT_USER: &str = "current user instead of the default profile";
/// The note of a check of a feature or a capability without administrator rights, as the audit
/// script writes it.
pub const NOTE_ADMIN: &str = "administrator rights needed";
/// The note of a feature that this build of Windows does not have.
pub const NOTE_NOT_ON_BUILD: &str = "not present on this build";
pub(super) const SERVICES: &str = "HKLM:\\SYSTEM\\CurrentControlSet\\Services";
pub(super) const USER_PACKAGES: &str = "HKCU:\\Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages";
pub(super) const PROVISIONED_PACKAGES: &str =
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

/// The rules a read of the whole PC covers: every rule of the catalog but the imported policies.
pub fn whole_pc_rules(catalog: &Catalog) -> Vec<String> {
    catalog
        .rules()
        .iter()
        .filter(|r| !r.id.starts_with(crate::admx::RULE_PREFIX))
        .map(|r| r.id.clone())
        .collect()
}

/// The name of the profile a read of the whole PC makes: the name of the computer, `This PC`
/// when the read has none.
pub fn pc_profile_name(computer: &str) -> String {
    if computer.is_empty() {
        "This PC".to_owned()
    } else {
        computer.to_owned()
    }
}

/// The edition name of a Windows edition id.
pub fn edition_of(edition_id: &str) -> Option<&'static str> {
    EDITION_IDS
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(edition_id))
        .map(|(_, name)| *name)
}

impl Data {
    /// The value as the audit script reports it: a DWORD as a signed number, a list joined by
    /// commas, bytes as numbers, another kind of value named as such.
    pub fn audit_text(&self) -> String {
        match self {
            Data::Dword(v) => (*v as i32).to_string(),
            Data::Qword(v) => v.to_string(),
            Data::Sz(s) | Data::ExpandSz(s) => s.clone(),
            Data::MultiSz(items) => items.join(","),
            Data::Binary(bytes) => bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            Data::None(_) | Data::Other { .. } => "(another kind of value)".to_owned(),
        }
    }

    /// The numbers a value stands for when a rule wants a number: a DWORD both unsigned and
    /// signed, a QWORD, a text of digits.
    pub(super) fn numbers(&self) -> Vec<i64> {
        match self {
            Data::Dword(v) => vec![i64::from(*v), i64::from(*v as i32)],
            Data::Qword(v) => vec![*v as i64],
            Data::Sz(s) | Data::ExpandSz(s) => s.trim().parse().into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// What the program reads of a running Windows. Paths are written as in the catalog
/// (`HKLM:\...`, `HKCU:\...`, `HKU:\...`); the name `(Default)` is the default value of a key.
pub trait Machine {
    /// The value, `None` when the key or the value does not exist, an error when it cannot be read.
    fn value(&self, path: &str, name: &str) -> Result<Option<Data>, String>;
    /// The names of the subkeys, `None` when the key does not exist.
    fn subkeys(&self, path: &str) -> Result<Option<Vec<String>>, String>;
    /// The values of a key with their names, `None` when the key does not exist.
    fn values(&self, path: &str) -> Result<Option<Vec<(String, Data)>>, String>;
    fn is_admin(&self) -> bool;
    fn computer_name(&self) -> String;
    /// The data of the forms (edition, time zone, languages).
    fn system_info(&self) -> SystemInfo;
    /// The optional features and the capabilities of Windows; asked only of an administrator,
    /// once for a whole run.
    fn components(&self) -> Components;
}

/// The optional features and the capabilities of Windows, each a name with its state.
#[derive(Debug, Clone, PartialEq)]
pub struct Components {
    /// States as `Get-WindowsOptionalFeature` names them: `Enabled`, `Disabled`,
    /// `DisabledWithPayloadRemoved`, `EnablePending`, `DisablePending`.
    pub features: Result<Vec<(String, String)>, String>,
    /// States as `Get-WindowsCapability` names them: `Installed`, `NotPresent`, `Staged`.
    pub capabilities: Result<Vec<(String, String)>, String>,
}
