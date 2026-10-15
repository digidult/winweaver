//! WinWeaver core: the rule catalog, its translations, profiles and the resolver.
//!
//! Pure Rust without any Windows API, so it compiles and is tested on every platform.
//! The specification is in `docs/spec/` of the repository.

pub mod admx;
pub mod apply;
pub mod bookmarks;
pub mod catalog;
pub mod clock;
pub mod diff;
pub mod i18n;
pub mod import;
pub mod jsonfile;
pub mod profile;
pub mod render;
pub mod resolver;
pub mod resources;
pub mod scripts;
pub mod thispc;
pub mod validate;

/// The version of the program, taken from the workspace.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
