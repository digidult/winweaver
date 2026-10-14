//! WinWeaver on Windows: the registry and the system read through the Windows API, behind the
//! trait `winweaver_core::thispc::Machine` (`docs/spec/04-architecture.md` section 5), the
//! executor of the elevated helper, and the colours of the title bar of the window.

use winweaver_core::thispc::Machine;

#[cfg(windows)]
mod execute;
#[cfg(windows)]
mod native;

#[cfg(windows)]
pub use native::NativeMachine;

/// The machine of this PC, or `None` where the Windows API is not available.
pub fn this_pc() -> Option<Box<dyn Machine>> {
    #[cfg(windows)]
    {
        Some(Box::new(NativeMachine))
    }
    #[cfg(not(windows))]
    {
        None
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
