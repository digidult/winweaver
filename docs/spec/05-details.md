# 05. Details

The facts every change of a part must respect: how a part works where the code alone does
not say it, the names of its functions and files, and the tests that guard it. They hold the
current state; the history of a change is in `../releases/`, the plans in `../todo/`. Before a
change, read the section of the part it touches.

## 1. This PC, the apply and the accounts

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

- Of the actions of one rule on one value (a removal for a clean start, then the value) only the last is
  checked (`thispc::plan`). A rule partly in effect is read as off, and the read names what differs
  (`Reading::differing`, `CaptureSummary::partly`, a warning on the rule in the message bar).

- The local accounts are read natively the way the audit script reads them: `NetUserEnum` (level 20, normal
  accounts) without the disabled ones and the relative ids 500, 501, 503 and 504; the group is Administrators
  or Users by direct membership (`NetUserGetLocalGroups`) in the groups S-1-5-32-544 and S-1-5-32-545, whose
  names `LookupAccountSidW` gives in the language of Windows. No password is read; the capture keeps the
  accounts of the profile when none of Administrators or Users is found.

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

- The installation media on a running PC is never the drive of Windows: a standard user can make
  a folder at its root (`C:\Drivers`, `C:\sources\sxs`), and what is found there would be
  installed by an administrator. The native apply looks on every drive but that one
  (`execute::media_drives`, the drive of `ffi::system_drive`), and the stand-alone apply script
  writes the search of `Op::RunFromMedia` itself (`scripts::media_lines`, `Find-MediaFolder` of
  `Apply.runtime.ps1`) in place of the script of the catalog. The script of the catalog is
  written for the installation and still looks at every drive in the answer file
  (`tests/render.rs`, `tests/scripts.rs`). The drive of Windows alone is left out: another
  fixed drive a standard user can write to is still looked at.

- The elevated helper asks Windows for its folders (`ffi::system_dir` and `ffi::windows_dir`:
  `GetSystemDirectoryW`, `GetSystemWindowsDirectoryW`) and never reads `SystemRoot` or `WINDIR`:
  the environment of a process is set by whoever starts it, and the programs of a plan run from
  the system folder as administrator (`system_program`).

- The limits of a users file, the same in `userstore` and in `Create-Users.runtime.ps1` (a test
  of `users::tests` compares them): the iterations of the passwords are 100 000 to 10 000 000
  (`MIN_ITERATIONS`, `MAX_ITERATIONS`; a file outside them is refused in `userstore::check`
  before a key is derived, with the reason), and the folder of a profile is one that does not
  exist or is empty (`winweaver_win::users::profile_folder_problem`, `Test-ProfileFolder`): the
  folder gets a new access list and a new owner, so the root of a drive (`users::is_drive_root`,
  an error of the check of the file too), a path with `..`, a link, a file and a folder that
  holds something are refused with the reason, and the profile an account has in that folder
  already is left as it is (`Registered::Here`, `Get-ProfilePlace`). A password that is replaced
  is wiped (`Passwords::put`), the map of a container is read straight into `Passwords` so that
  a part read before an error is wiped too, and the texts of the two password windows are wiped
  when a window goes (`Drop` of `MasterAsk` and `PasswordEdit`).

## 2. Languages, regions, the forms and the answer file

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

- The form Profile (`Form::Profile`, first of `Form::ALL`: the name, the
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

- Text parameters have `max_length` (`Param::Text`, checked by the catalog check and the check of a
  profile, the field counts its characters against it; `set_param` of MCP takes it as its limit):
  the support contacts have the limits of the answer file (256, the URL 2083), the autologon
  account 20; an ADMX `maxLength` becomes it. The fields of the parameters take the width of the
  panel, the title and the default above them.

- Only on (`ProfileAct::OnlyOn`, `ProfileDoc::only_on`, `ProfileDoc::shows`): the tree shows the
  rules that are on (chosen or shown on through a linked rule) with their groups open, like a
  search; the switch of the command bar is `commands::toggle`, drawn pressed when on. MCP:
  `only_on` of `list_groups` (with the count `on` of each group), `list_rules` and `search_rules`.

- A control character never reaches the answer file: XML 1.0 has none but the tab and the line
  breaks, Windows Setup refuses a file that is not XML, and a line break inside a value ends a
  line of an embedded script. `validate::control_problems` names the texts that are written in
  the modes chosen (the form Installation, the languages, the accounts, the text and list
  parameters of the rules in effect) as errors of the check, and `render::build` refuses the
  same list with the same words, so a build that was not checked (the preview of MCP) writes
  none either; a last look at the whole file refuses any other control character. A tab stays
  in the free texts alone: a password, a display name, a description, a text parameter.

- The slots of a template are filled in one pass (`render::params::fill`): a value is put in as
  it is and never read again, so a profile named `{{blocks}}` is a name. A slot of a template
  without a value is still an error. A value of a parameter (`substitute`) is likewise never
  read again.

## 3. The catalog, its imports and the links of rules

- The menu Catalogs lists every catalog file of `imports/` (`Full::import_list`, read again by
  `ImportAct::RefreshList` when the menu opens: `catalogs_menu` sees a gap in `cumulative_pass_nr`), a
  loaded one with `icons::CHECK` and its text in the accent colour (`ww::shell::menu_entry_colored`), a
  click loads or unloads it (`ImportAct::Toggle`, the setting by default); with
  `Extra::catalog_menu_opens` (General, A click on a catalog of the menu Catalogs) it lists the loaded
  ones alone and a click opens the tab (`CatalogAct::Open`), the settings loading the others. The
  comparison of two catalogs marks every field of the selected policy on both sides
  (`catalogdiff::Mark`: the same, differs, absent, only here; a tinted frame and a word, a legend
  above). A profile tab has the disc `icons::PROFILE`, the gear is the settings'.

- Imports: a policy is `admx.<prefix>.<name>` with `<id>.off`; groups `admx.<import>` under `policies`; a catalog
  file brings only Reg and RegRemove actions under `HKLM:\` or `DU:\` (checked at every load). Turning a group
  on skips imported policies; the read of the whole PC covers the loaded ones (`thispc::whole_pc_rules`: every rule of the catalog).

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

- The editor of a catalog (`Doc::CatalogEdit`, the folder `cataledit/`: `mod.rs` the document, the
  forms and Apply; `rule.rs`, `param.rs`, `action.rs`, `group.rs`, `texts.rs` the parts of the forms):
  Edit in the bar of a catalog tab (`Act::EditCatalog`, an import or the file of the tab) or New catalog
  of the menu of the templates (`custom-<n>`, an empty import, loaded). A policy is a `Rule` edited in
  place; what is JSON in the catalog stays text in the form (`ActionRow`: the key in the form of regedit,
  which `action::catalog_key` turns into `HKLM:\`, `DU:\` in the phase of the default user and `HKCU:\`
  in the others, or `HKU:\`; a value in decimal or `0x` hex, lines, hex bytes, or `{name}` for a
  parameter; `ParamRow`: the values of a choice). Apply goes through `CatalogFile::put_rule_with_texts` or
  `put_group`, which check the whole file, and the form starts again from the file. Save (Ctrl+S) checks
  and writes the file and loads a loaded import again; a removal asks first.
- The renames of a catalog file: `CatalogFile::rename_rule` (the policy and its `.off` rule, the
  requires and conflicts, the keys of the translations), `rename_group` (the parents, the groups of the
  rules, the translations) and `rename_id` (the id of the file and every id that is `admx.<old id>` or
  starts with `admx.<old id>.`, the ids a new catalog and its policies get); a new id is an id of an
  import no other rule or group of the file has, and the whole file is checked. The editor applies its
  form first, then renames, and keeps the renames of rules since the save (`EditorDoc::renamed`, a rule
  renamed twice is one rename from its first id). At the save (`WinWeaver::catalog_saved`) an import
  whose id changed (`EditorDoc::saved_id`) is written as `<new id>.wwcatalog` and the old file goes,
  unless another import has the new id; the loaded imports of the settings and the catalog tabs take the
  new id; `Profile::rename_rules` renames the choices of the open profiles (marked changed) and
  `Bookmarks::rename_rules` the bookmarks, before the catalog is merged again. A profile file that is not
  open keeps the old ids, which show as unknown options.
- A catalog file is checked as a catalog of its own (`CatalogFile::check`): the checks of the built-in
  catalog run on its rules in a catalog of the three roots and its groups (`as_catalog`; a policy without
  actions yet, one the editor just made, only for its place), and its rules may need or conflict with
  rules of the built-in catalog (`embedded::cached`), which `merge` keeps. Its texts are
  `i18n::GroupText` and `RuleText`, the types of the translation files, read from the pairs of version 1
  too, and `merge_texts` adds them whole. `CatalogFile::code_rules` (`Rule::runs_code`: ps, the commands
  and settings of the answer file, exe) is what the question before an import counts
  (`WinWeaver::import_catalog_file` for the dialog, an archive and a browser, `import_open_catalog` for a
  tab) and what `set_catalog_policy` of MCP refuses; `--import-admx` needs `--code allow` for it.

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

- A catalog file (`.wwcatalog`) or an ADMX file (`admx::read_file`, with the ADML of the folders of the
  cultures next to it) opens like any document, without importing it (`CatalogDoc::from_file`); its
  command bar offers Import.

- The tab of an import (`Doc::Catalog`, "Open" in Settings, Policy templates) shows its tree without check boxes
  and the policy with its parameters and registry values; "Show in the profile tree" loads an import that is
  not loaded, brings the first profile tab to the front, opens the groups above the policy and selects it.

- Linked policies (`resolver::links`): an imported policy without parameters whose every Reg or RegRemove
  action one built-in rule in effect has too (path and value name ignoring case, the same kind and value)
  is linked to it: the tree and the group counts show it on (`Resolution::shown_on`), its check box is
  disabled with the built-in rule in the tip, the profile keeps it off, and `get_rule` names it in
  `linked_by`. A policy chosen on stays a normal rule.

- Rules that write the same values (`registry::same_values`): the detail of a rule lists every rule
  of every catalog that writes or removes one of its registry values, which is the same both ways
  (a test checks it); the list is kept in the memory of egui by the rule and the count of rules.

- The parts of the description of an option have the colours `section-what`, `-risk`, `-writes`,
  `-deps`, `-more`, `-ref` (`views::SECTION_COLORS`, installed by `WinWeaver::new` with
  `style::install_colors`): a stripe on the left and a faint tint of the card.

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

- A line of a script (`ps`) and a command of the answer file (`xml-pe-command`,
  `xml-specialize-command`) are code (`Action::code_fields`): they are written as the catalog
  has them and never filled with the value of a parameter, which would be run as code (as
  SYSTEM, during the installation). The check of the catalog refuses a `{name}` of a parameter
  of the rule in them (`check::check_rule`, the test `a_parameter_in_code_is_refused`; their
  other braces are the ones of PowerShell); a value reaches a script only through an action
  that quotes it (`reg`, `reg-list`, `exe` arguments through `ps_quote`) and the XML through
  `xml_escape`. The type of a parameter is checked before a build (`validate`), not before the
  scripts, the registry files and the native apply, which is one more reason for the quoting.

## 4. The registry, registry files, Group Policy and the scripts

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

- Scripts and registry files (`winweaver_core::export`): one place makes them for the selection of a
  profile, the whole profile (every rule; the ones off return to the defaults), a diff (its other
  side, `export::diff_profile`) and the command line (`--script apply|revert|audit FILE`, `--reg
  apply|revert FILE`, `--only IDS`, `--out FILE|FOLDER`, FILE a `.wwprofile` or a `.wwdiff`). A selection
  of only passes of the installation, the first sign-in or the sign-in screen has nothing for a PC
  (`export::applicable`): the block This PC of the detail panel is not drawn and its items of the
  menu This PC are off.

- The registry tab: the values are `list::show` (the arrows, Home, End, the page keys, Enter edits,
  Delete asks), Rename (F2) renames the selected value or else the key (`registry::rename_value`;
  `Registry::rename_key` copies and deletes, `NativeRegistry` uses `RegRenameKey`).

- Registry files: the outline is a tree of the keys by default (`regfileview::tree_rows`, the folded
  keys in `RegFileDoc::closed`; List and Tree in the command bar set `Extra::regfile_tree`). A key the
  file names in several entries is the last of them in the tree, what the import leaves; a key a
  line above deletes, with itself or a key it is in, is made anew (`regfileview::made_anew`: the
  icon ↻ in the outline, a chip and a line in its head). A list that replaces the values of its key
  is written so (`[-key]`, then `[key]`), and the tree showed the deletion alone before 0.1.52. The
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

- The Group Policy editor (`PolAct`, `polview::PolEdit`, `EntryKind`): entries made, changed (Enter
  or a double click in the list) and removed after a question (Delete in the list), the root of a
  file outside Group Policy chosen in the bar (`PolAct::Root`, a change of the view, not of the
  file), the entries of another `.pol` file (`PolFile::merge`) or the changes of a `.reg` file of
  that root (`PolFile::merge_reg`) added, the file written back as PReg (`PolFile::to_bytes`,
  `save`) or as a registry file (`PolFile::to_reg`); the files of Group Policy of this Windows are
  never written (`PolDoc::is_system`, `pol_is_system`): Save asks for a place for a copy. The draft
  of a `.pol` tab is its bytes as hexadecimal with the root chosen (`POL_DRAFT`).

- A registry file is read in linear time: a value that goes on in the next lines is collected
  in one text (`RegFile::parse`), whatever the number of its lines. A Group Policy file with a
  control character in a key, a value name or a name of a list is refused at the read
  (`pol::refuse_control`): the file becomes lines of text when it is exported; `merge_reg` leaves
  such a change of a registry file out (a lone CR inside a line), since the file could not be
  read again. Every comment of
  `PolFile::to_reg` and of `RegFile::from_plan` is made by `RegFile::note`, which leaves no
  line break in it, and an entry made in memory with such a name is named in a comment and
  never written as a line. A `]` inside a key needs nothing: regedit, and `line_of`, end a key
  at the last `]` of its line.

## 5. The MCP server and the logs

- Logs and crashes (`winweaver_mcp::log`, set up first in `main`): the MCP server writes `logs/mcp-<date>.jsonl`
  (one JSON object a line: `time`, `level`, then `text` or the fields of an event; `log::event`)
  (transport start and stop, every request with its time, refusals and transport errors; ids and switches
  only, never a token, a password, a key or a free text). The panic hook and the filter of unhandled Windows
  exceptions (`winweaver_win::on_fatal_exception`: access violation, stack overflow) write the crash into the
  MCP log for a headless process or a thread named `mcp-*`, else into `winweaver-<date>.log`. A panic inside a
  request is caught (`Server::handle_text`): the request gets the error -32603 and the server goes on; a
  release build has no console, so `eprintln!` of a headless process also goes to the log (`refuse`).

- MCP: the server, its protocol, transports, journal, log, tab and command line are the ones of W#
  (`wwmcp`, a git dependency like `ww`); `winweaver_mcp::Tools` is the source `winweaver`, the tools,
  their options and resources, and a start makes new tools (a workspace of its own) on the profile of
  the option `profile` (`winweaver_mcp::server`, `mcpview::McpPanel::renew`). Every setting of a server
  is a configuration file `mcp/<name>.wwmcp` (`wwmcp::Config`, strict, saved by the MCP tab at every
  change; `settings.json` keeps no MCP field (an `mcp_token` of an older file is read once into
  `mcp/default.wwmcp`): the transport for a start without the window, port, token, mode, `detailed_log`
  and in `plugins.winweaver` the switches of the tools and the options read_pc, registry,
  edit_library and profile. A file of the format `winweaver-mcp` is written in the format of W# by
  `winweaver_mcp::legacy` when the tab opens it, at a start of `--mcp` for the folder `mcp` and for the
  file named. `winweaver --mcp FILE` serves as the file says, `--mcp stdio|http --config FILE` too, and the
  options of `wwmcp::cli` change it for that start (`--read-pc`, `--registry`, `--edit-library` and
  `--profile` are the options of the source by their keys). The journal keeps every call with its
  arguments and answer (`redact` leaves out password, token, authorization, secret and the
  product_key of WinWeaver; an answer is cut at 16 KB); the detailed log writes them as DETAIL events.
  The MCP tab shows the selected call in a right panel with buttons to the places it names
  (`mcpview::places`: rules, workspace profiles, catalog files, registry keys); the texts of the tab are
  the ones of `wwmcp` (`wwmcp::SOURCES` in the test of the translations), the names of the options and
  the groups are translated at run time (`tr_any`) from the same `ui.json`;
  errors start with not_allowed, not_found, invalid, write_failed or read_failed; passwords and the product
  key are replaced by `***` in every answer (the last fact of this section).

- MCP, bookmarks and catalogs: `list_bookmarks`, `list_catalogs` (by `CatalogHeader::read`), `get_catalog` (the
  groups, or the policies of a group) and `get_catalog_policy` read always, like profiles; `add_bookmark`,
  `set_bookmark_comment`, `remove_bookmark`, `set_catalog_policy` and `remove_catalog_policy` need
  the option `edit_library` (`--edit-library`, the MCP tab, off by default). A catalog
  changes through `CatalogFile::put_rule` (the whole file is checked, a failure changes nothing),
  `remove_rule` (with its `.off` and the conflicts with it), `new_policy`, `new_group`; `admx::catalog_path`
  turns a key of regedit into `HKLM:\` or `DU:\`. The ids of catalogs are file names checked against `..`.

- No secret leaves through an answer of a tool (the journal of W# is redacted by `redact`; the
  answers are not, so the tools hide them themselves): `get_profile` and the resource of the
  profile show `***` (`redacted`), `preview_build` hides the secrets of the whole answer file
  and cuts it after (`profiles::beginning`; `hide_secrets` hides a value without its closing
  tag to its end), and `compare_profiles` says of the product key only whether each side has
  one (`hidden`) and of the accounts only that they differ. A new tool that returns a part of a
  profile or of an answer file goes through one of them.

## 6. The window and W#

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

- Open the folder (`Act::ShowInFolder`, `winweaver_win::show_in_folder`): a build, a saved script, registry file,
  profile, diff, catalog file (exported or edited), theme or MCP configuration records its file with the message of the status bar
  (`App::shown_file`), and the status bar shows the button while that message stays; Settings, Files opens
  the folders of the program (made first when missing). Explorer is started only at a click (`/select,` for
  a file), never by a test, and not waited for.

- Every removal asks first (`App::ask`, Escape says no): an import, a bookmark of the bookmarks tab (the star
  of the navigation row stays a toggle), closing a tab discarding its changes, forgetting the recent files,
  renewing the MCP token. The answer of a dialog stands beside Cancel, so a new one joins the list of the
  test `the_answers_of_a_dialog_do_not_read_as_cancel` (its translations may not start like Cancel).

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

- The settings tab has the sections General, Policy templates (load, export, rename, remove, import) and Files
  (the folders next to the executable, the recent files); a rename changes the name of the catalog file and
  the title of its tree and saves the file.

- The title bar takes the colours of the theme (`winweaver_win::color_title_bar`: DwmSetWindowAttribute 20 for
  the dark frame, 34 to 36 for the border, caption and text on Windows 11), only for the window of this
  process and again whenever the visuals change; the system theme is never written.

- Drafts: `drafts/<key>.json`, key `p-<fnv of the path as it is kept>` or `n-<time>-<n>`; written 2 s after
  the last change and at the close; a save or a discard deletes them.

- Paths in the files of the program (`winweaver_core::paths`): a file inside
  the program folder is kept relative to it and starts with `.` (`.\profiles\Office.wwprofile`), any other
  file keeps its absolute path, and in memory every path is absolute. `Settings::load_from` and `save_to`
  (recent, open_tabs; relative to the folder of the settings), `Bookmarks::load` and `save` (the path of
  every bookmark but the registry ones), `Diff::load` and `save` (both sides) and the drafts (their path
  and `key_for_path`) convert at the file; `stored_text` changes only an absolute path, so a title of a
  preset or a key of the registry stays, and `resolved_text` only a text that starts with `.` and a
  separator. The ids of imports made of a folder, an archive or an ADMX file hash the kept form too
  (`kept_name` of the window). A new file that keeps a path converts the same way.

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

- W# (`docs/ww.md` of supakov/ww): `ww::Window<WinWeaver>` owns `shell: Shell<WinWeaver>` (the tabs, the
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
  shell (the sources of W# at `ww::SOURCES`, its checkout of git) are in every `ui.json` of
  WinWeaver (the test reads both crates).

- The settings file: `Settings::load(program)` reads the
  first of `<exe stem>.settings.json`, `<program>.settings.json` and `settings.json` that exists and
  writes back to it (`Shell::settings_path`), so two copies of an executable with different names keep
  their own settings and a plain `settings.json` goes on. `Settings::wrap_lines` folds the lines of
  a text or a page to the width of the view (the setting Wrap lines of General); off, the view scrolls
  sideways (an infinite width for the page). A page always
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

- The panel of the work in the background (`App::jobs`, `ww::Job`, `shell/jobs.rs`): WinWeaver
  lists the read of this PC, an import, a change of this PC and the search of the registry (with
  its Stop, which stops the search of every registry tab), each with the time since it was first
  seen (`WinWeaver::track_jobs`, `Full::jobs_since`); the panel folds (`ShellAct::FoldJobs`).

- Menus (`ww::shell::menu_item`, `menu_entry`, `sub_menu`): every item keeps the place of an icon
  at its left (`icon_place`, an empty image of the size), painted by `style::paint_icon`, so the
  texts line up with or without one; the menus of WW itself (a tab, the breadcrumb, the recent
  searches, the copy menus of texts) have icons too. The tab row (`shell/tabs.rs`) is a scroll area
  without a bar when the tabs are wider than the window: the wheel scrolls it sideways, a new active
  tab or count scrolls the active one into view, and a button after the new tab lists every tab.

- The messages of the check are translated at the display: `validate.rs` makes each with `tr`
  or `trf` of `winweaver_core::message` (the English text is the key, the values are kept
  apart), `validate::check` gives them in English (the command line, MCP, the tests) and the
  window calls `validate::check_shown` with `message_text` (the key through `tr`, the values filled in
  one pass). The test `every_text_of_the_interface_is_translated` reads the files of
  `CORE_MESSAGES` beside the sources of the window, so a new message of the check needs its
  eight translations; a new file of the core with messages for the window goes into that list.
  The test finds a literal on the line after the bracket too (as rustfmt puts a long one). The
  line of the counts is `check_summary` of the window (`validate::summary` in English), the one
  of a read of this PC `capture_line` and `verdict_text`; the notes and the forms a capture
  names, the steps of a report and the warnings of a build stay English inside their sentence.

## 7. wwlite and the browser edition

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
  edition: a change of the browser edition changes it too.
  A page draws with the fonts of egui alone (Ubuntu Light, Noto Emoji, its icon font), and a sign
  they lack is an empty box: the cross of a tab (U+2715) was one. `ww::icons` gives a build for
  wasm32 signs they have for the cross, the arrows and the small triangles; a new icon a page shows
  is checked against those fonts (a glyph of 0 in their cmap is the box).
  The menu of a page has no Open recent and no Exit (`cfg(not(target_arch = "wasm32"))` in
  `app/menu.rs`). `docs/wasm.md` section 6 builds the page by hand, without CI.

- winweaverweb (`crates/winweaverweb`, `docs/wasm.md` section 0a): winweaver-ui with `full` and
  without `pc` and `mcp` for wasm32; `winweaver-win` is a dependency on every target, since its
  functions answer that there is no Windows elsewhere (no PC, a registry in memory). The writes of
  the core and of the window go through `ww_base::files` (`jsonfile::write_atomic`,
  `PolFile::save`, `RegFile::save`, the answer file, `ww::dialogs::write_file`), the list of the
  imports too (`import_files`); in a browser a path under `ww_base::files::DOWNLOAD` is a download.
  A new save or open of a file keeps to them, or the browser edition loses it: no `std::fs`
  outside the native code (a browser has none, so `create_dir_all` failed an import there).
- A file dialog that opens (`pick_file`) answers nothing in a browser: an act that reads a file
  calls `WinWeaver::pick_for` there with a `Purpose` (`app/web.rs`), whose bytes come to a later
  frame, and keeps the native dialog under `cfg(not(target_arch = "wasm32"))`. A button or a menu
  item of an act that needs this PC or its disk is left out without the feature `pc`
  (`act::offered`, `#[cfg(feature = "pc")]` in `commands.rs`), and `act::needs_pc` names the act so
  that `apply` refuses it too.

- The catalog files of the list of the imports are read through the file layer in a browser:
  `admx::CatalogHeader::read`, `admx::rule_ids` and `admx::file_size` read the first bytes, the
  stream and the size of a file from the disk natively and the whole file through
  `ww_base::files::read` under `cfg(target_arch = "wasm32")`, where `std::fs` finds nothing (the
  list showed sizes of 0 and ids for names, and Load catalogs found no file).

## 8. Tests, CI, builds, archives and the command line

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
  (`gh run download <id> -n screenshots -D <folder>`); the way to see the window without starting
  it on the
  customer's PC. A renderer that cannot start writes `error.txt` there instead of failing the tests.

- Window tests: `App::new` and `App::apply` test the logic without frames; `egui_kittest` (a dev-dependency
  with its eframe feature) draws whole frames without a screen and clicks by the labels of AccessKit, so no
  window ever opens. A test that needs a label finds it with `query_all_by_label`, since a text can appear
  twice (a tab and a breadcrumb).

- The archive: the job Package copies the exe, LICENSE, `package/README.md`, the presets as `profiles/`,
  `package/lang/` and `agents/` into `dist/WinWeaver` and uploads `dist/`, so the artifact is the one archive; a
  tag zips the same folder for the release. Agent texts stay short (local models with small contexts) and name no
  model: the person chooses it.

- The presentation (`docs/presentation/make_presentation.py`): `SLIDES` is the list of slides (a title, lines, a screenshot of the artifact
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

- The workflow gives the token of a run the right to read the repository and nothing more
  (`permissions: contents: read` at its top); the job `release` alone asks for `contents: write`.
  wasm-bindgen is unpacked in CI only when its SHA-256 is the one its release names (the file
  `.sha256sum` beside the archive), the check `build-all.cmd` makes for its download.
