//! The folders next to the executable.

use std::path::PathBuf;

use ww_base::paths;

/// The folder of the executable, or the current folder when it cannot be found.
pub fn exe_dir() -> PathBuf {
    paths::program_dir()
}

/// A folder next to the executable, by its name.
pub fn dir(name: &str) -> PathBuf {
    exe_dir().join(name)
}
