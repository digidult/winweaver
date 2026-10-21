//! The folders next to the executable.

use crate::*;

/// The folder of the imports of policy templates next to the executable.
pub fn imports_dir() -> PathBuf {
    exe_dir().join("imports")
}

/// The folder of the colour themes next to the executable.
pub fn themes_dir() -> PathBuf {
    exe_dir().join("themes")
}

/// The folder of the translations next to the executable.
pub fn lang_dir() -> PathBuf {
    exe_dir().join("lang")
}

/// The folder of the built files next to the executable.
pub fn output_dir() -> PathBuf {
    exe_dir().join("output")
}

/// The folder of the user's profiles next to the executable.
pub fn profiles_dir() -> PathBuf {
    exe_dir().join("profiles")
}
