# 03. Feature inventory

Draft 0.1, 14.10.2026. Everything the predecessor does, as WinWeaver must do it, plus what is new (marked
NEW). Each item names the user-visible behaviour; the mechanism is in `04-architecture.md`.

## 1. Rule catalog

- A rule has an id (`<topic>.<name>`, stable across versions), a title, a description, search tags, a
  group, a phase (windowsPE, specialize, oobeSystem, default-user, user-first-logon, post-oobe), a default
  state, parameters (enum, string, number, list, boolean; with defaults, ranges, `required` and
  `differs_from` constraints) and actions.
- Action kinds: answer file settings (a pass, a component, a path, a value), registry values (key, name,
  kind, value, the Windows default for the return to defaults, a `{param}` placeholder in the value only),
  registry removals, lists of values (a `reg-list` with a prefix or `name=value` items), script lines for
  the embedded scripts of the answer file, commands of the passes, app removals.
- Relations: `requires` (a rule switched off switches off the rules that need it; a rule switched on
  switches on what it needs, with a message), `conflicts` (both ways), `linked` (an imported policy that an
  enabled built-in rule covers is shown checked but stays off).
- The tree has three roots by mechanism: answer file settings, policies (registry under `Policies` keys,
  and every imported template), other (registry values elsewhere, scripts, app removals); a thematic group
  sits under the root of most of its rules and a rule of another mechanism lives in a mirror group under
  the right root. A rule in the wrong root fails the tests.
- The catalog is English with translation files per language (titles, descriptions, tags, group titles).
- Generated groups (browsers, File Explorer namespaces and desktop icons) come from tables in tools, not
  from hand edits.
- Catalog version in the profile; a profile of another catalog version loads, its unknown rules are listed
  under the last root.

## 2. Profile editing

- Rules on and off with the resolver cascade; parameters edited in the detail panel; groups turned on and
  off as a whole (an imported root never switches its thousands of policies on).
- Forms: Installation (edition from a list of 14, key mode generic, ask or a key, computer name random,
  fixed or a template with `{serial}`, `{mac}`, `{random}`; workgroup; disk mode ask or a disk and
  partition id; account mode file or ask; options such as no Dynamic Update), Accounts (name, display
  name, group, password; starter accounts without passwords are a customer decision and are not
  "fixed"), Languages (display language, input languages and layouts, region, time zone, switch keys).
- Drafts of unsaved changes; recent files; a new profile from a preset.

## 3. Check and build

- Check (`F7`): every rule and form validated; errors, warnings and infos listed in the message bar with a
  click that selects the item. Examples: a non-ASCII account name, a workgroup outside 15 safe characters,
  a disabled rule that a parameter depends on, an enum clash (`differs_from`), an empty required text, a
  KMS client key without a KMS host (info), the mode "ask" with a hidden screen.
- Build (`F9`): `autounattend.xml` for x64 and arm64 architectures, the passes with their components,
  the embedded scripts (Setup-System, Setup-User, Post-OOBE with the scheduled task), the commands with
  Path up to 259 characters, exit 0 everywhere, no comments inside components, everything ASCII (texts
  outside ASCII go through Base64 into the scripts), the profile embedded in `Extensions/Profile`.
- Native answer file settings (`xml-setting` actions) are merged into the components by pass; a path that
  is both a value and a parent is a build error shown as a message.
- The built file is checked by the program's own answer file checker (the port of the 37 checks of the
  predecessor's checker, in Rust, run on the built document before it is written) and the result shown.
- Import: an answer file with an embedded profile opens as that profile; one without is imported by its
  actions (the `$Config` block or the rule markers of the embedded scripts are parsed and the conditions
  evaluated).

## 4. Comparison, diff, merge

- Compare the open profile with a file or a preset: a diff tab (`01-ui.md` section 7) grouped by the
  thematic group under each root, with the rule description and the forms side by side.
- Save the differences as a diff file; apply a diff to the open profile (merge through the resolver);
  apply the changes to this PC (the differences only).

## 5. This PC

- Read: the audit of every rule the catalog can judge on a running Windows (all but the installation,
  first sign-in and sign-in screen rules) read natively; a rule is in effect when every value matches,
  partly in effect, not in effect, not readable (needs elevation), or as-default (what a clean Windows
  already has is no evidence, so the rule keeps the state of the open profile); parameters take the
  values read when they fit; the forms take the edition, time zone, locales, input languages and the
  accounts (no passwords), never the computer name. The result is a copy of the open profile in a new
  tab, or the selection read into the open profile.
- Apply: the rules on are applied, the rules off are returned to their Windows defaults, after a backup of
  every value touched (a `.wwundo` file in `output/`) and a confirmation; needs elevation (UAC) once per
  session through the elevated helper (`04-architecture.md` section 5). Return to defaults: the same for
  a selection using the `default` fields of the actions; a value under `Policies` needs none; a rule whose
  default is `unknown` is skipped and listed.
- Undo: the backup applied back.
- Scripts: Apply, Undo and Audit PowerShell scripts generated for a PC without the program (the only
  PowerShell the program writes besides the answer file); each script is self-contained and labelled with
  the profile name.
- The elevated read is offered when values are not readable.

## 6. Policy templates (ADMX)

- Import the templates of this Windows (`C:\Windows\PolicyDefinitions`) or of a folder; every policy
  becomes a rule `admx.<namespace>.<policy>` with its elements as parameters (enum, boolean, decimal,
  text, list, multi-text) and a Disabled rule `<id>.off`; categories become groups under the policies
  root. An import of a folder already imported asks to update in place or add a tree; trees can be
  loaded and unloaded (only the checked catalog files are ever read), renamed, removed.
- Options of a profile whose catalog is not loaded are kept, listed in the tree as unknown, warned of at
  the open and at the build, and left out of it; Load catalogs finds the imports that hold them by the ids
  of their rules and loads only those.
- Trust order of imports when two hold the same policy: bundled, this Windows, folder, catalog file. A
  policy is one rule owned by the most trusted import, shown as an alias in the other trees.
- Export the shown templates as a catalog file; import a catalog file as an import of the kind "package".
- Import a zip archive in memory, nothing unpacked to the disk: its ADMX files with the ADML of the folders of
  the cultures next to them (the archives of Chrome, Edge, Firefox), else its one catalog file; the size of
  the archive, the count of its entries, each entry and all it unpacks to are limited.
- Every record is checked on load: safe characters in keys, value names and values (no control characters,
  no U+FFFE and U+FFFF, no `]]>`), keys without empty, `.` or `..` segments, ranges, unique parameters,
  a category depth limit; a defect skips the policy and is logged, never stops the import.
- A policy whose values an enabled built-in rule writes is linked (section 1).

## 7. MCP server

- Transports: stdio (`winweaver.exe --mcp stdio`, no window) and HTTP on 127.0.0.1 with a bearer token
  (from the window or headless `--mcp http --port N --token T`). Protocol: MCP 2025-06-18 on JSON-RPC 2.0.
- Modes: read (the default at every start, never persisted), edit (changes in memory), files (save,
  save as, new profile). The option "read this PC" (never persisted) allows the tool `read_this_pc`.
- Tools (the names stay from the predecessor so that the agent skill and the instructions carry over):
  `get_status`, `list_groups`, `list_rules`, `get_rule`, `search_rules`, `get_profile`, `set_rule`,
  `set_param`, `set_group`, `set_profile_info`, `check_profile`, `preview_build`, `write_answer_file`,
  `load_profile`, `save_profile`, `new_profile`, `list_profiles`, `compare_profiles`, `read_this_pc`.
  Resources: the catalog, the profile, the presets, the skill files. Every error has a kind at the start
  of its text (`window_busy`, `not_allowed`, `not_found`, `invalid`, `write_failed`, `read_failed`).
- Nothing through MCP ever applies, elevates, deletes, replaces or imports; passwords and product keys
  are redacted in every answer and in the journal.
- The server has its own workspace of documents; the window shows them in the MCP tab and can open them
  as tabs. A request that needs the window while a dialog is open gets `window_busy`.
- The agent skills (`agents/skills/`) and the instructions for any agent (`agents/`) ship with the
  program and are served as resources.

## 8. Languages and themes

- Interface, catalog and documentation in English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish; time zones of their regions with their
  titles in each; a language is a folder in
  `lang/`; a missing text falls back to English; the setting "follow Windows" picks the Windows display
  language.
- Light and dark themes embedded and following Windows by default; more themes as JSON files in
  `themes/`; the title bar coloured on Windows 11.

## 9. Documentation in the window (NEW)

- Markdown tabs render the user documentation shipped in `docs/` (and `lang/<code>/docs/`), the guide
  and any `.md` file: headings, lists, tables, code, links (internal links open the page in a tab, links
  to a rule or a catalog select the item in its document, external links open the browser after a
  confirmation).
- The guide opens at the first start (a setting) and from the menu and `F1`.
- The rule list of the documentation is generated from the catalog at build time.

## 10. Bookmarks (NEW)

`01-ui.md` section 9 and `02-documents.md` section 6.

## 11. Logging

`logs/winweaver-<date>.log` with the level of the settings; the MCP processes log into their own files;
every message of the message bar is logged; passwords and keys never are.

## Open questions

1. The 37 checks of the answer file checker: port them all into the Rust build check, or keep a separate
   PowerShell checker for people who check files by hand?
2. Return to defaults needs a Windows default per value; the catalog data carries them (`default` or
   `unknown`). Should the conversion of the catalog require a decision for every `unknown`?
