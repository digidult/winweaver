//! The methods of the window by theme: one file, one impl block of `WinWeaver` each. `shell`
//! is the state of the window (`ww::Shell`): the tabs, the settings, the status.

#[cfg(feature = "full")]
mod diff;
mod files;
mod frame;
#[cfg(feature = "full")]
mod imports;
#[cfg(feature = "full")]
mod marks;
#[cfg(feature = "mcp")]
mod mcp;
mod menu;
#[cfg(feature = "full")]
mod pc;
#[cfg(feature = "full")]
mod pol;
mod profile;
#[cfg(feature = "full")]
mod registry;
mod session;
mod settings;
mod shell;
#[cfg(feature = "full")]
mod users;
#[cfg(all(feature = "full", target_arch = "wasm32"))]
pub(crate) mod web;

#[cfg(feature = "full")]
pub(crate) use session::show_single;
