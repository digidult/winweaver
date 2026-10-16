//! The WinWeaver executable: the window by default, a few command line functions.
//!
//! In a release build the executable has no console (`windows_subsystem`): the command line
//! functions attach to the console of the parent when there is one. The MCP stdio transport
//! needs no console; the client gives the process its pipes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use winweaver_core::catalog::{self, Catalog};
use winweaver_core::i18n::CatalogTexts;

const USAGE: &str = "WinWeaver

  winweaver                   open the window
  winweaver --version         print the version
  winweaver --check-catalog   load the embedded catalog and report its problems
  winweaver --mcp stdio [--mode read|edit|files] [--read-pc] [--profile NAME]
                              serve MCP on standard input and output
  winweaver --mcp http --token TOKEN [--port N] [--mode ...] [--read-pc] [--profile NAME]
                              serve MCP on http://127.0.0.1:N/mcp (port 8765 by default)
  winweaver --help            this text";

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

/// The value of an option such as `--port 8765` or `--port=8765`.
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

/// The catalog with the imports of policy templates the settings of the window load (no other
/// catalog file is read), the templates of this Windows first; a file that cannot be read or
/// merged is left out.
fn with_imports(catalog: Catalog, dir: &std::path::Path) -> Catalog {
    let settings = winweaver_ui::Settings::load_from(&winweaver_ui::Settings::path())
        .unwrap_or_default();
    let ids = winweaver_ui::loaded_import_ids(&settings, dir);
    let (files, _) = winweaver_ui::read_imports(dir, &ids);
    match winweaver_core::admx::merge(&catalog, &files) {
        Ok((merged, _)) => merged,
        Err(_) => catalog,
    }
}

/// A message for the console of the parent and the MCP log: a release build started by a client
/// has no console, so the log is where the reason of an early exit stays.
fn refuse(text: &str) {
    eprintln!("{text}");
    winweaver_mcp::log::line("ERROR", text);
}

fn mcp(args: &[String]) -> ExitCode {
    let transport = args.first().map(String::as_str).unwrap_or("");
    winweaver_mcp::log::line(
        "INFO",
        &format!(
            "WinWeaver {} started for MCP over {transport} (process {})",
            winweaver_core::APP_VERSION,
            std::process::id()
        ),
    );
    let mode = match option(args, "--mode")
        .as_deref()
        .map(winweaver_mcp::Mode::parse)
    {
        None => winweaver_mcp::Mode::Read,
        Some(Some(m)) => m,
        Some(None) => {
            refuse("--mode is read, edit or files");
            return ExitCode::from(2);
        }
    };
    let catalog = match Catalog::embedded() {
        Ok(c) => c,
        Err(e) => {
            refuse(&format!("catalog: {e}"));
            return ExitCode::FAILURE;
        }
    };
    let dir = winweaver_ui::exe_dir();
    let catalog = with_imports(catalog, &winweaver_ui::imports_dir());
    let server = winweaver_mcp::Server::new(
        catalog,
        &winweaver_ui::PRESETS,
        winweaver_mcp::Options {
            mode,
            read_pc: args.iter().any(|a| a == "--read-pc"),
            profiles_dir: dir.join("profiles"),
            output_dir: dir.join("output"),
        },
    );
    if let Some(name) = option(args, "--profile")
        && let Err(e) = server.select(&name)
    {
        refuse(&format!("--profile {name}: {e}"));
        return ExitCode::from(2);
    }
    match transport {
        "stdio" => match winweaver_mcp::stdio::serve(&server) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                refuse(&format!("stdio: {e}"));
                ExitCode::FAILURE
            }
        },
        "http" => {
            let Some(token) = option(args, "--token").filter(|t| t.len() >= 32) else {
                refuse("--token of at least 32 characters is needed");
                return ExitCode::from(2);
            };
            let port = match option(args, "--port").map(|p| p.parse::<u16>()) {
                None => 8765,
                Some(Ok(p)) => p,
                Some(Err(_)) => {
                    refuse("--port is a number");
                    return ExitCode::from(2);
                }
            };
            match winweaver_mcp::http::HttpServer::start(std::sync::Arc::new(server), port, token) {
                Ok(http) => {
                    eprintln!("WinWeaver MCP on {} (mode {})", http.url(), mode.name());
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    refuse(&format!("http: {e}"));
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            refuse("--mcp stdio or --mcp http");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// The elevated helper: runs a plan file of the output folder whose bytes have the SHA-256 given on
/// the command line and writes its report next to it. It reads the file once and refuses any other
/// file, so a plan changed after the permission of Windows never runs.
fn elevated_apply(args: &[String]) -> ExitCode {
    use std::io::Read;
    let (Some(plan_path), Some(expected)) = (args.first(), args.get(1)) else {
        return ExitCode::from(2);
    };
    let out = winweaver_ui::output_dir();
    let plan_path = std::path::PathBuf::from(plan_path);
    let name = plan_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = plan_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    let in_output = match (std::fs::canonicalize(&parent), std::fs::canonicalize(&out)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !in_output
        || !name.starts_with("plan-")
        || !name.ends_with(".json")
        || name.contains("-report")
    {
        return ExitCode::from(3);
    }
    let limit = winweaver_core::apply::PLAN_MAX_BYTES;
    let mut bytes = Vec::new();
    let read =
        std::fs::File::open(&plan_path).and_then(|f| f.take(limit + 1).read_to_end(&mut bytes));
    if read.is_err() || bytes.len() as u64 > limit {
        return ExitCode::from(3);
    }
    if expected.len() != 64 || winweaver_core::apply::digest(&bytes) != *expected {
        return ExitCode::from(3);
    }
    let Ok(plan) = winweaver_core::apply::Plan::from_bytes(&bytes, &name) else {
        return ExitCode::from(3);
    };
    let stamp = winweaver_core::clock::now_utc_iso()
        .replace([':', '-'], "")
        .replace('T', "-")
        .trim_end_matches('Z')
        .to_owned();
    let report = winweaver_win::execute_plan(&plan, &out, &stamp);
    let report_path = plan_path.with_file_name(name.replace(".json", "-report.json"));
    let text = serde_json::to_string_pretty(&report).unwrap_or_default();
    match std::fs::write(&report_path, text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The crash handlers first: a panic or a fatal exception of Windows is written to the logs
    // folder next to the executable, the MCP log for a process that only serves MCP.
    let headless = args.first().map(String::as_str) == Some("--mcp");
    winweaver_mcp::log::init(&winweaver_ui::exe_dir().join("logs"), headless);
    match args.first().map(String::as_str) {
        None => match winweaver_ui::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                winweaver_mcp::log::window_line("ERROR", &format!("the window failed: {e}"));
                ExitCode::FAILURE
            }
        },
        Some("--mcp") => mcp(&args[1..]),
        Some("--smoke-window") => {
            attach_console();
            match winweaver_ui::smoke_window() {
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
        Some("--elevated-apply") => elevated_apply(&args[1..]),
        Some("--version") => {
            attach_console();
            println!("WinWeaver {}", winweaver_core::APP_VERSION);
            ExitCode::SUCCESS
        }
        Some("--check-catalog") => {
            attach_console();
            check_catalog()
        }
        Some("--help") => {
            attach_console();
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown argument {other}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check_catalog() -> ExitCode {
    let catalog = match Catalog::embedded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("catalog: {e}");
            return ExitCode::FAILURE;
        }
    };
    let texts = match CatalogTexts::embedded() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("translations: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "catalog {}: {} rules, {} groups, languages: en, {}",
        catalog.version(),
        catalog.rules().len(),
        catalog.groups().len(),
        texts
            .iter()
            .map(|t| t.code.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let problems = catalog::check::check(&catalog);
    for problem in &problems {
        println!("problem: {problem}");
    }
    if problems.is_empty() {
        println!("no problems");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
