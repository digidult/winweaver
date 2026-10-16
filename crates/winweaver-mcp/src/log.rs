//! The log of the MCP server and the crash handlers of the process. The server writes a file of
//! the day in the logs folder next to the executable, `mcp-<date>.log`, apart from the log of the
//! window: the start and the stop of a transport, every request with its result and its time, the
//! refusals and errors of the transports. A panic, or an exception of Windows that nothing handles
//! (an access violation, a stack overflow), is written before the process ends: into the MCP log
//! from a thread of the server or a process that only serves MCP, into the log of the window
//! (`winweaver-<date>.log`) from any other thread. No token, password, key or free text of a
//! profile is ever written.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use winweaver_core::clock;

/// The logs folder and whether the process only serves MCP; set once by [`init`].
static TARGET: OnceLock<(PathBuf, bool)> = OnceLock::new();
/// One line at a time, so that lines of two threads never mix.
static WRITING: Mutex<()> = Mutex::new(());

/// The file of the MCP log of today, once [`init`] ran.
pub fn file_of_today() -> Option<PathBuf> {
    TARGET.get().map(|(dir, _)| dir.join(name("mcp")))
}

fn name(kind: &str) -> String {
    let now = clock::now_utc_iso();
    format!("{kind}-{}.log", now.get(..10).unwrap_or("unknown"))
}

/// Writes a line. A crash takes the lock only when it is free: the thread that crashed may hold
/// it.
fn append(file: &Path, level: &str, text: &str, crash: bool) {
    let guard = if crash {
        WRITING.try_lock().ok()
    } else {
        Some(WRITING.lock().unwrap_or_else(|e| e.into_inner()))
    };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    {
        let _ = writeln!(f, "[{}] [{level}] {text}", clock::now_utc_iso());
        let _ = f.flush();
    }
    drop(guard);
}

/// Appends a line to the MCP log; nothing before [`init`] (the tests).
pub fn line(level: &str, text: &str) {
    if let Some((dir, _)) = TARGET.get() {
        let text = text.replace(['\r', '\n'], " ");
        append(&dir.join(name("mcp")), level, &text, false);
    }
}

/// Appends a line to the log of the window, for what fails before the window can write it.
pub fn window_line(level: &str, text: &str) {
    if let Some((dir, _)) = TARGET.get() {
        let text = text.replace(['\r', '\n'], " ");
        append(&dir.join(name("winweaver")), level, &text, false);
    }
}

/// The log a crash of the current thread belongs to.
fn crash_file() -> Option<PathBuf> {
    let (dir, headless) = TARGET.get()?;
    let server_thread = std::thread::current()
        .name()
        .is_some_and(|n| n.starts_with("mcp"));
    let kind = if *headless || server_thread {
        "mcp"
    } else {
        "winweaver"
    };
    Some(dir.join(name(kind)))
}

/// Sets the logs folder and installs the crash handlers, once per process. `headless` is true
/// for a process that only serves MCP (`--mcp stdio` or `--mcp http`).
pub fn init(dir: &Path, headless: bool) {
    if TARGET.set((dir.to_owned(), headless)).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let place = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".to_owned());
        if let Some(file) = crash_file() {
            let trace = std::backtrace::Backtrace::force_capture().to_string();
            append(
                &file,
                "PANIC",
                &format!(
                    "thread '{}' at {place}: {} | {} | backtrace: {}",
                    thread.name().unwrap_or("unnamed"),
                    message.replace(['\r', '\n'], " "),
                    process_line(),
                    trace.replace(['\r', '\n'], " ")
                ),
                true,
            );
        }
        previous(info);
    }));
    winweaver_win::on_fatal_exception(fatal);
}

/// The program, its version and the process, for the lines of a crash.
fn process_line() -> String {
    format!(
        "WinWeaver {} process {}",
        winweaver_core::BUILD,
        std::process::id()
    )
}

/// The name of a few exception codes of Windows.
fn exception_name(code: u32) -> &'static str {
    match code {
        0xC000_0005 => "access violation",
        0xC000_00FD => "stack overflow",
        0xC000_0409 => "stack buffer overrun",
        0xC000_0374 => "heap corruption",
        0xC000_001D => "illegal instruction",
        0xC000_0094 => "integer division by zero",
        0x8000_0003 => "breakpoint",
        _ => "exception",
    }
}

/// Writes an exception of Windows that nothing handles; Windows ends the process after it.
fn fatal(code: u32, address: usize) {
    if let Some(file) = crash_file() {
        let thread = std::thread::current();
        append(
            &file,
            "FATAL",
            &format!(
                "{} 0x{code:08X} at 0x{address:X} in thread '{}' | {}",
                exception_name(code),
                thread.name().unwrap_or("unnamed"),
                process_line()
            ),
            true,
        );
    }
}

/// The text of the payload of a caught panic.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exceptions_have_names() {
        assert_eq!(exception_name(0xC000_0005), "access violation");
        assert_eq!(exception_name(0xC000_00FD), "stack overflow");
        assert_eq!(exception_name(1), "exception");
        let payload: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(panic_text(payload.as_ref()), "boom");
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        assert_eq!(panic_text(payload.as_ref()), "bang");
    }
}
