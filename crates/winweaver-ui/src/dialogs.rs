//! The file dialogs and the names of files.

use crate::*;

pub(crate) use ww::dialogs::{kept_name, write_file};

/// A file name stem made of a profile name.
pub fn file_stem(name: &str) -> String {
    ww::dialogs::file_stem(name, "profile")
}

/// The path with the profile extension when it has none.
/// A path the save dialog gave, with the extension of a registry file when it has none.
pub(crate) fn reg_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(regfile::EXTENSION)
    }
}

pub(crate) fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(profile::EXTENSION)
    }
}

pub(crate) fn file_dialog(kind: &str, extension: &str) -> rfd::FileDialog {
    let dir = profiles_dir();
    let dialog = rfd::FileDialog::new().add_filter(kind, &[extension]);
    widgets::in_folder(dialog, &dir)
}

pub(crate) fn profile_dialog() -> rfd::FileDialog {
    file_dialog("WinWeaver profile", profile::EXTENSION)
}

pub(crate) fn pick_profile() -> Option<PathBuf> {
    profile_dialog().set_title(tr("Open a profile")).pick_file()
}

/// The open dialog for every kind of file the program opens.
pub(crate) fn open_dialog() -> rfd::FileDialog {
    let dir = profiles_dir();
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
        .add_filter(tr("Policy templates (opened without importing)"), &["admx"])
        .add_filter(tr("Answer files (the profile they embed)"), &["xml"])
        .add_filter(tr("WinKickOff profiles and audit reports"), &["json"])
        .add_filter(tr("Markdown pages"), &["md"]);
    widgets::in_folder(dialog, &dir)
}

pub(crate) fn ask_save_path(proposed: &str) -> Option<PathBuf> {
    let _ = std::fs::create_dir_all(profiles_dir());
    profile_dialog()
        .set_title(tr("Save the profile as"))
        .set_file_name(proposed)
        .save_file()
}
