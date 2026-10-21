# Refactoring plan (October 2026)

The customer read the code on 21.10.2026 and found it hard to navigate and ugly: files of thousands of
lines, functions of hundreds, a document kind turned into a number and compared with `if kind == 0`,
the same helper written seven times. This plan names what changes, in which order and how each step is
checked, so that the work costs few CI minutes and no behaviour. The survey behind it (file and
function sizes, the repeated code, the inconsistencies) was made on 21.10.2026 on 0.22.1.

The plan has two parts. Part 1 (this document) makes the code navigable inside the present crates.
Part 2, the framework W# (`ww` in code, `WW` in documents) that carries the window with its tabs out of
WinWeaver, with the editor `wedit` as its second user, gets its own document once part 1 is merged; the
steps of part 1 already draw the line it will cut along.

## 1. Goals, measurable

- No source file over 1000 lines, and none over 800 without its tests; no function over 120 lines.
- Tests of a crate live in `tests/` or in `src/<module>/tests.rs`, never at the end of a source file.
- No number or string stands for a kind that an enum can name (document kinds, settings sections, tree
  keys, result kinds, REG types listed once).
- One implementation of each helper: the Windows API wrappers, the value conversions, the parse and
  save of the four JSON documents, the search filter, the selection scroll, the modal window, the
  side-and-central layout.
- The same behaviour: every existing test passes unchanged (moved, not edited), the answer file and
  the scripts of the presets are byte for byte the same, the MCP tools answer the same. The two places
  where the window and the MCP server disagree are listed in section 5 and settled on purpose.

## 2. Way of work

- One branch, `refactor`, one pull request, version 0.23.0 (a change of structure, no feature).
- The Linux job checks the whole workspace for Windows (clippy with `--all-targets` on the target
  `x86_64-pc-windows-gnu`, about two minutes, one Linux minute each): `gh workflow run ci.yml --ref
  refactor -f windows=false -R stanislavperec-ua/winweaver` after every group of steps. The Windows job
  (tests, the build, fifteen minutes at double cost) runs three times: after the core and Windows
  steps, after the window steps, and once more when the pull request is ready.
- A step is one commit that moves code or renames it without changing what it does; a step that
  changes behaviour is named as such in its commit and in section 5. Steps are grouped so that one CI
  run checks three to six commits; a compile error is read from the log of the failed step
  (`gh run view <id> --log-failed`) and fixed in the next group.
- Order: leaves first. The core has no dependency inside the workspace, then the Windows crate, the
  MCP server, the window, the executable. A step never touches two crates when it can touch one.
- Formatting as rustfmt wants it before the push (max width 100, the known break points), since an
  unformatted push stops the Linux job before clippy.
- Every text of the interface stays inside `tr` and `trf`; the translation files do not change in
  part 1 unless a text disappears.

## 3. Steps of the core (`winweaver-core`)

### A1. `admx.rs` (2589 lines) becomes the folder `admx/`

| File | Holds | Lines today |
|---|---|---|
| `mod.rs` | the constants, the re-exports, `CatalogHeader`, `rule_ids`, `catalog_path` | 1-47, 1629-1640, 1782-1858 |
| `xml.rs` | `Node`, `parse_xml`, `element` | 49-163 |
| `templates.rs` | `Texts`, `Val`, `Item`, `Element`, `Category`, `Policy`, `Templates`, the ADML `Resources`, the ADMX parse (`value_of`, `items_of`, `reference`, `elements_of`, `read_pair`), `read_folder`, `read_file` | 165-661 |
| `rules.rs` | the conversion to rules: `LangTexts`, `slug`, `en`, `cut`, `reg_of`, `policy_rules`, `to_catalog_file`, `safe_text`, `safe_key` | 663-1307 |
| `file.rs` | `CatalogFile` with `load`, `save`, `new_policy`, `put_rule`, `remove_rule`, `new_group`, `check` | 1309-1538 |
| `compare.rs` | `PolicyDifference`, `Comparison`, `compare` | 1540-1627 |
| `archive.rs` | `Archive`, `read_archive`, `entry_text` | 1642-1780 |
| `merge.rs` | `merge`, `merge_texts` | 1860-1944 |
| `tests.rs` | the eleven tests | 1946-2589 |

Inside: `policy_rules` (327 lines) splits by the kind of element (the enabled and disabled rules, one
function per `Element` kind); the `Rule` literal written twice gets a constructor; the "unique id with
a `-n` suffix" loop written three times becomes `unique_id`; the check `safe_text(x, 16383) &&
!x.contains('{')` written seven times becomes `safe_value`. `safe_text` and `safe_key` become
`pub(crate)`.

### A2. `render.rs` (1395 lines) becomes the folder `render/`

| File | Holds |
|---|---|
| `mod.rs` | `RenderError`, `Build`, the templates, `runtime_version`, the re-exports |
| `text.rs` | `ps_quote`, `xml_escape`, `ascii_json`, `ascii_text`, `comment_text`, `base64` |
| `params.rs` | `value_text`, `substitute`, `list_items`, `list_values`, `list_entries`, `fill_value`, `fill`, `params_of` |
| `ps.rs` | `reg_path`, `reg_value`, `script_lines`, `reg_list_line`, `block`, `join_blocks`, `ps_list` |
| `xml.rs` | `Setting`, `setting_lines`, `Command`, `command_lines`, `component` |
| `answer.rs` | `build`, one function per pass: `collect` (the rules and their actions), `scripts`, `windows_pe`, `specialize`, `oobe_system`, `extras`, `settings_xml`, `extensions`, `header` |
| `name.rs` | `computer_name_parts`, `EXAMPLE_SERIAL`, `EXAMPLE_MAC`, `example_name` |
| `accounts.rs` | `account_xml`, `account_text_lines`, `input_languages` |

`build` (513 lines) keeps its signature; its body becomes the calls of the pass functions, each taking
the collected rules and returning its lines. The duplicate of `params_of` inside `build` and the copy
of `ps_list` inside `reg_list_line` go. The cycle `validate` to `render` (`is_layout`) and `render` to
`validate` (`list_items`, `list_entries`) is cut by A5.

### A3. `thispc.rs` (1316 lines) becomes the folder `thispc/`

`mod.rs` (the types: `Probe`, `Check`, `Plan`, `Machine`, `RegData`, the results), `plan.rs`
(`wildcard`, `audit_path`, `checks_of`, `plan`), `report.rs` (the audit report JSON), `evaluate.rs`
(`evaluate` split by the kind of check, `run`), `judge.rs`, `capture.rs` (`capture_forms` and the
rest of the capture), `memory.rs` (`MemoryMachine`). `MemoryRegistry` of `registry.rs` moves to
`registry/memory.rs` the same way; both doubles stay compiled, since the MCP and window tests use them
across crates.

### A4. One trait for the four JSON documents

`profile.rs`, `diff.rs`, `bookmarks.rs` and `users.rs` repeat the error enum (`Json`, `Io`, `Format`,
`Version`) and the bodies of `parse` (the format and version check), `to_json` and `save` (a temporary
file, then a rename). `jsonfile` gets `DocError` and the trait `Document` with `FORMAT`, `VERSION`,
`EXTENSION` and the default methods `parse_checked`, `load`, `save`; each document implements the
trait and drops its copy. `CatalogFile::save` (`admx/file.rs`) uses the same save. The four error
enums become type aliases of `DocError` so that no caller changes.

### A5. One module for the values of actions

`render::value_text`, `regfile::text`, `thispc::text_of` and `execute::text` turn a JSON value into a
text four ways; the number has three (one defaults to 0, one errs, one is a closure); the items of a
MultiString two. `catalog/values.rs` holds `text`, `number`, `items`, `entries` once; `render::params`
and `validate` call it, which cuts the module cycle of A2. `regfile::data_of` and `execute::encode`
share it in A7.

### A6. `validate.rs`

`check_rules` (152 lines, a `match` over `Param` with ten alike `push` calls) gets one `problem`
helper per parameter kind; `check_accounts` (129 lines) splits into the checks of the name, the
password and the texts. The `is_*` validators stay.

### A7. The Windows crate (`winweaver-win`)

`ffi.rs` holds once what seven files write: `wide` (seven copies), `wide_text` and `take_text` and
`buf_text` (three copies each), `utf16` (three), `NetBuf` (the `NetApiBufferFree` guard), `Sid`,
`enum_users(level, f)` (the `NetUserEnum` paging loop, three copies of 35 lines), `lookup_sid`,
`set_dacl(kind, path, sddl)` (two copies of 45 lines: a registry key and a folder), `net_error`,
`win32_error`, `hresult_error` (the three ways a status is printed today). `execute::execute` (208
lines, one match over `Op`) becomes `fn step(op) -> Step` with one function per operation; the
`BackupEntry` literal written twice uses `named`; the two service arms share one function. `users.rs`
and `accounts.rs` and `native.rs` read the accounts through `enum_users`.

### A8. To assess after A5, not promised

`registry::Data` (the types of regedit) and `thispc::RegData` are two value enums with two decoders
(`Data::from_raw`, `native::convert`). If `RegData` is a subset of `Data`, it goes; if not, the two
decoders share `ffi`.

## 4. Steps of the MCP server (`winweaver-mcp`)

### B1. A table of tools

`tools.rs` (1774 lines) names every tool twice: in `definitions` (303 lines of `vec![tool(..)]`) and
in `call` (820 lines, a `match name`). The table `TOOLS: &[Tool]` with `name`, `description`,
`schema`, `mode` (the mode a tool needs, the registry and library gates) and `handler: fn(&mut State,
&Args) -> Result<Value, ToolError>` names each tool once; `definitions` and `call` read the table.
The handlers go into `tools/{profiles, rules, pc, registry, bookmarks, catalogs}.rs`, each under 300
lines. The resources list stays data.

### B2. Arguments and errors

`Args` wraps the JSON object with `text`, `optional_text`, `bool`, `number`, `list`, all through one
null-filtering `get`, so that the five direct `args.get(..)` sites treat null like the rest.
`ToolError.kind` becomes an enum (`Invalid`, `NotFound`, `NotAllowed`, `ReadFailed`, `WriteFailed`)
that prints the same words. `resolver::resolve` recomputed in nine arms becomes `State::resolution`.

### B3. The edits of a profile in one place

`Act::SetParam` of the window and `set_param` of the server, `Act::SetGroup` and `set_group`, repeat
the same logic in two crates; `profile::edit` (the core) gets `set_param`, `set_group`,
`set_install`, `set_accounts`, `set_languages`, and both crates call it. See section 5 for the one
difference.

## 5. Behaviour settled on purpose

- Turning a group on skips the imported policies in the window (`Act::SetGroup`, as AGENTS.md says)
  but not in the MCP tool `set_group`. The window is right; the tool follows it (B3). Release notes.
- `null` as an argument of an MCP tool is "absent" in the helpers and "present" at the five direct
  reads (B2). It becomes "absent" everywhere. Release notes.
- The `RegFile`, `Users`, `Mcp` and `Registry` tabs reset their selection scroll inside the view, the
  other tabs through `Act::Revealed` and `Act::Follow`. One mechanism remains, the one of the view
  (C5). No visible change.

## 6. Steps of the window (`winweaver-ui`)

The window is the part W# will take; these steps cut along its future line: what knows nothing of
profiles, catalogs and the registry (the tab row, the command bar, the tree and list widgets, the
styles and themes, the translations, the drafts, the settings, the renderers, the Markdown viewer,
the modal windows) is written so that it can move to a crate of its own without a change.

### C1. `lib.rs` (7164 lines) becomes the folder `app/`

| File | Holds |
|---|---|
| `lib.rs` | the modules, the re-exports, `run`, `smoke_window` |
| `app/mod.rs` | `App` with `new` and the small accessors |
| `app/docs.rs` | `Doc`, the trait `Tab`, `ProfileDoc`, `Sel`, `Search`, the typed `active::<T>` accessors |
| `app/acts.rs` | `Act` and the enums it holds |
| `app/frame.rs` | the `eframe::App` impl: the frame, the panels per document, the polls |
| `app/apply.rs` | `apply`, which hands each group of acts to its handler |
| `app/files.rs` | open and save of every document, the dialogs, the recent files, `file_stem`, the folders next to the executable |
| `app/profile.rs` | the profile acts: select, reveal, toggle, params, forms, check, build |
| `app/pc.rs` | This PC: the read in its thread, the checks, the plans, the elevated run, the scripts and registry files |
| `app/imports.rs` | the policy templates: load, import, rename, export, remove, the settings section, `ImportEntry` |
| `app/registry.rs` | the registry tab acts, the rights, the search thread |
| `app/regfile.rs`, `app/users.rs`, `app/diff.rs`, `app/marks.rs`, `app/mcp.rs`, `app/themes.rs` | one file per document kind |
| `app/session.rs` | the drafts, the remembered tabs, the restore, the log |
| `app/settings.rs` | the settings tab (`settings_view` and its sections) |
| `app/tests/*.rs` | the 26 tests, by theme |

Nothing changes in the bodies at this step; it is a move. The free functions at the end of `lib.rs`
go to the file of their theme (`comparison_page` to `app/diff.rs`, the dialog helpers to
`app/files.rs`).

### C2. Documents of one shape

`Doc::Bookmarks`, `Doc::Mcp` and `Doc::Settings` carry no struct, so their state sits on `App`
(`marks_tab`, `marks_search`, `mcp`, `mcp_search`, `settings_section`, `renaming`). They get
`MarksDoc`, `McpDoc` and `SettingsDoc`, and every variant is `Doc::Kind(Box<..Doc>)`. The trait `Tab`
(`title`, `dirty`, `path`, `search`, `icon`, `search_hint`, `kind`) replaces the seven exhaustive
matches of `title`, `active_search`, `remember_tabs`, `doc_icon`, `search_hint` and the six-way
`dirty`; `settings_section` becomes `enum SettingsSection`. The frame function matches the variant
and calls `fn view(&mut self, ui, ctx: &ViewCtx, acts)` of the document: the numbers 0 to 12 go.

### C3. Acts by subsystem

`Act` has about 138 variants and `apply` 709 lines. The variants group into enums by subsystem
(`Act::Profile(ProfileAct)`, `Pc`, `Registry`, `RegFile`, `Users`, `Diff`, `Marks`, `Mcp`,
`Catalog`, `Imports`, `Theme`, `Session`, `Nav`), and `apply` becomes a match of thirteen arms that
call `apply_profile`, `apply_pc` and so on in the files of C1. The twelve `Ask*` pairs become
`Act::Ask(Question)` where `Question` holds the title, the text, the label of yes and the act a yes
runs; `App::ask` is the one entry.

### C4. Commands as data

`chrome::toolbar` (292 lines) writes 59 buttons inline, the menu another list, and `shortcuts` a
third; "Ctrl+S" appears fourteen times. `commands.rs` holds `Command { id, icon, label, keys, act,
enabled }` and a function `commands_of(doc) -> Vec<Command>`; the command bar, the menu of a tab and
the keyboard read it, and an icon is a named constant in `icons.rs` (29 distinct icons, named once).
The layout numbers of the tab row and the navigation row become constants with names.

### C5. Shared widgets, one copy each

`widgets.rs` (future W#): `modal(ctx, title, |ui| ..)` for the eight OK-Cancel-Escape windows,
`side_panel(ui, id, width, |ui| ..)` for the five identical left panels and `list_split` for the two
list tabs, `scroll_for(reveal, follow)` for the eight copies of the reveal-or-follow block,
`filter_words(hay, words)` for the nine search filters, `table(ui, rows, sel, ..)` for the five list
boilerplates, `status::saved`, `opened`, `failed` for the hundred status texts of the same shape,
`open_or_focus(docs, pred, make)` for the nineteen position-or-push sites, `with_directory(dialog,
path)` for the nine dialog idioms.

### C6. Near duplicates of the views

The REG value editor of `regview` and `regfileview` (the type list, the hints, the multi-line
switch) becomes `regedit.rs` with `value_fields`; the two password windows of `usersview` become one
`password_pair`; the two "rules that write this key" (`Writers` of `regview`, `writers_of` of
`regfileview`) become one; the catalog rows of `catalogview` and `cataledit` share `catalog_rows`
with one key scheme; the six `save_*` and four `open_*_path` functions of `lib.rs` become
`save_doc` and `open_doc_path` driven by the trait of C2; the five `title` fallbacks become one
default of the trait.

### C7. Names instead of numbers and strings

The tree keys `g:`/`r:` and the diff keys `rule:` become enums with `Display`; the REG type lists of
`regfileview::TYPES` and `cataledit::KINDS` read `registry::NEW_TYPES`; `MARKDOWN_MAX_BYTES` is used
where `4 * 1024 * 1024` is written; the display caps, the draft delay, the window sizes and the
journal cuts become named constants; `e.result == "ok"` of the journal reads an enum.

## 7. Documentation (D)

AGENTS.md sections 3 and 6 follow the new files; `docs/spec/04-architecture.md` section 1 names the
folders; `docs/releases/v0.23.0.md` lists the two settled behaviours of section 5; this document
gets a closing section with what was done and what was left.

## 8. Order and the CI runs

| Group | Steps | Check |
|---|---|---|
| 0 | the cross check on the Linux job, `Cargo.lock` (done 21.10.2026) | Linux |
| 1 | A1, A4 | Linux |
| 2 | A2, A5, A6 | Linux |
| 3 | A3, A8 | Linux |
| 4 | A7 | Linux, then Windows (the tests of DISM, the registry and the accounts run on the runner) |
| 5 | B1, B2, B3 | Linux |
| 6 | C1 | Linux |
| 7 | C2, C3 | Linux |
| 8 | C4, C5 | Linux |
| 9 | C6, C7 | Linux, then Windows (the window tests and the screenshots) |
| 10 | D, the version, the pull request ready | Windows (the archive of the acceptance) |

About twelve Linux runs of two minutes and three Windows runs of fifteen: less than one Windows run
a day of the way the work went before.

## 9. Left out on purpose

- The data files, the templates, the catalog and the translations do not change.
- The MCP protocol, the file formats and the command line do not change.
- `tree::show` (184 lines) and `list::show` (249 lines) are one painted widget each; they split only
  if the W# work asks for it.
- Performance: nothing here is measured as slow; the goal is navigation and one copy of each thing.

## 10. What was done (21.10.2026)

Part 1 is on the branch `refactor`, pull request #59, version 0.23.0: twelve Linux runs of the cross check
and the Windows run of the window tests, as section 8 planned; the acceptance build is the archive of
that run.

Done as planned: A1 to A8 (the folders `admx/`, `render/`, `thispc/`, `registry/`; the trait `Document`;
`catalog::values`; `validate` by what it checks; `ffi.rs` and `execute/`; `registry::Data` as the one
value type, `RegData` gone), B1 to B3 (the table `TOOLS`, `Args` and `Kind`, the edits of a profile in
the core), C1 to C7 (`app/` by theme; `MarksDoc`, `McpDoc`, `SettingsDoc`, the trait `Tab` and
`DocKind`, the frame by kind; the acts by subsystem and `Question`; `commands.rs`, `icons.rs`, the keys
once, the layout constants; `widgets.rs`, `Scroll::wanted`, `matches_words`, the status helpers,
`show_or_open`, `focus_tab`, `open_tab`, `in_folder`; `regedit.rs`, the password fields, one `Writers`,
one `catalog_rows` with `TreeKey`, `open_file`, the title of a tab from its path; `ItemKey`,
`RegKind::type_name`, `NEW_TYPES`, `Outcome`, the named sizes and delays), and D. Beyond the plan the
files still over a thousand lines became folders: `views/`, `regfileview/`, `chrome/`, the window tests
in `tests/` by theme, `execute/`; the free functions of the window's `lib.rs` went to `folders.rs`,
`importlist.rs` and `dialogs.rs`.

Settled behaviour, as section 5 and the release notes say: `set_group` of the server skips the imported
policies; `null` is absent for every tool; a DWORD or QWORD with a fraction is refused; the errors of
the Windows API print the text of Windows; a button of the command bar with nothing to do is disabled.

Not done, and why:

- The save of a document through the trait (C6, `save_doc`): the six `save_*` functions differ in
  their dialogs, their steps before the save (the seal of a users file, the check of a catalog, the
  timestamps and draft of a profile) and after it (the themes read again, an import reloaded). They
  become one when the trait of the document is the framework's (part 2), not twice.
- Two acts still ask by themselves (C3): `UsersAct::AskRun` checks the elevation, the passwords and the
  plan before it asks, `RegistryAct::AskDelete` refuses a hive; both report in the status bar. The other
  ten pairs are `Act::Ask(Question)`.
- `table` (C5) and `view()` per document (C2): the five list tabs already draw through `list::show`;
  the frame draws a tab by its kind in one method each, which W# will turn into the method of the
  document.
- Fourteen functions are over 120 lines (the painted widgets `tree::show` and `list::show`, named in
  section 9; `App::commands`, a table of data; `mcpview::server_card`, `marksview::marks_detail`,
  `cataledit::form_view`, `forms::install_form`, `diffview::diff_detail`, `chrome::nav_row`,
  `chrome::tab`, `thispc::evaluate`, `apply_profile`, `admx::rules::to_catalog_file`, `profile::check`).
  They are forms and views of one piece each; the survey script counted some wrongly (`reg_of`,
  `policy_rules`, braces inside strings), so the next pass measures with `cargo clippy`'s
  `too_many_lines` instead.
- Files over 800 lines without their tests: `regview.rs` (830) and the window's `lib.rs` (726 with the
  types of the window) are under a thousand; the rest are at or under 800.

What part 2 starts from: `widgets.rs`, `tree.rs`, `list.rs`, `style.rs`, `themes.rs`, `i18n.rs`,
`session.rs`, `settings.rs`, `renderer.rs`, `mdview.rs`, `chrome/`, `doc.rs`, `commands.rs`,
`icons.rs`, `question.rs` and the dispatch of `app/apply.rs` know nothing of profiles, catalogs or the
registry; they are the window W# takes, with `Tab`, `DocKind` and the acts of the chrome (`NavAct`,
`FileAct`, `SettingsAct`, `ThemeAct`) as its first interface.
