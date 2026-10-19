# AGENTS.md: map of the WinWeaver repository

For agents and developers: where things are, what to read first, which rules apply, the state of the work.
Updated with every change of structure, commands or task status. Last update: 19.10.2026 (0.17.0: the MCP tools of the permissions of registry keys; 0.16.2: the acceptance of 0.11.0 recorded, the release notes of 0.16.2; 0.16.1: the unknown options and the groups of the diff and bookmark tabs as links; 0.16.0: the permissions of registry keys; 0.15.0: the script steps of the catalog applied natively; 0.14.0: links to every rule, group, catalog and key a view names; 0.13.0: registry files, their tab and their save from This PC; 0.12.0: MCP configuration files,
the details of a call, the JSON lines log, the mockups removed; 0.11.0: the search of the registry; 0.10.0: apps removed natively through the package API; 0.9.0: capabilities removed natively through DISM; 0.8.1: This PC read in the background; 0.8.0: features and capabilities read natively through DISM; 0.7.0: the editor of a catalog; 0.6.0: the MCP tools
of the bookmarks and the catalogs; 0.5.0: the registry
tab, registry bookmarks, the MCP tools of the registry; 0.4.0: links to the
catalogs from an option and a bookmark, the same policy in the trees of other imports, the policy of a catalog
tab in the cards of a profile, the parts of the description open at first, wider editors of lists, the guide
and the README with the organization and the way of work; before it on 17.10.2026 0.3.0: templates in
UTF-16, policies of two imports in both trees, ids by namespace for a shared prefix, the comparison of
catalogs, a question before every removal; before it on 16.10.2026 version 0.2.0 with
the commit in every build, CI only for pull requests that are ready, archives of templates, the license in the
executable; two renderers,
OpenGL and DirectX 12, with Automatic, the setting and --renderer; the imports read only when checked in the
settings, the unknown options of a profile with Load catalogs; eight built-in
languages besides English, the MCP log and the crash handlers, the files for AI
agents in `agents/`, the README and the Belarusian example of the archive in `package/`, one archive from CI;
before it on 15.10.2026 the list of
the customer of 15.10.2026: the chrome of each tab, the painted tree and lists with the keyboard, the
diff tab with comments, bookmarks of options, theme files with fonts and their editor; before it on
14.10.2026 lists of values, linked policies, the default user profile applied natively, the local
accounts read natively, the colour themes, and the map rewritten after the first day).

Repository: https://github.com/digidult/winweaver (private, branch `main`). Commits are authored by
`digidult <271767060+digidult@users.noreply.github.com>` (a local `git config` of this clone, never global).
A commit made by an agent ends with the line `Co-Authored-By: <agent model> <noreply@anthropic.com>`.

## 1. The project in three sentences

WinWeaver is the successor of WinKickOff: a portable editor of Windows 11 installation profiles that builds
`autounattend.xml`, compares profiles, reads the settings of this PC natively, generates stand-alone scripts
that apply and revert them, and serves an MCP server; importing ADMX policy templates and applying natively
come next. It is written in Rust with eframe (egui + glow and wgpu + winit) for Windows 10 and 11, one executable with
data folders next to it. The data of the predecessor's rule catalog, its runtime templates and its whole
functionality carry over; the file formats are new and no program code is ported (clean room).

## 2. What to read first

| Task | Start with |
|---|---|
| Understand the scope and the principles | `docs/spec/00-scope.md` |
| Design or change the interface | `docs/spec/01-ui.md`, then `crates/winweaver-ui/src/style.rs` (the palette, the painted widgets) |
| File types, formats, what lives next to the executable | `docs/spec/02-documents.md` |
| What the program must do (feature inventory) | `docs/spec/03-features.md` |
| Crates, modules, threads, build | `docs/spec/04-architecture.md` |
| Use or change the MCP server | `crates/winweaver-mcp/src/lib.rs`, `tools.rs`, then `agents/skills/winweaver/SKILL.md` |
| Change what an AI agent or the archive gets | `agents/README.md`, `package/README.md` |

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
│   │                      import (an answer file or a WinKickOff profile as a profile), apply (the plan of a
│   │                      native apply, return or undo, the backup format of the scripts), admx (ADMX and ADML to
│   │                      rules, catalog files .wwcatalog, their check, merge), registry (keys and values in the types
│   │                      of regedit, paths of regedit, the rules that write a key, a registry in memory), regfile (`.reg`
│   │                      files: every line kept, the registry steps of a plan as a file), security (the
│   │                      owner and the access list of a key in SDDL); tests/ on the real data
│   ├── winweaver-win/     the Windows API: native.rs (thispc::Machine: the registry, the elevation, the locale
│   │                      names, the language of Windows, the local accounts and their groups), dism.rs
│   │                      (the features and capabilities through dismapi.dll), apps.rs (the removal of
│   │                      apps through the package API of Windows), execute.rs
│   │                      (the executor of an apply plan with its backup, the start of the executable as
│   │                      administrator)
│   ├── winweaver-mcp/     the MCP server: lib.rs (protocol, modes, workspace, journal, token), config.rs (the
│   │                      .wwmcp configuration files), log.rs (the JSON lines log, the crash handlers), tools.rs (the
│   │                      36 tools and 3 resources), stdio.rs, http.rs (127.0.0.1, bearer token, Host and
│   │                      Origin checks); tests/server.rs (both transports on a free loopback port)
│   ├── winweaver-ui/      the window on eframe (variant A): lib.rs (tabs, menu, command bar, navigation,
│   │                      search, shortcuts, file dialogs, This PC, build, session, the actions applied after
│   │                      each frame), views.rs (tree, detail panel), forms.rs (Installation, Accounts,
│   │                      Languages), diffview.rs, mdview.rs (Markdown tabs, the guide), marksview.rs,
│   │                      mcpview.rs (the MCP tab), session.rs (drafts, the log), settings.rs (settings.json),
│   │                      i18n.rs (tr and trf, the translations of the interface), themes.rs (theme files:
│   │                      colours and fonts), themeview.rs (the editor of a theme as a tab),
│   │                      catalogview.rs (the tab of an import or of a catalog file: its tree without check
│   │                      boxes, a policy), chrome.rs (the tab row, the command bar of the active tab, the
│   │                      navigation row with the search of the tab, the message bar, the status bar),
│   │                      style.rs (the palette of light and dark, Segoe UI and Consolas, the painted buttons,
│   │                      cards and chips), tree.rs (the painted tree of a profile and of a catalog tab: one
│   │                      stop of Tab, the keys inside), list.rs (the painted list with columns of the
│   │                      diff, the bookmarks and the MCP journal: widths by drag, the keys, a mini button),
│   │                      renderer.rs (the renderers built in, the order Automatic tries, the start of a window),
│   │                      regview.rs (the registry tab: the tree of keys, the values, the editor of a value),
│   │                      cataledit.rs (the editor of a catalog file: its tree, the form of a policy),
│   │                      regfileview.rs (the tab of a `.reg` file: the outline, the values of a key)
│   └── winweaver/         the executable: the window, --renderer, --mcp stdio, --mcp http, --elevated-apply
│                          <plan> <digest> (the helper: only a plan-*.json of output/ with that SHA-256),
│                          --version, --check-catalog, --smoke-window (CI)
├── data/                  embedded into the executable at build time
│   ├── reference/         the parameter reference of the predecessor (00-20 and README), carried over with the
│   │                      catalog data; the `doc` field of a rule links into it with a heading anchor
│   ├── catalog/           groups.json, 00-17 rules files, lang/<code>.json for uk, ru, de, es, fr, id, it, tr (catalog 0.11 of the
│   │                      predecessor)
│   ├── presets/           office, strict, laptop, home .wwprofile (converted once from the predecessor)
│   ├── resources/         timezones.json, keyboards.json
│   ├── templates/         the runtime templates of the predecessor carried over as data: the answer file
│   │                      skeleton and the scripts with {{slot}}s (they name WinWeaver now), VERSION 0.11
│   ├── lang/<code>/       ui.json: the translations of the interface (uk, ru, de, es, fr, id, it, tr); guide.md: the guide translated
│   ├── themes/            light, dark, latte, matrix .wwtheme: the embedded themes (a base, the colours it
│   │                      changes, the fonts); Latte and Matrix are the colours of the predecessor
│   └── docs/              guide.md, the English guide shown at the first start
├── agents/                shipped in the archive: AGENTS.md (the assistant for any agent, short for small
│                          contexts), README.md, skills/ (winweaver: the tools, served as winweaver://skill;
│                          winweaver-review, -change, -this-pc), pi-agent/ (Dockerfile of the latest pi and
│                          Node.js LTS, its AGENTS.md, README.md)
├── package/               what the archive adds: README.md for the user (not this repository's), lang/be/ui.json
│                          (Belarusian, every text: the example of a language, checked by a test of i18n.rs)
└── docs/
    ├── releases/          release notes v<version>.md
    └── spec/              clean room specification (00 scope, 01 UI, 02 documents, 03 features, 04 architecture)
```

Next to the executable at run time (never versioned): `settings.json`, `bookmarks.wwmarks`, `profiles/`,
`output/` (the answer file, the scripts, plan and backup files of the native apply), `imports/` (catalog files of
the imported policy templates), `drafts/` (unsaved profiles), `mcp/` (MCP configurations, `default.wwmcp` of the
MCP tab), `logs/` (winweaver-<date>.log, mcp-<date>.jsonl),
`lang/<code>/` (more or overriding translations), `themes/` (more themes, `<id>.wwtheme`, also `<id>.json`
of the predecessor; the id of an embedded theme replaces it).

## 4. Project rules

1. Language: everything in the repository is English (code, comments, documentation, commit messages, issue
   texts). The interface, the catalog and the user documentation are English sources with translation
   files built in for Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish; other languages are files added next to the executable.
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
    has a translation in `data/lang/<code>/ui.json` of every built-in language (uk, ru, de, es, fr, id, it,
    tr); the test `every_text_of_the_interface_is_translated` fails otherwise. A new rule needs its texts in
    every `data/catalog/lang/<code>.json` (`translations_cover_every_rule`); `package/lang/be/ui.json` may
    lag (English fills the gaps).
11. Versions (the customer's rule of 16.10.2026): `version` of the workspace `Cargo.toml` grows with every
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
```

CI does not run at every push, since its Windows minutes are the dear ones:

1. Open a pull request as a draft (`gh pr create --draft`) and push to it as often as needed: a draft runs
   nothing.
2. When the work is complete, `gh pr ready <number>` (or "Ready for review" on GitHub) runs CI once. Every
   later push to a ready pull request runs it again, so gather the fixes into one push; `gh pr ready
   <number> --undo` makes it a draft again.
3. A check on demand, of a draft or any branch: `gh workflow run ci.yml --ref <branch>` (or Actions, CI,
   Run workflow); `gh run list --branch <branch> --workflow ci.yml` finds the run.
4. A head commit with `[skip ci]` in its message runs nothing; a newer push cancels the run of the same pull
   request or branch.
5. The Linux job comes first and the Windows job only after it: an unformatted push, or clippy or a test of
   the core that fails, stops before Windows.
6. `main` runs after every merge (its archive is the build for the acceptance), a tag `v*` for the release.
7. The mirror (the rule of 18.10.2026): when the Actions minutes of digidult/winweaver are used up, its run
   fails in a few seconds with the annotation "The job was not started because recent account payments have
   failed or your spending limit needs to be increased". Only then the same commit runs in the private
   mirror https://github.com/stanislavperec-ua/winweaver (the remote `mirror` of the clone): `git push mirror
   <branch>`, then `gh workflow run ci.yml --ref <branch> -R stanislavperec-ua/winweaver`; after a merge
   `main` is pushed there too (its run gives the archive for the acceptance); tags never go there. The pull
   requests and the merges stay in digidult/winweaver, and a pull request links the run of the mirror. CI
   goes back to digidult/winweaver as soon as its runs start again (the minutes reset on the first day of a
   month, or the customer sets a budget) and moves to the mirror again the next time they run out; a ready
   pull request always tries digidult/winweaver first, so the switch back needs nothing.

When `cargo fmt` finds differences, the Linux job fails and uploads `fmt.patch` with `Cargo.lock` in the
artifact `ci-files`: `gh run download <run id> -n ci-files -D <folder>`, then `git apply <folder>/fmt.patch`.

The MCP server without the window (the stdio transport is started by the client; the HTTP one needs a token
of at least 32 characters):

```bash
winweaver --mcp stdio --mode read
winweaver --mcp http --port 8765 --token <token> --mode edit --read-pc
```

A release: set `version` in the workspace `Cargo.toml`, add `docs/releases/v<version>.md`, merge, then tag
(the customer's call); the job `release` publishes the zip with the notes:

```bash
git tag -a v0.3.0 -m "WinWeaver 0.3.0"
git push origin v0.3.0
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
- Input languages are read as Windows lists them: it shortens a tag whose region is the main one of its
  language (`uk` for `uk-UA`), so `resources::keyboard_of_read` maps a tag without a region to the
  keyboards of its language and picks the one whose layout is among the input method tips (the values
  named like `0422:00020422` of `User Profile\<tag>`, the `InputMethodTips` of the audit script).
- Of the actions of one rule on one value (a removal for a clean start, then the value) only the last is
  checked (`thispc::plan`). A rule partly in effect is read as off, and the read names what differs
  (`Reading::differing`, `CaptureSummary::partly`, a warning on the rule in the message bar).
- The local accounts are read natively the way the audit script reads them: `NetUserEnum` (level 20, normal
  accounts) without the disabled ones and the relative ids 500, 501, 503 and 504; the group is Administrators
  or Users by direct membership (`NetUserGetLocalGroups`) in the groups S-1-5-32-544 and S-1-5-32-545, whose
  names `LookupAccountSidW` gives in the language of Windows. No password is read; the capture keeps the
  accounts of the profile when none of Administrators or Users is found.
- The editions and their keys are the 14 of the predecessor (`resources::EDITION_KEYS`); `ASK_KEY` is
  `00000-00000-00000-00000-00000` with `WillShowUI` Always.
- The answer file is ASCII: account display names and descriptions outside ASCII go to Post-OOBE.ps1 as
  Base64 (`Set-AccountText`), the profile is embedded as ASCII JSON in `Extensions/Profile`.
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
  MCP tab at every change through `App::persist_mcp`; `settings.json` keeps no MCP field any more, an old
  `mcp_token` is read once into `mcp/default.wwmcp`): the transport for a start without the window, port, token,
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
  on skips imported policies; the read of the whole PC leaves them out.
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
  counts with its policy); Compare in the bar of a catalog tab opens it as a Markdown page
  (`comparison_page`, at most 1000 lines a section).
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
- Registry files (`winweaver_core::regfile`, `Doc::RegFile`, `regfileview.rs`): `RegFile::parse` keeps every
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
- Links (the list of 18.10.2026, item 4): a rule named in a view leads where `views::RuleLink` says
  (`Profile`: `Act::Reveal`, select and open the tree down to it; `Catalog(file)`: `Act::CatalogReveal`
  in the tree of the catalog tab for a policy of its file; `Elsewhere`: `Act::ShowInProfile` in the first
  profile tab, which now scrolls to it). `rule_description` takes the `RuleLink` of its tab and lists the
  rules a rule needs, the ones that need it and its conflicts both ways. `views::linked_text` turns the
  ids of rules and the registry paths of a text into links (the message bar); the crumbs of a catalog tab
  select every group above, the crumb of a rule of a profile keeps the group it is shown in. The registry
  tab names the rules that write the selected key or value (`regview::Writers`, kept until the selection
  or the count of rules changes), the catalog editor opens a key in the registry tab and its policy in the
  catalog tab and the profile, the places of an MCP call show the titles of rules. The warning of the
  unknown options has its own target (`validate::Target::Unknown`) and opens their group; the group of a
  rule in the diff and bookmark tabs shows it in the first profile tab (`Act::ShowGroupInProfile`).
- Every removal asks first (`App::ask`, Escape says no): an import, a bookmark of the bookmarks tab (the star
  of the navigation row stays a toggle), closing a tab discarding its changes, forgetting the recent files,
  renewing the MCP token.
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
  OpenGL through glow took half the memory of the full wgpu (the customer, 16.10.2026); DirectX 12 is wgpu with its `dx12` backend alone (no
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
- Screenshots: the Windows job sets `WINWEAVER_SCREENSHOTS`, and the test `screenshots_of_the_window` draws the
  main tabs with wgpu (no screen) into `target/screenshots`, uploaded as the artifact `screenshots`
  (`gh run download <id> -n screenshots -D <folder>`); the way to see the window without starting it on the
  customer's PC. A renderer that cannot start writes `error.txt` there instead of failing the tests.
- Window tests: `App::new` and `App::apply` test the logic without frames; `egui_kittest` (a dev-dependency
  with its eframe feature) draws whole frames without a screen and clicks by the labels of AccessKit, so no
  window ever opens. A test that needs a label finds it with `query_all_by_label`, since a text can appear
  twice (a tab and a breadcrumb).
- The chrome follows the active tab (customer list of 15.10.2026, items 6, 9 to 11): the command bar holds the
  commands of that tab (icons above labels; none for Settings; icons alone at the end of the
  navigation row below 1100 px), the navigation row appears only for a profile and a catalog tab (the
  history, the breadcrumb, the star); the other tabs keep their search at the right end of the command
  bar. A narrow row shortens the breadcrumb to its first and last parts with a menu of the others. Every tab but Settings and a theme has its own search (`active_search`): the profile tree, the policies
  of a catalog tab, the differences, the blocks of a Markdown page, the bookmarks, the MCP journal. Clearing
  a search opens the groups above the selection and scrolls it into view (`Act::SearchCleared`,
  `reveal`). The message bar folds to one line or opens with a height the user drags.
- The look comes from `style.rs`: the palette of light and dark of variant A (the HTML mockups it came from
  were removed on 18.10.2026, when the design had settled; a
  theme overrides its tokens, `style::COLOR_NAMES`, also with the names of the first version) and the
  fonts of the theme. `install_fonts` (before the first frame) names the families `light`, `light-strong`,
  `light-mono` and the same for `dark`; `style::apply` loads the files of the families of the two themes
  into them (`style::FAMILIES`, `MONO_FAMILIES` in the fonts folder of Windows, egui's fonts as the
  fallback) and scales the text styles by the size of the theme. Painted widgets take their fonts and
  sizes from `style::font`, `strong_font`, `mono`, `size` and `row_height`, never `FontId::proportional`;
  without `install_fonts` (tests) the families of egui are used.
- Themes (`themes.rs`, item 4 of the list): a `.wwtheme` file is a name, a base, colours and fonts; the
  window keeps the list in `App::themes` (read at the start, when the settings open and after a save) and
  applies `wanted_theme` whenever it changes: the theme of an editor tab in front (a live preview), else
  the one of the settings, else Light and Dark as Windows shows its apps. A change of a font in the
  settings saves the file of the shown theme (`Theme::file_in`: an embedded theme gets
  `themes/<id>.wwtheme`).
- Lists (`list::show`, item 5): one stop of Tab like the tree; the arrows, Home, End and the page keys move
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
- Keyboard (items 2 and 5 of the list): a tree is one stop of Tab (`tree::show` makes a focusable area,
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
- Pull requests: the CI of a pull request is read with the app's PR tools (`get_status`), not by polling with gh.
- The archive: the job Package copies the exe, LICENSE, `package/README.md`, the presets as `profiles/`,
  `package/lang/` and `agents/` into `dist/WinWeaver` and uploads `dist/`, so the artifact is the one archive; a
  tag zips the same folder for the release. Agent texts stay short (local models with small contexts) and name no
  model: the person chooses it.
- Drafts: `drafts/<key>.json`, key `p-<fnv of the path>` or `n-<time>-<n>`; written 2 s after the last change
  and at the close; a save or a discard deletes them.

## 7. Work status

| Area | State | Date |
|---|---|---|
| Specification | Draft 0.1; decisions of 14.10.2026 recorded in `docs/spec/01-ui.md` and below | 14.10.2026 |
| UI design | Variant A with the compact layout and the row switches of C and the secondary tabs of D (decided) | 14.10.2026 |
| Core | Catalog, profiles, resolver, check, diff, bookmarks, build of the answer file, This PC (plan, judgement, capture, reports), stand-alone scripts, import of answer files and WinKickOff profiles; tests on the real data | 14.10.2026 |
| Windows API | Reading the registry, the elevation, the locales, the Windows language and the local accounts with their groups; the native read of the whole catalog is tested on the Windows runner | 14.10.2026 |
| Window | Tabs, menu, command bar of each tab (icons above labels), navigation, search per tab, the painted tree and lists with the keyboard, detail panel with copy and selectable texts, forms, check, build, diff tab with comments, bookmarks of options, Markdown tabs with anchors, This PC (read, check, scripts, reports), MCP tab with its journal, settings, translations uk, ru, de, es, fr, id, it and tr, drafts, restored tabs, log, themes with fonts (Light, Dark, Latte, Matrix, more as files) and their editor, the title bar in their colours; pull requests #13 to #17 for the list of 15.10.2026 | 15.10.2026 |
| MCP server | stdio and HTTP, 36 tools, 3 resources, modes, journal, token; tests of both transports | 14.10.2026 |
| Native apply | Plans, the elevated helper, the backup and undo (pull request #2), the default user profile loaded and unloaded by the helper; tested in a test key of HKCU and of the default profile on the runner | 14.10.2026 |
| Policy templates | ADMX import, catalog files, the settings section (pull request #3), lists and multi-line texts (`reg-list`, list parameters), policies linked to built-in rules; only the checked imports read, the unknown options of a profile in their group with Load catalogs | 16.10.2026 |
| This PC | Features and capabilities read natively through DISM as administrator (0.8.0) | 18.10.2026 |
| Native apply | Capabilities removed through DISM (0.9.0) and apps through the package API (0.10.0), named in the backup, not restored by an undo | 18.10.2026 |
| Native apply | The script steps of the catalog applied natively (0.15.0): media, services, keys, accounts, the copies of the answer file | 18.10.2026 |
| Registry | The permissions of keys: owner, access list, own entries changed after a question (0.16.0) | 18.10.2026 |
| Not yet | Nothing of the plan; what comes next comes from the use | |
| Acceptance | Build 0.11.0 accepted by the customer in a VM on 18.10.2026: all 31 steps of the scenario passed, the four stand-alone scripts (apply, undo, return to defaults, audit) checked for the first time, with the native apply and undo, This PC, the registry search and the catalog editor; earlier the answer file and a `.wwdiff` of a real PC. The steps of 0.16.1 (parts G, E and F of the same page) wait | 19.10.2026 |

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
