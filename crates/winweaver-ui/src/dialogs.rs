//! The file dialogs and the names of files.

use crate::*;

#[cfg(feature = "full")]
pub(crate) use ww::dialogs::{kept_name, write_file};

/// A file name stem made of a profile name.
pub fn file_stem(name: &str) -> String {
    ww::dialogs::file_stem(name, "profile")
}

/// A path the save dialog gave, with the extension of a registry file when it has none.
#[cfg(feature = "full")]
pub(crate) fn reg_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(regfile::EXTENSION)
    }
}

/// The path with the profile extension when it has none.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(profile::EXTENSION)
    }
}

#[cfg(feature = "full")]
pub(crate) fn file_dialog(kind: &str, extension: &str) -> rfd::FileDialog {
    let dir = profiles_dir();
    let dialog = rfd::FileDialog::new().add_filter(kind, &[extension]);
    widgets::in_folder(dialog, &dir)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn profile_dialog() -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new().add_filter("WinWeaver profile", &[profile::EXTENSION]);
    widgets::in_folder(dialog, &profiles_dir())
}

/// The kinds of files the dialog of a browser offers (the light edition there): profiles, answer
/// files, older profiles; a name and its extensions each.
#[cfg(target_arch = "wasm32")]
pub(crate) fn open_filters() -> Vec<(String, Vec<String>)> {
    let kind = |name: &str, ext: &str| (name.to_owned(), vec![ext.to_owned()]);
    vec![
        kind(tr("Profiles"), profile::EXTENSION),
        kind(tr("Answer files (the profile they embed)"), "xml"),
        kind(tr("Older profiles and audit reports"), "json"),
    ]
}

#[cfg(feature = "full")]
pub(crate) fn pick_profile() -> Option<PathBuf> {
    profile_dialog().set_title(tr("Open a profile")).pick_file()
}

/// The open dialog for every kind of file the program opens.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_dialog() -> rfd::FileDialog {
    let dir = profiles_dir();
    #[cfg(not(feature = "full"))]
    let dialog = rfd::FileDialog::new()
        .set_title(tr("Open"))
        .add_filter(tr("Profiles"), &[profile::EXTENSION])
        .add_filter(tr("Answer files (the profile they embed)"), &["xml"])
        .add_filter(tr("Older profiles and audit reports"), &["json"]);
    #[cfg(feature = "full")]
    let dialog = rfd::FileDialog::new()
        .set_title(tr("Open"))
        .add_filter(
            tr("WinWeaver files"),
            &[
                profile::EXTENSION,
                diff::EXTENSION,
                bookmarks::EXTENSION,
                admx::CATALOG_EXTENSION,
                themes::EXTENSION,
                winweaver_mcp::config::EXTENSION,
                users::EXTENSION,
            ],
        )
        .add_filter(tr("Registry files"), &[regfile::EXTENSION])
        .add_filter(tr("Group Policy files"), &[winweaver_core::pol::EXTENSION])
        .add_filter(tr("Policy templates (opened without importing)"), &["admx"])
        .add_filter(tr("Answer files (the profile they embed)"), &["xml"])
        .add_filter(tr("Older profiles and audit reports"), &["json"])
        .add_filter(tr("Markdown pages"), &["md"]);
    widgets::in_folder(dialog, &dir)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ask_save_path(proposed: &str) -> Option<PathBuf> {
    let _ = std::fs::create_dir_all(profiles_dir());
    profile_dialog()
        .set_title(tr("Save the profile as"))
        .set_file_name(proposed)
        .save_file()
}
