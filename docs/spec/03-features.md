# 03. Feature inventory

What WinWeaver does, by the behaviour the user sees; the mechanism is in `04-architecture.md`. Section 17
lists the planned changes.

## 1. Rule catalog

- The catalog is data (`data/catalog/`: `groups.json`, the rule files `00` to `17`, `lang/<code>.json`),
  embedded in the executable; `winweaver --check-catalog` loads it and reports its problems.
- A rule has an id (`<topic>.<name>`, stable), a group, a phase (windowspe, specialize-xml, specialize,
  oobe-xml, default-user, user-first-logon, post-oobe), a title, a level (optional, baseline, recommended,
  risky), a default state (on in a new profile), search tags, a link into the reference (`doc`, a page and
  a heading), the texts of its description (summary, effect, risk, versions, rollback, verify, note),
  parameters, relations and actions.
- Parameters: enum (with `differs_from` and `same_allowed` against another enum of the rule), int (with
  min and max), string (`required` or not), list (the items of a `reg-list`, or the lines of a multi-line
  value). Every parameter has a default.
- Action kinds: commands of the windowsPE and specialize passes, native settings of the answer file (a pass,
  a component, a path, a value), elements of the OOBE section, lines of the embedded scripts (`ps`),
  registry values (key, name, kind, value, the Windows default for the return to defaults: a value,
  `absent` or `unknown`; a `{param}` placeholder in the value only), registry removals, lists of values
  (`reg-list`: the items named by a prefix and a number, or `name=value` items; without `additive` the
  other values of the key go first), the start type of a service, a program run with arguments, an
  optional feature, capabilities removed by a name pattern, Store apps removed by package name.
- Relations: `requires` (a rule switched off switches off the rules that need it; a rule switched on
  switches on what it needs, with a message), `conflicts` (both ways), linked policies (an imported policy
  without parameters whose every registry action a built-in rule in effect has too is shown on, its check
  box disabled with the built-in rule in the tip, and it stays off in the profile; chosen on, it stays a
  normal rule).
- The tree has three roots by mechanism (`Catalog::mechanism`, read from the actions of a rule):
  Installation holds the answer file settings and commands and the scripts of the installation (the
  specialize scripts, the first sign-in, the task after OOBE); Policies holds every rule that only writes
  registry values, their removals and lists, under any key, and every imported template; Other holds the
  rest: services, features, capabilities, apps, programs, scripts on a running Windows and their mixes
  with registry values. A rule of only script steps lives under Installation or Other, as the catalog
  decides. A thematic group sits under the root of most of its rules; the few rules of other mechanisms
  (security, network, logging, telemetry, system) sit directly in the root Other beside its group of the
  apps, and the network and the logging of PowerShell are part of Security, the browsers part of Apps
  under Policies. A rule in the wrong root fails the tests, and only the root Other holds rules directly.
- The catalog is English with a translation file per built-in language (the titles and texts of the rules
  and the groups); a test fails when a rule lacks its texts in one of them.
- The catalog version is kept in the profile; a profile of another catalog version loads, and its unknown
  rules are listed under the last root.

## 2. Profile editing

- Rules on and off with the resolver cascade; parameters edited in the detail panel; groups turned on and
  off as a whole (turning a group on skips the imported policies under it, so an import never switches
  its thousands of policies on).
- Forms: Installation (edition from a list of 14 with their generic keys, key mode generic, ask (the empty
  key with the dialog of Setup) or a key, time zone, account mode file or ask, computer name random, fixed
  or a template with `{serial}`, `{mac}` and `{random}`, each with an optional length `:N` from 1 to 15
  (the last N characters of the serial number or the MAC address, N random characters of A-Z and 2-9
  without I and O; a value that cannot be read becomes random characters), the form describing every
  placeholder and showing what the template gives with sample values; workgroup; disk mode ask or a disk
  and partition), Accounts (name, display name, group, description, password; starter accounts without
  passwords are a customer decision and are not "fixed"), Languages (display language, system and user
  locale, input languages and layouts: every input language and keyboard layout Windows lists, by their
  names in the language of Windows, each item of the list with a layout of its choice; the browser
  edition of wwlite takes the embedded list of a Windows 11 with English names, section 18).
- The accounts of a users file go into the form Accounts, and the accounts of a profile make a users file
  (section 12).
- The user region (`default-user.region`): its two text parameters pick a region of Windows (`pick`
  `region-id` for the GeoID, `region` for the two-letter code; a catalog has both of the pair). The
  regions come from the Windows the program runs on (`EnumSystemGeoNames`, their GeoIDs and names in the
  language of Windows), never from an embedded list; the fields are typed freely with the list beside
  them, and a choice sets both.
- Drafts of unsaved profiles; the tabs of the last session; recent files; a new profile from a preset.

## 3. Check and build

- Check (`F7`): every rule and form validated; errors, warnings and infos listed in the message bar with a
  link that selects the item. Examples: a product key that is not five groups of five, a computer name or
  workgroup outside 1 to 15 Latin letters, digits and hyphens, a template that cannot be expanded, no
  account in the account mode file, an account name or password outside ASCII, a display name outside
  ASCII (set after OOBE), an account twice, an automatic sign-in that names an account with a password or
  one the profile lacks, a tag that is not a language tag, an input language Windows does not know, a rule
  chosen on but blocked, a baseline rule off, two rules in conflict, a parameter outside its options or
  range, an empty required text, two parameters that must differ, a text of a script outside ASCII, a list
  item without `=` or a name twice, options of a catalog that is not loaded, a region whose GeoID and
  code do not name one region Windows lists (a warning, nothing where Windows is not).
- Build (`F9`): `output/autounattend.xml` from the runtime templates (`data/templates/`: the skeleton and
  the scripts with their slots): the passes with their components for amd64 and arm64, the embedded
  scripts (Setup-System, Setup-User, Post-OOBE with its scheduled task, the script of the computer name
  template), commands of at most 259 characters. Everything the generator writes is ASCII: account texts
  outside ASCII go through Base64 into Post-OOBE, the name of the profile shows only in comments, the
  header and the first lines of the scripts with `?` for each other character, and the profile is embedded
  in `Extensions/Profile` as ASCII JSON, so a name outside ASCII comes back whole. The file names its
  build and the version of the runtime templates.
- Native answer file settings (`xml-setting` actions) are merged into the components by pass; a path that
  is both a value and a parent, or set twice, is a build error shown as a message, as is a command longer
  than 259 characters.
- Open: an answer file with an embedded profile opens as that profile; one without is refused with a
  message. A JSON profile in the format with `format_version` and `rules` with `enabled` is
  converted into a profile, with notes of what it left out.

## 4. Comparison, diff, merge

- Compare the open profile with a file, with a preset or with its own saved file (Unsaved changes; the
  preset of its name when it has no file): a diff tab (`01-ui.md` section 7) with the forms and the rules.
- Save the differences as a diff file with a comment on any item; apply every difference or one to the
  base profile (a merge through the resolver); go to an item in the profile of either side, opening its
  file or preset.

## 5. This PC

- Read: the rules the catalog can judge on a running Windows (all but the installation passes, the first
  sign-in and the sign-in screen) are read natively in a thread of their own. A rule is in effect when every
  checked value matches, partly in effect (read as off, with a warning that names what differs), not in
  effect, unknown (the optional features and the capabilities are read through the DISM API only when the
  window runs as administrator; the window says so), or as on a clean Windows (a value equal to its
  default, an absent policy value: no evidence, so the rule keeps the state of the open profile). The
  values of the default user profile are read in the current user, with a note. The read of the whole PC
  covers the policies of the loaded imports too. Parameters take the values read when they fit (list parameters are
  not read); the forms take the edition, the time zone, the locales, the input languages and layouts and
  the local accounts with their groups (no password), never the computer name.
- The result is a new profile named after the computer, or the selection read into the open profile; Check
  the selection shows the state on this PC without changing the profile.
- Apply: the selection made to match the profile, natively: the rules on are applied with the rules they
  need, the rules off return to their Windows defaults; after a confirmation that counts the changes no
  undo takes back, Windows asks for the permission (UAC) and the elevated helper backs up every value it
  touches, then changes the registry (also the default user profile, its hive loaded), the start types of
  services, runs programs of the system folder (optional features through `dism.exe`), removes
  capabilities and apps, and does the script steps of the catalog it knows natively; a script step it does
  not know is left to the stand-alone script and listed.
- Return to defaults: the same for a selection with the `default` fields of the actions; a rule whose
  default is `unknown` is skipped and listed.
- Undo: a backup applied back, the newest change first.
- Scripts: Apply, Revert (return to defaults), Audit and Undo PowerShell scripts for a PC without the
  program (`Undo-Apply.ps1`), and registry files for the apply and the return (section 14); each script is
  self-contained and labelled with the profile name and the build. The report of the audit script opens
  as a new profile, paired with the plan rule by rule, check by check.

## 6. Policy templates (ADMX)

- Import the templates of this Windows (`C:\Windows\PolicyDefinitions`), of a folder, of a zip archive or a
  catalog file; every import is a catalog file in `imports/`. Every policy becomes a rule
  `admx.<prefix>.<name>` with its elements as parameters (choices, numbers, texts, lists, multi-line
  texts) and a Disabled rule `<id>.off`; categories become groups `admx.<import>` under the policies root.
- An ADMX file (with the ADML of the folders of the cultures next to it) or a catalog file opens as a
  catalog tab without importing it; Import in its bar imports it.
- Only the checked catalog files are ever read; the settings list the others by their header (the first
  16 KB) with their size, and unchecking one drops it from memory and closes its tab. The menu Catalogs of
  a profile lists every file of the folder, read again when it opens: a loaded one marked and in the
  accent colour opens its tab, another one loads at a click and opens its tab.
- Options of a profile whose catalog is not loaded are kept, listed in the tree as unknown, warned of at
  the open and at the check, and left out of the build; Load catalogs reads only the rule ids of the files
  that are not loaded, and checks and loads the ones that hold some, the templates of this Windows first.
- A policy two imports hold is one rule of the first (the templates of this Windows first) and an alias in
  the groups of the others, so every checked import shows its whole tree.
- Export a catalog file; rename an import (the file and the title of its tree); remove it after a question.
- Compare two catalogs (loaded imports or open catalog files) policy by policy in a tab: the differing
  policies in the tree of their categories with switches for each kind, the selected one side by side;
  the report as a Markdown page or saved as a file.
- Copy to: the selected policy (with its Disabled rule) or group (with the groups under it) of a catalog
  tab into a catalog open in its editor or into a new one, with its ids and its texts in other languages
  (`CatalogFile::copy_from`).
- The editor of a catalog (`01-ui.md` section 18) changes the policies of a catalog file, and New catalog
  makes an empty import.
- Templates in UTF-8 or UTF-16 with its mark (the templates of vendors such as Chrome, Edge, Brave and
  Office); a second copy of a policy in one import is left out and listed as skipped, and policies of two
  namespaces with one prefix and one name are told apart by the namespace.
- A zip archive is read in memory, nothing unpacked to the disk: its ADMX files with the ADML of the folders
  of the cultures next to them, else its one catalog file. Limits: 256 MB of archive, 20 000 entries,
  512 MB unpacked, each entry up to the limit of its kind whatever size it declares.
- Every record is checked on import and at every load: safe characters in keys, value names and values,
  keys without empty, `.` or `..` segments, only registry values under `HKLM:\` and `DU:\`, ranges,
  unique parameters, a depth limit; a policy the program cannot represent is skipped and listed, never
  stops the import.
- A policy whose values a built-in rule in effect writes is linked (section 1).

## 7. MCP server

- Transports: stdio (`winweaver --mcp stdio`, no window) and HTTP: a POST of one JSON-RPC message to
  `http://127.0.0.1:<port>/mcp` with a bearer token of at least 32 characters, the Host and Origin headers
  checked against DNS rebinding; started from the MCP tab or without the window (`--mcp http`, `--mcp
  FILE`). Protocol: MCP 2025-06-18 on JSON-RPC 2.0; a message is at most 4 MB.
- The server is the one of W# (`wwmcp`): WinWeaver is its source of tools `winweaver`
  (`winweaver_mcp::Tools`). Modes: read (nothing changes, the default), edit (profiles of the workspace
  change in memory), files (profiles saved and the answer file written, only into the folders of the
  program). Options of the source, all off by default: `read_pc` (the tool `read_this_pc`), `registry`
  (off, read or write), `edit_library` (the bookmarks and the catalog files); `profile` starts the
  workspace. The user turns any tool, a group of tools or the whole source off in the MCP tab or the
  configuration file; a tool that is off is not listed and does not run for a client.
- 36 tools: `get_status`; the catalog (`list_groups`, `list_rules`, `get_rule`, `search_rules`); the
  profiles (`get_profile`, `set_rule`, `set_param`, `set_group`, `set_profile_info`, `check_profile`,
  `preview_build`, `write_answer_file`, `load_profile`, `save_profile`, `new_profile`, `list_profiles`,
  `compare_profiles`, `read_this_pc`); the bookmarks (`list_bookmarks`, `add_bookmark`,
  `set_bookmark_comment`, `remove_bookmark`); the catalog files (`list_catalogs`, `get_catalog`,
  `get_catalog_policy`, `set_catalog_policy`, `remove_catalog_policy`); the registry
  (`read_registry_key`, `search_registry`, `set_registry_value`, `delete_registry_value`,
  `create_registry_key`, `delete_registry_key`, `read_registry_rights`, `set_registry_rights`). Resources:
  the catalog, the current profile, the presets and the skill for agents (`winweaver://skill`). Every error
  starts with its kind (`not_allowed`, `not_found`, `invalid`, `write_failed`, `read_failed`).
- `get_rule` names the pick of a text parameter chosen from a list of Windows (`region-id`, `region`);
  `set_param` of such a parameter takes a GeoID or a two-letter code of a region Windows lists and sets
  both parameters of the rule, naming the region and both values in its answer.
- Nothing through MCP applies a profile to this PC, elevates or imports templates; the registry changes
  only with `registry` write (and the rights of the process), the bookmarks and catalog files only with
  `edit_library`. Passwords and product keys are `***` in every answer.
- The server has its own workspace of profiles; the window shows them in the MCP tab and opens them as tabs.
  Its journal keeps the last 500 calls with their arguments and answers (passwords, keys, tokens and
  authorizations left out, an answer cut at 16 KB). Call a tool of the tab runs any tool by its name as
  the program, without the switches and the options.
- A panic inside a request is caught: the request gets the error -32603 and the server goes on.
- The agent skills (`agents/skills/`) and the instructions for any agent (`agents/`) ship in the archive;
  the skill of the tools is also served as a resource.

## 8. Languages and themes

- Interface, catalog and guide in English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian
  and Turkish; time zones with their titles in each. The language "follow Windows" takes the display
  language of Windows and switches the interface and the catalog texts together. A folder `lang/<code>/`
  next to the executable adds or overrides a language of the interface and its guide; a missing text falls
  back to English. Every text of the interface is translated in every built-in language (a test checks
  it); the archive brings Belarusian as the example of a language file.
- Themes: Light, Dark, Latte and Matrix embedded, Light and Dark following Windows by default; more themes
  as `.wwtheme` files in `themes/`; the editor of a theme with a live preview; the fonts of each theme;
  icons in colour; the title bar in the colours of the theme on Windows 11.

## 9. Documentation in the window

- Markdown tabs show the guide, the reference of the parameters (`data/reference/`, embedded), the licence
  and any `.md` file: headings, lists, quotes, tables, code with a copy button, links (`01-ui.md` section
  19). The `doc` link of a rule opens its page of the reference at its heading.
- The guide opens at the first start (a setting) and from the menu and `F1`, in the language of the
  interface.

## 10. Bookmarks

Options of profiles, catalogs and diffs, and keys and values of the registry, with comments: `01-ui.md`
section 9 and `02-documents.md` section 6. A bookmark of the registry lists the rules that write its key.

## 11. Logging

- `logs/winweaver-<date>.log`: every new message of the status bar, what the window draws with and the
  renderers that failed, the crashes of the window.
- `logs/mcp-<date>.jsonl`: one JSON object a line (time, level, then a text or the fields of an event):
  the start and stop of a transport, every request with its time, refusals and transport errors, by ids
  and switches only; the detailed log of a configuration adds every request and answer with the secrets
  left out.
- The panic hook and the filter of unhandled Windows exceptions (access violation, stack overflow) write a
  crash into the MCP log for a process without the window or an MCP thread, else into the log of the
  window. Passwords, product keys and tokens are never logged.

## 12. Users files

A users file (`.wwusers`, `02-documents.md` section 11) lists the local accounts to make on a PC with their
passwords encrypted under a master password. Its tab: the accounts in a list (name, group, state, whether a
password is set, profile folder) and the selected one in a form with the orders for an account that exists;
the command bar adds, duplicates, removes and moves accounts, unlocks and locks the passwords (the master
password window), sets the password of an account, sets the master password, checks the list, reads the
accounts of this PC into it, creates the accounts on this PC or sets their passwords (as administrator,
after a question that names the accounts that exist and will change), and saves the stand-alone script.
While unlocked, the password of the selected account can be shown and copied. The menu New makes an empty
users file or one of the accounts of this PC (every normal account with its group, flags and profile
folder; no password). The form Accounts of a profile loads the accounts of a users file, with their
passwords when the master password is given, and makes a users file of its accounts.

## 13. Registry

- The registry tab (`01-ui.md` section 15): browse the keys of this PC, read and change values in the types
  of regedit, make and delete keys and values, find under a key, edit the permissions of a key (the own
  entries of its access list; the inherited ones are shown), bookmark a key or a value. Every key a view
  names (the actions of a rule, the catalog editor, an MCP call, a registry file) opens the tab there, and
  the tab names the rules that write the selected key or value.
- The same over MCP when the configuration allows it (section 7): a search returns at most 500 places
  within 100 000 keys; a change of permissions takes an account by its name, a checked `S-1-` identifier or
  an alias of two capitals, so no text of the caller reaches the security descriptor.

## 14. Registry files

- A `.reg` file opens in a tab (`01-ui.md` section 16) and saves back with every line kept
  (`02-documents.md` section 12); its keys open in the registry tab, and the rules that write them are
  named.
- This PC saves the apply and the return of a selection as registry files, with a comment naming the rule
  before its steps and the steps a registry file cannot hold as comments.

## 15. Files, folders and the command line

- Open (`Ctrl+O`) takes every kind of file of `02-documents.md` section 1; a file dropped on the window
  opens the same way. Recent files (10) are in the menu and in the settings.
- Open the folder: a build, a saved script, registry file, profile, diff, catalog file, theme or MCP
  configuration shows Open the folder in the status bar while its message stays; Settings, Files opens the
  folders of the program. Explorer starts only at a click.
- The command line (`winweaver --help`): the window; `--renderer auto|opengl|directx` (the renderer tried
  first, for this start); `--version`; `--check-catalog`; `--mcp FILE`; `--mcp stdio` and `--mcp http` with
  `--config FILE`, `--token`, `--port`, `--mode`, `--read-pc`, `--registry`, `--edit-library`,
  `--detailed-log` and `--profile`, which change what the configuration file says for that start;
  `--smoke-window` (a small window draws two frames and closes, for CI). The guide in every language and
  `package/README.md` list every parameter.
- The command line writes files without the window, through the functions the window uses
  (`crates/winweaver/src/cli.rs`): `--build PROFILE` (the check, then `autounattend.xml` with its scripts
  beside it; PROFILE a file, a preset or a name of `profiles/`); `--import-admx system|PATH` (the templates
  of this Windows, a folder, an `.admx`, `.zip` or `.wwcatalog` file into `imports/`, named among the loaded
  imports of the settings); `--export-catalog ID|PATH` (the catalog file of an import or of templates read
  without importing them); `--read-pc [--base PROFILE]` (this PC read into a new profile on the base, Office
  by default, or its answer file); `--diff A B` (a `.wwdiff`); `--script` and `--reg`. `--out` names the
  file when it ends with the extension the command writes, else it is a folder for the file of the default
  name (`export::out_path`). The exit code is 0 when the files were written, 2 for a wrong use, 1 for a
  failure, and every file written is named.

## 16. More of the window

- Registry files: the keys as a tree (by default) or a list; a value, a key or the whole file
  applied to this PC after a question that counts the changes and the deletions.
- The registry tab: the values as a list with the keyboard; a value or a key renamed (F2).
- Group Policy files (`.pol`): the files of the computer and of the users of this Windows, or any
  file, as a tree of keys with what each entry does; a button beside a key or a value that Group
  Policy sets in the detail of a rule, the registry tab and a registry file; a setting opens them
  with a read of this PC. The tab edits them: entries made, changed and removed (after a question)
  in a window like the values of a registry file, the root of a file outside Group Policy chosen,
  the entries of another `.pol` file or the changes of a `.reg` file of that root added, the file
  saved in the format of Group Policy (the files of this Windows as copies elsewhere) or as `.reg`,
  with a draft of its unsaved changes.
- The catalog by mechanism: Installation holds the answer file settings and the scripts of the
  installation, Policies every rule that only writes the registry, Other the rest; a text parameter
  has a limit (`max_length`, the limits of the answer file for the support contacts, `maxLength` of
  ADMX), and the fields of the parameters take the width of the panel.
- The tab row scrolls when the tabs are wider than the window and lists them in a menu; every menu
  item has an icon; Only on in the bar of a profile shows the rules that are on (MCP: `only_on` of
  `list_groups`, `list_rules` and `search_rules`); the work in the background (a read of this PC, an
  import, a change of this PC, a search of the registry) shows in a panel with its time and a Stop.
- Drafts of every document with unsaved changes: profiles, registry files, diffs, the catalog
  editor, users files (with the passwords of their last save, encrypted) and the theme editor.
- `.reg` and `.ps1` files of the selection, of the whole profile and of a diff, in the window and
  from the command line (`--script`, `--reg`, `--only`, `--out`).
- The detail of a rule lists the rules of every catalog that write the same registry values, both
  ways; the parts of the description have colours of the theme.

## 17. wwlite

The light edition `wwlite.exe`: the window of WinWeaver with the profile editor, the forms, the
check, the build of `autounattend.xml`, the presets, the catalogs of the folder `imports` and the
themes of the folder `themes`, a short guide as plain text, English and Ukrainian built in and
other languages as files (German in its archive). It has nothing of This PC, the MCP server, the
registry, registry and Group Policy files, diffs, bookmarks, imports of policy templates, the
catalog views and editor, users files, the theme editor (the fonts of the theme and their size stay
in Settings) and Markdown pages: winweaver-ui is built
without its feature `full`, so none of that code is in the executable.

The browser edition (`wwlite-web-<version>`, `docs/wasm.md`): the same window built for
`wasm32-unknown-unknown`, served as a page from any static web server. It opens a profile, an answer
file with the profile it embeds or an older profile through the file dialog of the browser or a drop on
the page; Save and Save as download the profile, Build downloads `autounattend.xml` alone (the scripts
are embedded in it); the settings, the drafts and the fonts of the theme stay in the storage of the
page; the input languages and layouts come from the embedded list of a Windows 11 with English names.
It has no folders (no catalogs of `imports`, no themes of `themes`, no languages of `lang` beyond the
built-in English and Ukrainian), no recent files, no tabs of the last session, no log file. Its keys
work in a browser (item 16 of the list of 28.10.2026): Alt+N for New, Alt+W for Close, Alt+PgDn and
Alt+PgUp for the next and the previous tab, Alt+Shift+PgUp/PgDn to move a tab, since a browser keeps
Ctrl+N, Ctrl+W, Ctrl+Tab and Ctrl+Shift+PgUp/PgDn for itself, and Alt+I for the guide, since F1 opens
the help of the browser; the page stops the browser from acting on F2, F3, F7, F9, Ctrl+F, Ctrl+D and
Ctrl+B, and eframe on Ctrl+O and Ctrl+S.
