//! The file dialogs and the names of files.

use crate::*;

/// The file dialog of the window: rfd's natively. A page has none it could wait for, so in a
/// browser this one answers no file at once: what saves goes to a download there, what opens to
/// the dialog of the page (`ShellAct::Open`, then `App::open_bytes`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use rfd::FileDialog;

/// The file dialog in a browser: a file to save is a download under the name the dialog was given
/// (a path under `ww_base::files::DOWNLOAD`, which the file layer of the page turns into a
/// download), a folder to save into the downloads; a file to open, none (the dialog of the page
/// does it).
#[cfg(all(target_arch = "wasm32", feature = "full"))]
#[derive(Default)]
pub(crate) struct FileDialog {
    name: String,
}

#[cfg(all(target_arch = "wasm32", feature = "full"))]
impl FileDialog {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn set_title(self, _title: impl Into<String>) -> Self {
        self
    }
    pub(crate) fn add_filter(
        self,
        _name: impl Into<String>,
        _extensions: &[impl ToString],
    ) -> Self {
        self
    }
    pub(crate) fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    pub(crate) fn set_directory(self, _dir: impl AsRef<std::path::Path>) -> Self {
        self
    }
    pub(crate) fn pick_file(self) -> Option<PathBuf> {
        None
    }
    pub(crate) fn pick_folder(self) -> Option<PathBuf> {
        Some(PathBuf::from(ww_base::files::DOWNLOAD))
    }
    pub(crate) fn save_file(self) -> Option<PathBuf> {
        let name = if self.name.is_empty() {
            "file".to_owned()
        } else {
            self.name
        };
        Some(ww_base::files::download_path(&name))
    }
}

/// A dialog that starts in a folder (`ww::widgets::in_folder` natively).
#[cfg(any(feature = "full", not(target_arch = "wasm32")))]
pub(crate) fn in_folder(dialog: FileDialog, dir: &std::path::Path) -> FileDialog {
    #[cfg(not(target_arch = "wasm32"))]
    {
        widgets::in_folder(dialog, dir)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
        dialog
    }
}

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
pub(crate) fn file_dialog(kind: &str, extension: &str) -> FileDialog {
    let dir = profiles_dir();
    let dialog = FileDialog::new().add_filter(kind, &[extension]);
    in_folder(dialog, &dir)
}

#[cfg(any(feature = "full", not(target_arch = "wasm32")))]
pub(crate) fn profile_dialog() -> FileDialog {
    let dialog = FileDialog::new().add_filter("WinWeaver profile", &[profile::EXTENSION]);
    in_folder(dialog, &profiles_dir())
}

/// The kinds of files the dialog of a browser offers (the light edition there): profiles, answer
/// files, older profiles; a name and its extensions each.
#[cfg(all(target_arch = "wasm32", not(feature = "full")))]
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
                #[cfg(feature = "mcp")]
                wwmcp::config::EXTENSION,
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
