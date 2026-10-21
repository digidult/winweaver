//! WinWeaver core: the rule catalog, its translations, profiles and the resolver.
//!
//! Pure Rust without any Windows API, so it compiles and is tested on every platform.
//! The specification is in `docs/spec/` of the repository.

pub mod admx;
pub mod apply;
pub mod bookmarks;
pub mod catalog;
pub mod diff;
pub mod i18n;
pub mod import;
pub mod profile;
pub mod regfile;
pub mod registry;
pub mod render;
pub mod resolver;
pub mod resources;
pub mod scripts;
pub mod security;
pub mod thispc;
pub mod users;
pub mod userstore;
pub mod validate;

pub use ww_base::{clock, jsonfile, paths};

/// The version of the program, taken from the workspace: the second number grows with a feature,
/// the third with a fix.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The commit the build is made from, 12 hexadecimal digits or "unknown" (`build.rs`).
pub const COMMIT: &str = env!("WINWEAVER_COMMIT");

/// The version with the commit as its build metadata (`0.2.0+1a2b3c4d5e6f`): what `--version`, About,
/// the logs, the MCP server, the answer file and the scripts name.
pub const BUILD: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("WINWEAVER_COMMIT"));
