//! wwrt: the runtime the programs with plugins share (docs/plugins.md). One DLL holds std, egui,
//! eframe, W# and the interface of the plugins; the executable and every plugin link it instead of
//! copies of their own, so a plugin is its own code alone and draws in the window of the program
//! with the egui of the program.
//!
//! A program or a plugin depends on wwrt alone and takes every crate wwrt holds through it
//! (`wwrt::ww`, `wwrt::egui`, `wwrt::serde`, `wwrt::windows`): a dependency of its own on one of
//! them would be a second copy of that crate, which rustc refuses.

pub use serde;
pub use serde_json;
#[cfg(windows)]
pub use windows;
pub use ww;
pub use ww::eframe;
pub use ww::eframe::egui;

pub mod blocks;
pub mod plugin;
