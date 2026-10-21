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
use winreg::enums::{HKEY_USERS, KEY_READ, KEY_SET_VALUE, RegType};
use winreg::{RegKey, RegValue};
use winweaver_core::apply::{self, BackupEntry, Op, Plan, Report, StepResult};
use winweaver_core::catalog::{RegKind, values};
use winweaver_core::registry::Data;
use winweaver_core::thispc::Machine;

use crate::ffi::{buffer_text, hive_path, system_root, user_name, utf16_bytes, wide};
use crate::native::NativeMachine;

mod hive;
mod ops;
#[cfg(test)]
mod tests;

pub(crate) use hive::*;
pub(crate) use ops::*;

/// What a step did: its status (`ok`, `failed`, `skipped`) and its note.
pub(crate) type Outcome = (&'static str, String);

pub(crate) fn number(data: &Value) -> Result<i64, String> {
    values::number(data).ok_or_else(|| format!("{} is not a number", values::text(data)))
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
        RegKind::String => (utf16_bytes(&values::text(data)), RegType::REG_SZ),
        RegKind::ExpandString => (utf16_bytes(&values::text(data)), RegType::REG_EXPAND_SZ),
        RegKind::MultiString => {
            let mut bytes = Vec::new();
            for item in values::multi_items(data) {
                bytes.extend(utf16_bytes(&item));
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
pub(crate) fn snapshot(path: &str, name: &str) -> BackupEntry {
    let (existed, value_type, value) = match NativeMachine.value(path, name) {
        Ok(Some(data)) => {
            let (t, v) = match data {
                Data::Dword(v) => (Some("DWord"), Value::from(v as i32)),
                Data::Qword(v) => (Some("QWord"), Value::from(v as i64)),
                Data::Sz(s) => (Some("String"), Value::from(s)),
                Data::ExpandSz(s) => (Some("ExpandString"), Value::from(s)),
                Data::MultiSz(items) => (Some("MultiString"), Value::from(items)),
                Data::Binary(bytes) => (
                    Some("Binary"),
                    Value::from(bytes.into_iter().map(u64::from).collect::<Vec<_>>()),
                ),
                Data::None(_) | Data::Other { .. } => (None, Value::Null),
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

pub(crate) fn write_value(
    path: &str,
    name: &str,
    kind: RegKind,
    data: &Value,
) -> Result<(), String> {
    let value = encode(kind, data)?;
    let (root, sub) = hive_path(path)?;
    let (key, _) = root.create_subkey(&sub).map_err(|e| e.to_string())?;
    let name = if name == "(Default)" { "" } else { name };
    key.set_raw_value(name, &value).map_err(|e| e.to_string())
}

pub(crate) fn delete_value(path: &str, name: &str) -> Result<bool, String> {
    let (root, sub) = hive_path(path)?;
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

/// An entry of the backup that names a change no undo takes back, as the apply script names them.
pub(crate) fn named(kind: &str, name: &str) -> BackupEntry {
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
pub(crate) fn outcome(failed: bool, notes: Vec<String>) -> Outcome {
    let status = if failed { "failed" } else { "ok" };
    (status, notes.join("; "))
}

/// The outcome of a step that did one thing.
pub(crate) fn done(result: Result<(), String>) -> Outcome {
    match result {
        Ok(()) => ("ok", String::new()),
        Err(e) => ("failed", e),
    }
}

/// The backup of a run: every registry value saved before its first change, and the changes no
/// undo takes back, named.
#[derive(Default)]
pub(crate) struct Backup {
    pub(crate) entries: Vec<BackupEntry>,
    pub(crate) seen: HashSet<(String, String)>,
}

impl Backup {
    /// Saves a value as it is now, the first time it is named.
    pub(crate) fn remember(&mut self, path: &str, name: &str) {
        if self.seen.insert((path.to_lowercase(), name.to_lowercase())) {
            self.entries.push(snapshot(path, name));
        }
    }
}

/// One step of a plan.
fn step(backup: &mut Backup, op: &Op) -> Outcome {
    match op {
        Op::SetValue {
            path,
            name,
            kind,
            data,
        } => {
            backup.remember(path, name);
            done(write_value(path, name, *kind, data))
        }
        Op::DeleteValue { path, name } => {
            backup.remember(path, name);
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
        } => set_list(backup, path, *kind, names, values, *additive),
        Op::ServiceStart { name, start } => service_start(backup, name, *start, None),
        Op::ServiceStartFrom { name, from, start } => {
            service_start(backup, name, *start, Some(*from))
        }
        Op::Run { file, args } => match run(file, args) {
            Ok(note) => ("ok", note),
            Err(e) => ("failed", e),
        },
        Op::RemoveCapability { pattern } => remove_capabilities(pattern, backup),
        Op::RemoveApps { names } => remove_apps(names, backup),
        Op::RunFromMedia {
            folder,
            pattern,
            deep,
            file,
            args,
        } => run_from_media(folder, pattern, *deep, file, args),
        Op::SetValueInSubkeys {
            path,
            name,
            kind,
            data,
        } => set_in_subkeys(backup, path, name, *kind, data),
        Op::DeleteKey { path } => remove_key(backup, path),
        Op::PasswordNeverExpires { accounts } => never_expires(accounts, backup),
        Op::DisableBuiltinAccounts => disable_builtin(backup),
        Op::CollectErrors { file } => collect_errors(file),
        Op::DeleteFiles { paths } => delete_files(paths, backup),
    }
}

/// Runs a plan: every registry value is saved to `backup-<stamp>.json` in `out_dir` before its
/// first change. Programs and unknown services are reported, never fatal. The hive of the default
/// user profile is loaded for the steps that write it and unloaded after the last step.
pub fn execute(plan: &Plan, out_dir: &Path, stamp: &str) -> Report {
    let mut backup = Backup::default();
    let mut report = Report {
        purpose: plan.purpose.clone(),
        ..Report::default()
    };
    let hive = plan.uses_default_user().then(DefaultUser::mount);
    for item in &plan.steps {
        let (status, note) = match &hive {
            Some(Err(why)) if item.op.in_default_user() => ("failed", why.clone()),
            _ => step(&mut backup, &item.op),
        };
        report.results.push(StepResult {
            rule: item.rule.clone(),
            what: item.op.describe(),
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
    if !backup.entries.is_empty() {
        let name = format!("backup-{stamp}.json");
        let text =
            serde_json::to_string_pretty(&backup.entries).unwrap_or_else(|_| "[]".to_owned());
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
