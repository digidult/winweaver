//! Users files (`.wwusers`, `docs/spec/02-documents.md` section 11): the local accounts to make
//! on a PC, with their passwords encrypted under a master password in the same file
//! (`userstore`). A file is edited in its own tab, read from this PC, applied to this PC
//! natively, saved as a stand-alone script, and loaded into the accounts of a profile.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::clock;
use crate::jsonfile::{self, JsonError};
use crate::profile::Account;
use crate::render::{self, RenderError};
use crate::userstore::{Container, Passwords};

pub const FORMAT: &str = "winweaver-users";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "wwusers";
/// The longest name of a local account Windows takes.
pub const NAME_MAX: usize = 20;
const MAX_BYTES: u64 = 4 * 1024 * 1024;
const SCRIPT: &str = include_str!("../../../data/templates/Create-Users.runtime.ps1");

#[derive(Debug, thiserror::Error)]
pub enum UsersError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error("{name}: {source}")]
    Io {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{name}: not a WinWeaver users file (format {found})")]
    Format { name: String, found: String },
    #[error("{name}: users file version {found} is newer than this program knows ({VERSION})")]
    Version { name: String, found: u32 },
}

/// What to do with the enabled flag of an account: nothing to one that exists (a new one is
/// made enabled), enable it, or disable it (a new one is made disabled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Keep,
    Enabled,
    Disabled,
}

impl State {
    pub const ALL: [State; 3] = [State::Keep, State::Enabled, State::Disabled];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Keep => "keep",
            State::Enabled => "enabled",
            State::Disabled => "disabled",
        }
    }
}

/// One local account of the file. The first fields describe the account; `apply_password`,
/// `state` and `force_group` are the explicit orders for an account that already exists on the
/// PC, which is left as it is without them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct User {
    pub name: String,
    pub full_name: String,
    pub description: String,
    /// In the group Administrators, else in Users.
    pub admin: bool,
    pub password_never_expires: bool,
    /// The folder of the profile, empty for the one Windows chooses.
    pub profile_path: String,
    /// Set the password of the file even though the account exists.
    pub apply_password: bool,
    pub state: State,
    /// Make the membership of Administrators and Users match `admin` exactly.
    pub force_group: bool,
}

impl User {
    pub fn group(&self) -> &'static str {
        if self.admin { "Administrators" } else { "Users" }
    }

    /// Whether the row asks for changes to an account that already exists.
    pub fn touches_existing(&self) -> bool {
        self.apply_password || self.force_group || self.state != State::Keep
    }
}

/// A users file: the accounts and, when a master password was set, their passwords encrypted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UsersFile {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub comment: String,
    pub created: String,
    pub modified: String,
    pub users: Vec<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passwords: Option<Container>,
}

impl Default for UsersFile {
    fn default() -> Self {
        Self::new("Users")
    }
}

impl UsersFile {
    pub fn new(name: &str) -> Self {
        let now = clock::now_utc_iso();
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            name: name.to_owned(),
            comment: String::new(),
            created: now.clone(),
            modified: now,
            users: Vec::new(),
            passwords: None,
        }
    }

    pub fn load(path: &Path) -> Result<Self, UsersError> {
        let name = jsonfile::display_name(path);
        let text = jsonfile::read_text(path, MAX_BYTES)?;
        Self::parse(&text, &name)
    }

    pub fn parse(text: &str, name: &str) -> Result<Self, UsersError> {
        let file: Self = jsonfile::parse(text, name)?;
        if file.format != FORMAT {
            return Err(UsersError::Format {
                name: name.to_owned(),
                found: file.format,
            });
        }
        if file.version > VERSION {
            return Err(UsersError::Version {
                name: name.to_owned(),
                found: file.version,
            });
        }
        Ok(file)
    }

    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    /// Writes the file through a temporary one; `modified` is set to now.
    pub fn save(&mut self, path: &Path) -> Result<(), UsersError> {
        self.modified = clock::now_utc_iso();
        let name = jsonfile::display_name(path);
        let io = |source: std::io::Error| UsersError::Io {
            name: name.clone(),
            source,
        };
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_json()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    /// The user of a name, without regard to case.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.users
            .iter()
            .position(|u| u.name.eq_ignore_ascii_case(name.trim()))
    }

    /// The accounts of a profile the file gives: the group by `admin`, the display name of the
    /// full name, and the passwords when the file is unlocked.
    pub fn accounts(&self, passwords: Option<&Passwords>) -> Vec<Account> {
        self.users
            .iter()
            .map(|u| Account {
                name: u.name.clone(),
                display_name: u.full_name.clone(),
                group: u.group().to_owned(),
                description: u.description.clone(),
                password: passwords
                    .and_then(|p| p.get(&u.name))
                    .unwrap_or_default()
                    .to_owned(),
            })
            .collect()
    }
}

/// A finding of a check: an error stops an operation, a warning does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub error: bool,
    /// The account it is about, empty for the file.
    pub user: String,
    pub text: String,
}

fn issue(error: bool, user: &str, text: String) -> Issue {
    Issue {
        error,
        user: user.to_owned(),
        text,
    }
}

/// Why a name is not a name of a local account, or `None` when it is one.
pub fn name_problem(name: &str) -> Option<String> {
    const BAD: &str = "\"/\\[]:;|=,+*?<>@";
    let name = name.trim();
    if name.is_empty() {
        return Some("The name is empty".to_owned());
    }
    if name.chars().count() > NAME_MAX {
        return Some(format!("The name is longer than {NAME_MAX} characters"));
    }
    if name.chars().any(|c| BAD.contains(c)) {
        return Some(format!("The name holds one of {BAD}"));
    }
    if name.chars().any(char::is_control) {
        return Some("The name holds a control character".to_owned());
    }
    if name.ends_with('.') {
        return Some("The name ends with a dot".to_owned());
    }
    if name.chars().all(|c| c == '.' || c == ' ') {
        return Some("The name is dots and spaces only".to_owned());
    }
    None
}

/// Whether a text is an absolute Windows path (`D:\Profiles\user1` or `\\server\share\...`)
/// without the characters a path cannot hold; the empty path means the one Windows chooses.
pub fn is_profile_path(path: &str) -> bool {
    const BAD: &str = "<>:\"|?*";
    let path = path.trim();
    if path.is_empty() {
        return true;
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let rest = &path[2..];
        return rest.starts_with(['\\', '/']) && !rest.chars().any(|c| BAD.contains(c));
    }
    if let Some(rest) = path.strip_prefix("\\\\") {
        let mut parts = rest.splitn(3, '\\');
        let host = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        let ok = |s: &str| !s.is_empty() && !s.chars().any(|c| BAD.contains(c) || c == '/');
        return ok(host) && ok(share);
    }
    false
}

/// The checks of the accounts: the names, the paths, the duplicates and the shared profile
/// folders.
pub fn check(users: &[User]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut folders: Vec<(String, &str)> = Vec::new();
    for u in users {
        if let Some(why) = name_problem(&u.name) {
            issues.push(issue(true, &u.name, why));
            continue;
        }
        if !is_profile_path(&u.profile_path) {
            let why = format!("The profile path {} is not an absolute path", u.profile_path);
            issues.push(issue(true, &u.name, why));
        }
        if seen.iter().any(|s| s.eq_ignore_ascii_case(&u.name)) {
            issues.push(issue(true, &u.name, format!("The name {} is used twice", u.name)));
        } else {
            seen.push(&u.name);
        }
        let folder = u.profile_path.trim().trim_end_matches(['\\', '/']).to_lowercase();
        if !folder.is_empty() {
            match folders.iter().find(|(f, _)| *f == folder) {
                Some((_, owner)) if !owner.eq_ignore_ascii_case(&u.name) => {
                    let why = format!("The profile path is also the one of {owner}");
                    issues.push(issue(true, &u.name, why));
                }
                Some(_) => {}
                None => folders.push((folder, &u.name)),
            }
        }
    }
    issues
}

/// The accounts without a password (an error) and the passwords without an account (a warning).
pub fn check_passwords(users: &[User], passwords: &Passwords) -> Vec<Issue> {
    let mut issues = Vec::new();
    for u in users {
        if !passwords.has(&u.name) {
            issues.push(issue(true, &u.name, "No password for this account".to_owned()));
        }
    }
    for name in passwords.names() {
        if !users.iter().any(|u| u.name.eq_ignore_ascii_case(&name)) {
            let why = format!("A password of {name}, which is not in the list");
            issues.push(issue(false, &name, why));
        }
    }
    issues
}

/// What an operation on this PC does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Create the accounts that are missing; change the existing ones by their explicit orders,
    /// and with `update_existing` also by the rest of their row (never the password).
    Create { update_existing: bool },
    /// Set the passwords of the accounts that exist and change nothing else.
    Passwords,
}

/// The result of one account of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Created,
    Updated,
    AlreadyExists,
    PasswordSet,
    NotFound,
    Error,
    Skipped,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Created => "CREATED",
            Status::Updated => "UPDATED",
            Status::AlreadyExists => "ALREADY_EXISTS",
            Status::PasswordSet => "PASSWORD_SET",
            Status::NotFound => "NOT_FOUND",
            Status::Error => "ERROR",
            Status::Skipped => "SKIPPED",
        }
    }

    pub fn failed(self) -> bool {
        matches!(self, Status::Error | Status::NotFound)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub name: String,
    pub status: Status,
    /// The changes of an update, or the reason of an error; empty otherwise.
    pub message: String,
}

/// An operation on this PC: the accounts it takes, each with its password. The plan is checked
/// before anything changes: every account is valid and has a password.
#[derive(Debug)]
pub struct Plan {
    pub operation: Operation,
    pub users: Vec<User>,
}

impl Plan {
    /// The plan of the selected accounts (every one when the selection is empty), or the issues
    /// that stop it.
    pub fn new(
        operation: Operation,
        users: &[User],
        selected: &[String],
        passwords: &Passwords,
    ) -> Result<Self, Vec<Issue>> {
        let mut issues = check(users);
        issues.retain(|i| i.error);
        let chosen: Vec<User> = users
            .iter()
            .filter(|u| {
                selected.is_empty()
                    || selected.iter().any(|s| s.eq_ignore_ascii_case(&u.name))
            })
            .cloned()
            .collect();
        for name in selected {
            if !users.iter().any(|u| u.name.eq_ignore_ascii_case(name)) {
                issues.push(issue(true, name, "Not in the list".to_owned()));
            }
        }
        for u in &chosen {
            if !passwords.has(&u.name) {
                issues.push(issue(true, &u.name, "No password for this account".to_owned()));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        Ok(Self {
            operation,
            users: chosen,
        })
    }
}

/// The stand-alone script of a users file: it reads the file of `file_name` next to it, asks the
/// master password itself and creates the accounts, or sets the passwords only with a switch.
pub fn script(file_name: &str, app_version: &str) -> Result<String, RenderError> {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(s, _)| s);
    let ascii = |text: &str| -> Result<(), RenderError> {
        if text.is_ascii() && !text.contains('\'') {
            Ok(())
        } else {
            let why = format!("the script needs an ASCII file name without quotes: {text}");
            Err(RenderError::Profile(why))
        }
    };
    ascii(file_name)?;
    render::fill(
        SCRIPT,
        &[
            ("users_file", file_name),
            ("log_stem", stem),
            ("build", app_version),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(name: &str) -> User {
        User {
            name: name.to_owned(),
            ..User::default()
        }
    }

    #[test]
    fn names_and_paths_are_checked() {
        assert!(name_problem("admin1").is_none());
        assert!(name_problem("Пользователь").is_none());
        assert!(name_problem("").is_some());
        assert!(name_problem("a/b").is_some());
        assert!(name_problem("name.").is_some());
        assert!(name_problem(". .").is_some());
        assert!(name_problem(&"x".repeat(21)).is_some());
        assert!(is_profile_path(""));
        assert!(is_profile_path("D:\\Profiles\\user 1"));
        assert!(is_profile_path("\\\\server\\share\\a"));
        assert!(!is_profile_path("Profiles\\a"));
        assert!(!is_profile_path("D:\\a|b"));
        assert!(!is_profile_path("\\\\server"));
        let mut users = vec![user("a"), user("A"), user("b")];
        users[0].profile_path = "D:\\P\\".into();
        users[2].profile_path = "d:\\p".into();
        let issues = check(&users);
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues.iter().all(|i| i.error));
    }

    #[test]
    fn a_file_keeps_its_users_and_its_passwords() {
        let mut file = UsersFile::new("Office");
        file.users.push(User {
            name: "admin1".into(),
            admin: true,
            password_never_expires: true,
            ..User::default()
        });
        file.users.push(user("user1"));
        let mut passwords = Passwords::default();
        passwords.set("admin1", "Password1!");
        let container = crate::userstore::encrypt(&passwords, b"m", 100_000).unwrap();
        file.passwords = Some(container);
        let text = file.to_json();
        assert!(!text.contains("Password1!"));
        let back = UsersFile::parse(&text, "x").unwrap();
        assert_eq!(back, file);
        assert_eq!(back.find("USER1"), Some(1));
        let issues = check_passwords(&back.users, &passwords);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].error && issues[0].user == "user1");
        let accounts = back.accounts(Some(&passwords));
        assert_eq!(accounts[0].group, "Administrators");
        assert_eq!(accounts[0].password, "Password1!");
        assert_eq!(accounts[1].group, "Users");
        assert!(accounts[1].password.is_empty());
        assert!(back.accounts(None)[0].password.is_empty());
        assert!(UsersFile::parse("{\"format\": \"x\"}", "x").is_err());
        let selected = vec!["user1".to_owned()];
        let plan = Plan::new(Operation::Passwords, &back.users, &selected, &passwords);
        assert!(plan.is_err());
        let plan = Plan::new(Operation::Passwords, &back.users, &["ADMIN1".into()], &passwords);
        assert_eq!(plan.unwrap().users.len(), 1);
    }

    #[test]
    fn the_script_names_the_file_and_fills_every_slot() {
        let text = script("Office.wwusers", "1.0").unwrap();
        assert!(text.contains("'Office.wwusers'"));
        assert!(text.contains("Office-create-users-"));
        assert!(!text.contains("{{"));
        assert!(text.is_ascii());
        assert!(script("Офис.wwusers", "1.0").is_err());
    }
}
