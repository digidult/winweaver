# AGENTS.md: map of the WinWeaver repository

For agents and developers: where things are, what to read first, which rules apply, the state of the work.
Updated with every change of structure, commands or task status. Last update: 14.10.2026 (the map rewritten
after the first day: the core, the window, This PC, the scripts, the translations, the session and the MCP
server).

Repository: https://github.com/digidult/winweaver (private, branch `main`). Commits are authored by
`digidult <271767060+digidult@users.noreply.github.com>` (a local `git config` of this clone, never global).
A commit made by an agent ends with the line `Co-Authored-By: <agent model> <noreply@anthropic.com>`.

## 1. The project in three sentences

WinWeaver is the successor of WinKickOff: a portable editor of Windows 11 installation profiles that builds
`autounattend.xml`, compares profiles, reads the settings of this PC natively, generates stand-alone scripts
that apply and revert them, and serves an MCP server; importing ADMX policy templates and applying natively
come next. It is written in Rust with eframe (egui + glow + winit) for Windows 10 and 11, one executable with
data folders next to it. The data of the predecessor's rule catalog, its runtime templates and its whole
functionality carry over; the file formats are new and no program code is ported (clean room).

## 2. What to read first

| Task | Start with |
|---|---|
| Understand the scope and the principles | `docs/spec/00-scope.md` |
| Design or change the interface | `docs/spec/01-ui.md`, then `docs/design/README.md` and the mockups |
| File types, formats, what lives next to the executable | `docs/spec/02-documents.md` |
| What the program must do (feature inventory) | `docs/spec/03-features.md` |
| Crates, modules, threads, build | `docs/spec/04-architecture.md` |
| Use or change the MCP server | `crates/winweaver-mcp/src/lib.rs`, `tools.rs`, then `skills/winweaver/SKILL.md` |

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
├── .gitattributes         LF for every text file (data/templates too)
├── .github/workflows/ci.yml  CI: fmt (a patch artifact), clippy and tests of winweaver-core on Linux; clippy,
│                          tests, the catalog check, the release build and the zip on Windows; a tag v<version>
│                          publishes a release with docs/releases/v<version>.md
├── crates/
│   ├── winweaver-core/    pure Rust, compiles everywhere: catalog (model, embedded data, checks), jsonfile
│   │                      (strict reading), i18n (catalog translations), profile (.wwprofile), resolver,
│   │                      validate (the check of a profile), resources (time zones, keyboards, editions with
│   │                      their keys), diff (.wwdiff), bookmarks (.wwmarks), clock, render (autounattend.xml
│   │                      from the templates), thispc (the plan of checks of a running Windows, the judgement
│   │                      of the rules, the capture into a profile, the report of the audit script, an
│   │                      in-memory machine), scripts (stand-alone Apply, Revert, Audit and Undo scripts),
│   │                      import (an answer file or a WinKickOff profile as a profile); tests/ on the real data
│   ├── winweaver-win/     the Windows API behind thispc::Machine: the registry (winreg), the token elevation,
│   │                      the locale names and the language of Windows (windows-sys); reads only
│   ├── winweaver-mcp/     the MCP server: lib.rs (protocol, modes, workspace, journal, token), tools.rs (the
│   │                      19 tools and 3 resources), stdio.rs, http.rs (127.0.0.1, bearer token, Host and
│   │                      Origin checks); tests/server.rs (both transports on a free loopback port)
│   ├── winweaver-ui/      the window on eframe (variant A): lib.rs (tabs, menu, command bar, navigation,
│   │                      search, shortcuts, file dialogs, This PC, build, session, the actions applied after
│   │                      each frame), views.rs (tree, detail panel), forms.rs (Installation, Accounts,
│   │                      Languages), diffview.rs, mdview.rs (Markdown tabs, the guide), marksview.rs,
│   │                      mcpview.rs (the MCP tab), session.rs (drafts, the log), settings.rs (settings.json),
│   │                      i18n.rs (tr and trf, the translations of the interface)
│   └── winweaver/         the executable: the window, --mcp stdio, --mcp http, --version, --check-catalog
├── data/                  embedded into the executable at build time
│   ├── catalog/           groups.json, 00-17 rules files, lang/ru.json and lang/uk.json (catalog 0.11 of the
│   │                      predecessor)
│   ├── presets/           office, strict, laptop, home .wwprofile (converted once from the predecessor)
│   ├── resources/         timezones.json, keyboards.json
│   ├── templates/         the runtime templates of the predecessor carried over as data: the answer file
│   │                      skeleton and the scripts with {{slot}}s (they name WinWeaver now), VERSION 0.11
│   ├── lang/<code>/       ui.json: the translations of the interface (uk, ru); guide.md: the guide translated
│   └── docs/              guide.md, the English guide shown at the first start
├── skills/winweaver/      the Agent Skill for AI agents that use WinWeaver over MCP
└── docs/
    ├── releases/          release notes v<version>.md
    ├── spec/              clean room specification (00 scope, 01 UI, 02 documents, 03 features, 04 architecture)
    └── design/            UI design variants: README.md (comparison, the decision) and mockups/ (HTML)
```

Next to the executable at run time (never versioned): `settings.json`, `bookmarks.wwmarks`, `profiles/`,
`output/` (the answer file, the scripts), `drafts/` (unsaved profiles), `logs/` (winweaver-<date>.log),
`lang/<code>/` (more or overriding translations).

## 4. Project rules

1. Language: everything in the repository is English (code, comments, documentation, commit messages, issue
   texts). The interface, the catalog and the user documentation are English sources with Ukrainian and
   Russian translation files; other languages are files added next to the executable.
2. No em dash (U+2014) and no en dash (U+2013) in any text we create, in any language: documents, code,
   data, comments, interface strings, translations, commit messages. Number ranges use a hyphen.
3. Clean room: no program code of WinKickOff is copied or translated into this repository. Its data is
   carried over: the rule catalog, the translations, the presets, the time zones and keyboards, and, by the
   customer's decision of 14.10.2026, the runtime templates (PowerShell of the answer file and of the
   stand-alone scripts). Behaviour is written from the specification and from the output of the predecessor.
4. Dependencies: popular, well known, open source Rust crates that need no extra DLL next to the executable.
   The executable and its configuration stay portable: nothing is installed, nothing is written outside the
   program folder and the system temp folder.
5. No PowerShell inside the program: reading this PC uses the Windows API; the apply and the revert run as
   generated stand-alone scripts until the native elevated apply exists. PowerShell appears only in the
   generated answer file and in those scripts. The program never runs a script itself.
6. The customer's work PC is untouchable: only reading there. Rust is not installed on it and must not be:
   builds and tests run in CI or in a VM, and the program is not started on that PC by an agent.
7. Text files are UTF-8 without a byte order mark with LF line endings; `rustfmt` and `clippy -D warnings`.
8. Untrusted input (profiles, diffs, reports, answer files, MCP requests, later policy templates) is read
   with size, depth and character limits and turned into a message, never into a crash.
9. Every feature is developed in its own branch and reaches `main` through a pull request (the rule of the
   predecessor). The first day (14.10.2026) went straight to `main` while the repository was being created;
   from the branch `docs-refresh` on, work goes through pull requests, which the customer merges.
10. Every text of the interface is wrapped in `tr("English")` or `trf("English {name}", &[("name", &v)])` and
    has a Ukrainian and a Russian translation in `data/lang/<code>/ui.json`; the test
    `every_text_of_the_interface_is_translated` fails otherwise.

## 5. Commands

On a machine with Rust (CI or a VM, never the customer's PC):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p winweaver -- --check-catalog
cargo build --release --workspace
```

CI runs them on every push to `main`, every pull request and every tag. When `cargo fmt` finds differences,
the Linux job uploads `fmt.patch` with `Cargo.lock` in the artifact `ci-files`:
`gh run download <run id> -n ci-files -D <folder>`, then `git apply <folder>/fmt.patch`.

The MCP server without the window (the stdio transport is started by the client; the HTTP one needs a token
of at least 32 characters):

```bash
winweaver --mcp stdio --mode read
winweaver --mcp http --port 8765 --token <token> --mode edit --read-pc
```

A release: set `version` in the workspace `Cargo.toml`, add `docs/releases/v<version>.md`, merge, then tag
(the customer's call); the job `release` publishes the zip with the notes:

```bash
git tag -a v0.1.0-alpha.1 -m "WinWeaver 0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

Catalog data: edit `data/catalog/*.json` by hand (strict JSON, ASCII), add a new rules file to `RULE_FILES` in
`crates/winweaver-core/src/catalog/embedded.rs`; `tests/catalog.rs` fails on an unknown field, a missing
reference, a placeholder outside the allowed fields or a rule under the wrong root.

## 6. Facts that are easy to lose

- Without a local toolchain the workflow is: write, push, `gh run watch`, read the failed step with
  `gh run view <id> --log-failed`, fix. Clippy runs with `-D warnings` on the Windows job for the whole
  workspace; the Linux job only builds the core.
- Agents editing from Git Bash: a heredoc swallows doubled backslashes (`\\` becomes `\`), so Rust or Python
  text with backslashes is written with the file tools or a script file, never through `<<'EOF'`.
- The audit reads the default user profile (`DU:` paths) in HKCU of the current user, with the note "current
  user instead of the default profile". A rule whose matching values are what a clean Windows has (a value
  equal to its default, an absent policy value, a reg-remove of an absent value) is "as on a clean Windows"
  and keeps its state when read; an app absent from both package lists is evidence of its removal.
- Rules left out of This PC: the installation passes (windowsPE, the xml rules), the first sign-in and the
  sign-in screen (`HKU:\.DEFAULT`). The audit script and the native read produce the same checks in the same
  order, so a report of the script on another PC pairs with the plan (rule by rule, check by check).
- Features and capabilities are not read natively yet (they need DISM); the audit script reads them.
- The editions and their keys are the 14 of the predecessor (`resources::EDITION_KEYS`); `ASK_KEY` is
  `00000-00000-00000-00000-00000` with `WillShowUI` Always.
- The answer file is ASCII: account display names and descriptions outside ASCII go to Post-OOBE.ps1 as
  Base64 (`Set-AccountText`), the profile is embedded as ASCII JSON in `Extensions/Profile`.
- The language setting `""` follows the language of the Windows interface; it switches the interface and the
  catalog texts together (`i18n::current()`).
- MCP: the mode is never saved (read at every start), nor is read_pc; the token is saved in settings.json;
  errors start with not_allowed, not_found, invalid, write_failed or read_failed; passwords and the product
  key are replaced by `***` in every answer.
- Drafts: `drafts/<key>.json`, key `p-<fnv of the path>` or `n-<time>-<n>`; written 2 s after the last change
  and at the close; a save or a discard deletes them.

## 7. Work status

| Area | State | Date |
|---|---|---|
| Specification | Draft 0.1; decisions of 14.10.2026 recorded in `docs/design/README.md` and below | 14.10.2026 |
| UI design | Variant A with the compact layout and the row switches of C and the secondary tabs of D (decided) | 14.10.2026 |
| Core | Catalog, profiles, resolver, check, diff, bookmarks, build of the answer file, This PC (plan, judgement, capture, reports), stand-alone scripts, import of answer files and WinKickOff profiles; tests on the real data | 14.10.2026 |
| Windows API | Reading the registry, the elevation, the locales and the Windows language; the native read of the whole catalog is tested on the Windows runner | 14.10.2026 |
| Window | Tabs, menu, command bar (icons, compact below 1100 px), navigation, search, tree with row switches, detail panel, forms, check, build, diff tab, bookmarks, Markdown tabs, This PC (read, check, scripts, reports), MCP tab, settings, translations uk and ru, drafts, restored tabs, log | 14.10.2026 |
| MCP server | stdio and HTTP, 19 tools, 3 resources, modes, journal, token; tests of both transports | 14.10.2026 |
| Not yet | Native apply and revert (elevated helper), ADMX import and catalog files, translated documentation pages, egui_kittest tests, themes from files | |
| Acceptance | Nothing of WinWeaver has been tried in a VM yet: the build of the answer file, the read of a PC and the scripts need the acceptance the predecessor had | |

Decisions of the customer (14.10.2026): the UI is variant A with the compact layout of C for narrow windows and
its inline toggles, and the secondary tabs of D; the search field grows into the free width of the navigation
row; the extensions `.wwprofile`, `.wwdiff`, `.wwcatalog`, `.wwmarks`; the runtime scripts of the predecessor are
carried over as data (`data/templates/`); the minimum Windows version is the agent's choice guided by the
versions in use: Windows 10 and 11, the systems the Rust toolchain supports (Windows 7 and 8.1 are not); among
the Windows desktops online these are almost all, so nothing older is supported.

## 8. How to update this file

Structure changed: section 3. A command appeared: section 5. A fact that affects future changes: section 6.
A task finished or started: section 7. A decision: the specification and section 7. The date in the header on
every change.
