//! WinWeaver core: the rule catalog, its translations, profiles and the resolver.
//!
//! Pure Rust without any Windows API, so it compiles and is tested on every platform.
//! The specification is in `docs/spec/` of the repository.

pub mod catalog;
pub mod i18n;
pub mod jsonfile;
pub mod profile;
pub mod resolver;
pub mod resources;
pub mod validate;

/// The version of the program, taken from the workspace.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
