# AGENTS.md: map of the WinWeaver repository

For agents and developers: where things are, what to read first, which rules apply, the state of the work.
Updated with every change of structure, commands or task status. Last update: 04.11.2026.

Repository: https://github.com/digidult/winweaver (private, branch `main`). Commits are authored by
`digidult <271767060+digidult@users.noreply.github.com>` (a local `git config` of this clone, never global).
A commit made by an agent ends with the line `Co-Authored-By: <agent model> <noreply@anthropic.com>`.

## 1. The project in three sentences

WinWeaver is a portable editor of Windows 11 installation profiles: it builds `autounattend.xml`, compares
profiles, reads the settings of this PC natively, applies them natively or through generated stand-alone
scripts, imports ADMX policy templates as catalogs, edits the registry, registry files and users files, and
serves an MCP server. It is written in Rust with eframe (egui + glow and wgpu + winit) for Windows 10 and
11, one executable with data folders next to it, on its own framework W# (`ww`), which also carries the
text editor wedit.

## 2. What to read first

| Task | Start with |
|---|---|
| Understand the scope and the principles | `docs/spec/00-scope.md` |
| Design or change the interface | `docs/spec/01-ui.md`, then `crates/winweaver-ui/src/style.rs` (the palette, the painted widgets) |
| File types, formats, what lives next to the executable | `docs/spec/02-documents.md` |
| What the program must do (feature inventory) | `docs/spec/03-features.md` |
| Crates, modules, threads, build | `docs/spec/04-architecture.md` |
| Use or change the MCP server | `crates/winweaver-mcp/src/lib.rs`, `tools/mod.rs` (the table of the tools), then `agents/skills/winweaver/SKILL.md` |
| Change what an AI agent or the archive gets | `agents/README.md`, `package/README.md` |
| Write a program on W#, or change the shell of the window | `docs/ww.md`, then `crates/ww/src/shell/mod.rs` (the trait `App`), `crates/wedit` (the example) |
| Change wedit | `crates/wedit/src/app.rs`, `package-wedit/README.md` |
| Plugins of W#, wview | `docs/plugins.md` (the study, its measurements, the design of wview), then `wview/README.md` and `wview/crates/wwrt/src/plugin.rs` (the interface of the plugins) |
| Change wwlite, the light edition | `crates/wwlite`, the feature `full` of `crates/winweaver-ui` (`cfg(feature = "full")` marks what wwlite leaves out), `package-wwlite/README.md`; the browser edition: `docs/wasm.md`, `crates/wwlite/web`, `crates/ww/src/web.rs` and `store.rs` |

## 3. Folder structure

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
│                          the zip on Windows; the job build-web (wwlite for the browser, the artifact
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
│                          the interface 1 with the loader and its check of the build; input: the
│                          keys and the mouse of a view; blocks; testing), crates/wview (the viewer:
│                          lang/<code>/ui.json and guide.md, docs/guide.md), plugins/wv-text
│                          (Markdown and text), plugins/wv-image (WIC), each with lang/<code>.json,
│                          plugins/wv-hello (the template of the SDK); sdk/ (README.md, pack.py: the
│                          SDK of a release); package/README.md (for the user of the archive);
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

## 4. Project rules

1. Language: everything in the repository is English (code, comments, documentation, commit messages, issue
   texts). The interface, the catalog and the user documentation are English sources with translation
   files built in for Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish; other languages are files added next to the executable.
2. No em dash (U+2014) and no en dash (U+2013) in any text we create, in any language: documents, code,
   data, comments, interface strings, translations, commit messages. Number ranges use a hyphen.
3. The program code is the project's own; the rule catalog, its translations, the presets, the time
   zones and the runtime templates (PowerShell of the answer file and of the stand-alone scripts) are data.
   Behaviour is written from the specification. Old profiles of JSON and their answer files are still
   read (`winweaver_core::import`). The lists of the system (the input languages and keyboard layouts,
   the regions) are read from Windows at run time, never embedded, with one exception: the browser
   edition of wwlite has no Windows to ask and embeds `data/resources/locales.json` and
   `data/resources/regions.json`, the lists of a Windows 11 (section 6); a build for Windows never
   carries them.
4. Dependencies: popular, well known, open source Rust crates that need no extra DLL next to the executable.
   The executable and its configuration stay portable: nothing is installed, nothing is written outside the
   program folder and the system temp folder. The one exception is a program with plugins (wview): its
   executable comes with `wwrt.dll`, `std-<hash>.dll` and the folder `plugins` (docs/plugins.md).
5. No PowerShell inside the program: reading and applying on this PC use the Windows API (the native
   elevated apply); PowerShell appears only in the generated answer file and the stand-alone scripts. The
   program never runs a script itself.
6. The work PC of the customer is only read: no setting of Windows changes there, and the program is
   never started there by an agent. Builds and the safe tests run on the computer of the developer
   before a commit (section 5); the tests that change Windows run only in CI or in a VM. A toolchain
   on a work PC is installed only on the explicit word of its owner.
7. Text files are UTF-8 without a byte order mark with LF line endings; `rustfmt` and `clippy -D warnings`.
8. Untrusted input (profiles, diffs, reports, answer files, MCP requests, policy templates) is read
   with size, depth and character limits and turned into a message, never into a crash.
9. Every feature is developed in its own branch and reaches `main` through a pull request, which the
   customer merges.
10. Every text of the interface is wrapped in `tr("English")` or `trf("English {name}", &[("name", &v)])` and
    has a translation in `data/lang/<code>/ui.json` of every built-in language (uk, ru, de, es, fr, id, it,
    tr); the test `every_text_of_the_interface_is_translated` fails otherwise. It finds only the literals of
    `tr` and `trf`, so a message of the status bar is never made with `format!` (that is for the texts of
    files and logs, which stay English). A new rule needs its texts in
    every `data/catalog/lang/<code>.json` (`translations_cover_every_rule`); `package/lang/be/ui.json` may
    lag (English fills the gaps).
11. Versions: `version` of the workspace `Cargo.toml` grows with every
    pull request that reaches `main`: the second number for a feature (0.2.0 to 0.3.0, the third back to 0),
    the third for fixes only (0.2.0 to 0.2.1); a pull request with both counts as a feature. Every build
    names its commit: `winweaver_core::BUILD` is `<version>+<12 digits of the commit>` (`build.rs` of the core,
    `WINWEAVER_COMMIT` set by CI), shown by `--version`, About, the logs, the MCP server, the answer file
    and the scripts.

## 5. Commands

On a machine with Rust (CI or a VM, never the customer's PC):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p winweaver -- --check-catalog
cargo build --release --workspace
# one renderer only: the features glow (OpenGL) and wgpu (DirectX 12), both by default
cargo build --release -p winweaver --no-default-features --features wgpu
# wwlite alone: a build of the workspace gives winweaver-ui the feature full of WinWeaver
cargo clippy -p wwlite --all-targets -- -D warnings
cargo test -p wwlite
cargo build --release -p wwlite
# the browser edition of wwlite (the target wasm32-unknown-unknown, the renderer glow alone): the check,
# the build, then wasm-bindgen of the version Cargo.lock names writes the module and the glue next to
# crates/wwlite/web/index.html; the folder is served over http (never opened from file://)
rustup target add wasm32-unknown-unknown
cargo clippy -p wwlite --no-default-features --features glow --target wasm32-unknown-unknown -- -D warnings
cargo build --release -p wwlite --no-default-features --features glow --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist-web/wwlite-web target/wasm32-unknown-unknown/release/wwlite.wasm
```

On a computer of Windows, `build-all.cmd` of the root makes what CI makes (the list of 30.10.2026):
the release builds of the workspace and of wwlite alone, wwlite for the browser (wasm-bindgen of the
version of `Cargo.lock`, `single.py`), the folders of the step Package and the four archives of a
release, `dist/WinWeaver-<version>.zip`, `wedit-`, `wwlite-` and `wwlite-web-<version>.zip`, each
with its folder inside as the job `release` zips it (`tar.exe` of Windows). `check` runs the checks of
a commit first (the format, the three clippy runs, both test runs, `--check-catalog`), `screenshots`
the tests with `WINWEAVER_SCREENSHOTS` set to `dist/screenshots`, `presentation` the slides into
`dist/presentation` (python-pptx and pillow), `all` the three; `noweb` leaves out the browser
edition. The tools come from `WW_CARGO` (a script that runs cargo on half of the processor, say),
`WW_PYTHON` and `WW_WASM_BINDGEN`, else from the PATH; a wasm-bindgen that is missing comes once into
`target/tools` from its release on GitHub, as CI takes it, with its SHA-256 checked. Every tool is
found before the first long step. It never runs the tests that change Windows and starts the
program only for `--check-catalog`; the smoke tests of the window and of the command line stay in
CI. A change of the steps Package or build-web of `ci.yml` changes `build-all.cmd` too. The file keeps
CRLF (`.gitattributes`): cmd finds its labels by the offsets of the lines and reads a running script
from the disk again, so it is never edited while it runs. Then wview and its SDK, as CI makes them:
`dist/wview` (wview.exe, wwrt.dll, the `std-<hash>.dll` of the sysroot of `WW_RUSTC`, with the
toolchain gnu `libgcc_s_seh-1.dll` and `libwinpthread-1.dll` too, the plugins, README),
`dist/wview-sdk` (`wview/sdk/pack.py`) and their archives with the version of wview; `check` and
`screenshots` take the checks and the tests of wview along, `nowview` leaves wview out.

```bat
build-all.cmd
build-all.cmd check
build-all.cmd all
build-all.cmd noweb
```

The workspace of wview (`wview/`, docs/plugins.md) is checked and built in its folder, with the same
rules before every commit that touches it or the crates of W# it uses:

```bash
cd wview
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

Before every commit, on the computer of the developer (items 12 and 13 of the list of 28.10.2026), the
checks of the start of this section: `cargo fmt --all`, the clippy runs with `-D warnings` (the
workspace, wwlite alone, wwlite for wasm32), `cargo test --workspace` and `cargo test -p wwlite`. The
format and the fixes of clippy go into the commit they belong to, never into a commit of their own
(and never a commit such as "The format of the last run"). A plain `cargo test` leaves out the tests
that change Windows (a test key of HKCU, the hive of the default user, an account, DISM, the package
API): they carry `#[ignore = "changes this Windows: run in CI or a VM, AGENTS.md section 5"]` and run
only by name, in CI or in a VM, as administrator:

```bash
cargo test -p winweaver-win -- --ignored
```

CI builds only releases (item 11 of the list of 28.10.2026):

1. A tag `v<version>` runs everything and publishes the release. A tag is pushed only when the
   customer asks for the release of that version in so many words; finished work alone never makes one.
2. Any branch runs on demand: `gh workflow run ci.yml --ref <branch>` (or Actions, CI, Run workflow);
   `-f windows=false` stops after the Linux job, `-f web=false` leaves out the browser edition;
   `gh run list --branch <branch> --workflow ci.yml` finds the run. It is the place of the tests that
   change Windows, the screenshots of the window and the archives for an acceptance.
3. Pushes, pull requests and merges run nothing. A newer run of the same branch cancels the older one.
4. The Linux job comes first and the Windows job only after it; the job `build-web` (Ubuntu: wwlite for
   the browser, the artifact `wwlite-web-<version>`) also follows the Linux job.
5. The mirror: when the Actions minutes of digidult/winweaver are used up, its run fails in a few
   seconds with the annotation "The job was not started because recent account payments have failed
   or your spending limit needs to be increased". Then the same commit runs in the private mirror
   https://github.com/stanislavperec-ua/winweaver (the remote `mirror` of the clone): `git push mirror
   <branch>`, then `gh workflow run ci.yml --ref <branch> -R stanislavperec-ua/winweaver`; tags never go
   there. The pull requests and the merges stay in digidult/winweaver. The minutes reset on the first
   day of a month, or the customer sets a budget.

When the Linux job of a run finds a format difference, it uploads `fmt.patch` with `Cargo.lock` in the
artifact `ci-files`; that means a check was skipped before the commit: apply the patch to the commit it
belongs to (`git commit -a --amend --no-edit`, `git push --force-with-lease` of the feature branch,
never of `main`).

The MCP server without the window (the stdio transport is started by the client; the HTTP one needs a token
of at least 32 characters):

```bash
winweaver --mcp stdio --mode read
winweaver --mcp http --port 8765 --token <token> --mode edit --read-pc
```

A release: set `version` in the workspace `Cargo.toml`, add `docs/releases/v<version>.md`, merge, then tag
only when the customer asks for the release of that version in so many words; the job `release`
publishes the zips with the notes:

```bash
git tag -a v0.3.0 -m "WinWeaver 0.3.0"
git push origin v0.3.0
```

Catalog data: edit `data/catalog/*.json` by hand (strict JSON, ASCII), add a new rules file to `RULE_FILES` in
`crates/winweaver-core/src/catalog/embedded.rs`; `tests/catalog.rs` fails on an unknown field, a missing
reference, a placeholder outside the allowed fields or a rule under the wrong root.

## 6. Facts that are easy to lose

- The workflow: write, then the checks of section 5 on the computer of the developer, then the commit;
  CI only for a release or on demand (`gh run view <id> --log-failed` reads a failed step).
- Agents editing from Git Bash: a heredoc swallows doubled backslashes (`\\` becomes `\`), so Rust or Python
  text with backslashes is written with the file tools or a script file, never through `<<'EOF'`.
- The audit reads the default user profile (`DU:` paths) in HKCU of the current user, with the note "current
  user instead of the default profile". A rule whose matching values are what a clean Windows has (a value
  equal to its default, an absent policy value, a reg-remove of an absent value) is "as on a clean Windows"
  and keeps its state when read; an app absent from both package lists is evidence of its removal.
- Rules left out of This PC: the installation passes (windowsPE, the xml rules), the first sign-in and the
  sign-in screen (`HKU:\.DEFAULT`). The audit script and the native read produce the same checks in the same
  order, so a report of the script on another PC pairs with the plan (rule by rule, check by check).
- Features and capabilities are read natively through the DISM API (`winweaver_win::dism`: `dismapi.dll` of
  System32 loaded at the first read, initialized once a process and never shut down, since DISM promises no
  second start; its log goes to the temp folder). `Machine::components` is asked once a run and only when
  the process is elevated and the plan has such checks (`Plan::admin_checks`); without the rights the checks
  are unknown with the note of the audit script, and the window says so. The states take the names of
  PowerShell, a feature compares as `-like "<state>*"`, a feature absent from the build is as wanted, and a
  capability pattern matches as `Get-WindowsCapability -Name` (`thispc::wildcard`). The test of DISM runs on
  the CI runner as administrator (more than a minute there). The window reads This PC in a thread of its own
  named `this-pc` (`App::start_pc_read`, `poll_pc_read`, a `PcJob`; the machine is `App::pc`, one in memory
  for the tests), one read at a time with a spinner in the status bar; the result goes to the tab the read
  started from, found by its draft key, and a tab closed meanwhile is only named in the status.
- Input languages and keyboard layouts come from the system, the program has no list of them (the one
  exception is the browser edition, at the end of this fact):
  `resources::Locales` holds what Windows lists, read once at the first use by the reader the
  window and the MCP server name (`set_locale_reader(winweaver_win::locales)` in `App::new` and
  `Server::new`): every locale of `EnumSystemLocalesEx` with its locale id (empty for one Windows numbers
  at the first sign-in, such as `ru-UA`, also when `LocaleNameToLCID` gives the transient id Windows
  lends it once the user has it as an input language, `resources::is_transient`), its localized name, its parent and the first input profile of
  `LOCALE_SKEYBOARDSTOINSTALL`, and every layout of `Keyboard Layouts` in the registry with the name of
  `Layout Display Name` (`RegLoadMUIStringW`), input methods (`E...`) left out. The core tests read
  `crates/winweaver-core/tests/data/locales.json` (set in the helpers `preset` and `office`, before the
  first use, since the list is read once a process). An item of `languages.input` is a tag (the layout
  Windows gives the language first) or a layout id `LLLL:KKKKKKKK`; `Locales::input` gives the language
  and the profile of an item. The answer file gets the profile of each item (a language without a locale
  id gets the one of its parent, as `ru-UA` gets `0419:00000419`), `Setup-User.ps1` the tags, the chosen
  layouts (`$InputTips`, empty for the default; a language named by its tag and with a layout of its own
  keeps the default of its tag too) and the fallback, and the rule `user-logon.input-languages` sets the
  tips of every language it adds. Windows shortens a tag whose region is the main one (`uk` for `uk-UA`):
  `Locales::read_items` turns a read language and its input method tips into items (the default tip is
  the tag of the language, another one a layout id, a tip of a locale id handed at the first sign-in,
  `2000:00000419`, the tag as read). The check of a profile names an item Windows does not know. The
  browser edition of wwlite has no Windows to ask, so a build for `wasm32` embeds
  `data/resources/locales.json` (`resources::locales` takes it when no reader is named, under
  `cfg(target_arch = "wasm32")` alone, so a build for Windows never carries it): 912 languages and 218
  layouts of a Windows 11 (build 26300), read on 25.10.2026 through the Windows API by a PowerShell script
  that only reads (`EnumSystemLocalesEx`, `GetLocaleInfoEx`, `LocaleNameToLCID`, the key `Keyboard
  Layouts`), in the shape of `tests/data/locales.json`, ASCII, each list sorted by name as
  `winweaver_win::locales` sorts it; the test `the_list_of_the_browser_edition_is_whole` checks the file.
  What the list lacks against the native read: the names are the English display names
  (`LOCALE_SENGLISHDISPLAYNAME`) and the `Layout Text` values, not the names in the language of the
  user's Windows, and a language or layout a newer Windows adds is missing until the file is read again.
- Of the actions of one rule on one value (a removal for a clean start, then the value) only the last is
  checked (`thispc::plan`). A rule partly in effect is read as off, and the read names what differs
  (`Reading::differing`, `CaptureSummary::partly`, a warning on the rule in the message bar).
- The user region (`default-user.region`): the two text parameters pick a region (`Param::Text` has
  `pick`: `Pick::RegionId` on the GeoID, `Pick::Region` on the two-letter code; the catalog check wants both
  of the pair, `Rule::param_picked` finds them). `resources::Regions` (`regions.rs` of the core) holds what
  Windows lists, read once at the first use by the reader the window, the MCP server and the command line
  name (`set_region_reader(winweaver_win::regions)`: every nation of `EnumSystemGeoNames` with its GeoID
  and its name of `GetGeoInfoEx`); the core tests read `tests/data/regions.json` in the helper `preset` of
  `tests/validate.rs`, before the first check. The detail panel (`views::region_param`) types the field
  freely with the list beside it filtered by the text, and a choice sets both parameters
  (`ProfileAct::SetParam` twice); the check warns when the GeoID and the code do not name one region of the
  list, nothing where the list is empty (no Windows). MCP: `get_rule` names the `pick` of a parameter and
  `set_param` of such a parameter takes a code or a GeoID and sets both (`set_region`). A build for
  `wasm32` embeds `data/resources/regions.json` (`regions()` takes it when no reader is named, under
  `cfg(target_arch = "wasm32")` alone): the 249 nations of a Windows 11 read on 28.10.2026 by a
  PowerShell script that only reads (`EnumSystemGeoNames`, `GetGeoInfoEx` for the GeoID, the English
  names of .NET `RegionInfo`, five names by hand where it has none, ASCII without diacritics), sorted
  by name; the test `the_regions_of_the_browser_edition_are_whole` checks the file.
- The menu Catalogs lists every catalog file of `imports/` (`Full::import_list`, read again by
  `ImportAct::RefreshList` when the menu opens: `catalogs_menu` sees a gap in `cumulative_pass_nr`), a
  loaded one with `icons::CHECK` and its text in the accent colour (`ww::shell::menu_entry_colored`), a
  click loads or unloads it (`ImportAct::Toggle`, the setting by default); with
  `Extra::catalog_menu_opens` (General, A click on a catalog of the menu Catalogs) it lists the loaded
  ones alone and a click opens the tab (`CatalogAct::Open`), the settings loading the others. The
  comparison of two catalogs marks every field of the selected policy on both sides
  (`catalogdiff::Mark`: the same, differs, absent, only here; a tinted frame and a word, a legend
  above). A profile tab has the disc `icons::PROFILE`, the gear is the settings'.
- The list of 30.10.2026: the form Profile (`Form::Profile`, first of `Form::ALL`: the name, the
  author and the comment, `ProfileAct::SetInfo`), and the view of the profile (the first crumb)
  links to every form; the menu This PC is also a submenu of the main menu, its acts on the
  selection on only for a profile in front whose selection a PC has something of, the ones on the
  whole profile only for a profile, a read into a new profile, an undo and a report always (a read
  without a profile starts from an empty one); a preset becomes a profile with the culture of the
  user (`preset_profile`: `preset_languages` takes the language of the window with the country of
  Windows when they agree, else the usual country of the language, else en-US, and the input
  languages of this Windows; `Profile::localize` sets the three languages, the input, English
  International when none, and the region of the country); the stripe of a part of the description
  is inside its card and optional (`DescSections::stripes`), its tint always; the first frames fit
  the window only once its state is known and never a window maximized at the last close.
- Text sizes: a theme has a size of `style::SIZES` (10 to 36), offered as `style::SIZE_STEPS` (by one to
  20, then 22, 24, 26, 28, 32, 36). The chrome scales with it through `style::at_least` (never below the
  size of the layout): the buttons of the command bar, the icon buttons, the tabs (`shell/tabs.rs`), the
  widths the navigation row keeps, the list of the settings sections. A new fixed size of the chrome goes
  through `at_least`. The screenshots `35-size-28`, `35-size-36` and `36-size-*-settings` show it.
- Fields: a text of any length (a name, a description, a path, a title) takes the width of the panel
  (`forms::wide_field`, `desired_width(f32::INFINITY)` in a window over a tab); a short value takes a
  width in the text size of the theme (`forms::short_field`). The windows over a tab of the registry, a
  registry file and a Group Policy file open with `regedit::modal_width` (half of the window, three
  quarters for data of several lines). The screenshots `30-` to `34-` show the forms and the editors.
- The local accounts are read natively the way the audit script reads them: `NetUserEnum` (level 20, normal
  accounts) without the disabled ones and the relative ids 500, 501, 503 and 504; the group is Administrators
  or Users by direct membership (`NetUserGetLocalGroups`) in the groups S-1-5-32-544 and S-1-5-32-545, whose
  names `LookupAccountSidW` gives in the language of Windows. No password is read; the capture keeps the
  accounts of the profile when none of Administrators or Users is found.
- The computer name template (`validate::is_name_template`, `render::computer_name_parts`,
  `section-computer-name.ps1`): `{serial}`, `{mac}` and `{random}`, each with `:N` from 1 to 15 (15, 12 and 6
  without it); the serial number and the MAC address keep their last N characters (a shorter one stays
  whole, one that cannot be read becomes random characters), random ones come from A-Z and 2-9 without I
  and O, the name is cut at 15. `render::example_name` expands a template the same way with sample values
  (`EXAMPLE_SERIAL`, `EXAMPLE_MAC`, named under the example); the form shows it with the description of
  every placeholder (`forms::name_template_help`), and the guide
  has the section The name of the computer in every language. A change of the script changes all three.
- The editions and their keys are 14 (`resources::EDITION_KEYS`); `ASK_KEY` is
  `00000-00000-00000-00000-00000` with `WillShowUI` Always.
- The answer file is ASCII: account display names and descriptions outside ASCII go to Post-OOBE.ps1 as
  Base64 (`Set-AccountText`), the profile is embedded as ASCII JSON in `Extensions/Profile` (`\uXXXX`,
  so a name outside ASCII comes back whole). The name of the profile appears only in comments, the
  header of the answer file and the first line of the scripts, through `render::ascii_text` (`?` for a
  character outside ASCII); Windows Setup reads none of it.
- The language setting `""` follows the language of the Windows interface; it switches the interface and the
  catalog texts together (`i18n::current()`).
- Logs and crashes (`winweaver_mcp::log`, set up first in `main`): the MCP server writes `logs/mcp-<date>.jsonl`
  (one JSON object a line: `time`, `level`, then `text` or the fields of an event; `log::event`)
  (transport start and stop, every request with its time, refusals and transport errors; ids and switches
  only, never a token, a password, a key or a free text). The panic hook and the filter of unhandled Windows
  exceptions (`winweaver_win::on_fatal_exception`: access violation, stack overflow) write the crash into the
  MCP log for a headless process or a thread named `mcp-*`, else into `winweaver-<date>.log`. A panic inside a
  request is caught (`Server::handle_text`): the request gets the error -32603 and the server goes on; a
  release build has no console, so `eprintln!` of a headless process also goes to the log (`refuse`).
- MCP: every setting of a server is a configuration file `mcp/<name>.wwmcp` (`config::Config`, strict, saved by the
  MCP tab at every change through `App::persist_mcp`; `settings.json` keeps no MCP field (an
  `mcp_token` of an older file is read once into `mcp/default.wwmcp`): the transport for a start without the window, port, token,
  mode, read_pc, registry, edit_library, the first profile and `detailed_log`. `winweaver --mcp FILE` serves as the
  file says, `--mcp stdio|http --config FILE` too, and the other options change it for that start
  (`mcp_config` of `main.rs`). The journal keeps every call with its arguments and answer (`redact` leaves out
  password, product_key, token and authorization; an answer is cut at 16 KB); the detailed log writes them as
  DETAIL events. The MCP tab shows the selected call in a right panel (`mcpview::detail_view`) with buttons to the
  places it names (`mcpview::places`: rules, workspace profiles, catalog files, registry keys);
  errors start with not_allowed, not_found, invalid, write_failed or read_failed; passwords and the product
  key are replaced by `***` in every answer.
- Native apply: the window writes `output/plan-<stamp>.json` (with a random nonce, so no two plans are equal)
  and starts itself with `--elevated-apply <plan> <SHA-256 of its bytes>` through ShellExecuteEx runas; the
  helper reads the file once and refuses another folder, name or digest (`apply::digest`), so a plan changed
  after the permission never runs,
  backs up every registry value into `output/backup-<stamp>.json` (the format of the apply script, so
  `Undo-Apply.ps1` restores it too) and writes `plan-<stamp>-report.json`. The script steps of the
  catalog go natively through `apply::script_ops`, a table by rule id (`Op::RunFromMedia`,
  `ServiceStartFrom`, `SetValueInSubkeys`, `DeleteKey` with the backup of every value under the key,
  `PasswordNeverExpires`, `DisableBuiltinAccounts` that leaves the account in use on, `CollectErrors`,
  `DeleteFiles` only of `apply::ANSWER_FILE_COPIES`; `winweaver_win::accounts` for the flags); the test
  `the_script_steps_of_the_catalog_go_natively` fails when a script step This PC applies has no entry, and
  a new one stays with the stand-alone script until it has one. The changes no undo takes back are named
  in the backup (`account-changed`, `file-deleted`) and counted by `Plan::irreversible` in the
  confirmation. The only paths of other users are the sign-in screen (`HKU:\.DEFAULT`), which This PC and
  the apply script leave out. Capabilities are removed through DISM
  (`Op::RemoveCapability`, `dism::remove_capabilities`) and apps through the package API (`Op::RemoveApps`,
  `apps::remove_apps` with the crate `windows`: the provisioned package first, then the package of every
  user), named in the backup as `capability-removed` and `app-removed` the way the apply script names them,
  and no undo brings them back (the confirmation says so). The default user profile is
  written natively: `DU:` paths become `HKU:\UnattendDefault\...` (the key of the apply script), and the
  helper loads the hive there with the backup and restore privileges and unloads it after the last step;
  its test runs only on the CI runner as administrator (`GITHUB_ACTIONS`).
- Imports: a policy is `admx.<prefix>.<name>` with `<id>.off`; groups `admx.<import>` under `policies`; a catalog
  file brings only Reg and RegRemove actions under `HKLM:\` or `DU:\` (checked at every load). Turning a group
  on skips imported policies; the read of the whole PC covers the loaded ones (`thispc::whole_pc_rules`: every rule of the catalog, since 0.28.1).
- Archives (`admx::read_archive`, the button Import an archive): a `.zip` is read in memory with the crate
  `zip` (deflate only, pure Rust); its ADMX files with the ADML of `<dir>/<culture>/<name>.adml` (any case)
  become an import `archive-<fnv of the path>`, else its one `.wwcatalog` is imported like a catalog file.
  Limits: 256 MB of archive, 20 000 entries, 512 MB unpacked, an entry up to `FILE_MAX` or `CATALOG_MAX`
  whatever size it declares.
- Templates of vendors (Chrome, Edge, Brave, Office) are UTF-16 LE: `jsonfile::decode_bom` reads UTF-8 with or
  without its mark and UTF-16 with its mark, for ADMX, ADML, the entries of an archive and every
  `read_text_bom`. In one import a second copy of a policy (same namespace and name, as the x64 and x86
  templates of Office) is left out and listed as skipped; two namespaces with one prefix and one policy name
  (Zoom's user and machine templates, prefix `fullarmor`) take `admx.<slug of the namespace>.<name>`, and
  categories of one title in two namespaces get the end of the namespace in brackets. The ids of templates
  without such a clash (all of Windows) are unchanged.
- A policy two imports hold is one rule of the first (`admx::merge`) and an alias in the groups of the
  others (`Catalog::with_aliases`, `rules_in` and `rules_under` include them), so every checked import
  shows its whole tree. `admx::compare` sets two catalog files side by side by policy (the `.off` rule
  counts with its policy); Compare in the bar of a catalog tab opens it as a tab (`Doc::Compare`,
  `catalogdiff::CompareDoc`: the differing policies in the tree of their categories, switches for the
  policies only in one catalog and the different ones, the selected policy as it is in both); Save the
  report writes it as a Markdown file and As a page shows it (`comparison_page`, at most 1000 lines a
  section). Copy to (`CatalogAct::CopyTo`, `CatalogFile::copy_from`) copies the selected policy with
  its `.off` rule, or the selected group with the groups under it, into a catalog open in its editor
  or a new one: ids kept (the same policy in two catalogs is one rule of the program), groups made
  again by their titles, the texts of other languages along, the whole file checked first.
- A policy of two imports shows twice in the profile tree, so the key of a row is `(Sel, Option<group>)` and
  `ProfileDoc::focus` keeps the group of the selected rule: `Act::SelectIn` from the tree, `Act::ShowIn`
  (select and reveal) from the links of the detail panel (`views::other_catalogs`: the same id in the groups
  of `Catalog::alias_groups`, and rules of the same policy name in other imports). `views::catalog_links`
  opens the catalog tabs of the loaded imports at a rule (`Act::OpenCatalogAt`), from the detail panel and a
  bookmark. The policy of a catalog tab is drawn with `views::header`, `block` and `rule_description`;
  `Settings::desc_sections` gives the parts of the description open at first (the id of a part holds the
  setting, so a change shows at once).
- The registry (`winweaver_core::registry`, `winweaver_win::NativeRegistry` with winreg, `this_registry()`):
  paths in the form of regedit (`registry::normalize` takes `HKLM:\`, `HKLM\`, `Computer\...`; `DU:` is shown
  as the current user), `Data` in the types of regedit (REG_LINK and the resource lists are shown, not
  edited). The registry tab (`Doc::Registry`, `regview.rs`) reads through `App::registry` in the acts after
  the frame (tests put a `MemoryRegistry` there); a change without the rights says that it needs WinWeaver
  as administrator. Permissions (`Act::RegistryRights`, `regview::RightsEdit`, the window over the tab) reads
  the owner and the access list of the selected key as SDDL (`Registry::security`, `GetNamedSecurityInfoW` in
  `NativeRegistry`) into `security::Rights`: inherited entries are shown, the entries of the key itself change
  (allow or deny, full control or read, where they apply), `Registry::account_sid` adds an account by its name
  and `account_name` names the entries; Apply asks first and writes only the own entries
  (`Rights::dacl`, denials first, `set_security` with `SetNamedSecurityInfoW`, protected or not), Windows adds
  the inherited ones. `MemoryRegistry` keeps rights for the tests (`MEMORY_RIGHTS`). Find searches under the selected key
  (`registry::search`: the names of keys, of values and the data, depth first as regedit, with a limit and a
  stop; `regview::Search`, a thread named `registry-search`, `App::poll_registry_search`). `views::writes` puts a button
  next to every key that opens the tab there (`Act::OpenRegistry`). A bookmark of the registry has the
  document `registry` (`Bookmark::registry`: the key in `path`, the value in `item`) and lists the rules that
  write it (`registry::writers`). MCP: `Options::registry` (`RegistryAccess` off, read, write; off by default,
  `--registry`, the switches of the MCP tab, saved in its file) gates `read_registry_key`, `search_registry` (at most 500 places within 100000 keys), `set_registry_value`,
  `delete_registry_value`, `create_registry_key`, `delete_registry_key`, `read_registry_rights` (read) and
  `set_registry_rights` (write: replaces the entries of the key itself; an account is a name `account_sid`
  knows, a checked `S-1-` identifier or an alias of two capitals, so no text of the caller reaches the SDDL).
- Registry files (`winweaver_core::regfile`, `Doc::RegFile`, `regfileview/`): `RegFile::parse` keeps every
  line (`Line::Blank`, `Comment`, `Key` with its deletion, `Value` with `None` for a deletion, `Other` for a
  line it does not read), joins the values continued by `\`, and `to_text` writes them back with CR LF;
  `save` writes UTF-16 LE with its mark for version 5 and the text as it is for REGEDIT4. A text of several
  lines goes as hex(1). The tab is an outline (`regfileview::items`: a key with the comments right above it
  and the lines between its values, or comments alone) and the lines of the selected key in a list; a
  removal asks first and takes a key with its values, never its comments. The rules that write a key come
  from an index of the catalog made again when its count of rules changes; `HKEY_USERS\UnattendDefault` is
  looked up as the current user. This PC saves `Apply-<profile>.reg` and `Revert-<profile>.reg` through
  `RegFile::from_plan` (`Act::SaveRegFile`): the registry steps of `apply::plan_apply` or `plan_revert`, the
  default user profile at `HKEY_USERS\UnattendDefault` with the commands that load its hive in the
  comments, a list that is not additive as `[-key]` before its values, a comment `; Rule <id>` before the
  steps of each rule (the arrow of the comment shows the rule), and the steps a registry file cannot hold
  (programs, capabilities, apps, the skipped ones) as comments.
- MCP, bookmarks and catalogs: `list_bookmarks`, `list_catalogs` (by `CatalogHeader::read`), `get_catalog` (the
  groups, or the policies of a group) and `get_catalog_policy` read always, like profiles; `add_bookmark`,
  `set_bookmark_comment`, `remove_bookmark`, `set_catalog_policy` and `remove_catalog_policy` need
  `Options::edit_library` (`--edit-library`, the switch of the MCP tab, off by default). A catalog
  changes through `CatalogFile::put_rule` (the whole file is checked, a failure changes nothing),
  `remove_rule` (with its `.off` and the conflicts with it), `new_policy`, `new_group`; `admx::catalog_path`
  turns a key of regedit into `HKLM:\` or `DU:\`. The ids of catalogs are file names checked against `..`.
- The editor of a catalog (`Doc::CatalogEdit`, `cataledit.rs`): Edit in the bar of a catalog tab
  (`Act::EditCatalog`, an import or the file of the tab) or New catalog of the menu of the templates
  (`custom-<n>`, an empty import, loaded). The form of a policy edits its title, its description, the phase
  (the computer or the default user profile) and its Reg and RegRemove actions of DWORD, QWORD, SZ and
  EXPAND_SZ with keys in the form of regedit (`admx::catalog_path` turns them into `HKLM:\` or `DU:\`);
  other actions are kept as they are. Apply goes through `CatalogFile::put_rule`, Save (Ctrl+S) checks and
  writes the file and loads a loaded import again; a removal asks first.
- Links: a rule named in a view leads where `views::RuleLink` says
  (`Profile`: `Act::Reveal`, select and open the tree down to it; `Catalog(file)`: `Act::CatalogReveal`
  in the tree of the catalog tab for a policy of its file; `Elsewhere`: `Act::ShowInProfile` in the first
  profile tab, which scrolls to it). `rule_description` takes the `RuleLink` of its tab and lists the
  rules a rule needs, the ones that need it and its conflicts both ways. `views::linked_text` turns the
  ids of rules and the registry paths of a text into links (the message bar); the crumbs of a catalog tab
  select every group above, the crumb of a rule of a profile keeps the group it is shown in. The registry
  tab names the rules that write the selected key or value (`regview::Writers`, kept until the selection
  or the count of rules changes), the catalog editor opens a key in the registry tab and its policy in the
  catalog tab and the profile, the places of an MCP call show the titles of rules. The warning of the
  unknown options has its own target (`validate::Target::Unknown`) and opens their group; the group of a
  rule in the diff and bookmark tabs shows it in the first profile tab (`Act::ShowGroupInProfile`).
- Open the folder (`Act::ShowInFolder`, `winweaver_win::show_in_folder`): a build, a saved script, registry file,
  profile, diff, catalog file (exported or edited), theme or MCP configuration records its file with the message of the status bar
  (`App::shown_file`), and the status bar shows the button while that message stays; Settings, Files opens
  the folders of the program (made first when missing). Explorer is started only at a click (`/select,` for
  a file), never by a test, and not waited for.
- Every removal asks first (`App::ask`, Escape says no): an import, a bookmark of the bookmarks tab (the star
  of the navigation row stays a toggle), closing a tab discarding its changes, forgetting the recent files,
  renewing the MCP token. The answer of a dialog stands beside Cancel, so a new one joins the list of the
  test `the_answers_of_a_dialog_do_not_read_as_cancel` (its translations may not start like Cancel).
- Imports are read only when checked: `Settings::loaded_imports` names them (`None` in settings of an earlier
  version: every file but `hidden_imports`, written as the list at the first start), and `read_imports` opens
  no other file, for the window and for the headless MCP (`with_imports` of `main.rs`). The settings list the
  other catalog files of `imports/` by `admx::CatalogHeader::read` (the first 16 KB) with their size;
  unchecking one drops it from memory and closes its tab.
- Unknown options: choices whose rule the loaded catalog lacks stay in `Profile::unknown`. The tree shows them
  in the group `UNKNOWN_GROUP` at its end (`Sel::Unknown`, left out with `Settings::hide_unknown`), the open
  warns of their count (`ProfileDoc::new`), and the check of the build does too (a warning of `validate`).
  Load catalogs (`Act::LoadCatalogs`) reads only the rule ids of the files that are not loaded
  (`admx::rule_ids`, a stream), checks and loads the ones that hold some, the templates of this Windows
  first, and the selection follows an option between the catalog and the unknown group.
- Lists (`Param::List`, `Action::RegList`): a `reg-list` writes the items of its list parameter as the values
  of a key, named by the items, a prefix and a number from 1 (an empty prefix gives 1, 2, 3), or `name=value`
  items (`ListNames`); without `additive` the other values of the key go first (`Set-RegList`, `Test-RegList`
  of the runtime, `Op::SetList` of the helper, which backs up every value of the key). A `reg` action whose
  value is a list parameter alone writes its lines (MultiString). Empty items are left out; the check of a
  profile wants ASCII items and refuses a pair without `=` or a name twice. An ADMX `list` element becomes
  such a rule (the Disabled rule clears a list that is not additive), `multiText` a MultiString value; lists
  come before the other values of a policy. The read of this PC checks lists but reads no list parameters.
- Renderers (`renderer.rs`, the features `glow` and `wgpu` of winweaver-ui and winweaver, both by default):
  OpenGL through glow takes half the memory of the full wgpu; DirectX 12 is wgpu with its `dx12` backend alone (no
  Vulkan, no OpenGL of wgpu) and draws without a driver of the graphics card on WARP (the CI runner shows
  `Microsoft Basic Render Driver (Dx12, Cpu)`). Automatic tries OpenGL, then DirectX 12; a choice of the
  settings (`Settings::renderer`) or of `--renderer` is tried first and the others follow. A renderer fails
  when `run_native` ends with an error before the app is created (egui_glow refuses OpenGL below 2.0,
  which is all Windows has without a driver); eframe keeps its event loop between the tries. The window
  logs what it draws with (`renderer::graphics_text`) and puts the failures in the message bar. The CI
  step Window smoke test runs `--smoke-window` and fails when no renderer starts; on the runner OpenGL fails
  and DirectX 12 draws, so it tests the fall back. Tests and screenshots draw with the wgpu of egui_kittest.
- The command line: `USAGE` of `main.rs` and the section Command line of the guide (`data/docs/guide.md` and every
  `data/lang/<code>/guide.md`) and of `package/README.md` list every parameter; a new one goes into all of them.
  The commands that write files are `crates/winweaver/src/cli.rs` (`--build`, `--import-admx`,
  `--export-catalog`, `--read-pc`, `--diff`, `--script`, `--reg`), each through the function the window
  uses: `render::Build::write`, `winweaver_ui::import_source`, `admx_file_catalog` and `save_import`,
  `thispc::whole_pc_rules` and `pc_profile_name`, `Diff::between`, `export`. `--out` goes through
  `export::out_path` (a value with the extension of the command names the file, any other a folder for
  the default name); the exit code is 0, 2 for a wrong use, 1 for a failure. `--import-admx` names its
  import in `loaded_imports` of the settings file of the window, so a window open meanwhile overwrites it.
- Screenshots: the Windows job sets `WINWEAVER_SCREENSHOTS`, and the test `screenshots_of_the_window` draws the
  main tabs with wgpu (no screen) into `target/screenshots`, uploaded as the artifact `screenshots`
  (`gh run download <id> -n screenshots -D <folder>`); the way to see the window without starting it on the
  customer's PC. A renderer that cannot start writes `error.txt` there instead of failing the tests.
- Window tests: `App::new` and `App::apply` test the logic without frames; `egui_kittest` (a dev-dependency
  with its eframe feature) draws whole frames without a screen and clicks by the labels of AccessKit, so no
  window ever opens. A test that needs a label finds it with `query_all_by_label`, since a text can appear
  twice (a tab and a breadcrumb).
- The chrome follows the active tab: the command bar holds the
  commands of that tab (icons above labels; none for Settings; icons alone at the end of the
  navigation row below 1100 px), the navigation row appears only for a profile and a catalog tab (the
  history, the breadcrumb, the star); the other tabs keep their search at the right end of the command
  bar. A narrow row shortens the breadcrumb to its first and last parts with a menu of the others. Every tab but Settings and a theme has its own search (`active_search`): the profile tree, the policies
  of a catalog tab, the differences, the blocks of a Markdown page, the bookmarks, the MCP journal. Clearing
  a search opens the groups above the selection and scrolls it into view (`Act::SearchCleared`,
  `reveal`). The message bar folds to one line or opens with a height the user drags.
- The look comes from `style.rs`: the palette of light and dark (a theme overrides its tokens,
  `style::COLOR_NAMES`, also with the names of the older format) and the fonts of the theme. `install_fonts` (before the first frame) names the families `light`, `light-strong`,
  `light-mono` and the same for `dark`; `style::apply` loads the files of the families of the two themes
  into them (`style::FAMILIES`, `MONO_FAMILIES` in the fonts folder of Windows, egui's fonts as the
  fallback) and scales the text styles by the size of the theme. Painted widgets take their fonts and
  sizes from `style::font`, `strong_font`, `mono`, `size` and `row_height`, never `FontId::proportional`;
  without `install_fonts` (tests) the families of egui are used.
- Themes (`themes.rs`): a `.wwtheme` file is a name, a base, colours and fonts; the
  window keeps the list in `App::themes` (read at the start, when the settings open and after a save) and
  applies `wanted_theme` whenever it changes: the theme of an editor tab in front (a live preview), else
  the one of the settings, else Light and Dark as Windows shows its apps. A change of a font in the
  settings saves the file of the shown theme (`Theme::file_in`: an embedded theme gets
  `themes/<id>.wwtheme`).
- Lists (`list::show`): one stop of Tab like the tree; the arrows, Home, End and the page keys move
  the selection and set `follow`, Enter or a double click opens, Delete removes where the tab allows it,
  the widths of the columns change by dragging the lines between their titles (kept in the memory of egui).
  In the diff and bookmark tabs the list is the resizable side panel and the item on the right is the
  central panel: a side panel grows with what it holds, so a detail there moved the divider.
- Tabs move by drag (`Act::MoveTab`, `chrome::drop_place` gives the new index and the line that shows it),
  by `Ctrl+Shift+PgUp`/`PgDn` and by the menu of the right button; `App::move_tab` keeps the tab in front.
- Diffs keep comments by the key of an item (`Diff::keys`: `install:<field>`, `languages:<field>`,
  `accounts`, `rule:<id>`); `Diff::apply_item` applies one. The diff tab shows the item on the right and
  leads to it in the profile of either side, opening the file or the preset of that name.
- Bookmarks are options (`Bookmark::option`, `is_option`): the id and the profile as a link, no title; one
  option is one bookmark in every profile. Removing one with a comment asks first (`App::ask`, a question
  over the window, Escape says no).
- Keyboard: a tree is one stop of Tab (`tree::show` makes a focusable area,
  the rows sense clicks only); a click on a row gives it the focus, so the keys go on from the clicked
  row: arrows, Home, End, Page Up and Down, Left and Right fold and go to the parent or child, Space
  toggles, Enter opens a group. A move with the keys sets `follow`, and the row comes into view at the next
  frame; the detail panel shows the new selection at once.
- The detail panel has cards: the head with the title, the id with a copy button
  (`views::id_line`), chips and the switch; Parameters, This PC and the folding Description, whose texts
  are `style::selectable_text` (a menu on the right button: copy, select all, copy all). A `doc` link
  opens the reference page in a Markdown tab at its heading (`Act::OpenDoc`, `mdview::slug`); links to
  `.md` pages inside Markdown tabs are taken from the output of egui instead of the browser.
- A catalog file (`.wwcatalog`) or an ADMX file (`admx::read_file`, with the ADML of the folders of the
  cultures next to it) opens like any document, without importing it (`CatalogDoc::from_file`); its
  command bar offers Import.
- The tab of an import (`Doc::Catalog`, "Open" in Settings, Policy templates) shows its tree without check boxes
  and the policy with its parameters and registry values; "Show in the profile tree" loads an import that is
  not loaded, brings the first profile tab to the front, opens the groups above the policy and selects it.
- The settings tab has the sections General, Policy templates (load, export, rename, remove, import) and Files
  (the folders next to the executable, the recent files); a rename changes the name of the catalog file and
  the title of its tree and saves the file.
- The title bar takes the colours of the theme (`winweaver_win::color_title_bar`: DwmSetWindowAttribute 20 for
  the dark frame, 34 to 36 for the border, caption and text on Windows 11), only for the window of this
  process and again whenever the visuals change; the system theme is never written.
- Linked policies (`resolver::links`): an imported policy without parameters whose every Reg or RegRemove
  action one built-in rule in effect has too (path and value name ignoring case, the same kind and value)
  is linked to it: the tree and the group counts show it on (`Resolution::shown_on`), its check box is
  disabled with the built-in rule in the tip, the profile keeps it off, and `get_rule` names it in
  `linked_by`. A policy chosen on stays a normal rule.
- The archive: the job Package copies the exe, LICENSE, `package/README.md`, the presets as `profiles/`,
  `package/lang/` and `agents/` into `dist/WinWeaver` and uploads `dist/`, so the artifact is the one archive; a
  tag zips the same folder for the release. Agent texts stay short (local models with small contexts) and name no
  model: the person chooses it.
- Drafts: `drafts/<key>.json`, key `p-<fnv of the path as it is kept>` or `n-<time>-<n>`; written 2 s after
  the last change and at the close; a save or a discard deletes them.
- Users files (`winweaver_core::users` and `userstore`, `winweaver_win::users`, `usersview.rs`): one `.wwusers`
  file holds the accounts and their passwords encrypted (PBKDF2-SHA256, AES-256-CBC, HMAC-SHA256 over salt,
  iv and data, checked first; the reference vector of `userstore::tests` came from Python). The passwords
  live in `Passwords` (wiped on drop) only while the tab is unlocked, with the master password for the
  save (`UsersDoc::seal` encrypts them with a new salt and vector); nothing of them reaches the status bar,
  the logs or a plan file. The operations run in the process (`winweaver_win::run_users_plan`) and need
  the window to be elevated (`winweaver_win::elevated`), so the master password crosses no process: no
  helper and no plan file for them. The semantics: an account that exists changes
  only by its orders, no account is deleted, a profile elsewhere is kept, the account of the process stays
  in Administrators (known by the SID of its token, `native::current_sid`: a rename leaves `USERNAME`
  old until the next sign-in; the built-in accounts the apply disables are compared the same way);
  the tab lists the name and the full name, and a change of the group in the form sets `force_group`; a change of them changes `winweaver_win::users` and
  `data/templates/Create-Users.runtime.ps1` together. The script names its users file in Base64
  (`users::script`, any name; the comment of its first line through `render::ascii_text`). The test of
  the Windows runner makes and deletes
  the account `WwUsersTest`.
  The form Accounts makes a users file of its accounts (`users::from_accounts`, `Act::UsersFromProfile`; the
  passwords of the profile go into the tab unlocked, and `UsersDoc::seal` refuses a save until a master
  password is set); the tab shows and copies the password of the selected account (`password_row`,
  `show_password`, off at a lock). Compare, Unsaved changes (`Act::DiffUnsaved`) opens a diff of the
  profile in front against its file, or the preset of its name when it has none, through
  `compare_active`: the current profile left, the saved one right.
- Paths in the files of the program (`winweaver_core::paths`): a file inside
  the program folder is kept relative to it and starts with `.` (`.\profiles\Office.wwprofile`), any other
  file keeps its absolute path, and in memory every path is absolute. `Settings::load_from` and `save_to`
  (recent, open_tabs; relative to the folder of the settings), `Bookmarks::load` and `save` (the path of
  every bookmark but the registry ones), `Diff::load` and `save` (both sides) and the drafts (their path
  and `key_for_path`) convert at the file; `stored_text` changes only an absolute path, so a title of a
  preset or a key of the registry stays, and `resolved_text` only a text that starts with `.` and a
  separator. The ids of imports made of a folder, an archive or an ADMX file hash the kept form too
  (`kept_name` of the window). A new file that keeps a path converts the same way.

- Plugins (`docs/plugins.md`, `wview/`): a runtime as a Rust `dylib` (`wwrt.dll`, W# and egui
  re-exported) and the plugins as `cdylib`s on it, one build of all (one compiler, one profile, the
  same versions of the shared crates: `plugin::ABI`, its version, the interface, rustc, the target
  and the profile, is checked at the load, and another version of a shared crate fails the load
  by its symbols). The interface 1 (`wwrt::plugin`): `open` with the settings of the plugin (a
  JSON object a plugin in `Extra::plugins` of wview, by the name of its DLL, drawn by
  `settings_ui` in Settings, Plugins), `translations` (added by `ww::i18n::add`, on top of the
  embedded ones, under the files of `lang/`), a `Host` with the requests of a view
  (`Request::Browse`), `wwrt::input::area` for the keys and the mouse (a click gives the focus,
  the keys a view names are its own without Ctrl and Alt, Tab and Escape give them back), and
  `wwrt::testing` for the tests of the translations of a plugin (`plugin_texts`: tr, trf,
  `const NAME`, `name:`, `label:`). wview has the eight languages of WinWeaver: its `ui.json`
  holds the texts of the shell of W# as WinWeaver translates them and its own. `plugin!` exports
  nothing with the feature `library` of a plugin (each declares `library = []`): the tests of
  wview take the plugins as libraries (`crate-type = ["cdylib", "rlib"]`) into one executable, and
  egui_kittest comes through the feature `kittest` of wwrt. The release of wview is built in the
  tree of `wview/sdk/pack.py` (W# in `framework/` with its inherited fields written out, wwrt and
  wview in `crates/`, the plugins in `plugins/`), and that tree is its SDK: cargo hashes a path
  dependency by its place relative to the root of the workspace only inside it, else by its
  absolute path, and W# is outside `wview/` in the repository, so only a build in the tree makes a
  `wwrt.dll` that a plugin built in a copy of it anywhere loads. CI and build-all.cmd build the
  release there (`wview/target/tree` locally: the dlltool of gnu stops past 260 characters); CI
  builds the template `wv-hello` in another copy and loads it. For msvc every module of the
  family takes the CRT statically (`-C target-feature=+crt-static` in `wview/.cargo/config.toml`,
  since the tree has no parent config): the std-<hash>.dll of msvc imports no DLL of the CRT, and
  with the dynamic one every module imported `VCRUNTIME140.dll`. The package takes the
  std-<hash>.dll `wview.exe` imports (`wview/sdk/imports.py --std`; the first of the sysroot was
  another one and wview did not start), and a release build of wview attaches to the console of its
  parent only when its output is not redirected. CI: the Linux job checks the format and clippy of `wview`, the Windows job
  its clippy and tests (the screenshots `v1-` to `v5-`), the release build and the artifacts
  `wview-<version>` and `wview-sdk-<version>`; a release of WinWeaver does not carry them. The family links std dynamically
  (`-C prefer-dynamic`, `std-<hash>.dll` ships), never takes LTO (a dylib refuses it, a plugin on
  one stops) and keeps its dependencies at opt-level 2 in debug builds, where shared generics put
  128973 exports into the DLL (a DLL holds 65535; a release one has about 15600). Every crate the
  runtime holds is taken through it (`wwrt::serde`, `#[serde(crate = "wwrt::serde")]`). With the
  toolchain gnu a link writes 68 import libraries of Windows through dlltool, about 50 s a link.
- The structure of the code: a change of the window is an `Act`
  of one subsystem (`act.rs`), run by the `apply_<subsystem>` method in `app/<theme>.rs`; a question before
  a removal is `Act::Ask(Question)` built where the data is (`question.rs`, `remove_question` of a users
  file, a registry file and the bookmarks) and answered by `Act::Answer`; the only acts that still ask by
  themselves are `UsersAct::AskRun` and `RegistryAct::AskDelete`, since they check first. Every tab is a
  boxed document in `Doc` that answers `Tab` (`doc.rs`); the frame draws it by `DocKind`. The command bar
  is `App::commands` (`commands.rs`): a new button is one `Item`, its key one `Keys` constant, its icon one
  constant of `icons.rs`. A new left panel, modal window or file dialog uses `widgets.rs`; a new search
  filter `matches_words`; a tree or list that reveals its selection `tree::Scroll::wanted`. In the core a
  new JSON document implements `jsonfile::Document`; a new value of an action is read through
  `catalog::values`; in the Windows crate a new API call takes its wrappers from `ffi.rs`; in the MCP
  server a new tool is one row of `TOOLS` with its handler in `tools/<theme>.rs`.
- W# (`docs/ww.md`): `ww::Window<WinWeaver>` owns `shell: Shell<WinWeaver>` (the tabs, the
  settings, the status, the themes, the session) and `app: WinWeaver` apart, so every method of
  WinWeaver takes `shell: &mut Shell<WinWeaver>` (or `&Shell`) beside `self`; an act of the window
  is `Act::Shell(ShellAct::..)` (open, save, the tabs, the settings, the themes, `Ask` and `Answer`,
  `Message`, `ShowInFolder`, `OpenLink`, `About`), run by the shell, the others by `WinWeaver::apply`. A
  question is `Question::into()` (`Act::Shell(ShellAct::Ask)`), with a second answer by
  `Question::or` (`ShellAct::AnswerAlt`); closing a tab with changes asks Save, Don't save or Cancel
  (`Shell::close_question`, `ShellAct::SaveAndClose`), forgetting the recent files asks through
  `shell.forget_recent_act`. About is a window over the tabs (`App::about` gives its lines). The
  settings tab and the editor of a theme are `Doc::Shell(ShellDoc)`, drawn by the shell; a program
  section of the settings is `SettingsSection` with its `index()` for `ShellAct::OpenSection`. The
  tests build `Window::new(WinWeaver::new(catalog, texts), settings)` and read `w.shell.docs`,
  `w.app.marks`; a method of the app is called as `w.app.m(&mut w.shell, ..)`. The texts of the
  shell (`crates/ww/src`) are in every `ui.json` of WinWeaver (the test reads both crates) and in
  `package-wedit/lang/uk/ui.json`. wedit is built by the release build of the workspace and packaged
  by the job Package as `wedit-<version>.zip` (the exe, its README, `lang/`).
- The settings file: `Settings::load(program)` reads the
  first of `<exe stem>.settings.json`, `<program>.settings.json` and `settings.json` that exists and
  writes back to it (`Shell::settings_path`), so two copies of an executable with different names keep
  their own settings and a plain `settings.json` goes on. `Settings::wrap_lines` folds the lines of
  a text or a page to the width of the view (the setting Wrap lines of General); off, the view scrolls
  sideways (a `layouter` of infinite width in wedit, an infinite width for the page). A page always
  scrolls sideways for a table or a block of code wider than the view, and its code takes the colour
  `code_bg` of the theme. A program adds colours to the themes with `style::install_colors`
  (`ExtraColor`: name, label, light, dark); a theme file overrides them by name, the theme editor lists
  them after the palette, and `style::extra(ui, name)` gives the one of the theme shown. The fields of
  the form Languages (display language, system and user locale) are typed freely with the list of the
  languages of Windows beside them (`forms::locale_field`).
- Markdown pages: WW draws them itself (`ww::markdown`; pulldown-cmark parses): every paragraph, heading and cell is one `LayoutJob`, so text, code (`code_bg`, valign BOTTOM) and
  links share a baseline and wrap at the width of the view (`Settings::wrap_lines`; off, an infinite width
  and the view scrolls sideways); a link is found under the pointer by `Galley::cursor_from_pos`, a link
  `#anchor` scrolls the page to its heading, any other goes to `open_url`, and the shell takes it
  (`ShellAct::OpenLink`): a `.md` or `.txt` file or a file without an extension opens in the program,
  relative to the page (`Shell::linked_file`; WinWeaver shows a text file as it is, `MdDoc::plain`), a
  web link or a file of another kind is asked first and goes to the browser. A page scrolls with the
  arrows, the page keys, Home and End (`mdview::key_offset`), keeps its place a tab (its own scroll id)
  and its selected text when another tab comes in front (`LabelSelectionState` saved and given back). Lists put their markers in a column of their own on the baseline of the first line; blocks of
  code have a copy button; the whole page (its text, its blocks of code, its tables) is as wide as
  `Settings::page_width` (`PageWidth`: the view, or at most so many pixels or characters, never wider
  than the view; Width of pages in General); a table is bands of cells of that width, its columns shared by the
  length of their text and never narrower than their longest word (`markdown::column_widths`), every row
  as high as its highest cell. The scroll bars are solid (`ScrollStyle::solid`): visible whenever the
  content is larger than the view. The screenshots of CI draw the sample page `ww::sample::PAGE` and
  AGENTS.md at several widths and settings in both programs (`13-` to `20-`, `w1-` to `w11-`): look at them
  after every change of the drawing. `Settings::window` (`WindowPlace`) is
  where the window was at the close; `run` opens it there, and the first frame fits it to the monitor
  it opened on (`window::fit_window`: moved inside, shrunk to the monitor, never below
  `WINDOW_MIN_SIZE`); a maximized window keeps the place it had before.
- Icons: a theme names the font of the icons (`icon_font`, `style::ICON_FAMILIES`: Segoe UI Emoji by
  default, or Segoe UI Symbol), loaded after the font of the texts and before Segoe UI Symbol. egui draws a
  glyph in one colour, so with Segoe UI Emoji the icons of the command bar and the tabs are pictures drawn by
  DirectWrite through Direct2D into WIC with the colours of the font (`ww::coloricons`, once a size and a
  colour, as textures; `style::paint_icon`); without them, or when DirectWrite fails, an icon is a glyph.
- The editor of wedit: egui's TextEdit moves with the arrows, Home and End itself; Page Up and Page Down
  move the cursor by the height of the view through the layout of the last frame (`TextDoc::page_keys`,
  with Shift they select) and the view follows; the editor takes the focus at its first frame and again
  whenever its tab comes back to the front (a gap in `cumulative_pass_nr`).
- Scripts and registry files (`winweaver_core::export`): one place makes them for the selection of a
  profile, the whole profile (every rule; the ones off return to the defaults), a diff (its other
  side, `export::diff_profile`) and the command line (`--script apply|revert|audit FILE`, `--reg
  apply|revert FILE`, `--only IDS`, `--out FILE|FOLDER`, FILE a `.wwprofile` or a `.wwdiff`). A selection
  of only passes of the installation, the first sign-in or the sign-in screen has nothing for a PC
  (`export::applicable`): the block This PC of the detail panel is not drawn and its items of the
  menu This PC are off.
- Rules that write the same values (`registry::same_values`): the detail of a rule lists every rule
  of every catalog that writes or removes one of its registry values, which is the same both ways
  (a test checks it); the list is kept in the memory of egui by the rule and the count of rules.
- The parts of the description of an option have the colours `section-what`, `-risk`, `-writes`,
  `-deps`, `-more`, `-ref` (`views::SECTION_COLORS`, installed by `WinWeaver::new` with
  `style::install_colors`): a stripe on the left and a faint tint of the card.
- The registry tab: the values are `list::show` (the arrows, Home, End, the page keys, Enter edits,
  Delete asks), Rename (F2) renames the selected value or else the key (`registry::rename_value`;
  `Registry::rename_key` copies and deletes, `NativeRegistry` uses `RegRenameKey`).
- Registry files: the outline is a tree of the keys by default (`regfileview::tree_rows`, the folded
  keys in `RegFileDoc::closed`; List and Tree in the command bar set `Extra::regfile_tree`), the
  rules that write a key fold (`Extra::writers_open` at first) and scroll inside a third of the
  window. Apply to this PC applies the selected value, key or the whole file
  (`RegFile::changes`, `regfile::apply_changes` with the registry of the window) after a question
  that counts the changes and the deletions; the keys of the computer need the window as
  administrator. Every change this PC refused goes to the log and to the message bar of the tab
  (`RegFileDoc::applied`, the messages of `App::messages` for a registry file, its paths as links);
  the status names the first one.
- Group Policy files (`winweaver_core::pol`, `polview.rs`, `app/pol.rs`): `Registry.pol` of the
  computer (`Machine`, HKLM) and of the users (`User`, HKCU) of this Windows or any `.pol` file in a
  tab: the keys as a tree, the entries with what each does (set, set when absent, delete, delete
  every value, the lists of `**DeleteValues` and `**DeleteKeys`, `**SecureKey`). The files of this
  Windows are read once (`polview::system`, again at Read again) for `polview::link`, the small button
  beside a key or a value that Group Policy sets in the detail of a rule, the registry tab and a
  registry file. The menu, the command bars of a profile, a catalog and the registry have Group
  Policy; `Extra::pol_with_pc` opens the files of this Windows behind the tab in front after a read
  of this PC.
- wwlite (`crates/wwlite`): the window of WinWeaver built without the feature `full` of winweaver-ui
  (`winweaver_ui::lite()` is a const of the build): `App::name` gives `wwlite` (its title, its About,
  the settings file `wwlite.settings.json`), `App::edits_themes` is off while the fonts of the theme
  stay in the settings (`App::chooses_fonts`, item 15 of the list), `languages` is Ukrainian
  alone (`LANGUAGES_LITE`), the command bar of a profile has New, Open, Save, Check, Build and Only
  on, the menu has New, Open, Save, Guide (plain text, `pages::guide_lite_text`), License (plain),
  Settings (General and Files), About and Exit, `open_any` takes profiles, answer files and old JSON
  profiles only, the detail panel has no This PC, no registry buttons, no catalog links and no
  Reference. Everything else (`cfg(feature = "full")`: the acts, the documents, the modules of
  `app/` and the views of This PC, the registry, registry and Group Policy files, users files, the
  catalogs, diffs, bookmarks and MCP, the fields of `WinWeaver::full`, the MCP crate, the crypto of
  users files through the features `users` of the core and of the Windows crate) is not in its
  executable. Cargo unifies the features of one build, so wwlite is checked, tested and built alone
  (`-p wwlite`, the steps of CI); its tests say so and pass in a build with the feature. CI packages
  `wwlite-<version>.zip` from the build of wwlite alone. wwlite for Windows has no settings of the
  imports: `run` merges every catalog file of `imports/` next to it at the start
  (`with_every_import` of `lib.rs`, the templates of this Windows first, a file it cannot read left
  out), so the user chooses the catalogs by the files of the folder; the browser edition reads none.
- The catalog by mechanism (`Catalog::mechanism`, `Mechanism::roots`, the test
  `every_rule_sits_under_a_root_of_its_mechanism`): a rule of answer file settings alone is
  `Unattend` (under Installation), of registry actions alone `Registry` (under Policies, whatever
  the key), of PowerShell steps alone `Script` (Installation or Other), anything else or a mix
  `Other`. Installation holds `install`, `oobe`, `install.scripts`, `user-logon` and `post-oobe`;
  Policies the groups of registry rules (`update`, `defender`, `security` with the network and
  the logging of PowerShell, `privacy` with `ads` and `speech`, `system` with `explorer`,
  `apps.policies` with `onedrive` and the browsers `browsers.edge`, `browsers.chrome` and
  `browsers.brave`, `default-user`, the imports); Other its few rules directly (security,
  network, logging, telemetry, system: the customer's decision of 28.10.2026, a group of two or
  three rules is no use) and `apps` with `apps.remove`. Only the root Other holds rules directly
  (the test `only_the_root_other_holds_rules_directly`).
- Text parameters have `max_length` (`Param::Text`, checked by the catalog check and the check of a
  profile, the field counts its characters against it; `set_param` of MCP takes it as its limit):
  the support contacts have the limits of the answer file (256, the URL 2083), the autologon
  account 20; an ADMX `maxLength` becomes it. The fields of the parameters take the width of the
  panel, the title and the default above them.
- Only on (`ProfileAct::OnlyOn`, `ProfileDoc::only_on`, `ProfileDoc::shows`): the tree shows the
  rules that are on (chosen or shown on through a linked rule) with their groups open, like a
  search; the switch of the command bar is `commands::toggle`, drawn pressed when on. MCP:
  `only_on` of `list_groups` (with the count `on` of each group), `list_rules` and `search_rules`.
- The panel of the work in the background (`App::jobs`, `ww::Job`, `shell/jobs.rs`): WinWeaver
  lists the read of this PC, an import, a change of this PC and the search of the registry (with
  its Stop, which stops the search of every registry tab), each with the time since it was first
  seen (`WinWeaver::track_jobs`, `Full::jobs_since`); the panel folds (`ShellAct::FoldJobs`).
- The Group Policy editor (`PolAct`, `polview::PolEdit`, `EntryKind`): entries made, changed (Enter
  or a double click in the list) and removed after a question (Delete in the list), the root of a
  file outside Group Policy chosen in the bar (`PolAct::Root`, a change of the view, not of the
  file), the entries of another `.pol` file (`PolFile::merge`) or the changes of a `.reg` file of
  that root (`PolFile::merge_reg`) added, the file written back as PReg (`PolFile::to_bytes`,
  `save`) or as a registry file (`PolFile::to_reg`); the files of Group Policy of this Windows are
  never written (`PolDoc::is_system`, `pol_is_system`): Save asks for a place for a copy. The draft
  of a `.pol` tab is its bytes as hexadecimal with the root chosen (`POL_DRAFT`).
- Menus (`ww::shell::menu_item`, `menu_entry`, `sub_menu`): every item keeps the place of an icon
  at its left (`icon_place`, an empty image of the size), painted by `style::paint_icon`, so the
  texts line up with or without one; the menus of WW itself (a tab, the breadcrumb, the recent
  searches, the copy menus of texts) have icons too. The tab row (`shell/tabs.rs`) is a scroll area
  without a bar when the tabs are wider than the window: the wheel scrolls it sideways, a new active
  tab or count scrolls the active one into view, and a button after the new tab lists every tab.
- The presentation (`docs/presentation/make_presentation.py`, item 7 of the second list of
  25.10.2026): `SLIDES` is the list of slides (a title, lines, a screenshot of the artifact
  `screenshots` and a box of pixels for a fragment); the Windows job installs python-pptx and
  Pillow and uploads `presentation-<version>`. A new screenshot of the window may become a slide;
  a change of the layout of the window moves the boxes.
- The command line (`crates/winweaver/src/cli.rs`): `--build`, `--import-admx`, `--export-catalog`,
  `--read-pc`, `--diff` beside `--script` and `--reg`; `--out` names the file when it ends with the
  extension the command writes, else a folder for the default name (`export::out_path`).
- The Linux job checks the whole workspace for Windows (clippy on the target
  `x86_64-pc-windows-gnu`), so a run on demand with `-f windows=false` checks a change of any crate
  in a few minutes; the Windows job runs the tests of the window, the tests that change Windows and
  the archives.
- The browser edition of wwlite (`docs/wasm.md`): the same window built for `wasm32-unknown-unknown`
  with the feature `glow` alone (WebGL; the wgpu of WW is DirectX 12 only). `ww::web::start` (eframe's
  `WebRunner`, the canvas `wwlite` of `crates/wwlite/web/index.html`, `WebLogger` for the console)
  replaces `ww::run`, called by `winweaver_ui::run_web` from the `main` of wwlite as a task of the page.
  The files of the window go through `ww::store` (`exists`, `read`, `write`, `remove`, `list`,
  `log_line`): natively the files next to the executable as before, in a browser the `localStorage` of
  the page under the key `ww/<path>` and the log in the console; the settings, the drafts and a theme the
  fonts of the settings save live there (`Settings::load`, `session`, `themes` call the store). A file
  comes by its bytes: Open runs `rfd::AsyncFileDialog` in a task of the page (`Shell::pick_file`, the
  kinds of `App::open_filters`) and a drop reads `bytes_async` (`Shell::read_dropped`), both into
  `Shell::picked`, which the next frame turns into `ShellAct::OpenBytes(name, bytes)` and
  `App::open_bytes` (`open_any_bytes` of the light edition: a profile, an answer file with its profile,
  an older profile; the tab has no path, so it shows the name of the profile). A save is a download
  (`store::download`: a Blob and a link clicked): Save and Save as give the profile, Build the answer
  file alone (the scripts are embedded in it), the fonts of a theme its file. `App::open_dialog`, the
  sync dialogs of rfd, the title bar colours, `ww::run` and the fallback of renderers are native only
  (`cfg(not(target_arch = "wasm32"))`). CI: the Linux job checks it with clippy, the job `build-web`
  (Ubuntu, left out by `-f web=false`) builds it in release and runs the wasm-bindgen of the version
  of Cargo.lock into the artifact `wwlite-web-<version>` (`wwlite.html`, the edition in one file
  by `crates/wwlite/web/single.py`, the module packed with gzip into base64 and unpacked by
  `DecompressionStream`, so it opens from the disk too; README.md; `files/` with index.html, wwlite.js
  and wwlite_bg.wasm for a server), zipped by a tag. Keys: a page takes Alt+N for New, Alt+W for Close, Alt+PgDn and Alt+PgUp for the next and the previous tab and Alt+Shift+PgUp/PgDn to move a tab, since a browser keeps Ctrl+N, Ctrl+W, Ctrl+Tab and Ctrl+Shift+PgUp/PgDn for itself (`ww::keys`), and Alt+I for the guide, since F1 opens the help of the browser (`commands::keys::GUIDE`); the shell checks the moves
  (with Shift) before the switches, since a shortcut matches with an extra Shift. A listener of
  `crates/wwlite/web/index.html` stops the browser from acting on F2, F3, F7, F9, Ctrl+F,
  Ctrl+D and Ctrl+B (letters by `code`, so a Cyrillic layout keeps them), eframe on Ctrl+O, Ctrl+S
  and Ctrl+,; a new key of a page avoids the keys a browser keeps. Open: no question before the page closes, the folder
  `files/` is served over http, never from `file://`, and the names of the input languages and the regions are English (the embedded lists).
  The guide of a page is `data/docs/guide-web.txt` (`pages::guide_lite_text` under
  `cfg(target_arch = "wasm32")`, English and Ukrainian), written for someone who knows no other
  edition (item 14 of the list of 28.10.2026): a change of the browser edition changes it too.
  A page draws with the fonts of egui alone (Ubuntu Light, Noto Emoji, its icon font), and a sign
  they lack is an empty box: the cross of a tab (U+2715) was one. `ww::icons` gives a build for
  wasm32 signs they have for the cross, the arrows and the small triangles; a new icon a page shows
  is checked against those fonts (a glyph of 0 in their cmap is the box).
  The menu of a page has no Open recent and no Exit (`cfg(not(target_arch = "wasm32"))` in
  `app/menu.rs`). `docs/wasm.md` section 6 builds the page by hand, without CI.

## 7. Work status

| Area | State |
|---|---|
| Specification | `docs/spec/`: the scope, the interface, the documents, the features and the architecture as they are |
| Core | Catalog, profiles, resolver, check, diff, bookmarks, build of the answer file, This PC (plan, judgement, capture, reports), stand-alone scripts, import of answer files and old JSON profiles, ADMX import and catalog files, registry files, users files; tests on the real data |
| Windows API | The registry with the permissions of keys, the elevation, the locales and layouts, the language of Windows, the local accounts, DISM, the package API, the native apply with its backup and undo, the default user profile, the users of a users file |
| Window | WinWeaver on W#: tabs, menu, command bar of each tab, navigation, search per tab, the painted tree and lists, detail panel, forms, check, build, diff tab, bookmarks, Markdown pages, This PC, registry, registry files, catalogs and their editor, users files, MCP tab, settings, nine languages, drafts, restored tabs, log, themes with fonts and icons in colour |
| MCP server | stdio and HTTP, 36 tools, 4 resources, modes, journal, token, configuration files; tests of both transports |
| W# and wedit | The crates `ww-base` and `ww`, the editor `wedit` with its archive, `docs/ww.md` |
| wwlite | The light edition for Windows with its archive, built without the feature `full` of winweaver-ui; the browser edition (wasm32, `docs/wasm.md`) built by CI as `wwlite-web-<version>`: files from the browser and downloads, the settings and drafts in the storage of the page, the embedded list of input languages |
| Done in pull request #61 (0.25.0) | The list of the customer of 24.10.2026 in six groups: the shell (keys, focus, About, links, icons, the question of a closing tab), the documents, the profile (links between catalogs, colours of the panels, `.reg` and `.ps1` of a diff and a whole profile, the command line), the registry and `.pol` files, drafts of every document, `wwlite` and a study of a Wasm edition |
| Done in pull request #62 (0.26.0) | The list of the customer of 25.10.2026, 14 items: the command line (the answer file, the import and export of templates, a read of this PC, diffs, `--out` as a file), Open and Save as of the MCP tab, the comparison of catalogs as a tab with its report, the panel of the work in the background, the Group Policy editor, Copy to of a catalog, Only on, the scrolling tab row, the catalog by mechanism with the root Policies, icons in every menu, wwlite without the code of the full edition, the hammer of Build, the parameters in the width of the panel with their limits |
| Done in pull request #63 (0.27.0) | The second list of the customer of 25.10.2026: the region of the default user profile from the list of Windows, the editors in the width they have, text sizes up to 36, every catalog file in the menu Catalogs, wwlite for the browser (the store, the web start, the embedded input languages, the artifact wwlite-web), the presentation of the screenshots |
| Done in pull request #64 (0.28.0) | The list of the customer of 28.10.2026: a group removed in the catalog editor, no recent files and no Exit in the menu of the browser edition, its list of regions, the build of the page by hand, every catalog of `imports/` in wwlite, the built-in catalog flatter (the small groups of Other into its root, the network and the logging of PowerShell into Security, the browsers into Apps), the whole description of a rule and the titles of the values in the language of the window, the browser edition in one file `wwlite.html`, CI only for a release or on demand, the checks before a commit on the computer of the developer, the tests that change Windows ignored by a plain `cargo test`, the guide of the browser edition for a first-time user, its icons in the fonts of egui, its keys that work in a browser |
| Done in pull request #65 (0.29.0) | The lists of the customer of 29.10.2026: renamed accounts in a users file (the full name in the list, a change of group moves the account, the account of the process by its SID), the read of the whole PC with the loaded templates, every refused change of a registry file shown, the address row of the registry at large sizes, the click of the menu Catalogs with its setting, the marks of the fields of a comparison of catalogs, the disc of a profile tab |
| Done in pull request #66 (0.30.0) | The list of the customer of 30.10.2026: presets with the culture of the user, the form Profile, This PC in the main menu with its items on where they apply, a read of this PC without a profile, the stripes of the description inside their cards and optional, the window maximized without a blink |
| Done in pull request #67 (0.30.1) | `build-all.cmd`: what CI makes (the four archives of a release, on demand the checks, the screenshots and the slides) built on a computer of Windows; the programs of 0.30.0 |
| Done in pull request #68 (0.30.2) | The languages of Windows without a locale id of their own read without the transient id Windows lends them as input languages of the user (`ru-UA` as 2000) |
| Done in pull request #69 (0.30.3) | The study of plugins (`docs/plugins.md`, then `study/plugins`): the ways of Rust compared, the runtime wwrt with the loader and its check, wview with the plugins of text and images, the measurements, the design and the phases of wview; the programs of 0.30.2 |
| In pull request #70 (0.31.0, wview 0.2.0) | Phase 2 of wview, part 1: the workspace `wview/`, the interface 1 (the settings and the translations of a plugin, the requests of a view, the keys and the mouse), Settings, Plugins, the image view with its mouse and keys, the eight languages, the guide; `ww::i18n::add`. Part 2: the SDK of a release (`wview/sdk`, the template `wv-hello`, built and loaded by CI), wview and its SDK in CI and build-all.cmd, the tests of the window with the screenshots `v1-` to `v5-` |
| Acceptance | The acceptance page in a VM: every part up to Q of 0.22.1 accepted; parts R to V of 0.22.1 and W to Z of 0.24.0 wait |

Decisions of the customer: the UI has the compact layout for narrow windows, its inline toggles and the
secondary tabs; the search field grows into the free width of the navigation row; the extensions
`.wwprofile`, `.wwdiff`, `.wwcatalog`, `.wwmarks`, `.wwusers`, `.wwtheme`, `.wwmcp`; the runtime scripts are
data (`data/templates/`); wview is a direction of its own, apart from WinWeaver, and lives with its
plugins in a workspace of its own next to the one of WinWeaver; its plugins are native ones with the
SDK of a release (every example) and, for other languages, a stable interface of C with one
example, the Mandelbrot set (04.11.2026, `docs/plugins.md` sections 6.5 to 6.10); Windows 10 and 11 are supported, the systems the Rust toolchain supports (Windows 7
and 8.1 are not).

## 8. How to update this file

Structure changed: section 3. A command appeared: section 5. A fact that affects future changes: section 6.
A task finished or started: section 7. A decision: the specification and section 7. The date in the header on
every change.
