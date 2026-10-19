//! WinWeaver on Windows: the registry and the system read through the Windows API, behind the
//! trait `winweaver_core::thispc::Machine` (`docs/spec/04-architecture.md` section 5), the
//! executor of the elevated helper, and the colours of the title bar of the window.

use winweaver_core::thispc::Machine;

#[cfg(windows)]
mod accounts;
#[cfg(windows)]
mod apps;
#[cfg(windows)]
mod dism;
#[cfg(windows)]
mod execute;
#[cfg(windows)]
mod locales;
#[cfg(windows)]
mod native;

#[cfg(windows)]
mod registry;

#[cfg(windows)]
pub use native::NativeMachine;
#[cfg(windows)]
pub use registry::NativeRegistry;

/// The registry of this PC for the registry tab and the MCP server; one in memory where the
/// Windows API is not available.
pub fn this_registry() -> std::sync::Arc<dyn winweaver_core::registry::Registry> {
    #[cfg(windows)]
    {
        std::sync::Arc::new(NativeRegistry)
    }
    #[cfg(not(windows))]
    {
        std::sync::Arc::new(winweaver_core::registry::MemoryRegistry::default())
    }
}

/// The machine of this PC, or `None` where the Windows API is not available.
pub fn this_pc() -> Option<Box<dyn Machine + Send + Sync>> {
    #[cfg(windows)]
    {
        Some(Box::new(NativeMachine))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The input languages and keyboard layouts of this Windows, the reader the window and the MCP
/// server name for `winweaver_core::resources::set_locale_reader`; none where the Windows API is
/// not available.
pub fn locales() -> winweaver_core::resources::Locales {
    #[cfg(windows)]
    {
        locales::read()
    }
    #[cfg(not(windows))]
    {
        winweaver_core::resources::Locales::default()
    }
}

/// The language of the Windows interface as a two-letter code (`uk`, `ru`, `en`), if known.
pub fn ui_language() -> Option<String> {
    #[cfg(windows)]
    {
        native::ui_language()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Runs an apply plan with the Windows API (in the elevated helper), backing up into `out_dir`.
pub fn execute_plan(
    plan: &winweaver_core::apply::Plan,
    out_dir: &std::path::Path,
    stamp: &str,
) -> winweaver_core::apply::Report {
    #[cfg(windows)]
    {
        execute::execute(plan, out_dir, stamp)
    }
    #[cfg(not(windows))]
    {
        let _ = (out_dir, stamp);
        winweaver_core::apply::Report {
            purpose: plan.purpose.clone(),
            error: "the plan runs on Windows only".to_owned(),
            ..Default::default()
        }
    }
}

/// Colours the title bar of a window of this process: the dark frame for a dark theme (Windows 10
/// and 11), and on Windows 11 the caption, its text and the border, each `[red, green, blue]`. An
/// attribute the system does not know is ignored; nothing outside the window changes.
pub fn color_title_bar(hwnd: isize, dark: bool, caption: [u8; 3], text: [u8; 3], border: [u8; 3]) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Graphics::Dwm::{
            DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
            DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
        };
        let rgb = |c: [u8; 3]| u32::from(c[0]) | (u32::from(c[1]) << 8) | (u32::from(c[2]) << 16);
        let values = [
            (DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark)),
            (DWMWA_CAPTION_COLOR, rgb(caption)),
            (DWMWA_TEXT_COLOR, rgb(text)),
            (DWMWA_BORDER_COLOR, rgb(border)),
        ];
        let window = hwnd as windows_sys::Win32::Foundation::HWND;
        for (attribute, value) in values {
            // SAFETY: the handle is a window of this process; the value is a 4-byte BOOL or
            // COLORREF that lives during the call.
            unsafe {
                DwmSetWindowAttribute(
                    window,
                    attribute as u32,
                    std::ptr::from_ref(&value).cast(),
                    4,
                );
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, dark, caption, text, border);
    }
}

/// Shows a file in its folder in Explorer, selected, or opens a folder; Explorer is started at the
/// request of the user and not waited for.
pub fn show_in_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned());
        let explorer = std::path::Path::new(&root).join("explorer.exe");
        let mut command = std::process::Command::new(explorer);
        if path.is_file() {
            command.raw_arg(format!("/select,\"{}\"", path.display()));
        } else if path.is_dir() {
            command.raw_arg(format!("\"{}\"", path.display()));
        } else {
            return Err(format!("{} does not exist", path.display()));
        }
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("folders open in Explorer on Windows only".to_owned())
    }
}

/// Starts the executable as administrator with the arguments and waits for it.
pub fn run_elevated(exe: &std::path::Path, args: &[String]) -> Result<u32, String> {
    #[cfg(windows)]
    {
        execute::run_elevated(exe, args)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, args);
        Err("elevation exists on Windows only".to_owned())
    }
}

/// Calls `report` with the code and the address of an exception that nothing handles (an access
/// violation, a stack overflow), before Windows ends the process and reports it as it always does.
/// Once per process; nothing outside Windows.
pub fn on_fatal_exception(report: fn(u32, usize)) {
    #[cfg(windows)]
    {
        use std::sync::OnceLock;
        use windows_sys::Win32::System::Diagnostics::Debug::{
            EXCEPTION_POINTERS, SetUnhandledExceptionFilter,
        };
        static REPORT: OnceLock<fn(u32, usize)> = OnceLock::new();
        if REPORT.set(report).is_err() {
            return;
        }
        unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
            // SAFETY: Windows passes the records of the exception, or a null pointer is skipped.
            let (code, address) = unsafe {
                match info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) {
                    Some(record) => (
                        record.ExceptionCode as u32,
                        record.ExceptionAddress as usize,
                    ),
                    None => (0, 0),
                }
            };
            if let Some(report) = REPORT.get() {
                report(code, address);
            }
            // EXCEPTION_CONTINUE_SEARCH: Windows goes on as without the filter.
            0
        }
        // SAFETY: the filter reads the record and writes a line; it replaces no other filter of
        // the program.
        unsafe {
            SetUnhandledExceptionFilter(Some(filter));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = report;
    }
}
