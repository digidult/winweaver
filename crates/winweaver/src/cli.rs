//! The commands of the command line that write files without the window: the answer file of a
//! profile, an import and an export of policy templates, a read of this PC, a diff of two
//! profiles, the stand-alone scripts and the registry files. Each makes what the window makes,
//! through the same functions. A command that writes one file takes the file or its folder in
//! `--out` (`export::out_path`); the files beside it, such as `Undo-Apply.ps1`, go into the same
//! folder. Every file written is named on the standard output, the warnings and the reason of a
//! stop on the standard error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use winweaver_core::admx::{self, CatalogFile};
use winweaver_core::catalog::Catalog;
use winweaver_core::diff::{self, Diff, Side};
use winweaver_core::export::{self, ScriptKind};
use winweaver_core::profile::{self, Profile};
use winweaver_core::validate::{self, Issue, Severity, Target};
use winweaver_core::{clock, regfile, render, resolver, thispc};
use winweaver_ui::ImportSource;

/// The commands of this module, the first word of their command line.
pub const COMMANDS: [&str; 7] = [
    "--build",
    "--import-admx",
    "--export-catalog",
    "--read-pc",
    "--diff",
    "--script",
    "--reg",
];

/// Why a command stopped: a wrong use of the command line or a failure of its work.
enum Stop {
    Usage(String),
    Failed(String),
}

fn usage(text: impl Into<String>) -> Stop {
    Stop::Usage(text.into())
}

fn failed(text: impl Into<String>) -> Stop {
    Stop::Failed(text.into())
}

/// Runs a command and turns its end into the exit code: 0 when it wrote its files, 2 for a wrong
/// use of the command line, 1 for a failure.
pub fn run(command: &str, args: &[String]) -> ExitCode {
    // The input languages and layouts of this Windows, read at the first check or build.
    winweaver_core::resources::set_locale_reader(winweaver_win::locales);
    let result = match command {
        "--build" => build(args),
        "--import-admx" => import_admx(args),
        "--export-catalog" => export_catalog(args),
        "--read-pc" => read_pc(args),
        "--diff" => diff_profiles(args),
        "--script" => export_files(args, false),
        "--reg" => export_files(args, true),
        other => Err(usage(format!("unknown argument {other}"))),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Stop::Usage(text)) => {
            eprintln!("{text}\nwinweaver --help lists the parameters");
            ExitCode::from(2)
        }
        Err(Stop::Failed(text)) => {
            eprintln!("{text}");
            ExitCode::FAILURE
        }
    }
}

/// The words after a command: its positional arguments and its options with their values, as
/// `--name value` or `--name=value`.
struct Words {
    positional: Vec<String>,
    options: Vec<(String, String)>,
}

impl Words {
    /// The words of `args`; an option that is not among `known` or comes without its value is a
    /// wrong use.
    fn parse(args: &[String], known: &[&str]) -> Result<Self, Stop> {
        let mut words = Self {
            positional: Vec::new(),
            options: Vec::new(),
        };
        let mut rest = args.iter().peekable();
        while let Some(arg) = rest.next() {
            let Some(option) = arg.strip_prefix("--") else {
                words.positional.push(arg.clone());
                continue;
            };
            let (name, value) = match option.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => {
                    let next = rest.next_if(|a| !a.starts_with("--"));
                    (option, next.cloned())
                }
            };
            if !known.contains(&name) {
                return Err(usage(format!("unknown option --{name}")));
            }
            let Some(value) = value else {
                return Err(usage(format!("--{name} needs a value")));
            };
            words.options.push((name.to_owned(), value));
        }
        Ok(words)
    }

    /// The value of an option, the last one when it is given twice.
    fn option(&self, name: &str) -> Option<&str> {
        let found = self.options.iter().rev().find(|(n, _)| n == name);
        found.map(|(_, v)| v.as_str())
    }

    /// The file to write: the one `--out` names, else the file `name` in the folder of `--out`
    /// or in `dir`.
    fn target(&self, dir: &Path, name: &str, ext: &str) -> PathBuf {
        export::out_path(self.option("out").map(Path::new), dir, name, ext)
    }
}

/// A path of the command line made absolute against the current folder, as the window keeps the
/// paths of its files.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Makes the folder of a file to write when it is missing.
fn make_parent(path: &Path) -> Result<(), Stop> {
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| failed(format!("{}: {e}", dir.display())))
}

/// Writes a file and names it.
fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), Stop> {
    std::fs::write(path, bytes).map_err(|e| failed(format!("{}: {e}", path.display())))?;
    println!("wrote {}", path.display());
    Ok(())
}

/// The embedded catalog with the imports the settings of the window load.
fn catalog() -> Result<Catalog, Stop> {
    let catalog = Catalog::embedded().map_err(|e| failed(format!("catalog: {e}")))?;
    Ok(crate::with_imports(catalog, &winweaver_ui::imports_dir()))
}

/// A preset by its name, in any case.
fn preset(name: &str) -> Option<(&'static str, &'static str)> {
    let found = winweaver_ui::PRESETS
        .iter()
        .find(|(preset, _)| preset.eq_ignore_ascii_case(name));
    found.copied()
}

/// A profile of the command line with its file: a file (`.wwprofile`), else a preset by its name,
/// else a profile of the folder `profiles` next to the program by its name.
fn profile_arg(text: &str) -> Result<(Profile, Option<PathBuf>), Stop> {
    let given = Path::new(text);
    let path = if given.is_file() {
        absolute(given)
    } else if let Some((name, body)) = preset(text) {
        let profile = Profile::parse(body, name).map_err(|e| failed(format!("{name}: {e}")))?;
        return Ok((profile, None));
    } else {
        let dir = winweaver_ui::profiles_dir();
        let named = dir.join(format!("{text}.{}", profile::EXTENSION));
        if text.contains(['/', '\\', ':']) || !named.is_file() {
            let shown = dir.display();
            let text = format!("{text}: no such file, preset or profile in {shown}");
            return Err(failed(text));
        }
        named
    };
    let profile = Profile::load(&path).map_err(|e| failed(e.to_string()))?;
    Ok((profile, Some(path)))
}

/// An issue of the check as a line of the console: its severity, what it belongs to, its text.
fn issue_line(issue: &Issue) -> String {
    let severity = match issue.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    };
    let target = match &issue.target {
        Target::Profile => String::new(),
        Target::Install => "install: ".to_owned(),
        Target::Accounts => "accounts: ".to_owned(),
        Target::Languages => "languages: ".to_owned(),
        Target::Rule(id) => format!("{id}: "),
        Target::Unknown => "unknown options: ".to_owned(),
    };
    format!("{severity}: {target}{}", issue.text)
}

/// Checks a profile and builds its answer file into `target`, the scripts it embeds beside it, as
/// Build of the window does; an error of the check stops the build, the warnings are printed.
fn answer_file(catalog: &Catalog, profile: &Profile, target: &Path) -> Result<(), Stop> {
    let resolution = resolver::resolve(catalog, profile);
    let issues = validate::check(catalog, profile, &resolution);
    for issue in &issues {
        if issue.severity != Severity::Info {
            eprintln!("{}", issue_line(issue));
        }
    }
    let errors = validate::errors(&issues);
    if errors > 0 {
        let text = format!("the check found {errors} errors: nothing was built");
        return Err(failed(text));
    }
    let build = match render::build(catalog, profile, winweaver_core::BUILD) {
        Ok(b) => b,
        Err(e) => return Err(failed(format!("the build failed: {e}"))),
    };
    for warning in &build.warnings {
        eprintln!("warning: build: {warning}");
    }
    make_parent(target)?;
    let written = build.write(target).map_err(failed)?;
    let rules = build.rule_ids.len();
    println!("wrote {} ({rules} rules)", target.display());
    for path in written.iter().skip(1) {
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// `--build PROFILE [--out FILE|FOLDER]`: the answer file of a profile.
fn build(args: &[String]) -> Result<(), Stop> {
    let words = Words::parse(args, &["out"])?;
    let [name] = words.positional.as_slice() else {
        let text = "--build needs a profile: a .wwprofile file or the name of a preset";
        return Err(usage(text));
    };
    let catalog = catalog()?;
    let (mut profile, _) = profile_arg(name)?;
    profile.split_unknown(&catalog);
    let dir = winweaver_ui::output_dir();
    let target = words.target(&dir, render::ANSWER_FILE, "xml");
    answer_file(&catalog, &profile, &target)
}

/// A source of policy templates on the command line.
enum Source {
    ThisWindows,
    Folder(PathBuf),
    Admx(PathBuf),
    Archive(PathBuf),
    Catalog(PathBuf),
}

impl Source {
    /// `system` for the templates of this Windows, a folder of ADMX files, or a file by its
    /// extension: `.admx`, `.zip` or `.wwcatalog`.
    fn of(text: &str) -> Result<Self, Stop> {
        if text.eq_ignore_ascii_case("system") {
            return Ok(Self::ThisWindows);
        }
        let path = absolute(Path::new(text));
        if path.is_dir() {
            return Ok(Self::Folder(path));
        }
        if !path.is_file() {
            let shown = path.display();
            return Err(failed(format!("{shown}: no such file or folder")));
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "admx" => Ok(Self::Admx(path)),
            "zip" => Ok(Self::Archive(path)),
            "wwcatalog" => Ok(Self::Catalog(path)),
            _ => {
                let shown = path.display();
                let text = format!("{shown}: not an .admx, .zip or .wwcatalog file");
                Err(usage(text))
            }
        }
    }

    /// The catalog file of the source, as an import makes it; a catalog file whose id one of
    /// `taken` has gets the prefix `package-`. Templates of which no policy was read are a
    /// failure, with the first reasons.
    fn read(&self, taken: &[String]) -> Result<CatalogFile, Stop> {
        let source = |kind, path: &PathBuf| {
            winweaver_ui::import_source(kind, Some(path.as_path()), "", taken)
        };
        let file = match self {
            Self::ThisWindows => {
                let build = winweaver_ui::windows_build();
                winweaver_ui::import_source(ImportSource::ThisWindows, None, &build, taken)
            }
            Self::Folder(p) => source(ImportSource::Folder, p),
            Self::Archive(p) => source(ImportSource::Archive, p),
            Self::Catalog(p) => source(ImportSource::CatalogFile, p),
            Self::Admx(p) => winweaver_ui::admx_file_catalog(p).map(|mut f| {
                f.id = winweaver_ui::package_id(&f.id, taken);
                f
            }),
        };
        let file = file.map_err(failed)?;
        let templates = !matches!(self, Self::Catalog(_));
        if templates && file.rules.is_empty() {
            let reasons: Vec<String> = file
                .skipped
                .iter()
                .take(5)
                .map(|(policy, why)| format!("{policy}: {why}"))
                .collect();
            let text = format!("no policy was read: {}", reasons.join("; "));
            return Err(failed(text));
        }
        Ok(file)
    }
}

/// Names an import among the ones the settings of the window load, so the window and the MCP
/// server read it at their next start. Settings of an earlier version, which load every file but
/// the hidden ones, get that list first, as the window does at its start. Whether they changed.
fn load_at_start(settings: &mut winweaver_ui::Settings, dir: &Path, id: &str) -> bool {
    let earlier = settings.extra.loaded_imports.is_none();
    let mut ids = winweaver_ui::loaded_import_ids(settings, dir);
    let named = ids.iter().any(|x| x == id);
    if !named {
        ids.push(id.to_owned());
    }
    if earlier {
        settings.extra.hidden_imports.clear();
    }
    settings.extra.loaded_imports = Some(ids);
    earlier || !named
}

/// `--import-admx system|PATH`: imports policy templates as Settings, Policy templates does: the
/// catalog file goes into the imports folder, and the settings name it among the loaded imports.
fn import_admx(args: &[String]) -> Result<(), Stop> {
    let words = Words::parse(args, &[])?;
    let [text] = words.positional.as_slice() else {
        let text = "--import-admx needs system, a folder of ADMX files or a file of templates";
        return Err(usage(text));
    };
    let source = Source::of(text)?;
    let dir = winweaver_ui::imports_dir();
    let (settings, settings_path) = winweaver_ui::Settings::load("WinWeaver");
    let taken = match &settings {
        Ok(s) => winweaver_ui::loaded_import_ids(s, &dir),
        Err(_) => Vec::new(),
    };
    let file = source.read(&taken)?;
    let path = winweaver_ui::save_import(&dir, &file).map_err(failed)?;
    let (rules, skipped) = (file.rules.len(), file.skipped.len());
    let shown = path.display();
    println!("wrote {shown} ({rules} rules, {skipped} skipped)");
    let id = &file.id;
    match settings {
        Ok(mut settings) => {
            let changed = load_at_start(&mut settings, &dir, id);
            if changed && let Err(e) = settings.save_to(&settings_path) {
                let text = format!("{}: {e}", settings_path.display());
                return Err(failed(text));
            }
            println!("the import {id} loads at the next start of the window");
        }
        Err(e) => {
            eprintln!("warning: the settings were not read: {e}");
            eprintln!("check the import {id} in Settings, Policy templates");
        }
    }
    Ok(())
}

/// `--export-catalog ID|PATH [--out FILE|FOLDER]`: the catalog file of an import of the imports
/// folder, as Export of Settings, Policy templates writes it, or of templates read as an import
/// reads them, without importing them.
fn export_catalog(args: &[String]) -> Result<(), Stop> {
    let words = Words::parse(args, &["out"])?;
    let [text] = words.positional.as_slice() else {
        let text = "--export-catalog needs the id of an import, system, a folder or a file";
        return Err(usage(text));
    };
    let name = format!("{text}.{}", admx::CATALOG_EXTENSION);
    let import = winweaver_ui::imports_dir().join(name);
    let plain = !text.contains(['/', '\\', ':']) && !Path::new(text).exists();
    let file = if plain && import.is_file() {
        let mut file = CatalogFile::load(&import).map_err(failed)?;
        file.id = text.clone();
        file
    } else {
        Source::of(text)?.read(&[])?
    };
    let name = format!("{}.{}", file.id, admx::CATALOG_EXTENSION);
    let dir = winweaver_ui::output_dir();
    let target = words.target(&dir, &name, admx::CATALOG_EXTENSION);
    make_parent(&target)?;
    file.save(&target).map_err(failed)?;
    let (rules, skipped) = (file.rules.len(), file.skipped.len());
    let shown = target.display();
    println!("wrote {shown} ({rules} rules, {skipped} skipped)");
    Ok(())
}

/// `--read-pc [--base PROFILE] [--out FILE|FOLDER]`: reads this PC as Read this PC of the window
/// does, into a new profile on a base (the preset Office, the first tab of the window, by
/// default), named after the computer; written as a profile, or built into an answer file when
/// `--out` names an `.xml` file.
fn read_pc(args: &[String]) -> Result<(), Stop> {
    let words = Words::parse(args, &["base", "out"])?;
    if !words.positional.is_empty() {
        let text = "--read-pc takes only --base PROFILE and --out FILE|FOLDER";
        return Err(usage(text));
    }
    let catalog = catalog()?;
    let mut base = match words.option("base") {
        Some(text) => profile_arg(text)?.0,
        None => {
            let (name, body) = winweaver_ui::PRESETS[0];
            let preset = Profile::parse(body, name);
            preset.unwrap_or_else(|_| Profile::from_defaults(name, &catalog))
        }
    };
    base.split_unknown(&catalog);
    let Some(machine) = winweaver_win::this_pc() else {
        return Err(failed("this PC can be read on Windows only"));
    };
    let now = clock::now_utc_iso();
    let ids = thispc::whole_pc_rules(&catalog);
    let plan = thispc::plan(&catalog, &base, &ids);
    let report = thispc::run(&*machine, &plan, true, &now);
    let readings = thispc::judge(&catalog, &plan, &report.results);
    let system = report.system.as_ref();
    let (mut profile, summary) = thispc::capture(&catalog, &base, &readings, system);
    profile.name = thispc::pc_profile_name(&report.computer);
    profile.created = now.clone();
    profile.modified = now;
    println!("read this PC: {}", summary.line());
    if !summary.forms.is_empty() {
        println!("forms: {}", summary.forms.join(", "));
    }
    for note in &summary.notes {
        println!("note: {note}");
    }
    for (rule, differing) in &summary.partly {
        let what = differing.join("; ");
        let text = format!("{rule}: partly in effect, so read as off: {what}");
        eprintln!("warning: {text}");
    }
    if !report.admin && plan.admin_checks() > 0 {
        let note = "note: features and capabilities of Windows are read only as administrator";
        println!("{note}");
    }
    if !plan.excluded.is_empty() {
        let n = plan.excluded.len();
        let note = "rules act only during the installation or at a first sign-in";
        println!("note: {n} {note}");
    }
    let out = words.option("out").map(Path::new);
    if let Some(xml) = out.filter(|p| export::names_file(p, "xml")) {
        return answer_file(&catalog, &profile, xml);
    }
    let stem = export::file_stem(&profile.name);
    let name = format!("{stem}.{}", profile::EXTENSION);
    let dir = winweaver_ui::profiles_dir();
    let target = words.target(&dir, &name, profile::EXTENSION);
    make_parent(&target)?;
    profile.save(&target).map_err(|e| failed(e.to_string()))?;
    println!("wrote {}", target.display());
    Ok(())
}

/// `--diff A B [--out FILE|FOLDER]`: the differences from the profile A to the profile B as a
/// diff file, as Compare of the window makes them.
fn diff_profiles(args: &[String]) -> Result<(), Stop> {
    let words = Words::parse(args, &["out"])?;
    let [a, b] = words.positional.as_slice() else {
        let text = "--diff needs two profiles: .wwprofile files or names of presets";
        return Err(usage(text));
    };
    let (base, base_path) = profile_arg(a)?;
    let (other, other_path) = profile_arg(b)?;
    let base_side = Side::of(&base, base_path.as_deref());
    let other_side = Side::of(&other, other_path.as_deref());
    let made = Diff::between(&base, &other, base_side, other_side);
    let name = format!(
        "{}-vs-{}.{}",
        export::file_stem(&base.name),
        export::file_stem(&other.name),
        diff::EXTENSION
    );
    let dir = winweaver_ui::profiles_dir();
    let target = words.target(&dir, &name, diff::EXTENSION);
    make_parent(&target)?;
    made.save(&target).map_err(|e| failed(e.to_string()))?;
    let n = made.count();
    println!("wrote {} ({n} differences)", target.display());
    Ok(())
}

/// `--script apply|revert|audit FILE` and `--reg apply|revert FILE`, with `--only IDS` and
/// `--out FILE|FOLDER`: the stand-alone script (with the undo script beside an apply or a
/// return) or the registry file of a profile or a diff, for the rules of `--only` or every rule
/// (a diff: its rules), with the catalog of the imports the settings load.
fn export_files(args: &[String], reg: bool) -> Result<(), Stop> {
    let text = if reg {
        "--reg needs apply or revert and a .wwprofile or .wwdiff file"
    } else {
        "--script needs apply, revert or audit and a .wwprofile or .wwdiff file"
    };
    let words = Words::parse(args, &["only", "out"])?;
    let [word, file] = words.positional.as_slice() else {
        return Err(usage(text));
    };
    let kind = ScriptKind::parse(word).ok_or_else(|| usage(text))?;
    if reg && kind == ScriptKind::Audit {
        return Err(usage(text));
    }
    let catalog = catalog()?;
    let path = PathBuf::from(file);
    let is_diff = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(diff::EXTENSION));
    let (profile, mut ids) = if is_diff {
        let diff = Diff::load(&path).map_err(|e| failed(e.to_string()))?;
        export::diff_profile(&diff)
    } else {
        let profile = Profile::load(&path).map_err(|e| failed(e.to_string()))?;
        (profile, export::all_rules(&catalog))
    };
    if let Some(only) = words.option("only") {
        let named: Vec<String> = only
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        let wanted = export::rules_of(&catalog, &named).map_err(usage)?;
        ids.retain(|id| wanted.contains(id));
    }
    if !export::applicable(&catalog, &ids) {
        let text = "nothing to apply on a PC: the rules are passes of the installation";
        return Err(failed(text));
    }
    let stem = export::file_stem(&path.file_stem().unwrap_or_default().to_string_lossy());
    let dir = winweaver_ui::output_dir();
    if reg {
        let made = export::reg(&catalog, &profile, &ids, kind == ScriptKind::Revert, &stem);
        if made.values == 0 {
            let text = "nothing for a registry file: the rules write no registry value";
            return Err(failed(text));
        }
        let target = words.target(&dir, &made.name, regfile::EXTENSION);
        make_parent(&target)?;
        made.file.save(&target).map_err(failed)?;
        let n = made.values;
        println!("wrote {} ({n} values)", target.display());
        return Ok(());
    }
    let version = winweaver_core::BUILD;
    let made = export::scripts(&catalog, &profile, &ids, kind, &stem, version).map_err(failed)?;
    let Some((first, beside)) = made.files.split_first() else {
        return Err(failed("no script was made"));
    };
    let target = words.target(&dir, &first.name, "ps1");
    let own = target.file_name().unwrap_or_default();
    let clash = beside.iter().any(|f| own.eq_ignore_ascii_case(&f.name));
    if clash {
        let name = own.to_string_lossy();
        let text = format!("--out: the script cannot take the name {name}");
        return Err(usage(text));
    }
    make_parent(&target)?;
    write_bytes(&target, &first.bytes)?;
    let folder = target.parent().unwrap_or(Path::new(""));
    for f in beside {
        write_bytes(&folder.join(&f.name), &f.bytes)?;
    }
    Ok(())
}
