//! The executor of an apply plan, run by the elevated helper: registry values and keys, service
//! start types, programs of the system folder (also on a folder of the installation media),
//! capability removals through DISM, app removals through the package API, the flags of local
//! accounts and the copies of the answer file. Each registry change is backed up first; the
//! changes no undo takes back are named in the backup. It never runs a script: the script steps of
//! the catalog come here as the native steps of `apply::script_ops`.

use std::borrow::Cow;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, KEY_SET_VALUE, RegType,
};
use winreg::{RegKey, RegValue};
use winweaver_core::apply::{self, BackupEntry, Op, Plan, Report, StepResult};
use winweaver_core::catalog::RegKind;
use winweaver_core::thispc::{Machine, RegData};

use crate::native::NativeMachine;

fn split(path: &str) -> Result<(RegKey, String), String> {
    let (root, sub) = path
        .split_once(":\\")
        .ok_or_else(|| format!("unsupported registry path {path}"))?;
    let hive = match root.to_ascii_uppercase().as_str() {
        "HKLM" => HKEY_LOCAL_MACHINE,
        "HKCU" => HKEY_CURRENT_USER,
        "HKU" => HKEY_USERS,
        _ => return Err(format!("unsupported registry path {path}")),
    };
    Ok((RegKey::predef(hive), sub.to_owned()))
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn number(data: &Value) -> Result<i64, String> {
    match data {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .ok_or_else(|| format!("{n} is not a whole number")),
        Value::String(s) => s.trim().parse().map_err(|_| format!("{s} is not a number")),
        Value::Bool(b) => Ok(i64::from(*b)),
        other => Err(format!("{other} is not a number")),
    }
}

fn text(data: &Value) -> String {
    match data {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The raw bytes of a value of a kind.
pub fn encode(kind: RegKind, data: &Value) -> Result<RegValue<'static>, String> {
    let (bytes, vtype) = match kind {
        RegKind::DWord => (
            (number(data)? as u32).to_le_bytes().to_vec(),
            RegType::REG_DWORD,
        ),
        RegKind::QWord => (
            (number(data)? as u64).to_le_bytes().to_vec(),
            RegType::REG_QWORD,
        ),
        RegKind::String => (utf16(&text(data)), RegType::REG_SZ),
        RegKind::ExpandString => (utf16(&text(data)), RegType::REG_EXPAND_SZ),
        RegKind::MultiString => {
            let items: Vec<String> = match data {
                Value::Array(list) => list.iter().map(text).collect(),
                Value::String(s) if s.is_empty() => Vec::new(),
                other => vec![text(other)],
            };
            let mut bytes = Vec::new();
            for item in &items {
                bytes.extend(utf16(item));
            }
            bytes.extend([0, 0]);
            (bytes, RegType::REG_MULTI_SZ)
        }
        RegKind::Binary => {
            let list = data.as_array().ok_or("a binary value is a list of bytes")?;
            let bytes: Result<Vec<u8>, String> = list
                .iter()
                .map(|v| {
                    number(v)
                        .and_then(|n| u8::try_from(n).map_err(|_| format!("{n} is not a byte")))
                })
                .collect();
            (bytes?, RegType::REG_BINARY)
        }
    };
    Ok(RegValue {
        bytes: Cow::Owned(bytes),
        vtype,
    })
}

/// The backup entry of a value as it is now.
fn snapshot(machine: &NativeMachine, path: &str, name: &str) -> BackupEntry {
    let (existed, value_type, value) = match machine.value(path, name) {
        Ok(Some(data)) => {
            let (t, v) = match data {
                RegData::Dword(v) => (Some("DWord"), Value::from(v as i32)),
                RegData::Qword(v) => (Some("QWord"), Value::from(v as i64)),
                RegData::Str(s) => (Some("String"), Value::from(s)),
                RegData::ExpandStr(s) => (Some("ExpandString"), Value::from(s)),
                RegData::Multi(items) => (Some("MultiString"), Value::from(items)),
                RegData::Binary(bytes) => (
                    Some("Binary"),
                    Value::from(bytes.into_iter().map(u64::from).collect::<Vec<_>>()),
                ),
                RegData::Other => (None, Value::Null),
            };
            (true, t.map(str::to_owned), v)
        }
        _ => (false, None, Value::Null),
    };
    BackupEntry {
        kind: "reg".to_owned(),
        path: path.to_owned(),
        name: name.to_owned(),
        existed,
        value_type,
        value,
    }
}

fn write_value(path: &str, name: &str, kind: RegKind, data: &Value) -> Result<(), String> {
    let value = encode(kind, data)?;
    let (root, sub) = split(path)?;
    let (key, _) = root.create_subkey(&sub).map_err(|e| e.to_string())?;
    let name = if name == "(Default)" { "" } else { name };
    key.set_raw_value(name, &value).map_err(|e| e.to_string())
}

fn delete_value(path: &str, name: &str) -> Result<bool, String> {
    let (root, sub) = split(path)?;
    let key = match root.open_subkey_with_flags(&sub, KEY_READ | KEY_SET_VALUE) {
        Ok(k) => k,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    let name = if name == "(Default)" { "" } else { name };
    match key.delete_value(name) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// The key of the default user profile under HKEY_USERS (`UnattendDefault`).
fn default_user_subkey() -> &'static str {
    apply::DEFAULT_USER_KEY
        .strip_prefix("HKU:\\")
        .unwrap_or(apply::DEFAULT_USER_KEY)
}

/// Enables a privilege the token of the process holds, such as `SeRestorePrivilege`.
fn enable_privilege(name: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID,
    };
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    let name_w = wide(name);
    // SAFETY: the name is null-terminated and the LUID is written by the call.
    if unsafe { LookupPrivilegeValueW(std::ptr::null(), name_w.as_ptr(), &mut luid) } == 0 {
        return Err(format!("the privilege {name} is not known"));
    }
    let mut token: HANDLE = std::ptr::null_mut();
    let access = TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY;
    // SAFETY: the pseudo handle of the process is always valid; the token handle is closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut token) } == 0 {
        return Err("the token of the process cannot be opened".to_owned());
    }
    let state = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let none = std::ptr::null_mut();
    // SAFETY: the state holds one privilege; the previous state is not asked for.
    let ok = unsafe { AdjustTokenPrivileges(token, 0, &state, 0, none, std::ptr::null_mut()) };
    // SAFETY: the error of the call above, read before any other call of the thread.
    let error = unsafe { GetLastError() };
    // SAFETY: the token handle came from OpenProcessToken and is closed once.
    unsafe { CloseHandle(token) };
    if ok == 0 || error == ERROR_NOT_ALL_ASSIGNED {
        return Err(format!("the process does not hold the privilege {name}"));
    }
    Ok(())
}

/// The hive file of the default user profile, such as `C:\Users\Default\NTUSER.DAT`.
fn default_user_hive() -> Result<PathBuf, String> {
    use windows_sys::Win32::UI::Shell::GetDefaultUserProfileDirectoryW;
    let mut buffer = [0u16; 512];
    let mut size = buffer.len() as u32;
    // SAFETY: the buffer is valid for the size passed, in characters.
    if unsafe { GetDefaultUserProfileDirectoryW(buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err("the folder of the default user profile is not known".to_owned());
    }
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..end])).join("NTUSER.DAT"))
}

/// The hive of the default user profile, loaded at the key of the stand-alone apply script while
/// a plan writes it.
struct DefaultUser {
    /// Whether this process loaded the hive; a hive loaded before stays loaded.
    loaded_here: bool,
}

impl DefaultUser {
    /// Loads the hive with the backup and restore privileges, or takes the one already loaded
    /// at the key.
    fn mount() -> Result<Self, String> {
        use windows_sys::Win32::System::Registry::{HKEY_USERS as USERS, RegLoadKeyW};
        let sub = default_user_subkey();
        if RegKey::predef(HKEY_USERS).open_subkey(sub).is_ok() {
            return Ok(Self { loaded_here: false });
        }
        enable_privilege("SeRestorePrivilege")?;
        enable_privilege("SeBackupPrivilege")?;
        let file = default_user_hive()?;
        let (sub_w, file_w) = (wide(sub), wide(&file.display().to_string()));
        // SAFETY: both texts are null-terminated and HKEY_USERS is a predefined key.
        let status = unsafe { RegLoadKeyW(USERS, sub_w.as_ptr(), file_w.as_ptr()) };
        if status != 0 {
            let file = file.display();
            return Err(format!(
                "the default user profile {file} was not loaded (error {status})"
            ));
        }
        Ok(Self { loaded_here: true })
    }

    /// Unloads the hive when this process loaded it; every key of the hive must be closed.
    fn unload(&mut self) -> Result<(), String> {
        use windows_sys::Win32::System::Registry::{HKEY_USERS as USERS, RegUnLoadKeyW};
        if !self.loaded_here {
            return Ok(());
        }
        self.loaded_here = false;
        let sub_w = wide(default_user_subkey());
        // SAFETY: the text is null-terminated and HKEY_USERS is a predefined key.
        let status = unsafe { RegUnLoadKeyW(USERS, sub_w.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "the default user profile was not unloaded (error {status})"
            ))
        }
    }

    fn unmount(mut self) -> Result<(), String> {
        self.unload()
    }
}

impl Drop for DefaultUser {
    fn drop(&mut self) {
        let _ = self.unload();
    }
}

/// The names of the values of a key, `(Default)` for the default value; none when the key does
/// not exist.
fn value_names(path: &str) -> Result<Vec<String>, String> {
    let (root, sub) = split(path)?;
    let key = match root.open_subkey_with_flags(&sub, KEY_READ) {
        Ok(k) => k,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    key.enum_values()
        .map(|v| {
            v.map(|(name, _)| {
                if name.is_empty() {
                    "(Default)".to_owned()
                } else {
                    name
                }
            })
        })
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// The program of the system folder a plan may run: a bare file name of letters, digits, dots,
/// hyphens and underscores that ends with `.exe`.
pub fn system_program(file: &str) -> Result<PathBuf, String> {
    let ok = file.to_ascii_lowercase().ends_with(".exe")
        && file.len() <= 64
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    if !ok {
        return Err(format!("{file} is not a program of the system folder"));
    }
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned());
    Ok(Path::new(&root).join("System32").join(file))
}

fn run(file: &str, args: &[String]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let program = system_program(file)?;
    let output = std::process::Command::new(&program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    match output.status.code() {
        Some(0) => Ok(String::new()),
        Some(3010) => Ok("a restart is needed".to_owned()),
        code => {
            let first = String::from_utf8_lossy(&output.stdout)
                .lines()
                .chain(String::from_utf8_lossy(&output.stderr).lines())
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            Err(format!(
                "exit {}: {first}",
                code.map_or("none".to_owned(), |c| c.to_string())
            ))
        }
    }
}

/// The names of the subkeys of a key; none when the key does not exist.
fn subkey_names(path: &str) -> Result<Vec<String>, String> {
    let (root, sub) = split(path)?;
    let key = match root.open_subkey_with_flags(&sub, KEY_READ) {
        Ok(k) => k,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    key.enum_keys()
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// Every value under a key and its subkeys, as a path and a name; a tree deeper than 32 keys or
/// with more than 10000 values is refused, so it is never deleted without its backup.
fn values_under(path: &str, out: &mut Vec<(String, String)>, depth: usize) -> Result<(), String> {
    if depth > 32 || out.len() > 10_000 {
        return Err(format!("{path} is too large to back up"));
    }
    for name in value_names(path)? {
        out.push((path.to_owned(), name));
    }
    for sub in subkey_names(path)? {
        values_under(&format!("{path}\\{sub}"), out, depth + 1)?;
    }
    Ok(())
}

/// Deletes a key with everything under it; false when it did not exist.
fn delete_key(path: &str) -> Result<bool, String> {
    let (root, sub) = split(path)?;
    match root.delete_subkey_all(&sub) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Whether a folder holds a file of an extension, in its subfolders too when `deep`; at most
/// 100000 entries are looked at.
fn has_file(dir: &Path, extension: &str, deep: bool, seen: &mut usize) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        *seen += 1;
        if *seen > 100_000 {
            return false;
        }
        let path = entry.path();
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            if deep && has_file(&path, extension, deep, seen) {
                return true;
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(extension))
        {
            return true;
        }
    }
    false
}

/// The first drive from C: whose `folder` holds a file of `pattern` (`*.cab`), as the script steps
/// look for the folders of the installation media. A drive without a disk asks nothing.
fn media_folder(folder: &str, pattern: &str, deep: bool) -> Option<PathBuf> {
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX, SetErrorMode,
    };
    let extension = pattern.strip_prefix("*.")?;
    let relative = !folder.contains("..") && !folder.contains(':') && !folder.starts_with('\\');
    if !relative || !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    // SAFETY: the error mode of the process; the previous one comes back below.
    let before = unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX) };
    let found = (b'C'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))).join(folder))
        .find(|dir| dir.is_dir() && has_file(dir, extension, deep, &mut 0));
    // SAFETY: the mode the process had.
    unsafe { SetErrorMode(before) };
    found
}

/// An entry of the backup that names a change no undo takes back, as the apply script names them.
fn named(kind: &str, name: &str) -> BackupEntry {
    BackupEntry {
        kind: kind.to_owned(),
        path: String::new(),
        name: name.to_owned(),
        existed: true,
        value_type: None,
        value: Value::Null,
    }
}

/// The status of a step that did several things: failed when one of them failed.
fn outcome(failed: bool, notes: Vec<String>) -> (&'static str, String) {
    let status = if failed { "failed" } else { "ok" };
    (status, notes.join("; "))
}

/// Sets "password never expires" on the accounts of the profile that exist.
fn never_expires(accounts: &[String], backup: &mut Vec<BackupEntry>) -> (&'static str, String) {
    let mut notes = Vec::new();
    let mut failed = false;
    if accounts.is_empty() {
        notes.push("the profile has no accounts".to_owned());
    }
    for name in accounts {
        match crate::accounts::set_flag(name, crate::accounts::NEVER_EXPIRES, true) {
            Ok(Some(true)) => {
                let what = format!("{name}: password never expires");
                backup.push(named("account-changed", &what));
                notes.push(format!("{name}: set"));
            }
            Ok(Some(false)) => notes.push(format!("{name}: set already")),
            Ok(None) => notes.push(format!("{name}: no such account")),
            Err(e) => {
                failed = true;
                notes.push(e);
            }
        }
    }
    outcome(failed, notes)
}

/// Disables the built-in Administrator and Guest, never the account the helper runs as.
fn disable_builtin(backup: &mut Vec<BackupEntry>) -> (&'static str, String) {
    let found = match crate::accounts::builtin() {
        Ok(found) => found,
        Err(e) => return ("failed", e),
    };
    let me = std::env::var("USERNAME").unwrap_or_default();
    let mut notes = Vec::new();
    let mut failed = false;
    for name in found {
        if name.eq_ignore_ascii_case(&me) {
            notes.push(format!("{name}: left on, it is the account in use"));
            continue;
        }
        match crate::accounts::set_flag(&name, crate::accounts::DISABLED, true) {
            Ok(Some(true)) => {
                backup.push(named("account-changed", &format!("{name}: disabled")));
                notes.push(format!("{name}: disabled"));
            }
            Ok(Some(false)) => notes.push(format!("{name}: disabled already")),
            Ok(None) => {}
            Err(e) => {
                failed = true;
                notes.push(e);
            }
        }
    }
    outcome(failed, notes)
}

/// The lines of the error file of the installation in the note, then the file deleted; no other
/// file is read.
fn collect_errors(file: &str) -> (&'static str, String) {
    use std::io::Read;
    if !file.eq_ignore_ascii_case(apply::SETUP_ERRORS) {
        let why = format!("{file} is not the error file of the installation");
        return ("failed", why);
    }
    let mut bytes = Vec::new();
    let read = std::fs::File::open(file).and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes));
    match read {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return ("ok", "no errors".to_owned()),
        Err(e) => return ("failed", e.to_string()),
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(50)
        .collect();
    let mut notes = if lines.is_empty() {
        vec!["the file was empty".to_owned()]
    } else {
        vec![format!("errors of the installation: {}", lines.join(" | "))]
    };
    if let Err(e) = std::fs::remove_file(file) {
        notes.push(format!("{file} was not deleted: {e}"));
    }
    outcome(false, notes)
}

/// Deletes the copies of the answer file that exist; no other file is deleted.
fn delete_files(paths: &[String], backup: &mut Vec<BackupEntry>) -> (&'static str, String) {
    let mut notes = Vec::new();
    let mut failed = false;
    for path in paths {
        if !apply::ANSWER_FILE_COPIES
            .iter()
            .any(|c| c.eq_ignore_ascii_case(path))
        {
            failed = true;
            notes.push(format!("{path} is not a copy of the answer file, left"));
            continue;
        }
        match std::fs::remove_file(path) {
            Ok(()) => {
                backup.push(named("file-deleted", path));
                notes.push(format!("deleted {path}"));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                failed = true;
                notes.push(format!("{path}: {e}"));
            }
        }
    }
    if notes.is_empty() {
        notes.push("none was there".to_owned());
    }
    outcome(failed, notes)
}

/// Removes apps for every user and for new accounts; each app found goes into the backup the way
/// the apply script names it, so `Undo-Apply.ps1` lists it as not restorable.
fn remove_apps(names: &[String], backup: &mut Vec<BackupEntry>) -> (&'static str, String) {
    let done = match crate::apps::remove_apps(names) {
        Ok(done) => done,
        Err(e) => return ("failed", e),
    };
    for name in &done.found {
        backup.push(BackupEntry {
            kind: "app-removed".to_owned(),
            path: String::new(),
            name: name.clone(),
            existed: true,
            value_type: None,
            value: Value::Null,
        });
    }
    let mut notes = Vec::new();
    if done.found.is_empty() {
        notes.push("none is installed or provisioned".to_owned());
    }
    notes.extend(done.done.iter().cloned());
    notes.extend(done.failed.iter().cloned());
    let status = if done.failed.is_empty() {
        "ok"
    } else {
        "failed"
    };
    (status, notes.join("; "))
}

/// Removes the capabilities of a pattern; each one removed goes into the backup the way the
/// apply script names it, so `Undo-Apply.ps1` lists it as not restorable.
fn remove_capabilities(pattern: &str, backup: &mut Vec<BackupEntry>) -> (&'static str, String) {
    let done = match crate::dism::remove_capabilities(pattern) {
        Ok(done) => done,
        Err(e) => return ("failed", e),
    };
    for name in &done.removed {
        backup.push(BackupEntry {
            kind: "capability-removed".to_owned(),
            path: String::new(),
            name: name.clone(),
            existed: true,
            value_type: None,
            value: Value::Null,
        });
    }
    let mut notes = Vec::new();
    if done.removed.is_empty() && done.failed.is_empty() {
        notes.push("none is installed".to_owned());
    }
    if !done.removed.is_empty() {
        notes.push(format!("removed {}", done.removed.join(", ")));
    }
    if done.restart {
        notes.push("Windows finishes it after a restart".to_owned());
    }
    notes.extend(done.failed.iter().cloned());
    let status = if done.failed.is_empty() {
        "ok"
    } else {
        "failed"
    };
    (status, notes.join("; "))
}

/// Runs a plan: every registry value is saved to `backup-<stamp>.json` in `out_dir` before its
/// first change. Programs and unknown services are reported, never fatal. The hive of the default
/// user profile is loaded for the steps that write it and unloaded after the last step.
pub fn execute(plan: &Plan, out_dir: &Path, stamp: &str) -> Report {
    let machine = NativeMachine;
    let mut backup: Vec<BackupEntry> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut remember = |path: &str, name: &str, backup: &mut Vec<BackupEntry>| {
        if seen.insert((path.to_lowercase(), name.to_lowercase())) {
            backup.push(snapshot(&machine, path, name));
        }
    };
    let mut report = Report {
        purpose: plan.purpose.clone(),
        ..Report::default()
    };
    let hive = plan.uses_default_user().then(DefaultUser::mount);
    for step in &plan.steps {
        if let Some(Err(why)) = &hive
            && step.op.in_default_user()
        {
            report.results.push(StepResult {
                rule: step.rule.clone(),
                what: step.op.describe(),
                status: "failed".to_owned(),
                note: why.clone(),
            });
            continue;
        }
        let (status, note) = match &step.op {
            Op::SetValue {
                path,
                name,
                kind,
                data,
            } => {
                remember(path, name, &mut backup);
                match write_value(path, name, *kind, data) {
                    Ok(()) => ("ok", String::new()),
                    Err(e) => ("failed", e),
                }
            }
            Op::DeleteValue { path, name } => {
                remember(path, name, &mut backup);
                match delete_value(path, name) {
                    Ok(true) => ("ok", String::new()),
                    Ok(false) => ("ok", "it did not exist".to_owned()),
                    Err(e) => ("failed", e),
                }
            }
            Op::SetList {
                path,
                kind,
                names,
                values,
                additive,
            } => match value_names(path) {
                Err(e) => ("failed", e),
                Ok(present) => {
                    for n in present.iter().chain(names) {
                        remember(path, n, &mut backup);
                    }
                    let mut failed = Vec::new();
                    if !*additive {
                        for n in present
                            .iter()
                            .filter(|n| !names.iter().any(|x| x.eq_ignore_ascii_case(n)))
                        {
                            if let Err(e) = delete_value(path, n) {
                                failed.push(e);
                            }
                        }
                    }
                    for (n, v) in names.iter().zip(values) {
                        if let Err(e) = write_value(path, n, *kind, &Value::from(v.as_str())) {
                            failed.push(e);
                        }
                    }
                    if failed.is_empty() {
                        ("ok", String::new())
                    } else {
                        ("failed", failed.join("; "))
                    }
                }
            },
            Op::ServiceStart { name, start } => {
                let key = apply::service_key(name);
                match machine.value(&key, "Start") {
                    Ok(Some(_)) => {
                        remember(&key, "Start", &mut backup);
                        match write_value(&key, "Start", RegKind::DWord, &Value::from(*start)) {
                            Ok(()) => ("ok", String::new()),
                            Err(e) => ("failed", e),
                        }
                    }
                    Ok(None) => ("skipped", "the service is not present".to_owned()),
                    Err(e) => ("failed", e),
                }
            }
            Op::Run { file, args } => match run(file, args) {
                Ok(note) => ("ok", note),
                Err(e) => ("failed", e),
            },
            Op::RemoveCapability { pattern } => remove_capabilities(pattern, &mut backup),
            Op::RemoveApps { names } => remove_apps(names, &mut backup),
            Op::RunFromMedia {
                folder,
                pattern,
                deep,
                file,
                args,
            } => match media_folder(folder, pattern, *deep) {
                None => ("skipped", format!("no drive has {folder} with {pattern}")),
                Some(source) => {
                    let source = source.display().to_string();
                    let args: Vec<String> = args
                        .iter()
                        .map(|a| a.replace("{source}", &source))
                        .collect();
                    match run(file, &args) {
                        Ok(note) => ("ok", note),
                        Err(e) => ("failed", e),
                    }
                }
            },
            Op::ServiceStartFrom { name, from, start } => {
                let key = apply::service_key(name);
                match machine.value(&key, "Start") {
                    Ok(Some(RegData::Dword(now))) if now == *from => {
                        remember(&key, "Start", &mut backup);
                        match write_value(&key, "Start", RegKind::DWord, &Value::from(*start)) {
                            Ok(()) => ("ok", String::new()),
                            Err(e) => ("failed", e),
                        }
                    }
                    Ok(Some(_)) => ("ok", format!("the start type is not {from}, unchanged")),
                    Ok(None) => ("skipped", "the service is not present".to_owned()),
                    Err(e) => ("failed", e),
                }
            }
            Op::SetValueInSubkeys {
                path,
                name,
                kind,
                data,
            } => match subkey_names(path) {
                Err(e) => ("failed", e),
                Ok(subs) if subs.is_empty() => ("skipped", "the key has no subkeys".to_owned()),
                Ok(subs) => {
                    let mut failed = Vec::new();
                    for sub in &subs {
                        let at = format!("{path}\\{sub}");
                        remember(&at, name, &mut backup);
                        if let Err(e) = write_value(&at, name, *kind, data) {
                            failed.push(e);
                        }
                    }
                    let note = format!("{} subkeys", subs.len());
                    outcome(!failed.is_empty(), [vec![note], failed].concat())
                }
            },
            Op::DeleteKey { path } => {
                let mut found = Vec::new();
                match values_under(path, &mut found, 0) {
                    Err(e) => ("failed", e),
                    Ok(()) => {
                        for (at, name) in &found {
                            remember(at, name, &mut backup);
                        }
                        match delete_key(path) {
                            Ok(true) => ("ok", format!("{} values backed up", found.len())),
                            Ok(false) => ("ok", "it did not exist".to_owned()),
                            Err(e) => ("failed", e),
                        }
                    }
                }
            }
            Op::PasswordNeverExpires { accounts } => never_expires(accounts, &mut backup),
            Op::DisableBuiltinAccounts => disable_builtin(&mut backup),
            Op::CollectErrors { file } => collect_errors(file),
            Op::DeleteFiles { paths } => delete_files(paths, &mut backup),
        };
        report.results.push(StepResult {
            rule: step.rule.clone(),
            what: step.op.describe(),
            status: status.to_owned(),
            note,
        });
    }
    if let Some(Ok(hive)) = hive
        && let Err(e) = hive.unmount()
    {
        report.results.push(StepResult {
            rule: "default-user".to_owned(),
            what: "the unload of the default user profile".to_owned(),
            status: "failed".to_owned(),
            note: e,
        });
    }
    if !backup.is_empty() {
        let name = format!("backup-{stamp}.json");
        let text = serde_json::to_string_pretty(&backup).unwrap_or_else(|_| "[]".to_owned());
        match std::fs::create_dir_all(out_dir)
            .and_then(|()| std::fs::write(out_dir.join(&name), text))
        {
            Ok(()) => report.backup = name,
            Err(e) => report.error = format!("the backup was not written: {e}"),
        }
    }
    report
}

/// Starts the executable as administrator (Windows asks the user) with the arguments, hidden,
/// and waits for it; the exit code, or an error when the user declined.
pub fn run_elevated(exe: &Path, args: &[String]) -> Result<u32, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, INFINITE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
    let params = args
        .iter()
        .map(|a| format!("\"{}\"", a.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ");
    let verb = wide("runas");
    let file = wide(&exe.display().to_string());
    let params = wide(&params);
    // SAFETY: a zeroed SHELLEXECUTEINFOW is a valid empty structure; the strings live until the
    // call returns and the process handle is closed below.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = SW_HIDE;
    // SAFETY: the structure is filled as the function expects.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        // SAFETY: reads the error of the call above.
        let code = unsafe { GetLastError() };
        return Err(if code == 1223 {
            "the permission was declined".to_owned()
        } else {
            format!("the helper did not start (error {code})")
        });
    }
    let mut exit = 0u32;
    // SAFETY: the handle is the process of the call above, waited for and closed once.
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        GetExitCodeProcess(info.hProcess, &mut exit);
        CloseHandle(info.hProcess);
    }
    Ok(exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use winweaver_core::apply::{Step, plan_undo};

    #[test]
    fn a_plan_writes_backs_up_and_undoes_in_a_test_key() {
        let key = format!("HKCU:\\Software\\WinWeaverTest-{}", std::process::id());
        let step = |op| Step {
            rule: "test".to_owned(),
            op,
        };
        let plan = Plan {
            format: apply::PLAN_FORMAT.to_owned(),
            version: apply::PLAN_VERSION,
            nonce: "n".to_owned(),
            purpose: "apply".to_owned(),
            steps: vec![
                step(Op::SetValue {
                    path: key.clone(),
                    name: "D".into(),
                    kind: RegKind::DWord,
                    data: Value::from(7),
                }),
                step(Op::SetValue {
                    path: key.clone(),
                    name: "S".into(),
                    kind: RegKind::String,
                    data: Value::from("text"),
                }),
                step(Op::SetValue {
                    path: key.clone(),
                    name: "M".into(),
                    kind: RegKind::MultiString,
                    data: serde_json::json!(["a", "b"]),
                }),
                step(Op::SetValue {
                    path: key.clone(),
                    name: "B".into(),
                    kind: RegKind::Binary,
                    data: serde_json::json!([1, 2, 255]),
                }),
                step(Op::SetValue {
                    path: key.clone(),
                    name: "Q".into(),
                    kind: RegKind::QWord,
                    data: Value::from("5"),
                }),
                step(Op::DeleteValue {
                    path: key.clone(),
                    name: "Absent".into(),
                }),
                step(Op::SetValue {
                    path: format!("{key}\\L"),
                    name: "old".into(),
                    kind: RegKind::String,
                    data: Value::from("o"),
                }),
                step(Op::SetList {
                    path: format!("{key}\\L"),
                    kind: RegKind::String,
                    names: vec!["1".into(), "2".into()],
                    values: vec!["a".into(), "b".into()],
                    additive: false,
                }),
                step(Op::ServiceStart {
                    name: "WinWeaverNoSuchService".into(),
                    start: 4,
                }),
                step(Op::Run {
                    file: "hostname.exe".into(),
                    args: vec![],
                }),
            ],
            skipped: vec![],
        };
        let out = std::env::temp_dir().join(format!("winweaver-apply-{}", std::process::id()));
        let report = execute(&plan, &out, "test");
        assert_eq!(report.count("failed"), 0, "{report:?}");
        assert_eq!(report.count("skipped"), 1, "the missing service is skipped");
        let m = NativeMachine;
        assert_eq!(m.value(&key, "D"), Ok(Some(RegData::Dword(7))));
        assert_eq!(m.value(&key, "S"), Ok(Some(RegData::Str("text".into()))));
        assert_eq!(
            m.value(&key, "M"),
            Ok(Some(RegData::Multi(vec!["a".into(), "b".into()])))
        );
        assert_eq!(
            m.value(&key, "B"),
            Ok(Some(RegData::Binary(vec![1, 2, 255])))
        );
        assert_eq!(m.value(&key, "Q"), Ok(Some(RegData::Qword(5))));
        let list = format!("{key}\\L");
        assert_eq!(
            m.value(&list, "old"),
            Ok(None),
            "the list deletes the other values"
        );
        assert_eq!(m.value(&list, "2"), Ok(Some(RegData::Str("b".into()))));
        let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
        assert_eq!(entries.len(), 9);
        assert!(entries.iter().all(|e| !e.existed));
        let undo = execute(&plan_undo(&entries, "n"), &out, "undo");
        assert_eq!(undo.count("failed"), 0, "{undo:?}");
        assert_eq!(m.value(&key, "D"), Ok(None));
        let (root, sub) = split(&key).unwrap();
        root.delete_subkey_all(&sub).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
        assert!(system_program("..\\x.exe").is_err() && system_program("cmd").is_err());
    }

    #[test]
    fn the_default_user_profile_is_loaded_written_and_unloaded() {
        // It writes the default user profile: only on the build machine of CI, as administrator.
        if std::env::var_os("GITHUB_ACTIONS").is_none() || !crate::native::elevated() {
            return;
        }
        let users = RegKey::predef(HKEY_USERS);
        let loaded_before = users.open_subkey(default_user_subkey()).is_ok();
        let key = format!(
            "{}\\Software\\WinWeaverTest-{}",
            apply::DEFAULT_USER_KEY,
            std::process::id()
        );
        let plan = Plan {
            format: apply::PLAN_FORMAT.to_owned(),
            version: apply::PLAN_VERSION,
            nonce: "n".to_owned(),
            purpose: "apply".to_owned(),
            steps: vec![Step {
                rule: "test".to_owned(),
                op: Op::SetValue {
                    path: key.clone(),
                    name: "D".into(),
                    kind: RegKind::DWord,
                    data: Value::from(7),
                },
            }],
            skipped: vec![],
        };
        assert!(plan.uses_default_user());
        let out = std::env::temp_dir().join(format!("winweaver-du-{}", std::process::id()));
        let report = execute(&plan, &out, "du");
        assert_eq!(report.count("failed"), 0, "{report:?}");
        assert_eq!(
            users.open_subkey(default_user_subkey()).is_ok(),
            loaded_before,
            "the hive is unloaded again"
        );
        let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, key);
        let undo = execute(&plan_undo(&entries, "n"), &out, "du-undo");
        assert_eq!(undo.count("failed"), 0, "{undo:?}");
        let hive = DefaultUser::mount().unwrap();
        assert_eq!(NativeMachine.value(&key, "D"), Ok(None));
        let sub = key.strip_prefix("HKU:\\").unwrap();
        users.delete_subkey_all(sub).unwrap();
        hive.unmount().unwrap();
        std::fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn the_native_script_steps_work_and_undo_in_a_test_key() {
        let key = format!("HKCU:\\Software\\WinWeaverSteps-{}", std::process::id());
        let (one, two, deep) = (
            format!("{key}\\A\\One"),
            format!("{key}\\A\\Two"),
            format!("{key}\\Gone\\Deep"),
        );
        for path in [&one, &two, &deep] {
            write_value(path, "x", RegKind::String, &Value::from("old")).unwrap();
        }
        let out = std::env::temp_dir().join(format!("winweaver-steps-{}", std::process::id()));
        std::fs::create_dir_all(&out).unwrap();
        let kept = out.join("unattend.xml");
        std::fs::write(&kept, "kept").unwrap();
        let step = |op| Step {
            rule: "test".to_owned(),
            op,
        };
        let plan = Plan {
            format: apply::PLAN_FORMAT.to_owned(),
            version: apply::PLAN_VERSION,
            nonce: "n".to_owned(),
            purpose: "apply".to_owned(),
            steps: vec![
                step(Op::SetValueInSubkeys {
                    path: format!("{key}\\A"),
                    name: "N".into(),
                    kind: RegKind::DWord,
                    data: Value::from(2),
                }),
                step(Op::DeleteKey {
                    path: format!("{key}\\Gone"),
                }),
                step(Op::ServiceStartFrom {
                    name: "WinWeaverNoSuchService".into(),
                    from: 4,
                    start: 3,
                }),
                step(Op::RunFromMedia {
                    folder: "WinWeaverNoSuchFolder".into(),
                    pattern: "*.cab".into(),
                    deep: false,
                    file: "hostname.exe".into(),
                    args: vec![],
                }),
                // A file that is not a copy of the answer file is never deleted.
                step(Op::DeleteFiles {
                    paths: vec![kept.display().to_string()],
                }),
            ],
            skipped: vec![],
        };
        let report = execute(&plan, &out, "steps");
        assert_eq!(report.count("skipped"), 2, "{report:?}");
        assert_eq!(report.count("failed"), 1, "{report:?}");
        assert!(kept.exists());
        let m = NativeMachine;
        assert_eq!(m.value(&one, "N"), Ok(Some(RegData::Dword(2))));
        assert_eq!(m.value(&two, "N"), Ok(Some(RegData::Dword(2))));
        assert_eq!(m.value(&deep, "x"), Ok(None), "the key is deleted");
        // The undo makes the deleted key again with its value and removes the new values.
        let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
        let undo = execute(&plan_undo(&entries, "n"), &out, "steps-undo");
        assert_eq!(undo.count("failed"), 0, "{undo:?}");
        assert_eq!(m.value(&deep, "x"), Ok(Some(RegData::Str("old".into()))));
        assert_eq!(m.value(&one, "N"), Ok(None));
        let (root, sub) = split(&key).unwrap();
        root.delete_subkey_all(&sub).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
    }
}
