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
  archives `WinWeaver-<version>.zip`, `wedit-<version>.zip` and `wwlite-<version>.zip`), a Windows job of
  wview beside it (`docs/plugins.md`) and a job on Ubuntu that builds the browser edition
  (`wwlite-web-<version>`); a tag `v<version>` publishes a release with `docs/releases/v<version>.md`.
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

## 10. The folders and the modules

The repository, folder by folder and module by module.

```
(repository root)
├── AGENTS.md              this file
├── README.md              purpose, status, links
├── LICENSE                MIT
├── Cargo.toml             the workspace (edition 2024, resolver 3, rust-version 1.88), shared dependencies
├── rust-toolchain.toml    stable with rustfmt and clippy
├── rustfmt.toml           max_width 100
├── .cargo/config.toml     static CRT for the Windows target
├── .gitattributes         LF for every text file (data/templates too), CRLF for .cmd, .bat, .ps1
├── build-all.cmd          what CI makes, built on a computer of Windows: the archives of a release
│                          in dist/, on demand the checks, the screenshots, the slides (section 5)
├── .github/workflows/ci.yml  CI: fmt (a patch artifact), clippy (wwlite for wasm32 too) and tests of
│                          winweaver-core on Linux; clippy, tests, the catalog check, the release build and
│                          the zip on Windows; the job build-wview beside it (wview, its archive and its
│                          SDK); the job build-web (wwlite for the browser, the artifact
│                          wwlite-web-<version>); a tag v<version> publishes a release with
│                          docs/releases/v<version>.md
├── crates/
│   ├── ww-base/           W# base, pure Rust: jsonfile (strict JSON, the trait Document, atomic saves),
│   │                      paths (a file inside the program folder kept relative to it), clock; the core
│   │                      re-exports them as `winweaver_core::{jsonfile, paths, clock}`
│   ├── ww/                W# (the framework, `docs/ww.md`): shell/ (mod: the trait App, Shell, ShellAct,
│   │                      ShellDoc, Nav, Messages; window: Window<A> as eframe::App, the acts of the window,
│   │                      the keys, the question, the session restored; tabs, bar, nav, status, jobs: the chrome
│   │                      drawn from what the program gives; settings: the settings tab with General;
│   │                      theme: the theme shown, its editor, the title bar colours), run.rs (run::<A>,
│   │                      smoke_window::<A>), doc.rs (Document, Search, Draft, Scrolled), settings.rs
│   │                      (Settings<X: Extra>), session.rs (drafts, the log), store.rs (the files of the
│   │                      window: next to the executable natively, the storage of the page in a browser,
│   │                      downloads), web.rs (the start in a browser, wasm32), style.rs, themes.rs,
│   │                      themeview.rs, i18n.rs, tree.rs, list.rs, widgets.rs, mdview.rs, renderer.rs,
│   │                      commands.rs, keys.rs, icons.rs, coloricons.rs (the icons in colour through
│   │                      DirectWrite), dialogs.rs, folders.rs; data/themes (Light, Dark)
│   ├── wedit/             the editor on W#: main.rs (the command line, ww::run), app.rs (Wedit: Doc, Act,
│   │                      Extra, impl ww::App), textdoc.rs (a text file in the editor of egui)
│   ├── winweaver-core/    pure Rust, compiles everywhere: catalog/ (model, embedded data, checks, values: the
│   │                      texts, numbers and items of the values of actions, read once), jsonfile (strict
│   │                      reading, the trait Document: the parse with its format and version check, the load
│   │                      and the atomic save of the four JSON documents, DocError), i18n (catalog
│   │                      translations), paths (a file inside the program folder kept relative to it),
│   │                      profile (.wwprofile, set_param and set_group of an edit), resolver, validate (the
│   │                      check of a profile by what it checks), resources (time zones, editions with their
│   │                      keys, the input languages and layouts Windows lists), regions (the regions of
│   │                      Windows with their GeoIDs, read like the locales; re-exported by resources),
│   │                      diff (.wwdiff, ItemKey: the
│   │                      keys of its items), bookmarks (.wwmarks), clock, render/ (autounattend.xml from the
│   │                      templates: text, params, ps, xml, languages, name, accounts, answer: one function a
│   │                      pass), thispc/ (plan, report, evaluate, judge, capture, memory: the checks of a
│   │                      running Windows, the judgement of the rules, the capture into a profile, the report
│   │                      of the audit script, an in-memory machine), scripts (stand-alone Apply, Revert,
│   │                      Audit and Undo scripts), import (an answer file or an old JSON profile as a
│   │                      profile), apply (the plan of a native apply, return or undo, the backup format of
│   │                      the scripts), admx/ (xml, templates, rules, file, compare, archive, merge: ADMX and
│   │                      ADML to rules, catalog files .wwcatalog, their check, merge), registry/ (data: the
│   │                      one type of a value in the types of regedit, search, memory: keys and values, paths
│   │                      of regedit, the rules that write a key, a registry in memory), regfile (`.reg`
│   │                      files: every line kept, the registry steps of a plan as a file), security (the
│   │                      owner and the access list of a key in SDDL), users (.wwusers files: the accounts,
│   │                      the checks, the plan of an operation, the script), userstore (their passwords
│   │                      encrypted), export (the scripts and registry files of a profile, a selection
│   │                      or a diff, for the window and the command line), pol (Group Policy files,
│   │                      PReg); tests/ on the real data, src/<module>/tests.rs for the folders
│   ├── winweaver-win/     the Windows API: ffi.rs (the wrappers once: wide strings, the NetApi buffer, a
│   │                      SID, the accounts enumerated, the text of an error, a DACL set), native.rs
│   │                      (thispc::Machine: the elevation, the locale names, the language of Windows, the
│   │                      local accounts and their groups), registry.rs (NativeRegistry, the rights of a
│   │                      key), accounts.rs (the flags of accounts), dism.rs (the features and capabilities
│   │                      through dismapi.dll), apps.rs (the removal of apps through the package API of
│   │                      Windows), execute/ (the executor of an apply plan with its backup, one function an
│   │                      operation in ops.rs, the hive of the default user in hive.rs, the start of the
│   │                      executable as administrator), locales.rs (the input languages and layouts of
│   │                      Windows), regions.rs (the regions of Windows: EnumSystemGeoNames, GetGeoInfoEx),
│   │                      users.rs (the accounts of this PC read, created and changed for a users file)
│   ├── winweaver-mcp/     the MCP server: lib.rs (protocol, modes, workspace, journal with Outcome, token),
│   │                      config.rs (the .wwmcp configuration files), log.rs (the JSON lines log, the crash
│   │                      handlers), tools/ (mod.rs: the table TOOLS of the 36 tools, each with its name,
│   │                      description, schema, what it needs and its handler; Args, ToolError with its Kind;
│   │                      profiles, rules, bookmarks, catalogs, registry: the handlers by theme; the 4
│   │                      resources), stdio.rs, http.rs (127.0.0.1, bearer token, Host and Origin checks);
│   │                      tests/server.rs (both transports on a free loopback port)
│   ├── winweaver-ui/      WinWeaver on the window of W#: lib.rs (the types: Sel, ProfileDoc, WinWeaver and
│   │                      WinWeaver::new, run and smoke_window through ww, the helpers), app/ (the methods
│   │                      of WinWeaver by theme, one file one impl block, each taking the shell: shell
│   │                      (impl ww::App: the first tab, the files loaded at a start, open, save, draw by
│   │                      kind, the command bar, the menus, the navigation row, the messages, the status
│   │                      bar, the settings sections, the keys, apply: the dispatcher to the apply_* of
│   │                      each subsystem, the dialogs, the polls after a frame), menu (the menu of the tab
│   │                      row, the menus This PC, Compare and Catalogs), frame (the active tab drawn by
│   │                      its kind), profile, files (open_any: a file of any kind), pc, users, diff,
│   │                      imports, registry, marks, session (one tab of a kind, a draft restored), mcp,
│   │                      pol (the Group Policy files in tabs),
│   │                      settings (the lines of WinWeaver in General, the section Files, the rename of
│   │                      an import)), act.rs (Act: Shell(ShellAct<WinWeaver>) for the acts of the window,
│   │                      one variant a subsystem for the rest: ProfileAct, FileAct, SettingsAct, PcAct,
│   │                      UsersAct, DiffAct, MarksAct, McpAct, CatalogAct, ImportAct, RegistryAct,
│   │                      RegFileAct), question.rs (the questions of WinWeaver; From<Question> for Act),
│   │                      doc.rs (Doc: every tab a boxed document and Shell(ShellDoc); impl ww::Document
│   │                      with the icon, the search hint, the draft; DocKind; SettingsSection with its
│   │                      index in the settings tab), commands.rs (the command bar of each tab as
│   │                      ww::commands items; Menu with its button; the keys of WinWeaver), icons.rs
│   │                      (the icons beyond WW's), widgets.rs, folders.rs, importlist.rs, dialogs.rs
│   │                      (open_dialog: the filters of every file WinWeaver opens), views/ (mod: the
│   │                      titles, the links; header and block from ww::style; profile_tree, detail,
│   │                      rule), forms.rs, diffview.rs, marksview.rs, mcpview.rs, settings.rs (Extra:
│   │                      the fields of WinWeaver in settings.json), pages.rs (the guide, the license,
│   │                      the reference), catalogview.rs, catalogdiff.rs (two catalogs compared in a
│   │                      tab), regview.rs, regedit.rs, cataledit.rs,
│   │                      regfileview/, usersview.rs, polview.rs (the tab of a .pol file, the files of
│   │                      this Windows read once, the links of the other tabs), tests/ (the window
│   │                      tests: a Window of WinWeaver,
│   │                      `w.shell` and `w.app`), translations.rs (the test of the texts of winweaver-ui
│   │                      and ww)
├── data/
│   ├── reference/         the parameter reference of the catalog (00-20 and README, the root page of the
│   │                      documentation in the executable); the `doc` field of a rule links into it with a
│   │                      heading anchor
│   ├── catalog/           groups.json, 00-17 rules files, lang/<code>.json for uk, ru, de, es, fr, id, it, tr
│   │                      (catalog 0.11)
│   ├── presets/           office, strict, laptop, home .wwprofile
│   ├── resources/         timezones.json; locales.json (the input languages and keyboard layouts of a
│   │                      Windows 11 with English names, read once through the Windows API) and
│   │                      regions.json (its regions with their GeoIDs): embedded in a browser build
│   │                      only, never in one for Windows, which asks the system
│   ├── templates/         the runtime templates: the answer file skeleton and the scripts with {{slot}}s,
│   │                      VERSION 0.11;
│   │                      Create-Users.runtime.ps1, the stand-alone script of a users file
│   ├── lang/<code>/       ui.json: the translations of the interface (uk, ru, de, es, fr, id, it, tr); guide.md: the guide translated
│   ├── themes/            light, dark, latte, matrix .wwtheme: the embedded themes (a base, the colours it
│   │                      changes, the fonts and the font of the icons)
│   └── docs/              guide.md, the English guide shown at the first start; guide-lite.txt, the plain
│                          guide of wwlite (its Ukrainian one in lang/uk/guide-lite.txt); guide-web.txt,
│                          the guide of the browser edition for someone who sees only it (its
│                          Ukrainian one in lang/uk/guide-web.txt, both embedded in that build alone)
├── agents/                shipped in the archive: AGENTS.md (the assistant for any agent, short for small
│                          contexts), README.md, skills/ (winweaver: the tools, served as winweaver://skill;
│                          winweaver-review, -change, -this-pc), pi-agent/ (Dockerfile of the latest pi and
│                          Node.js LTS, its AGENTS.md, README.md)
├── crates/wwlite/         the light edition: main.rs (the command line, winweaver_ui::run without the
│                          feature full; on wasm32 the start in the canvas of the page through
│                          winweaver_ui::run_web), web/index.html (the page of the browser edition, with
│                          the module and the glue wasm-bindgen writes next to it), web/single.py (the
│                          same page as one file, wwlite.html: the edition by default), tests.rs (what it
│                          leaves out, the screenshots l1- to l4-; run alone: cargo test -p wwlite)
├── package-wwlite/        what the archive of wwlite adds: README.md, lang/de/guide-lite.txt (German, with
│                          data/lang/de/ui.json copied by CI)
├── package-wedit/         what the archive of wedit adds: README.md for its user, lang/uk/ui.json (every
│                          text of wedit and of WW in Ukrainian, the example of a language file)
├── wview/                 wview, a direction of its own (docs/plugins.md) and a workspace of its own:
│                          crates/wwrt (the runtime, a Rust dylib: W# and egui re-exported; plugin:
│                          the interface 2 with the loader and its check of the build; inner: the
│                          views inside a view; mime: the type of some bytes; input: the keys and
│                          the mouse of a view; blocks; testing), crates/wview (the viewer:
│                          lang/<code>/ui.json and guide.md, docs/guide.md), plugins/wv-text
│                          (Markdown and text), plugins/wv-image (WIC: wic, picture with the levels
│                          and EXIF, compose for the frames of a GIF, view with the tiles, sample of
│                          the tests), plugins/wv-binary (expr, format, engine, hex, content, view, wwbin,
│                          diff, compareview, sample of the tests;
│                          the description formats/pe.wwformat built in), each with lang/<code>.json,
│                          plugins/wv-hello (the template of the SDK); sdk/ (README.md, pack.py: the
│                          SDK of a release); package/README.md (for the user of the archive);
│                          stable/ (wview.h: the stable interface for other languages, README.md,
│                          wv-mandelbrot/mandelbrot.c and seahorse.mandel: its example in C);
│                          .cargo/config.toml links std dynamically; README.md
├── package/               what the archive adds: README.md for the user (not this repository's), lang/be/ui.json
│                          (Belarusian, every text: the example of a language, checked by a test of i18n.rs)
└── docs/
    ├── releases/          release notes v<version>.md
    ├── presentation/      make_presentation.py: the slides for a prospective user from the screenshots
    │                      of CI (python-pptx), uploaded by the Windows job as presentation-<version>
    ├── ww.md              the framework W# for the programs written on it
    ├── wasm.md            wwlite in a browser (WebAssembly): what was built, what stays open, the study
    ├── plugins.md         plugins of W# in Rust: the ways, the study, its measurements, the design of wview
    └── spec/              the specification (00 scope, 01 UI, 02 documents, 03 features, 04 architecture)
```

Next to the executable at run time (never versioned): `settings.json`, `bookmarks.wwmarks`, `profiles/`,
`output/` (the answer file, the scripts, plan and backup files of the native apply), `imports/` (catalog files of
the imported policy templates), `drafts/` (unsaved profiles), `mcp/` (MCP configurations, `default.wwmcp` of the
MCP tab), `logs/` (winweaver-<date>.log, mcp-<date>.jsonl),
`lang/<code>/` (more or overriding translations), `themes/` (more themes, `<id>.wwtheme`, also `<id>.json`
of the older format; the id of an embedded theme replaces it).
