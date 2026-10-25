//! wwlite, the light edition of WinWeaver: the profile editor, the check and the build of
//! `autounattend.xml`, the presets, the catalogs of the folder `imports` and the themes of the
//! folder `themes` next to the program, a short guide as plain text, English and Ukrainian built
//! in. Nothing reads or changes the PC; no MCP, registry, diffs, bookmarks, imports of policy
//! templates or users files. The window is the one of WinWeaver built without its feature
//! `full`, so none of that code is in the executable (`winweaver_ui::lite`).
//!
//! In a release build the executable has no console (`windows_subsystem`): the command line
//! functions attach to the console of the parent when there is one.
//!
//! Built for `wasm32-unknown-unknown` (the features `glow` alone), the same window runs in a
//! browser: `web/index.html` loads the module next to it and `main` starts the window in the
//! canvas `wwlite` of the page as a task of the page (`winweaver_ui::run_web`). Files come and
//! go through the browser (a dialog or a drop, downloads), the settings and the drafts stay in
//! the storage of the page.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

#[cfg(test)]
mod tests;

/// The id of the canvas of `web/index.html` the window draws in.
#[cfg(target_arch = "wasm32")]
const CANVAS_ID: &str = "wwlite";

#[cfg(not(target_arch = "wasm32"))]
const USAGE: &str = "wwlite, the light edition of WinWeaver

  wwlite                      open the window
  wwlite --renderer auto|opengl|directx
                              open the window, trying that renderer first (this start only)
  wwlite --smoke-window       open a small window, draw two frames, print what drew them
  wwlite --version            print the version
  wwlite --help               this text";

/// Attaches to the console of the parent process, so that a release build prints in it.
#[cfg(not(target_arch = "wasm32"))]
fn attach_console() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        // SAFETY: the call only connects the process to an existing console, if there is one.
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

/// The value of an option such as `--renderer directx` or `--renderer=directx`.
#[cfg(not(target_arch = "wasm32"))]
fn option(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    args.iter().enumerate().find_map(|(i, a)| {
        if a == name {
            args.get(i + 1).cloned()
        } else {
            a.strip_prefix(&prefix).map(str::to_owned)
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => window(None),
        Some(a) if a == "--renderer" || a.starts_with("--renderer=") => {
            match option(&args, "--renderer") {
                Some(name) if args.len() <= 2 => window(Some(&name)),
                _ => {
                    attach_console();
                    eprintln!("--renderer needs a name: auto, opengl or directx\n{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        Some("--smoke-window") => {
            attach_console();
            match winweaver_ui::smoke_window(option(&args, "--renderer").as_deref()) {
                Ok(info) => {
                    println!("the window started: {info}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    println!("the window did not start: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("--version") => {
            attach_console();
            println!("wwlite {}", winweaver_core::BUILD);
            ExitCode::SUCCESS
        }
        Some("--help") => {
            attach_console();
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            attach_console();
            eprintln!("unknown argument {other}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Opens the window; its failure goes to the console of the parent if there is one.
#[cfg(not(target_arch = "wasm32"))]
fn window(renderer: Option<&str>) -> ExitCode {
    match winweaver_ui::run(renderer) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            attach_console();
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// The browser edition: the window in the canvas of the page, started when the module loads; a
/// failure of the start is shown in the page and in the console of the browser.
#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(winweaver_ui::run_web(CANVAS_ID));
}
