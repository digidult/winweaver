//! The native plugins of a program (docs/plugins.md): a DLL in the folder `plugins` next to the
//! executable that opens files of some formats and shows each in a tab.
//!
//! A plugin is a `cdylib` crate that depends on wwrt alone, implements `Plugin` and `View` and
//! names its plugin with `wwrt::plugin!`. The host loads every DLL of the folder
//! (`Plugins::load_dir`), checks that it was built with this runtime (`ABI`), adds its
//! translations and keeps it for the whole run: a plugin is never unloaded, since its code and the
//! tables of its traits live as long as the views it made.

use std::ffi::{CStr, c_char};
use std::path::{Path, PathBuf};

use serde_json::Value;
use ww::eframe::egui;
use ww::{PageWidth, Search};

/// The version of this interface: the traits and the types of this module. A change of them
/// that an older plugin cannot follow takes the next number (`ABI` names it too).
pub const API: u32 = 1;

/// The build of this runtime: its version, the interface, the compiler, the target and the
/// profile (`build.rs`), nul-terminated. A plugin carries the one it was built against (`plugin!`),
/// and `Plugins::load_dir` takes only the plugins of its own.
pub const ABI: &str = concat!(
    "wwrt ",
    env!("CARGO_PKG_VERSION"),
    "; interface 1; ",
    env!("WWRT_BUILD"),
    "\0"
);

/// A kind of file a plugin opens.
pub struct Format {
    /// The name of the format, for the filters of the dialog Open (the host translates it).
    pub name: &'static str,
    /// Its extensions without the dot, in lower case.
    pub extensions: &'static [&'static str],
}

/// A command of the bar a view offers (`View::tools`), answered by `View::act`; the host
/// translates its label.
pub struct Tool {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

/// Where a step through the files of a folder goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    First,
    Previous,
    Next,
    Last,
}

/// What a view asks of the window, done after the frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// The file a step leads to in the folder of the view, among the files its plugin opens, in
    /// the place of the one shown.
    Browse(Step),
}

/// What the host lends a view to draw with: the settings of the window that concern a view, the
/// settings of its plugin and the requests the view makes.
pub struct Host<'a> {
    pub wrap_lines: bool,
    pub page_width: PageWidth,
    /// The settings of the plugin (`Plugin::settings_ui`), `Null` before the first change.
    pub settings: &'a Value,
    pub requests: Vec<Request>,
}

/// A plugin: the formats it opens and the view of a file of one of them.
pub trait Plugin: Send + Sync {
    /// The name of the plugin, for the list of the plugins and the messages (translated).
    fn name(&self) -> &'static str;
    fn formats(&self) -> &'static [Format];
    /// A view of the file with the settings of the plugin, or the text of why it cannot be opened.
    fn open(&self, path: &Path, settings: &Value) -> Result<Box<dyn View>, String>;
    /// The texts of the plugin in other languages: the code and a file in the form of `ui.json`
    /// for each, added to the translations of the window at the load.
    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
    /// Whether the plugin has settings for `settings_ui`.
    fn has_settings(&self) -> bool {
        false
    }
    /// Draws the settings of the plugin in its card of Settings, Plugins; gives whether they
    /// changed, and the host saves them with the settings of the window.
    fn settings_ui(&self, _ui: &mut egui::Ui, _settings: &mut Value) -> bool {
        false
    }
}

/// A file open in a tab.
pub trait View {
    /// Draws the view in the panel of its tab; the keys and the mouse come through
    /// `wwrt::input::area`.
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host);
    /// The commands of the bar of the tab.
    fn tools(&self) -> Vec<Tool> {
        Vec::new()
    }
    /// Answers a command of `tools`.
    fn act(&mut self, _id: &str) {}
    /// The search of the tab, when the view has one (the field at the end of the command bar).
    fn search(&mut self) -> Option<&mut Search> {
        None
    }
    /// Whether the view steps through the files of its folder: the bar then has Previous and Next.
    fn browses(&self) -> bool {
        false
    }
    /// A line of the status bar: the size of an image, the lines of a text.
    fn status(&self) -> String {
        String::new()
    }
}

/// A setting of a plugin: the field `name` of its settings as `T`, or `default` when it is not
/// there or of another type.
pub fn setting<T: serde::de::DeserializeOwned>(settings: &Value, name: &str, default: T) -> T {
    settings
        .get(name)
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(default)
}

/// Sets the field `name` of the settings of a plugin, making them an object when they are not.
pub fn set_setting(settings: &mut Value, name: &str, value: impl serde::Serialize) {
    if !settings.is_object() {
        *settings = Value::Object(serde_json::Map::new());
    }
    if let (Some(map), Ok(v)) = (settings.as_object_mut(), serde_json::to_value(value)) {
        map.insert(name.to_owned(), v);
    }
}

/// Names the plugin of a crate: `wwrt::plugin!(TextPlugin);` exports the build of wwrt it was made
/// with and the function that makes the plugin.
#[macro_export]
macro_rules! plugin {
    ($plugin:expr) => {
        /// The build of wwrt this plugin was made with (`wwrt::plugin::ABI`).
        #[unsafe(no_mangle)]
        pub extern "C" fn ww_plugin_abi() -> *const u8 {
            $crate::plugin::ABI.as_ptr()
        }

        /// The plugin; the host calls it once, after `ww_plugin_abi` matched its own build.
        #[unsafe(no_mangle)]
        pub fn ww_plugin() -> Box<dyn $crate::plugin::Plugin> {
            Box::new($plugin)
        }
    };
}

/// A plugin loaded from its DLL.
pub struct Loaded {
    /// The name of its DLL without the extension: the key of its settings.
    pub id: String,
    pub file: PathBuf,
    pub plugin: Box<dyn Plugin>,
}

impl Loaded {
    /// Whether the plugin opens a file of the extension of this path.
    pub fn opens(&self, path: &Path) -> bool {
        let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            return false;
        };
        self.plugin
            .formats()
            .iter()
            .any(|f| f.extensions.contains(&ext.as_str()))
    }
}

/// The plugins of a program, and the DLLs of its folder that did not load with the reason.
#[derive(Default)]
pub struct Plugins {
    pub loaded: Vec<Loaded>,
    pub failed: Vec<(PathBuf, String)>,
}

impl Plugins {
    /// Every DLL of the folder, by its name, but the one of wwrt itself (a build of the workspace
    /// writes the runtime and the plugins into one folder).
    pub fn load_dir(dir: &Path) -> Self {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| {
                        let dll = p.extension().is_some_and(|e| {
                            e.eq_ignore_ascii_case(std::env::consts::DLL_EXTENSION)
                        });
                        let runtime = p
                            .file_stem()
                            .is_some_and(|s| s.eq_ignore_ascii_case("wwrt"));
                        dll && !runtime
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        let mut out = Plugins::default();
        for file in files {
            match load(&file) {
                Ok(plugin) => out.add(file, plugin),
                Err(e) => out.failed.push((file, e)),
            }
        }
        out
    }

    /// A plugin made by the program itself or a test, next to the ones of DLLs; its
    /// translations join the window.
    pub fn add(&mut self, file: PathBuf, plugin: Box<dyn Plugin>) {
        ww::i18n::add(plugin.translations());
        let id = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.loaded.push(Loaded { id, file, plugin });
    }

    /// The index of the first plugin that opens the file.
    pub fn for_path(&self, path: &Path) -> Option<usize> {
        self.loaded.iter().position(|l| l.opens(path))
    }
}

/// The build of the DLL of wwrt itself, for About and the list of the plugins.
pub fn runtime() -> &'static str {
    ABI.trim_end_matches('\0')
}

/// A plugin of its DLL; this code runs in wwrt.dll, so `ABI` here is the build of that DLL.
fn load(path: &Path) -> Result<Box<dyn Plugin>, String> {
    let _quiet = Quiet::new();
    // SAFETY: loading a DLL runs its initialisation; the plugins come from the folder of the
    // program, which its user fills as he fills the folder of the program itself.
    let library = unsafe { libloading::Library::new(path) }.map_err(|e| {
        let text = e.to_string();
        if text.contains("os error 127") {
            format!("built against another build of wwrt: its imports are not in this one ({text})")
        } else {
            text
        }
    })?;
    // SAFETY: `ww_plugin_abi` is the function `plugin!` exports, with this signature, and it gives
    // a nul-terminated text of the plugin's constant data.
    let built = unsafe {
        let abi = library
            .get::<extern "C" fn() -> *const u8>(b"ww_plugin_abi\0")
            .map_err(|e| format!("not a plugin of wwrt: {e}"))?;
        CStr::from_ptr(abi().cast::<c_char>())
            .to_string_lossy()
            .into_owned()
    };
    let own = runtime();
    if built != own {
        return Err(format!(
            "built with another runtime ({built}); this one is {own}"
        ));
    }
    // SAFETY: a plugin of this very build (checked above) exports `ww_plugin` as `plugin!` writes
    // it, so the signature and the layout of `Box<dyn Plugin>` are the ones of this runtime.
    let plugin = unsafe {
        let make = library
            .get::<fn() -> Box<dyn Plugin>>(b"ww_plugin\0")
            .map_err(|e| format!("not a plugin of wwrt: {e}"))?;
        make()
    };
    // Never unloaded: the code and the tables of the plugin's traits live as long as its views.
    std::mem::forget(library);
    Ok(plugin)
}

/// No dialog of Windows while a plugin loads: a DLL that misses an import (one built against
/// another wwrt.dll) fails with an error the list of the plugins shows.
struct Quiet(u32);

impl Quiet {
    fn new() -> Self {
        let mut old = 0u32;
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Diagnostics::Debug::{
                SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX, SetThreadErrorMode,
            };
            // SAFETY: the call changes the error mode of this thread alone and gives the old one.
            unsafe {
                SetThreadErrorMode(
                    SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX,
                    &raw mut old,
                );
            }
        }
        Quiet(old)
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Diagnostics::Debug::SetThreadErrorMode;
            // SAFETY: the mode of this thread comes back; the old one is not wanted.
            unsafe {
                SetThreadErrorMode(self.0, std::ptr::null_mut());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_names_the_interface() {
        assert!(
            runtime().contains(&format!("interface {API};")),
            "{}",
            runtime()
        );
        assert!(ABI.ends_with('\0') && !runtime().ends_with('\0'));
    }

    #[test]
    fn a_setting_falls_back_to_its_default() {
        let mut settings = Value::Null;
        assert_eq!(setting(&settings, "max", 64u32), 64);
        set_setting(&mut settings, "max", 128u32);
        assert_eq!(setting(&settings, "max", 64u32), 128);
        set_setting(&mut settings, "max", "many");
        assert_eq!(setting(&settings, "max", 64u32), 64);
    }
}
