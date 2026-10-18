# 04. Architecture

Draft 0.1, 14.10.2026.

## 1. Workspace layout

```
crates/
├── winweaver-core/      catalog, resolver, profile, diff, validation, answer file rendering and checking,
│                        answer file import, script generation, catalog files, templates (ADMX) parsing,
│                        i18n, settings, drafts, bookmarks: pure Rust, no Windows API, fully unit tested
├── winweaver-win/       Windows API: registry read and write, services, scheduled tasks, the audit, the
│                        apply and the revert plans run natively, elevation of the helper process
├── winweaver-mcp/       the MCP server: JSON-RPC, protocol, tools, resources, stdio and HTTP transports,
│                        the workspace of documents, the journal
├── winweaver-ui/        eframe application: tabs, toolbar, navigation, search, tree, detail panel, forms,
│                        diff view, Markdown view, bookmarks, MCP tab, settings, themes, dialogs
└── winweaver/           the binary: argument parsing (window, --mcp stdio, --mcp http, --elevated-apply),
                         embedding of the data folder, logging setup
data/
├── catalog/             groups.json, rules/*.json, lang/<code>.json (converted from the predecessor's data)
├── presets/             the four presets, generated
├── lang/<code>/         ui.json and docs/*.md per language
├── themes/              light.json, dark.json and the extra themes
└── docs/                the English user documentation and the guide
tools/                   conversion of the predecessor's catalog, generation of presets and rule lists (Rust bins
                         or small scripts; they run in CI and on developer machines, never on the customer's PC)
```

`winweaver-core` compiles on Linux too, so the unit tests run on any CI runner; `winweaver-win` and the UI
are Windows only (`cfg(windows)`), and the UI tests run on the Windows runner.

## 2. Crates (dependencies policy P4)

| Need | Crate | Why |
|---|---|---|
| Window, rendering, widgets | `eframe`, `egui`, `egui_extras` (tables, images) | The chosen stack; glow (OpenGL) and wgpu with its DirectX 12 backend alone, winit |
| JSON | `serde`, `serde_json` | The standard; strict reading through a custom deserializer wrapper (duplicate keys, depth, size) |
| XML of the answer file | `quick-xml` | Writing with escaping and reading of imported files; no DTD, entities refused |
| Markdown | `pulldown-cmark` | Rendering of the documentation and the guide (a small renderer to egui on top) |
| Registry and Windows API | `windows-sys` (narrow feature set) and `winreg` | Registry, services, tasks, ShellExecuteEx for elevation, DWM title bar colours |
| Compression of catalog files | `flate2`, `xz2` or `lzma-rs` | gzip and xz; `lzma-rs` is pure Rust and preferred |
| HTTP of the MCP server | `tiny_http` | Small, synchronous, loopback only; no async runtime needed |
| Logging | `log`, `simplelog` or `fern` | Files next to the executable with rotation by date |
| Hashing of drafts and tokens | `sha2`, `rand` | Draft file names, MCP tokens |
| Time | `time` or `jiff` | Timestamps in files and logs |
| Tests of the UI | `egui_kittest` | Headless runs of the egui application with AccessKit queries |

Every crate above is pure Rust (the `windows-sys` bindings link only system DLLs). Nothing links a C
library that needs a redistributable; `cargo deny` in CI checks licences (MIT, Apache-2.0, BSD, Zlib,
Unicode) and duplicates.

## 3. Threads and state

- The UI thread owns the application state (the open documents, the catalog, the settings). Immediate
  mode: every frame reads the state and draws it; input changes the state in the same frame.
- Long work (an import, a read of this PC, a build with its check, a catalog file read) runs in a worker
  thread with a progress channel; the UI shows the progress and a Cancel flag; the result is applied on
  the UI thread when the worker finishes. A document the worker uses is marked busy for the user.
- The MCP server runs in its own threads (one per transport, a bounded pool for HTTP connections) and
  never touches the UI state directly: every call is a message to the UI thread (a channel pumped each
  frame; a request that needs an answer waits on a reply channel with a timeout) or, in the headless
  process, to the server's own workspace. Dialogs mark the window busy and the server answers
  `window_busy`.

## 4. Data flow of a profile

catalog (embedded + imports) + profile file -> resolver (dependencies, conflicts, linked) -> view model
(tree rows, states, counts) -> UI. A change: UI -> profile mutation -> resolver -> view model. Build:
profile -> validation -> renderer (passes, components, scripts) -> checker -> file. This PC: profile ->
audit plan -> native reads -> report -> capture (states and parameters) -> profile; apply plan -> backup ->
native writes -> report.

## 5. Elevation and the native apply

- Reads that need administrator rights, applies and reverts run in an elevated helper: the same executable
  started with `--elevated <plan file>` through `ShellExecuteExW` with the verb `runas`, hidden, waited
  for; the plan and the report are JSON files in `output/` with random names; the helper writes the
  backup before the first change, applies each action with the Windows API (registry, service start
  type, scheduled task through the Task Scheduler COM API or `schtasks` as the last resort, app removal
  through the deployment API, capability removal through the DISM API, Defender settings through the WMI or the policy keys), logs every step and
  reports per action. The non-elevated process shows the report.
- The helper refuses a plan file it did not get from its own folder and checks its bytes: the window
  passes the SHA-256 of the plan it wrote on the command line, the helper reads the file once and runs
  it only when its digest matches, so a plan another process changed in the output folder after the
  permission of Windows is refused. The plan keeps a random nonce, so no two plans are equal. The
  helper never runs scripts.
- The default user profile: the plan writes the `DU:` values at `HKU:\UnattendDefault`, the key of the
  stand-alone apply script; the helper enables its backup and restore privileges, loads `NTUSER.DAT` of
  the default profile (`GetDefaultUserProfileDirectoryW`) there with `RegLoadKeyW` for the steps that
  write it and unloads it after the last step. A hive already loaded at that key is used and left
  loaded. A backup names the same keys as one of the script, so either restores the other.
- The script steps of the catalog (`ps` actions) are not run: `apply::script_ops` gives each known one
  native steps that do what its PowerShell does (a program of the system folder on a folder of the
  installation media, the start type of a service that has one now, a value in every subkey of a key,
  a key deleted after the backup of every value under it, the flags of local accounts through the
  network management API, the error file of the installation and the copies of the answer file). A
  script step it does not know stays with the stand-alone script. The changes no undo takes back
  (programs, accounts, deleted files) are named in the backup and counted in the confirmation; the
  helper deletes no file but the copies of the answer file the rule names, never disables the account
  it runs as, and looks for media folders only below the roots of the drives.
- No PowerShell anywhere in this path (principle P2).

## 6. Answer file rendering

Template-free: the renderer builds the XML document from the profile with `quick-xml`, pass by pass,
component by component, and embeds the scripts assembled from the script lines of the rules into
fixed runtime sections (the functions `Set-Reg`, `Remove-Reg`, `Set-RegList`, `Invoke-Exe`, the error
wrappers, `exit 0`). The runtime sections have a version stored in the built file. The checker of the
built file runs the ported checks (length of commands, ASCII, exit codes, accounts, languages, the scheduled
task XML) before the file is written.

## 7. Build and CI

- `cargo build --release --target x86_64-pc-windows-msvc` with `-C target-feature=+crt-static`, LTO,
  `panic = "abort"` off (the UI needs unwinding to show a crash message), symbols stripped, an icon and a
  version resource embedded (`winres` or `embed-resource`), `windows_subsystem = "windows"` with a console
  re-attached for `--mcp stdio` (a second tiny console binary `winweaver-mcp.exe` if attaching proves
  unreliable, as the predecessor did).
- CI (GitHub Actions, `windows-latest`): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
  (core on Ubuntu too), `cargo deny`, the release build, a zip artifact `WinWeaver-<version>.zip` with the
  executable and the data folders; a tag `v<version>` publishes a release.
- The zip is the deliverable: unzip, run. Nothing is installed.

## 8. Testing

- Unit tests of `winweaver-core`: catalog integrity (ids, groups, roots by mechanism, placeholders only
  in allowed fields, ASCII of the catalog), resolver cases, profile round trips and migrations, strict
  JSON cases, diff and merge, validation cases, rendering against golden files, the checker against the
  golden answer file, import of the golden file, catalog file cases, ADMX parsing against fixtures.
- Tests of `winweaver-win` with a fake registry (a trait with an in-memory implementation) so that no
  test writes to the real registry; the real implementation is exercised read-only.
- UI tests with `egui_kittest`: the window opens with each preset, the tree shows the roots, a toggle
  cascades, search filters, the tabs open and close, the forms validate, keyboard paths of section 12 of
  `01-ui.md`. Headless, nothing on screen.
- MCP tests: both transports on loopback port 0, every tool, every error kind, redaction.

## 9. Known limitations to document

- The window has two renderers: OpenGL (glow) took half the memory of the full wgpu but needs a driver
  of OpenGL 2.0 or later, which Windows lacks without the driver of the graphics card (it offers OpenGL
  1.1); DirectX 12 (wgpu with that backend alone) also draws there, on WARP, slower but working.
  Automatic tries OpenGL, then DirectX 12; the settings and `--renderer` choose the first to try. An old
  driver may start OpenGL and draw wrongly: then DirectX 12 is chosen by hand.
- The first frame of egui is drawn after the catalog loads; a splash is the window itself with a progress
  bar, not a separate window.

## Open questions

1. `xz2` links liblzma (C, statically compiled, no DLL): acceptable, or `lzma-rs` only (pure Rust, slower)?
2. One executable with a console attached for stdio, or a second `winweaver-mcp.exe` as the predecessor
   ships?
