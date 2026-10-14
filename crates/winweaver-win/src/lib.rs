//! WinWeaver on Windows: the registry and the system read through the Windows API, behind the
//! trait `winweaver_core::thispc::Machine` (`docs/spec/04-architecture.md` section 5). This crate
//! only reads; the elevated apply comes with a later milestone.

use winweaver_core::thispc::Machine;

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
