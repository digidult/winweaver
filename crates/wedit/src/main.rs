//! The wedit executable: the window, with the files named on the command line in tabs.
//!
//! In a release build the executable has no console (`windows_subsystem`): the command line
//! functions attach to the console of the parent when there is one.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod shots;
#[cfg(test)]
mod tests;
mod textdoc;

use std::path::PathBuf;
use std::process::ExitCode;

use app::Wedit;

const USAGE: &str = "wedit, a text editor

  wedit [FILE...]             open the window, with the files in tabs
  wedit --renderer auto|opengl|directx [FILE...]
                              open the window, trying that renderer first (this start only)
  wedit --smoke-window        open a small window, draw two frames, print what drew them
  wedit --version             print the version
  wedit --help                this text";

/// Attaches to the console of the parent process, so that a release build prints in it.
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

/// What the command line asks for.
#[derive(Default)]
struct Args {
    help: bool,
    version: bool,
    smoke: bool,
    renderer: Option<String>,
    files: Vec<PathBuf>,
    unknown: Option<String>,
}

fn parse(args: &[String]) -> Args {
    let mut out = Args::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => out.help = true,
            "--version" => out.version = true,
            "--smoke-window" => out.smoke = true,
            "--renderer" => {
                out.renderer = args.get(i + 1).cloned();
                i += 1;
            }
            a if a.starts_with("--renderer=") => {
                out.renderer = Some(a["--renderer=".len()..].to_owned());
            }
            a if a.starts_with("--") => out.unknown = Some(a.to_owned()),
            a => out.files.push(PathBuf::from(a)),
        }
        i += 1;
    }
    out
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = parse(&raw);
    if let Some(option) = &args.unknown {
        attach_console();
        eprintln!("unknown option {option}\n\n{USAGE}");
        return ExitCode::FAILURE;
    }
    if args.help {
        attach_console();
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.version {
        attach_console();
        println!("wedit {}", app::VERSION);
        return ExitCode::SUCCESS;
    }
    if args.smoke {
        attach_console();
        return match ww::smoke_window::<Wedit>(args.renderer.as_deref()) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    let files = args.files;
    match ww::run::<Wedit>(args.renderer.as_deref(), move |_| Wedit::new(files.clone())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            attach_console();
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
