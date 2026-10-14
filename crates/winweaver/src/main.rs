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

/// The catalog with the imports of policy templates next to the executable, the templates of
/// this Windows first; a file that cannot be read or merged is left out.
fn with_imports(catalog: Catalog, dir: &std::path::Path) -> Catalog {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return catalog;
    };
    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == winweaver_core::admx::CATALOG_EXTENSION))
        .collect();
    paths.sort_by_key(|p| {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        (!name.starts_with("system"), name)
    });
    let files: Vec<winweaver_core::admx::CatalogFile> = paths
        .iter()
        .filter_map(|p| winweaver_core::admx::CatalogFile::load(p).ok())
        .collect();
    match winweaver_core::admx::merge(&catalog, &files) {
        Ok((merged, _)) => merged,
        Err(_) => catalog,
    }
}

fn mcp(args: &[String]) -> ExitCode {
    let transport = args.first().map(String::as_str).unwrap_or("");
    let mode = match option(args, "--mode")
        .as_deref()
        .map(winweaver_mcp::Mode::parse)
    {
        None => winweaver_mcp::Mode::Read,
        Some(Some(m)) => m,
        Some(None) => {
            eprintln!("--mode is read, edit or files");
            return ExitCode::from(2);
        }
    };
    let catalog = match Catalog::embedded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("catalog: {e}");
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
        eprintln!("--profile {name}: {e}");
        return ExitCode::from(2);
    }
    match transport {
        "stdio" => match winweaver_mcp::stdio::serve(&server) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("stdio: {e}");
                ExitCode::FAILURE
            }
        },
        "http" => {
            let Some(token) = option(args, "--token").filter(|t| t.len() >= 32) else {
                eprintln!("--token of at least 32 characters is needed");
                return ExitCode::from(2);
            };
            let port = match option(args, "--port").map(|p| p.parse::<u16>()) {
                None => 8765,
                Some(Ok(p)) => p,
                Some(Err(_)) => {
                    eprintln!("--port is a number");
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
                    eprintln!("http: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("--mcp stdio or --mcp http\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// The elevated helper: runs a plan file of the output folder whose nonce matches the one on the
/// command line and writes its report next to it. It refuses any other file.
fn elevated_apply(args: &[String]) -> ExitCode {
    let (Some(plan_path), Some(nonce)) = (args.first(), args.get(1)) else {
        return ExitCode::from(2);
    };
    let out = winweaver_ui::output_dir();
    let plan_path = std::path::PathBuf::from(plan_path);
    let name = plan_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = plan_path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let in_output = match (std::fs::canonicalize(&parent), std::fs::canonicalize(&out)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !in_output || !name.starts_with("plan-") || !name.ends_with(".json") || name.contains("-report") {
        return ExitCode::from(3);
    }
    let Ok(plan) = winweaver_core::apply::Plan::load(&plan_path) else {
        return ExitCode::from(3);
    };
    if plan.nonce.len() < 32 || plan.nonce != *nonce {
        return ExitCode::from(3);
    }
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
    match args.first().map(String::as_str) {
        None => match winweaver_ui::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        Some("--mcp") => mcp(&args[1..]),
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
