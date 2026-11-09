//! The WinWeaver executable: the window by default, the command line functions (`cli`: the
//! answer file, the imports and exports of policy templates, a read of this PC, diffs, scripts
//! and registry files), the MCP server and the elevated helper.
//!
//! In a release build the executable has no console (`windows_subsystem`): the command line
//! functions attach to the console of the parent when there is one. The MCP stdio transport
//! needs no console; the client gives the process its pipes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;

use std::process::ExitCode;

use winweaver_core::catalog::{self, Catalog};
use winweaver_core::i18n::CatalogTexts;

const USAGE: &str = "WinWeaver

  winweaver                   open the window
  winweaver --renderer auto|opengl|directx
                              open the window, trying that renderer first (this start only)
  winweaver --version         print the version
  winweaver --check-catalog   load the embedded catalog and report its problems
  winweaver --build PROFILE [--out FILE|FOLDER]
                              check a profile and build its answer file, autounattend.xml with
                              the scripts it embeds beside it; PROFILE is a .wwprofile file, a
                              preset (Office, Strict, Laptop, Home) or a name of the profiles folder
  winweaver --import-admx system|PATH
                              import policy templates as Settings, Policy templates does: those of
                              this Windows, a folder of ADMX files, an .admx, .zip or .wwcatalog
                              file; the window and the MCP server load it at their next start
  winweaver --export-catalog ID|system|PATH [--out FILE|FOLDER]
                              write the catalog file (<id>.wwcatalog) of an import of the imports
                              folder, or of templates read as --import-admx reads them
  winweaver --read-pc [--base PROFILE] [--out FILE|FOLDER]
                              read this PC into a new profile on the base (the preset Office by
                              default), <computer>.wwprofile in profiles; an .xml file in --out
                              gets its answer file; features and capabilities need administrator
  winweaver --diff A B [--out FILE|FOLDER]
                              write the differences from the profile A to the profile B as
                              <A>-vs-<B>.wwdiff in profiles; A and B as PROFILE of --build
  winweaver --script apply|revert|audit FILE [--only IDS] [--out FILE|FOLDER]
                              write the stand-alone script of a profile (FILE.wwprofile) or a
                              diff (FILE.wwdiff); an apply or a return gets Undo-Apply.ps1 beside it
  winweaver --reg apply|revert FILE [--only IDS] [--out FILE|FOLDER]
                              write the registry file of the apply or the return of a profile or
                              a diff; --only takes ids of rules or groups separated by commas
  --out FILE|FOLDER           with the commands above: the file when it ends with the extension
                              the command writes (.xml, .wwcatalog, .wwprofile, .wwdiff, .ps1,
                              .reg), else a folder for the file of the default name; without it
                              output next to the program (profiles for a profile and a diff)
  exit codes                  0 when the files were written, 2 for a wrong use of the
                              parameters, 1 for a failure
  winweaver --mcp FILE        serve MCP as a configuration file says (FILE.wwmcp, a path or
                              a name of the mcp folder next to the program)
  winweaver --mcp stdio [--config FILE] [--mode read|edit|files] [--read-pc]
                     [--registry off|read|write] [--edit-library] [--detailed-log] [--profile NAME]
                     [--off winweaver/TOOL]
                              serve MCP on standard input and output
  winweaver --mcp http [--config FILE] [--token TOKEN] [--port N] [--mode ...] [--read-pc]
                     [--registry ...] [--edit-library] [--detailed-log] [--profile NAME]
                              serve MCP on http://127.0.0.1:N/mcp (port 8765 by default)
                              the options change what the configuration file says, for this start;
                              a configuration file of WinWeaver 0.1.46 or earlier is written again
                              in the format of W# first
  winweaver --smoke-window [--renderer ...]
                              open a small window, draw two frames, print what drew them
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
    let settings = winweaver_ui::Settings::load("WinWeaver")
        .0
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
    wwmcp::log::line("ERROR", text);
}

/// `--mcp ...`: the tools of WinWeaver on the server of W#, by the transport of the
/// configuration, with the imports the settings of the window load.
fn mcp(args: &[String]) -> ExitCode {
    let dir = winweaver_ui::exe_dir();
    // A configuration file of an earlier version is written in the format of W# first.
    let (_, failed) = winweaver_mcp::legacy::migrate_folder(&wwmcp::Config::folder(&dir));
    for why in failed {
        wwmcp::log::line("WARN", &format!("not written in the format of W#: {why}"));
    }
    if let Some(file) = args
        .first()
        .filter(|a| !matches!(a.as_str(), "stdio" | "http"))
    {
        let path = wwmcp::cli::config_path(file, &dir);
        if let Err(e) = winweaver_mcp::legacy::migrate(&path) {
            refuse(&e);
            return ExitCode::from(2);
        }
    }
    if let Some(file) = wwmcp::cli::option(args, "--config") {
        let path = wwmcp::cli::config_path(&file, &dir);
        if let Err(e) = winweaver_mcp::legacy::migrate(&path) {
            refuse(&e);
            return ExitCode::from(2);
        }
    }
    let catalog = match Catalog::embedded() {
        Ok(c) => c,
        Err(e) => {
            refuse(&format!("catalog: {e}"));
            return ExitCode::FAILURE;
        }
    };
    let catalog = with_imports(catalog, &winweaver_ui::imports_dir());
    let tools = winweaver_mcp::Tools::new(
        catalog,
        &winweaver_ui::PRESETS,
        winweaver_mcp::Paths {
            marks_path: dir.join(winweaver_core::bookmarks::DEFAULT_FILE),
            imports_dir: winweaver_ui::imports_dir(),
            profiles_dir: dir.join("profiles"),
            output_dir: dir.join("output"),
        },
    );
    let tools = std::sync::Arc::new(tools);
    let sources: Vec<std::sync::Arc<dyn wwmcp::Source>> = vec![tools.clone()];
    let config = match wwmcp::cli::config(args, &dir, &sources) {
        Ok(c) => c,
        Err(e) => {
            refuse(&e);
            return ExitCode::from(2);
        }
    };
    let server = match winweaver_mcp::server(tools, config) {
        Ok(s) => s,
        Err(e) => {
            refuse(&e);
            return ExitCode::from(2);
        }
    };
    match wwmcp::cli::serve(std::sync::Arc::new(server)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            refuse(&e);
            ExitCode::FAILURE
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
    let program = format!("WinWeaver {}", winweaver_core::BUILD);
    wwmcp::log::init(&winweaver_ui::exe_dir().join("logs"), &program, headless);
    winweaver_win::on_fatal_exception(wwmcp::log::fatal);
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
        Some("--mcp") => mcp(&args[1..]),
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
        Some("--elevated-apply") => elevated_apply(&args[1..]),
        Some("--version") => {
            attach_console();
            println!("WinWeaver {}", winweaver_core::BUILD);
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
        Some(command) if cli::COMMANDS.contains(&command) => {
            attach_console();
            cli::run(command, &args[1..])
        }
        Some(other) => {
            attach_console();
            eprintln!("unknown argument {other}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Opens the window; its failure goes to the log, and to the console of the parent if there is one.
fn window(renderer: Option<&str>) -> ExitCode {
    match winweaver_ui::run(renderer) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            attach_console();
            eprintln!("{e}");
            wwmcp::log::window_line("ERROR", &format!("the window failed: {e}"));
            ExitCode::FAILURE
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
