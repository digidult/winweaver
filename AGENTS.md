# AGENTS.md: map of the WinWeaver repository

For agents and developers: where things are, what to read first, which rules apply, where the
information that changes is.
Updated with every change of structure, commands or task status. Last update: 13.11.2026.

Repository: https://github.com/digidult/winweaver (private, branch `main`). Commits are authored by
`digidult <271767060+digidult@users.noreply.github.com>` (a local `git config` of this clone, never global).
A commit made by an agent ends with the line `Co-Authored-By: <agent model> <noreply@anthropic.com>`.

## 1. The project in three sentences

WinWeaver is a portable editor of Windows 11 installation profiles: it builds `autounattend.xml`, compares
profiles, reads the settings of this PC natively, applies them natively or through generated stand-alone
scripts, imports ADMX policy templates as catalogs, edits the registry, registry files and users files, and
serves an MCP server. It is written in Rust with eframe (egui + glow and wgpu + winit) for Windows 10 and
11, one executable with data folders next to it, on the framework W# (`ww`, supakov/ww), a git
dependency of its branch `main`.

## 2. What to read first

| Task | Start with |
|---|---|
| Understand the scope and the principles | `docs/spec/00-scope.md` |
| Design or change the interface | `docs/spec/01-ui.md`, then `crates/winweaver-ui/src/style.rs` (the palette, the painted widgets) |
| File types, formats, what lives next to the executable | `docs/spec/02-documents.md` |
| What the program must do (feature inventory) | `docs/spec/03-features.md` |
| Crates, modules, threads, build | `docs/spec/04-architecture.md` |
| Use or change the MCP server | `crates/winweaver-mcp/src/lib.rs` (the source of the tools of WinWeaver), `tools/mod.rs` (the table of the tools), `agents/skills/winweaver/SKILL.md`; the server, its tab and its command line are W#'s (`docs/mcp.md` and `crates/wwmcp` of supakov/ww) |
| Change what an AI agent or the archive gets | `agents/README.md`, `package/README.md` |
| Change W#, the framework of the window, or its plugins | the repository supakov/ww (its AGENTS.md, `docs/ww.md`, `docs/plugins.md`); a change of W# is checked against WinWeaver before its pull request (section 5) |
| Change winweaverweb, WinWeaver in a browser | `crates/winweaverweb`, the features `pc` and `mcp` of `crates/winweaver-ui` (what the browser has not), `src/app/web.rs`, `docs/wasm.md` section 0a |
| Change wwlite, the light edition | `crates/wwlite`, the feature `full` of `crates/winweaver-ui` (`cfg(feature = "full")` marks what wwlite leaves out), `package-wwlite/README.md`; the browser edition: `docs/wasm.md`, `crates/wwlite/web`, `crates/ww/src/web.rs` and `store.rs` |

The information that changes:

| What | Where |
|---|---|
| The plans and the work to come | [docs/todo/README.md](docs/todo/README.md) |
| What each version brought (the history) | [docs/releases/](docs/releases/) |
| The facts a change of a part must respect | [docs/spec/05-details.md](docs/spec/05-details.md) |
| The folders and the modules | [docs/spec/04-architecture.md](docs/spec/04-architecture.md), section 10 |
| The pull requests open, the runs of CI | `gh pr list`, `gh run list --workflow ci.yml` |

## 3. Folder structure

```
(repository root)
├── AGENTS.md, README.md, LICENSE
├── Cargo.toml             the workspace; rust-toolchain.toml, rustfmt.toml, .cargo/config.toml,
│                          .gitattributes (LF, CRLF for .cmd, .bat, .ps1)
├── build-all.cmd          what CI makes, on a computer of Windows (section 5)
├── .github/workflows/ci.yml  CI (section 5)
├── crates/                winweaver-core, winweaver-win, winweaver-mcp, winweaver-ui, winweaver (the
│                          executable), wwlite, winweaverweb (WinWeaver in a browser); W# comes from
│                          supakov/ww
├── data/                  catalog, presets, resources, templates, lang, themes, docs, reference
├── agents/                what an AI agent gets in the archive
├── package/, package-wwlite/   what the archives add
└── docs/                  spec/ (the specification), releases/ (the history), todo/ (the plans),
                           wasm.md, presentation/
```

Every folder and module: `docs/spec/04-architecture.md`, section 10. Next to the executable at run time
(never versioned): `settings.json`, `bookmarks.wwmarks`, `profiles/`, `output/`, `imports/`, `drafts/`,
`mcp/`, `logs/`, `lang/<code>/`, `themes/` (`docs/spec/02-documents.md`).

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
   `data/resources/regions.json`, the lists of a Windows 11 (`docs/spec/05-details.md`); a build for Windows never
   carries them.
4. Dependencies: popular, well known, open source Rust crates that need no extra DLL next to the executable.
   The executable and its configuration stay portable: nothing is installed, nothing is written outside the
   program folder and the system temp folder. W# is a git dependency of the branch `main` of
   supakov/ww (`[workspace.dependencies]`), taken at its last commit before a build.
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
11. Versions: `version` of the workspace `Cargo.toml` (0.1.54 on 13.11.2026) grows with every
    pull request that reaches `main`. An agent changes the third number alone (0.1.45 to 0.1.46);
    the first and the second change only on the word of the customer. Every build
    names its commit: `winweaver_core::BUILD` is `<version>+<12 digits of the commit>` (`build.rs` of the core,
    `WINWEAVER_COMMIT` set by CI), shown by `--version`, About, the logs, the MCP server, the answer file
    and the scripts.
12. The documents hold the current state. The history of the changes is in `docs/releases/` (one file a
    version, written with it); what is planned is in `docs/todo/` (a file a subject), and a task done
    leaves `docs/todo/` for the lasting documentation: the specification (`docs/spec/`, with the facts
    of `05-details.md`), `docs/wasm.md`, the guides.

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

On a computer of Windows, `build-all.cmd` of the root makes what CI makes:
the release builds of the workspace and of wwlite alone, wwlite and winweaverweb for the browser
(wasm-bindgen of the version of `Cargo.lock`, `single.py`), the folders of the step Package and the
four archives of a release, `dist/WinWeaver-<version>.zip`, `wwlite-`, `wwlite-web-` and
`winweaverweb-<version>.zip`, each with its folder inside as the job `release` zips it (`tar.exe` of
Windows). `check` runs the checks of
a commit first (the format, the three clippy runs, both test runs, `--check-catalog`), `screenshots`
the tests with `WINWEAVER_SCREENSHOTS` set to `dist/screenshots`, `presentation` the slides into
`dist/presentation` (python-pptx and pillow), `all` the three; `noweb` leaves out the browser
editions (two archives). The tools come from `WW_CARGO` (a script that runs cargo on half of the processor, say),
`WW_PYTHON` and `WW_WASM_BINDGEN`, else from the PATH; a wasm-bindgen that is missing comes once into
`target/tools` from its release on GitHub, as CI takes it, with its SHA-256 checked. Every tool is
found before the first long step. It never runs the tests that change Windows and starts the
program only for `--check-catalog`; the smoke tests of the window and of the command line stay in
CI. A change of the steps Package or build-web of `ci.yml` changes `build-all.cmd` too. The file keeps
CRLF (`.gitattributes`): cmd finds its labels by the offsets of the lines and reads a running script
from the disk again, so it is never edited while it runs. It takes W# of the branch `main` first
(`cargo update -p ww -p ww-base`).

```bat
build-all.cmd
build-all.cmd check
build-all.cmd all
build-all.cmd noweb
```

Before every commit, on the computer of the developer, the
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

CI builds only releases:

1. A tag `v<version>` runs everything and publishes the release. A tag is pushed only when the
   customer asks for the release of that version in so many words; finished work alone never makes one.
2. Any branch runs on demand, and only when the user allows that run in chat (from 05.11.2026): the
   whole Rust check of this section has run on the computer of the developer first, and the run
   builds the parts of its task alone. `gh workflow run ci.yml --ref <branch>` (or Actions, CI, Run
   workflow); `-f checks=false` leaves out the Linux job (its checks ran locally), `-f package=false`
   the release builds, the archives and their checks (a run for the tests and the screenshots);
   `-f windows=false` stops after the Linux job, `-f web=false` leaves out the browser edition;
   `gh run list --branch <branch> --workflow ci.yml` finds the run. It is the place of the tests that
   change Windows, the screenshots of the window and the archives for an acceptance.
3. Pushes, pull requests and merges run nothing. A newer run of the same branch cancels the older one.
4. The Linux job comes first and the other jobs only after it, side by side: the Windows job
   (`build-windows`) and the job `build-web` (Ubuntu: wwlite for the browser, the artifact
   `wwlite-web-<version>`). A release waits for both. A whole run took 41 minutes on 04.11.2026,
   on the runner of 2 processors a private repository gets. Every job fetches W# (supakov/ww, a
   private repository) with the token of the secret `WW_TOKEN` (a token that reads supakov/ww; the
   mirror needs it too) and takes the last commit of `main`.
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

## 6. Working rules and the details of the parts

- The workflow: write, then the checks of section 5 on the computer of the developer, then the commit;
  CI only for a release or on demand, and a run only with the word of the user for it, of the parts of
  the task (`gh run view <id> --log-failed` reads a failed step).

- Agents editing from Git Bash: a heredoc swallows doubled backslashes (`\\` becomes `\`), so Rust or Python
  text with backslashes is written with the file tools or a script file, never through `<<'EOF'`.

Before a change of a part, read its section of `docs/spec/05-details.md`:

| Part | Section |
|---|---|
| This PC, the apply and the accounts | [1](docs/spec/05-details.md#1-this-pc-the-apply-and-the-accounts) |
| Languages, regions, the forms and the answer file | [2](docs/spec/05-details.md#2-languages-regions-the-forms-and-the-answer-file) |
| The catalog, its imports and the links of rules | [3](docs/spec/05-details.md#3-the-catalog-its-imports-and-the-links-of-rules) |
| The registry, registry files, Group Policy and the scripts | [4](docs/spec/05-details.md#4-the-registry-registry-files-group-policy-and-the-scripts) |
| The MCP server and the logs | [5](docs/spec/05-details.md#5-the-mcp-server-and-the-logs) |
| The window and W# | [6](docs/spec/05-details.md#6-the-window-and-w) |
| wwlite and the browser edition | [7](docs/spec/05-details.md#7-wwlite-and-the-browser-edition) |
| Tests, CI, builds, archives and the command line | [8](docs/spec/05-details.md#8-tests-ci-builds-archives-and-the-command-line) |

## 7. State and decisions

| Area | State |
|---|---|
| Specification | `docs/spec/`: the scope, the interface, the documents, the features and the architecture as they are |
| Core | Catalog, profiles, resolver, check, diff, bookmarks, build of the answer file, This PC (plan, judgement, capture, reports), stand-alone scripts, import of answer files and old JSON profiles, ADMX import and catalog files, registry files, users files; tests on the real data |
| Windows API | The registry with the permissions of keys, the elevation, the locales and layouts, the language of Windows, the local accounts, DISM, the package API, the native apply with its backup and undo, the default user profile, the users of a users file |
| Window | WinWeaver on W#: tabs, menu, command bar of each tab, navigation, search per tab, the painted tree and lists, detail panel, forms, check, build, diff tab, bookmarks, Markdown pages, This PC, registry, registry files, catalogs and their editor, users files, MCP tab, settings, nine languages, drafts, restored tabs, log, themes with fonts and icons in colour |
| MCP server | The server of W# (`wwmcp`) with the source `winweaver`: 36 tools in 6 groups, 4 resources, its options, the switches of the tools, the call by hand, configuration files of W# (older ones converted); tests of both transports |
| W# | In its repository since 07.11.2026: W# and the system of the plugins in supakov/ww; this repository keeps their history until 0.43.0 |
| wwlite | The light edition for Windows with its archive, built without the feature `full` of winweaver-ui; the browser edition (wasm32, `docs/wasm.md`) built by CI as `wwlite-web-<version>`: files from the browser and downloads, the settings and drafts in the storage of the page, the embedded list of input languages |
| Acceptance | Every part of every version up to 0.44.0 accepted (08.11.2026); the acceptance page of the customer (a claude.ai page with the steps of each part, run in a VM of Windows 11 on the archives of a build) holds no open part, and a new part comes when the customer asks for an acceptance |
| Plans | [docs/todo/](docs/todo/README.md); the history of every version: [docs/releases/](docs/releases/) (from 0.1.45) |

Decisions of the customer: the UI has the compact layout for narrow windows, its inline toggles and the
secondary tabs; the search field grows into the free width of the navigation row; the extensions
`.wwprofile`, `.wwdiff`, `.wwcatalog`, `.wwmarks`, `.wwusers`, `.wwtheme`, `.wwmcp`; the runtime scripts are
data (`data/templates/`); the repository is split (07.11.2026): W# and the system of the plugins in
supakov/ww, a git dependency of its branch `main` for every program, WinWeaver here; Windows 10 and 11 are supported, the systems the Rust toolchain
supports (Windows 7 and 8.1 are not).

## 8. How to update this file

A structure changed: section 3 and `docs/spec/04-architecture.md`. A command appeared: section 5. A
fact that affects future changes: its section of `docs/spec/05-details.md`. A rule or a decision of the
customer: section 4 or 7. A task planned: `docs/todo/`; a task done: out of `docs/todo/` into the lasting
documentation and the notes of its version in `docs/releases/`. The date in the header on every change.
