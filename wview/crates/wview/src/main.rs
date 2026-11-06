//! wview, the viewer of files with plugins (docs/plugins.md): the window of W# with the views of
//! the DLLs in the folder `plugins` next to the executable.
//!
//! In a release build the executable has no console (`windows_subsystem`): the command line
//! functions attach to the console of the parent when there is one.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod shots;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use wwrt::egui;
use wwrt::plugin::{self, Host, Plugins};
use wwrt::serde_json::Value;
use wwrt::ww::PageWidth;

const USAGE: &str = "wview, a viewer of files with plugins

  wview [FILE...]          open the window, with the files in tabs
  wview --plugins          list the plugins and the formats they open
  wview --probe FILE       open FILE with its plugin and draw its view in three frames, without
                           a window; print the times
  wview --help             this text

The plugins are the DLLs of the folder plugins next to wview.exe (or of the folder WVIEW_PLUGINS
names); a build of the workspace keeps them next to the executable.";

/// Attaches to the console of the parent process, so that a release build prints in it; a process
/// whose output goes to a file or a pipe (a script, CI) keeps it there.
fn attach_console() {
    #[cfg(windows)]
    {
        use wwrt::windows::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE,
        };
        // SAFETY: the calls only read the handle of the output and connect the process to an
        // existing console, if there is one.
        unsafe {
            let redirected =
                GetStdHandle(STD_OUTPUT_HANDLE).is_ok_and(|h| !h.is_invalid() && !h.0.is_null());
            if !redirected {
                let _ = AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
}

/// The folder of the plugins: `WVIEW_PLUGINS`, else `plugins` next to the executable, else the
/// folder of the executable itself (a build of the workspace, where cargo writes every DLL).
fn plugins_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WVIEW_PLUGINS") {
        return PathBuf::from(dir);
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let here = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let folder = here.join("plugins");
    if folder.is_dir() { folder } else { here }
}

/// The plugins with the formats they open, and the DLLs that did not load.
fn list(plugins: &Plugins, dir: &Path, took: std::time::Duration) -> String {
    let mut out = format!(
        "{} plugins of {} loaded in {took:?}\nruntime: {}\n",
        plugins.loaded.len(),
        dir.display(),
        plugin::runtime()
    );
    for loaded in &plugins.loaded {
        out.push_str(&format!(
            "\n{} ({})\n",
            loaded.plugin.name(),
            loaded.file.display()
        ));
        if let Some(origin) = &loaded.origin {
            out.push_str(&origin_text(origin));
        }
        for format in loaded.plugin.formats() {
            out.push_str(&format!(
                "  {}: {}",
                format.name,
                format.extensions.join(", ")
            ));
            if !format.mimes.is_empty() {
                out.push_str(&format!(" ({})", format.mimes.join(", ")));
            }
            out.push('\n');
        }
    }
    for failed in &plugins.failed {
        out.push_str(&format!(
            "\nnot loaded: {}: {}\n",
            failed.file.display(),
            failed.why
        ));
        if let Some(origin) = &failed.origin {
            out.push_str(&origin_text(origin));
        }
    }
    out
}

/// The SHA-256 and the signature of a DLL, two lines of the list of the plugins.
fn origin_text(origin: &wwrt::trust::Origin) -> String {
    let signature = match &origin.signature {
        wwrt::trust::Signature::None => "not signed".to_owned(),
        wwrt::trust::Signature::Trusted(name) => format!("signed by {name}"),
        wwrt::trust::Signature::Untrusted(why) => format!("a signature not trusted: {why}"),
    };
    format!("  SHA-256 {}\n  {signature}\n", origin.sha256)
}

/// Opens the file with its plugin and draws three frames of its view without a window.
fn probe(plugins: &Plugins, path: &Path) -> Result<String, String> {
    let i = plugins
        .for_file(path)
        .ok_or_else(|| format!("no plugin opens {}", path.display()))?;
    let loaded = &plugins.loaded[i];
    let started = Instant::now();
    let mut view = loaded.plugin.open(path, &Value::Null)?;
    let opened = started.elapsed();
    let settings = Value::Null;
    let mut host = Host::new(true, PageWidth::View, &settings);
    let ctx = egui::Context::default();
    let started = Instant::now();
    let mut shapes = 0;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.ui(ui, &mut host));
        shapes = output.shapes.len();
        // No renderer takes the textures here; egui wants them taken or cleared.
        output.textures_delta.clear();
    }
    Ok(format!(
        "{}: {}, opened in {opened:?}, three frames in {:?} ({shapes} shapes in the last); {}",
        path.display(),
        loaded.plugin.name(),
        started.elapsed(),
        view.status()
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        attach_console();
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let dir = plugins_dir();
    // The plugins it trusts are in the settings, read before any plugin loads.
    let (settings, _) = wwrt::ww::Settings::<app::Extra>::load(<app::Wview as wwrt::ww::App>::NAME);
    let trust = settings.map(|s| s.extra.trust).unwrap_or_default();
    let started = Instant::now();
    let plugins = Plugins::load_trusted(&dir, &trust);
    let took = started.elapsed();
    match args.first().map(String::as_str) {
        Some("--plugins") => {
            attach_console();
            println!("{}", list(&plugins, &dir, took));
            ExitCode::SUCCESS
        }
        Some("--probe") => {
            attach_console();
            let Some(file) = args.get(1) else {
                eprintln!("--probe needs a file\n\n{USAGE}");
                return ExitCode::from(2);
            };
            match probe(&plugins, Path::new(file)) {
                Ok(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some(a) if a.starts_with("--") => {
            attach_console();
            eprintln!("unknown option {a}\n\n{USAGE}");
            ExitCode::from(2)
        }
        _ => {
            let plugins = Arc::new(plugins);
            let files: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();
            let make = move |_: &_| app::Wview::new(plugins.clone(), files.clone());
            match wwrt::ww::run::<app::Wview>(None, make) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    attach_console();
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
