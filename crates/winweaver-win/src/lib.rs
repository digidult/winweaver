//! WinWeaver on Windows: the registry and the system read through the Windows API, behind the
//! trait `winweaver_core::thispc::Machine` (`docs/spec/04-architecture.md` section 5). This crate
//! only reads; the elevated apply comes with a later milestone.

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
