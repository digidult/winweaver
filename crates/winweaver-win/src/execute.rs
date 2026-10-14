//! The executor of an apply plan, run by the elevated helper: registry values, service start types
//! and programs of the system folder, each change backed up first. It never runs a script.

use std::borrow::Cow;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, KEY_SET_VALUE, RegType};
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
    text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
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
        RegKind::DWord => ((number(data)? as u32).to_le_bytes().to_vec(), RegType::REG_DWORD),
        RegKind::QWord => ((number(data)? as u64).to_le_bytes().to_vec(), RegType::REG_QWORD),
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
                .map(|v| number(v).and_then(|n| u8::try_from(n).map_err(|_| format!("{n} is not a byte"))))
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
                RegData::Binary(bytes) => (Some("Binary"), Value::from(bytes.into_iter().map(u64::from).collect::<Vec<_>>())),
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

/// The program of the system folder a plan may run: a bare file name of letters, digits, dots,
/// hyphens and underscores that ends with `.exe`.
pub fn system_program(file: &str) -> Result<PathBuf, String> {
    let ok = file.to_ascii_lowercase().ends_with(".exe")
        && file.len() <= 64
        && file.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
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
            Err(format!("exit {}: {first}", code.map_or("none".to_owned(), |c| c.to_string())))
        }
    }
}

fn describe(op: &Op) -> String {
    match op {
        Op::SetValue { path, name, data, .. } => format!("{path}\\{name} = {data}"),
        Op::DeleteValue { path, name } => format!("{path}\\{name} removed"),
        Op::ServiceStart { name, start } => format!("service {name} start type {start}"),
        Op::Run { file, args } => format!("{file} {}", args.join(" ")),
    }
}

/// Runs a plan: every registry value is saved to `backup-<stamp>.json` in `out_dir` before its
/// first change. Programs and unknown services are reported, never fatal.
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
    for step in &plan.steps {
        let (status, note) = match &step.op {
            Op::SetValue { path, name, kind, data } => {
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
        };
        report.results.push(StepResult {
            rule: step.rule.clone(),
            what: describe(&step.op),
            status: status.to_owned(),
            note,
        });
    }
    if !backup.is_empty() {
        let name = format!("backup-{stamp}.json");
        let text = serde_json::to_string_pretty(&backup).unwrap_or_else(|_| "[]".to_owned());
        match std::fs::create_dir_all(out_dir).and_then(|()| std::fs::write(out_dir.join(&name), text)) {
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
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
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
        let step = |op| Step { rule: "test".to_owned(), op };
        let plan = Plan {
            format: apply::PLAN_FORMAT.to_owned(),
            version: apply::PLAN_VERSION,
            nonce: "n".to_owned(),
            purpose: "apply".to_owned(),
            steps: vec![
                step(Op::SetValue { path: key.clone(), name: "D".into(), kind: RegKind::DWord, data: Value::from(7) }),
                step(Op::SetValue { path: key.clone(), name: "S".into(), kind: RegKind::String, data: Value::from("text") }),
                step(Op::SetValue { path: key.clone(), name: "M".into(), kind: RegKind::MultiString, data: serde_json::json!(["a", "b"]) }),
                step(Op::SetValue { path: key.clone(), name: "B".into(), kind: RegKind::Binary, data: serde_json::json!([1, 2, 255]) }),
                step(Op::SetValue { path: key.clone(), name: "Q".into(), kind: RegKind::QWord, data: Value::from("5") }),
                step(Op::DeleteValue { path: key.clone(), name: "Absent".into() }),
                step(Op::ServiceStart { name: "WinWeaverNoSuchService".into(), start: 4 }),
                step(Op::Run { file: "hostname.exe".into(), args: vec![] }),
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
        assert_eq!(m.value(&key, "M"), Ok(Some(RegData::Multi(vec!["a".into(), "b".into()]))));
        assert_eq!(m.value(&key, "B"), Ok(Some(RegData::Binary(vec![1, 2, 255]))));
        assert_eq!(m.value(&key, "Q"), Ok(Some(RegData::Qword(5))));
        let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
        assert_eq!(entries.len(), 6);
        assert!(entries.iter().all(|e| !e.existed));
        let undo = execute(&plan_undo(&entries, "n"), &out, "undo");
        assert_eq!(undo.count("failed"), 0, "{undo:?}");
        assert_eq!(m.value(&key, "D"), Ok(None));
        let (root, sub) = split(&key).unwrap();
        root.delete_subkey_all(&sub).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
        assert!(system_program("..\\x.exe").is_err() && system_program("cmd").is_err());
    }
}
