# 04. Architecture

## 1. Workspace layout

```
crates/
├── ww-base/             W# base, pure Rust: jsonfile (strict JSON, the trait Document, atomic saves), paths
│                        (a file inside the program folder kept relative to it), clock; re-exported by the core
├── ww/                  W# (the framework, docs/ww.md): shell/ (the trait App, Shell, Window as eframe::App,
│                        ShellAct, ShellDoc, the chrome drawn from what the program gives, the settings tab,
│                        the themes), run (run::<A>, smoke_window::<A>), doc (Document, Search, Draft),
│                        settings (Settings<X: Extra>), session (drafts, the log), store (the files of the
│                        window: next to the executable natively, the storage of the page and downloads in
│                        a browser), web (the start in a browser, wasm32), style, themes and their editor,
│                        coloricons, i18n, tree, list, widgets, markdown and mdview, renderer, commands,
│                        keys, icons, dialogs, folders; data/themes (Light, Dark)
├── wedit/               the text editor on W#: main (the command line), app (impl ww::App), textdoc
├── winweaver-core/      catalog/ (model, embedded data, checks, values), profile, resolver, validate,
│                        resources (time zones, editions, the input languages and layouts Windows lists),
│                        diff, bookmarks, i18n, render/ (the answer file, one file a pass), thispc/ (plan,
│                        evaluate, judge, capture, report, an in-memory machine), scripts, import, apply,
│                        admx/ (xml, templates, rules, file, compare, archive, merge), registry/ (data,
│                        search, memory), regfile, security, users, userstore: pure Rust, no Windows API,
│                        tested on any runner
├── winweaver-win/       Windows API: ffi (the wrappers once), native (thispc::Machine), registry, accounts,
│                        dism, apps, locales, users, execute/ (the plan run natively, one function an
│                        operation, the hive of the default user), the start of the helper elevated
├── winweaver-mcp/       the MCP server: lib (protocol, modes, workspace, journal), config (.wwmcp), log
│                        (the JSON lines log, the crash handlers), tools/ (the table of the 36 tools and their
│                        handlers by theme, the resources), stdio and http transports
├── winweaver-ui/        WinWeaver on the window of W#: lib (the types of the window), app/ (the methods of
│                        WinWeaver by theme, each taking the shell; shell: impl ww::App, the draw by kind,
│                        apply as a dispatcher; menu), act (Shell(ShellAct) for the acts of the window, the
│                        acts by subsystem for the rest), question, doc (every tab a document, impl
│                        ww::Document), commands and icons (the command bar as data), widgets, views/ (the
│                        tree and the detail panel of a profile), the views of the other tabs (diff,
│                        bookmarks, MCP, catalog, its editor, the registry, registry files, users files),
│                        forms, pages (the guide, the licence, the reference), settings (the fields of
│                        WinWeaver in the settings file)
└── winweaver/           the binary: cli (the commands that write files: --build, --import-admx,
                         --export-catalog, --read-pc, --diff, --script, --reg), argument parsing (the
                         window, --mcp, --elevated-apply, --check-catalog, --smoke-window,
                         --version), logging and crash handlers set up first
data/
├── catalog/             groups.json, 00-17 rule files, lang/<code>.json
├── presets/             office, strict, laptop, home .wwprofile
├── lang/<code>/         ui.json and guide.md of each built-in language
├── themes/              latte.wwtheme, matrix.wwtheme (Light and Dark are in crates/ww/data/themes)
├── resources/           timezones.json
├── templates/           the runtime templates: the answer file skeleton and the scripts with {{slot}}s, VERSION
├── reference/           the reference of the parameters, linked from the doc field of a rule
└── docs/                guide.md, the English guide
agents/                  shipped in the archive: AGENTS.md, README.md, skills/, pi-agent/
package/                 what the archive of WinWeaver adds: README.md for the user, lang/be/ui.json
package-wedit/           what the archive of wedit adds: README.md, lang/uk/ui.json
```

`ww-base` and `winweaver-core` compile on any system, so the tests of the core run on the Linux runner too;
`winweaver-win`, `ww` and the programs are built for Windows, and their tests run on the Windows runner. The Linux
job checks every crate for the Windows target without linking. The window itself is the framework W#
(`ww`, `docs/ww.md`): a program implements `ww::App` with its documents, its acts and what it draws inside
a tab, and the shell owns the tabs, the settings, the themes, the session and the chrome. WinWeaver and
the editor wedit are its two programs.

## 2. Crates (dependencies policy P4)

| Need | Crate | Why |
|---|---|---|
| Window, rendering, widgets | `eframe` and `egui` 0.36 (accesskit, default fonts, links; glow, and wgpu with its `dx12` backend alone), `wgpu` | The stack of the window: OpenGL through glow, DirectX 12 through wgpu, winit, AccessKit |
| Markdown | `pulldown-cmark` | Parsing of the pages; W# draws them itself (`ww::markdown`) |
| File dialogs | `rfd` | The open and save dialogs of Windows |
| Window handle | `raw-window-handle` | The handle of the window for the colours of its title bar |
| JSON | `serde`, `serde_json` (with `preserve_order`), `indexmap` | Documents in the order of the program; strict reading in `ww_base::jsonfile` |
| Errors | `thiserror` | The error types of the documents |
| XML of the policy templates | `quick-xml` | Reading ADMX and ADML; no DTD, entities refused |
| Archives of templates | `zip` (deflate only) with `flate2` | Zip archives read in memory, pure Rust |
| Hashes and encryption | `sha2`, `pbkdf2`, `hmac`, `aes`, `cbc`, `base64`, `zeroize`, `getrandom` | The digest of a plan, the encryption of the passwords of a users file, passwords wiped from memory, salts, vectors, nonces and tokens |
| Registry and Windows API | `windows-sys` (narrow feature sets), `windows` (the package API, DirectWrite for the icons in colour), `winreg` | Registry, security descriptors, the network management API, DWM, ShellExecuteEx, consoles |
| Tests of the window | `egui_kittest` | Headless runs of the window with AccessKit queries and screenshots |

The HTTP transport of the MCP server, the logs and the clock are written with the standard library; no
async runtime. Every crate is Rust; the Windows bindings link only system DLLs, and nothing links a C
library that needs a redistributable.

## 3. Threads and state

- The UI thread owns the state of the window (the tabs, the catalog with the loaded imports, the
  settings). Immediate mode: every frame reads the state and draws it; what a frame requests is a list of
  acts applied after it: `Act::Shell(ShellAct)` by the shell, the others by `WinWeaver::apply`, which hands
  each to the `apply_<subsystem>` of its theme.
- Long work runs in a thread of its own and sends its result through a channel that the window polls after
  every frame: the read of this PC (the thread `this-pc`, one read at a time, the result going to the tab
  it started from by its draft key), the search of the registry (`registry-search`, with a stop), the
  import of policy templates, and the run of the elevated helper (the thread waits for the helper and
  reads its report).
- The MCP server keeps its state (its workspace of profiles, its journal) behind a mutex, independent of
  the tabs; the window reads its workspace and journal for the MCP tab. The HTTP transport has a listening
  thread and a thread for each connection (at most 8, each with a timeout of 15 s); the stdio transport
  runs in the main thread of a process without the window.
- DISM is initialized once a process at its first use and never shut down, since it promises no second
  start; it is asked only when the process is elevated and the plan has such checks.

## 4. Data flow of a profile

catalog (embedded + loaded imports) + profile file -> resolver (dependencies, conflicts, linked policies)
-> rows of the tree (states, counts) -> UI. A change: UI -> act -> profile change -> resolver -> rows.
Check: profile -> `validate` -> messages. Build: profile -> `validate` -> render (the runtime templates
filled pass by pass, the script lines of the rules in effect, the profile embedded) -> `autounattend.xml`.
This PC: profile -> plan of checks -> native reads (or the report of the audit script) -> judgement ->
capture (states, parameters, forms) -> profile. Apply: selection -> plan of operations -> plan file and its
digest -> elevated helper -> backup -> native writes -> report.

## 5. Elevation and the native apply

- Applies, returns to defaults and undos run in an elevated helper: the same executable started with
  `--elevated-apply <plan file> <SHA-256 of its bytes>` through `ShellExecuteExW` with the verb `runas`,
  hidden, and waited for. The plan is `output/plan-<stamp>.json`; the helper writes the backup before the
  first change, applies each operation with the Windows API (registry values, lists and keys, the start
  type of a service, a program of the system folder, capability removal through the DISM API, app removal
  through the package API, the flags of local accounts through the network management API), and reports
  per step in `plan-<stamp>-report.json`. The window shows the report.
- The helper refuses a plan file from another folder or with another name, and checks its bytes: it reads
  the file once and runs it only when its digest matches the one on its command line, so a plan another
  process changed in the output folder after the permission of Windows is refused. The plan keeps a random
  nonce, so no two plans are equal. The helper never runs scripts.
- The default user profile: the plan writes the `DU:` values at `HKU:\UnattendDefault`, the key of the
  stand-alone apply script; the helper enables its backup and restore privileges, loads `NTUSER.DAT` of
  the default profile (`GetDefaultUserProfileDirectoryW`) there with `RegLoadKeyW` for the steps that
  write it and unloads it after the last step. A hive already loaded at that key is used and left
  loaded. A backup names the same keys as one of the script, so either restores the other.
- The script steps of the catalog (`ps` actions) are not run: `apply::script_ops` gives each known one
  native steps that do what its PowerShell does (a program of the system folder on a folder of the
  installation media, the start type of a service that has one now, a value in every subkey of a key,
  a key deleted after the backup of every value under it, the flags of local accounts, the error file of
  the installation and the copies of the answer file). A script step it does not know stays with the
  stand-alone script; a test fails when a script step This PC applies has no entry. The changes no undo
  takes back (programs, accounts, deleted files, capabilities, apps) are named in the backup and counted
  in the confirmation; the helper deletes no file but the copies of the answer file the rule names, never
  disables the account it runs as, and looks for media folders only below the roots of the drives.
- The operations of a users file run in the process of the window and need it to be elevated, so the
  master password crosses no process boundary: no helper and no plan file for them.
- No PowerShell anywhere in these paths (principle P2).

## 6. Answer file rendering

The renderer fills the runtime templates of `data/templates/` (the skeleton `autounattend.template.xml`
and the scripts `Setup-System`, `Setup-User`, `Post-OOBE` with their sections) at their `{{slot}}`s: the
settings of the passes and their components, the blocks of the rules in effect assembled from their script
lines, the profile data, and the profile embedded as ASCII JSON. The scripts call the functions of the
runtime (`Set-Reg`, `Remove-Reg`, `Set-RegList`, `Set-ServiceStart`, `Invoke-Exe`, `Set-AccountText` and
the others). The runtime has a version (`data/templates/VERSION`) named in the built file with the build of
the program and the catalog version. Commands longer than 259 characters and conflicting settings are build
errors.

## 7. Build and CI

- `cargo build --release --workspace`: the static C runtime (`.cargo/config.toml`), thin LTO, one codegen
  unit, symbols stripped. A release build has no console (`windows_subsystem = "windows"`); the command
  line functions attach to the console of the parent when there is one, and the MCP stdio transport uses
  the pipes the client gives. The features `glow` and `wgpu` choose the renderers, both by default.
  The feature `full` of winweaver-ui (on by default, named by winweaver) holds everything beyond the
  profile editor, the check and the build; wwlite is built without it and alone (`cargo build -p
  wwlite`, `cargo test -p wwlite`, `cargo clippy -p wwlite`), since cargo unifies the features of
  one build; users files are the feature `users` of the core and of the Windows crate.
- The browser edition of wwlite: `cargo build --release -p wwlite --no-default-features --features glow
  --target wasm32-unknown-unknown` (WebGL through glow; the wgpu of WW is DirectX 12 only), then
  `wasm-bindgen --target web` of the version `Cargo.lock` names writes the module and the glue next to
  `crates/wwlite/web/index.html`, which loads them; the folder is served over http. `ww::web::start`
  (eframe's `WebRunner`) replaces `ww::run`, `ww::store` keeps the files of the window in the storage of
  the page, a save is a download, the files to open come by their bytes (`App::open_bytes`), and the input
  languages and the regions come from `data/resources/locales.json` and `regions.json`, embedded for
  this target alone.
- `winweaver_core::BUILD` is `<version>+<12 digits of the commit>`, from `build.rs` of the core and
  `WINWEAVER_COMMIT` set by CI.
- CI (GitHub Actions, `AGENTS.md` section 5): a Linux job first (`cargo fmt --check` with a patch as an
  artifact, clippy of the whole workspace for `x86_64-pc-windows-gnu` and of wwlite for
  `wasm32-unknown-unknown`, the tests of the core), then a Windows job (clippy, `cargo test --workspace`
  with the screenshots of the window, the catalog check, the release build, the window smoke test, the
  archives `WinWeaver-<version>.zip`, `wedit-<version>.zip` and `wwlite-<version>.zip`) and a job on
  Ubuntu that builds the browser edition (`wwlite-web-<version>`); a tag `v<version>` publishes a release
  with `docs/releases/v<version>.md`.
- The zip is the deliverable: unzip, run. Nothing is installed.

## 8. Testing

- Tests of `winweaver-core` on the real data (`tests/`: the catalog, diffs, the build, the scripts, This
  PC, the check) and unit tests of each module: catalog integrity (ids, groups, roots by mechanism,
  placeholders only in allowed fields, translations of every rule), resolver cases, profile round trips,
  strict JSON, diffs, validation, the build of the presets and of profiles with every option, catalog
  files, ADMX parsing, registry files, the encryption of users files against a reference vector. The tests
  read the input languages of Windows from `tests/data/locales.json`.
- An in-memory machine (`thispc::memory`) and registry (`registry::memory`, with rights) stand in for this
  PC, so no test of the logic writes to the real registry.
- Tests of `winweaver-win` on the Windows runner: the native read of the whole catalog, the features and
  capabilities through DISM as administrator, the native apply in a test key of HKCU and of the default
  profile, the accounts of a users file with a test account made and deleted.
- Window tests with `egui_kittest`: a `Window` of WinWeaver (or wedit) drawn without a screen, clicked by
  the labels of AccessKit, so no window ever opens; the translations test of every text of
  `winweaver-ui` and `ww`; the screenshots of the main tabs and of sample pages into `target/screenshots`
  (the artifact `screenshots`) when `WINWEAVER_SCREENSHOTS` is set.
- MCP tests: the protocol and the list of tools, the modes, the switches of the registry, the bookmarks
  and the catalogs, the folders a write may reach, and both transports on a free loopback port (the token
  and the loopback host of HTTP).
- The window smoke test of CI (`--smoke-window`) fails when no renderer starts.

## 9. Known limitations

- The window has two renderers: OpenGL (glow) takes about half the memory of wgpu but needs a driver of
  OpenGL 2.0 or later, which Windows lacks without the driver of the graphics card (it offers OpenGL 1.1);
  DirectX 12 (wgpu with that backend alone) also draws there, on WARP, slower but working. Automatic tries
  OpenGL, then DirectX 12; the settings and `--renderer` choose the first to try, and a renderer that fails
  to start gives way to the next one. An old driver may start OpenGL and draw wrongly: then DirectX 12 is
  chosen by hand.
- The read of this PC takes the default user profile from the current user (with a note), and leaves out
  the installation passes, the first sign-in and the sign-in screen (`HKU:\.DEFAULT`), which the apply
  script also leaves out.
- The features and capabilities are read only when the window runs as administrator.
