//! The operations of a plan, one function each: the programs of the system folder and of the media, services, keys, accounts, apps, capabilities and files.

use super::*;

/// The names of the values of a key, `(Default)` for the default value; none when the key does
/// not exist.
pub(crate) fn value_names(path: &str) -> Result<Vec<String>, String> {
    let (root, sub) = hive_path(path)?;
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
    Ok(system_root().join("System32").join(file))
}

pub(crate) fn run(file: &str, args: &[String]) -> Result<String, String> {
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
pub(crate) fn subkey_names(path: &str) -> Result<Vec<String>, String> {
    let (root, sub) = hive_path(path)?;
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
pub(crate) fn values_under(
    path: &str,
    out: &mut Vec<(String, String)>,
    depth: usize,
) -> Result<(), String> {
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
pub(crate) fn delete_key(path: &str) -> Result<bool, String> {
    let (root, sub) = hive_path(path)?;
    match root.delete_subkey_all(&sub) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Whether a folder holds a file of an extension, in its subfolders too when `deep`; at most
/// 100000 entries are looked at.
pub(crate) fn has_file(dir: &Path, extension: &str, deep: bool, seen: &mut usize) -> bool {
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
pub(crate) fn media_folder(folder: &str, pattern: &str, deep: bool) -> Option<PathBuf> {
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

/// Sets "password never expires" on the accounts of the profile that exist.
pub(crate) fn never_expires(accounts: &[String], backup: &mut Backup) -> Outcome {
    let mut notes = Vec::new();
    let mut failed = false;
    if accounts.is_empty() {
        notes.push("the profile has no accounts".to_owned());
    }
    for name in accounts {
        match crate::accounts::set_flag(name, crate::accounts::NEVER_EXPIRES, true) {
            Ok(Some(true)) => {
                let what = format!("{name}: password never expires");
                backup.entries.push(named("account-changed", &what));
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
pub(crate) fn disable_builtin(backup: &mut Backup) -> Outcome {
    let found = match crate::accounts::builtin() {
        Ok(found) => found,
        Err(e) => return ("failed", e),
    };
    let me = user_name();
    let mut notes = Vec::new();
    let mut failed = false;
    for name in found {
        if name.eq_ignore_ascii_case(&me) {
            notes.push(format!("{name}: left on, it is the account in use"));
            continue;
        }
        match crate::accounts::set_flag(&name, crate::accounts::DISABLED, true) {
            Ok(Some(true)) => {
                backup
                    .entries
                    .push(named("account-changed", &format!("{name}: disabled")));
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
pub(crate) fn collect_errors(file: &str) -> Outcome {
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
pub(crate) fn delete_files(paths: &[String], backup: &mut Backup) -> Outcome {
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
                backup.entries.push(named("file-deleted", path));
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
pub(crate) fn remove_apps(names: &[String], backup: &mut Backup) -> Outcome {
    let done = match crate::apps::remove_apps(names) {
        Ok(done) => done,
        Err(e) => return ("failed", e),
    };
    for name in &done.found {
        backup.entries.push(named("app-removed", name));
    }
    let mut notes = Vec::new();
    if done.found.is_empty() {
        notes.push("none is installed or provisioned".to_owned());
    }
    notes.extend(done.done.iter().cloned());
    notes.extend(done.failed.iter().cloned());
    outcome(!done.failed.is_empty(), notes)
}

/// Removes the capabilities of a pattern; each one removed goes into the backup the way the
/// apply script names it, so `Undo-Apply.ps1` lists it as not restorable.
pub(crate) fn remove_capabilities(pattern: &str, backup: &mut Backup) -> Outcome {
    let done = match crate::dism::remove_capabilities(pattern) {
        Ok(done) => done,
        Err(e) => return ("failed", e),
    };
    for name in &done.removed {
        backup.entries.push(named("capability-removed", name));
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
    outcome(!done.failed.is_empty(), notes)
}

/// Writes the values of a list key; without `additive` the other values of the key go first.
pub(crate) fn set_list(
    backup: &mut Backup,
    path: &str,
    kind: RegKind,
    names: &[String],
    values: &[String],
    additive: bool,
) -> Outcome {
    let present = match value_names(path) {
        Ok(present) => present,
        Err(e) => return ("failed", e),
    };
    for n in present.iter().chain(names) {
        backup.remember(path, n);
    }
    let mut failed = Vec::new();
    if !additive {
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
        if let Err(e) = write_value(path, n, kind, &Value::from(v.as_str())) {
            failed.push(e);
        }
    }
    if failed.is_empty() {
        ("ok", String::new())
    } else {
        ("failed", failed.join("; "))
    }
}

/// Sets the start type of a service that is present; with `only_from`, only when its start type
/// is that one now.
pub(crate) fn service_start(
    backup: &mut Backup,
    name: &str,
    start: u32,
    only_from: Option<u32>,
) -> Outcome {
    let key = apply::service_key(name);
    match NativeMachine.value(&key, "Start") {
        Ok(Some(now)) => {
            if let Some(from) = only_from
                && !matches!(now, Data::Dword(n) if n == from)
            {
                return ("ok", format!("the start type is not {from}, unchanged"));
            }
            backup.remember(&key, "Start");
            done(write_value(
                &key,
                "Start",
                RegKind::DWord,
                &Value::from(start),
            ))
        }
        Ok(None) => ("skipped", "the service is not present".to_owned()),
        Err(e) => ("failed", e),
    }
}

/// Runs a program of the system folder with the folder of the installation media in its arguments.
pub(crate) fn run_from_media(
    folder: &str,
    pattern: &str,
    deep: bool,
    file: &str,
    args: &[String],
) -> Outcome {
    let Some(source) = media_folder(folder, pattern, deep) else {
        return ("skipped", format!("no drive has {folder} with {pattern}"));
    };
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

/// Writes a value into every subkey of a key.
pub(crate) fn set_in_subkeys(
    backup: &mut Backup,
    path: &str,
    name: &str,
    kind: RegKind,
    data: &Value,
) -> Outcome {
    let subs = match subkey_names(path) {
        Ok(subs) => subs,
        Err(e) => return ("failed", e),
    };
    if subs.is_empty() {
        return ("skipped", "the key has no subkeys".to_owned());
    }
    let mut failed = Vec::new();
    for sub in &subs {
        let at = format!("{path}\\{sub}");
        backup.remember(&at, name);
        if let Err(e) = write_value(&at, name, kind, data) {
            failed.push(e);
        }
    }
    let note = format!("{} subkeys", subs.len());
    outcome(!failed.is_empty(), [vec![note], failed].concat())
}

/// Deletes a key with everything under it, every value of it backed up first.
pub(crate) fn remove_key(backup: &mut Backup, path: &str) -> Outcome {
    let mut found = Vec::new();
    if let Err(e) = values_under(path, &mut found, 0) {
        return ("failed", e);
    }
    for (at, name) in &found {
        backup.remember(at, name);
    }
    match delete_key(path) {
        Ok(true) => ("ok", format!("{} values backed up", found.len())),
        Ok(false) => ("ok", "it did not exist".to_owned()),
        Err(e) => ("failed", e),
    }
}
