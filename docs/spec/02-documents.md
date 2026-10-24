# 02. Documents and files

## 1. Document types

| Document | Extension | Content | Tab |
|---|---|---|---|
| Profile | `.wwprofile` | The choices of rules with parameters and the installation forms | Tree and detail panel |
| Diff | `.wwdiff` | The differences between two profiles, with comments | List and the selected difference |
| Catalog file | `.wwcatalog` | The rules and groups of one import of policy templates, exchangeable between PCs | Tree without check boxes |
| Bookmark file | `.wwmarks` | Options and registry keys with comments | List and the selected bookmark |
| Users file | `.wwusers` | The local accounts to make on a PC with their passwords encrypted under a master password in the same file | List of accounts and the form of the selected one |
| Registry file | `.reg` | The format of regedit (version 5 in UTF-16 with its mark, or REGEDIT4): keys, values and their deletions, comments; every line is kept, the ones the program does not read too | Outline of keys and comments, the lines of the selected key |
| Theme | `.wwtheme` | The name, the base, the colours and the fonts of a theme | The editor of a theme |
| MCP configuration | `.wwmcp` | The transport, port, token, mode and switches of an MCP server, its first profile, the detailed log | The MCP tab |
| Markdown page | `.md` | The guide, the licence, the reference, any Markdown file | Rendered page |
| ADMX file | `.admx` | A policy template with the ADML files of the folders of the cultures next to it | Opened as a catalog, without importing it |
| Answer file | `.xml` | The build; opened as the profile it embeds | Opens as a profile |
| Audit report | `.json` | The results of the stand-alone audit script of another PC | Read into a new profile |

The program opens a file by its extension. A `.json` file is an audit report, or a profile in the JSON
format with `format_version` and `rules` that are `enabled`, which is converted into a profile
with notes of what it left out. An answer file without an embedded profile is refused with a message. The
extensions are registered nowhere (portability).

## 2. Common format

- JSON, UTF-8 without a byte order mark, LF, two-space indentation, keys in the fixed order of the
  program, so that diffs in version control are readable. A catalog file is written compact.
- Every file starts with the fields `format` (the document type, such as `winweaver-profile`) and
  `version` (an integer, 1 for every document). A file of another format, or of a newer version than the
  program knows, is refused with a message.
- Reading is strict: a byte order mark, text that is not UTF-8, nesting deeper than 32 levels and a file
  larger than its limit (16 MB for most documents, 64 MB for a catalog file, 32 MB for a registry file,
  4 MB for a bookmark file, 1 MB for the settings, 64 KB for a theme and an MCP configuration) are errors.
  The records of a catalog and an MCP configuration refuse an unknown field.
- Writing goes to a temporary file next to the target that is renamed over it, so a failed save leaves the
  old file whole.
- A path to a file inside the program folder (the folder of the executable) is kept relative to it and
  starts with `.`, such as `.\profiles\Office.wwprofile`, so the folder can move or go to another PC with
  its settings, bookmarks, drafts and diffs; a file elsewhere keeps its absolute path. The recent files and
  the open tabs of the settings, the profiles of bookmarks and of diffs, the file of a draft and its key
  follow this rule, and so do the ids of imports made of a folder, an archive or an ADMX file; in memory
  every path is absolute (`ww_base::paths`).

## 3. Profile

```json
{
  "format": "winweaver-profile",
  "version": 1,
  "catalog": "0.11",
  "name": "Office",
  "author": "",
  "comment": "",
  "created": "2026-10-14T10:00:00Z",
  "modified": "2026-10-14T10:00:00Z",
  "install": { "edition": "Pro", "product_key_mode": "generic", "product_key": "", "time_zone": "FLE Standard Time",
               "account_mode": "file", "computer_name_mode": "random", "computer_name": "", "workgroup": "",
               "disk_mode": "ask", "disk_id": "0", "partition_id": "3" },
  "languages": { "ui_language": "uk-UA", "system_locale": "uk-UA", "user_locale": "uk-UA", "input": ["en-US", "uk-UA", "ru-UA"] },
  "accounts": [ { "name": "Admin", "display_name": "Admin", "group": "Administrators", "description": "", "password": "" } ],
  "rules": {
    "defender.pua": { "on": true, "params": { "action": "block" } },
    "admx.windows.AllowTelemetry": { "on": true, "params": { "level": 0 } }
  }
}
```

- `product_key_mode` is `generic`, `ask` or `key`; `account_mode` is `file` (the accounts of the profile)
  or `ask` (Windows Setup asks for one); `computer_name_mode` is `random`, `fixed` or `template`;
  `disk_mode` is `ask` or `partition`.
- An item of `languages.input` is a language tag (`uk-UA`: the language with the layout Windows gives it
  first) or a layout id (`0422:00000422`: the language of that locale id with that layout); the first item
  is the default. The languages and layouts are the ones Windows lists, never a list of the program.
- `rules` holds only the rules that are on or have parameters; a rule not listed is off. Parameters are
  written only when they differ from the defaults. A `source` of a choice, when a file has one, is kept.
- `unknown` (written only when it has entries) keeps the choices of rules the loaded catalog does not have,
  unchanged, so that a profile survives a round trip through a catalog that is not loaded; they are left
  out of the build with a warning.
- Passwords of accounts are stored in the profile as the user typed them (the answer file needs them);
  the MCP server and the logs redact them.

## 4. Diff

```json
{
  "format": "winweaver-diff",
  "version": 1,
  "catalog": "0.11",
  "base": { "name": "Office", "path": ".\\profiles\\office.wwprofile", "modified": "..." },
  "other": { "name": "Strict", "path": "", "modified": "..." },
  "rules": { "asr.usb-untrusted": { "other": { "on": true } } },
  "install": { "product_key_mode": { "base": "generic", "other": "ask" } },
  "comments": { "rule:asr.usb-untrusted": "Strict blocks USB sticks\nask the customer first" }
}
```

A diff holds only what differs: `rules` (the choice of each side, absent for "off with the defaults"),
`install` and `languages` (the two values of each field), `accounts` (the whole list of both sides when it
differs). A diff applied to a profile sets the `other` side of every entry; the resolver then disables what
the new states make impossible and the message bar lists it. One item can be applied alone. `comments`
keeps an optional comment of several lines by the key of an item: `rule:<id>`, `install:<field>`,
`languages:<field>` or `accounts`.

## 5. Catalog file

The rules and groups of one import of policy templates: `format` `winweaver-catalog`, `version`, `kind`
(`admx`), `id`, `name`, `windows` (the build the templates came from), `created`, `groups`, `rules`,
`texts` (the titles and summaries in other languages, by language code) and `skipped` (the policies the
import left out, and why). Plain JSON. The file is checked at every load: ids under `admx.`, groups under
the policies root, the phases of the machine or of the default user profile, only registry values, their
removals and lists under `HKLM:\` or `DU:\`, safe keys, value names and texts, placeholders only for the
parameters of the rule, conflicts only inside the file (`03-features.md` section 6).

## 6. Bookmark file

```json
{
  "format": "winweaver-bookmarks",
  "version": 1,
  "items": [
    { "document": "profile", "path": ".\\profiles\\office.wwprofile", "item": "defender.pua", "comment": "Ask the customer about Audit mode\nand note the answer", "created": "..." },
    { "document": "registry", "path": "HKEY_LOCAL_MACHINE\\SOFTWARE\\Policies\\Microsoft\\Windows Defender", "item": "", "comment": "", "created": "..." }
  ]
}
```

A bookmark of an option holds its id (a rule, a group or a form) and, as a link, the profile it was made
in (`path`, empty for an unsaved profile or a catalog tab); the title is taken from the catalog when it is
shown, so an option is one bookmark in every profile. A bookmark of the registry has the document
`registry`, the key in `path` and the value in `item` (empty for the key). The comment may have several
lines. Opening a bookmark opens its profile (loading the file when no tab has it) and selects the option;
without a profile file it selects the option in the profile tab in front. Positions of other documents
with a `title` are read and kept.

## 7. Settings

- The settings file is named after the executable: `winweaver.settings.json` next to `winweaver.exe`, so
  two copies with different names keep their own. At the start the window reads the first of
  `<exe name>.settings.json`, `WinWeaver.settings.json` and `settings.json` that exists and writes back to
  it. The window is the only writer; a start without the window reads it only for the imports to load.
- The fields of the window (W#): `language` (empty to follow Windows), `theme` (`system`, `light`, `dark`)
  and `custom_theme`, `renderer`, `restore_tabs`, `wrap_lines`, `page_width`, `window` (the place and
  size at the last close, and whether it was maximized), `open_tabs` and `active_tab`, `recent` (10
  files), `search_history` (20 texts).
- The fields of WinWeaver beside them: `desc_open`, `desc_sections` (the parts of the description open at
  first), `show_guide_at_start`, `inline_switches`, `hide_unknown` and `loaded_imports` (the ids of the
  imports that are read and loaded). Without `loaded_imports`, every catalog file of `imports/` but the
  ones `hidden_imports` names is loaded, and the list is written. `mcp_port` and `mcp_token`, when a file
  has them, are read once into `mcp/default.wwmcp` and not written back.

## 8. Draft

The unsaved state of a modified profile is written to `drafts/<key>.json` two seconds after its last change
and at the close: `format` `ww-draft`, `version`, `path` (the file of the profile, kept as section 2 says,
empty for a profile never saved), `saved` and `document` (the profile). The key is `p-<FNV-1a of the path as
it is kept>` for a file and `n-<time>-<n>` for a new profile. At the next start the window restores every
draft into its tab and says how many it restored. A save or a discard deletes the draft. The other
documents with unsaved changes write drafts too, at most every two seconds while they change and
at the close: their `document` is `{"kind": ..., "text": ...}` with the kind `regfile` (the text of
the file), `diff`, `catalog-edit` (the catalog file and whether it is an import), `users` (the users
file with the passwords of its last save, encrypted) or `ww-theme` (the theme and its id).

## 9. Presets

The presets Office, Strict, Laptop and Home are profiles embedded in the executable at build time (from
`data/presets/*.wwprofile`). They open from the menu New as unsaved profiles named after the preset, and
the menu Compare of a profile compares with each of them; the archive also brings them as files in
`profiles/`.

## 10. What lives next to the executable

```
WinWeaver/
├── winweaver.exe              the window, the MCP server (also without the window, --mcp) and the
│                              elevated helper of the native apply
├── README.md                  for the user (package/README.md)
├── winweaver.settings.json    written by the window
├── bookmarks.wwmarks          created at the first bookmark
├── profiles/                  the presets as files; the user's profiles (the default folder of Open and Save)
├── lang/<code>/               ui.json and guide.md: a language added or overriding a built-in one
│                              (the archive brings Belarusian, be, as the example)
├── themes/                    <id>.wwtheme: more themes, or one replacing an embedded theme
├── imports/                   <id>.wwcatalog: the catalog files of the imported policy templates
├── drafts/                    the drafts of unsaved documents
├── mcp/                       <name>.wwmcp; default.wwmcp is the one the MCP tab opens first
├── logs/                      winweaver-<date>.log (the window), mcp-<date>.jsonl (the MCP server, its crashes)
├── output/                    autounattend.xml, the scripts and registry files, the plans, backups and
│                              reports of the native apply
└── agents/                    the instructions and skills for AI agents
```

Everything embedded (the catalog with its translations, the English texts and the built-in languages of the
interface, the presets, the four themes, the guide in every built-in language, the reference, the licence)
is in the executable, so a bare `winweaver.exe` runs; the folders add languages, themes and imports and
keep the files of the user. The folders are made when the program first writes into them.

## 11. Users file

`.wwusers`: the local accounts to make on a PC, in one file of the common format (section 2). The file is
portable and bound to no machine: its only secret is the master password of the operator.

```json
{
  "format": "winweaver-users",
  "version": 1,
  "name": "Office users",
  "comment": "",
  "created": "2026-10-20T10:00:00Z",
  "modified": "2026-10-20T10:00:00Z",
  "users": [
    { "name": "admin1", "full_name": "", "description": "", "admin": true, "password_never_expires": true,
      "profile_path": "", "apply_password": false, "state": "keep", "force_group": false }
  ],
  "passwords": { "kdf": "pbkdf2-sha256", "iterations": 600000, "salt": "<base64>", "iv": "<base64>",
                 "mac": "<base64>", "data": "<base64>" }
}
```

- An account: `name` (a name of a local account: up to 20 characters, none of `" / \ [ ] : ; | = , + * ? < > @`,
  not ending with a dot), `full_name`, `description`, `admin` (the group Administrators, else Users),
  `password_never_expires`, `profile_path` (empty, or an absolute path `D:\Profiles\x` or `\\server\share\x`),
  and the orders for an account that already exists on the PC, without which it is left as it is:
  `apply_password` (set the password of the file), `state` (`keep`, `enabled`, `disabled`; a new account is
  made enabled or disabled) and `force_group` (exactly the group of `admin`, the other membership removed).
  Names compare without case. The checks: a name twice, a profile folder of two accounts, and, when the
  passwords are unlocked, an account without a password (an error) and a password without an account (a
  warning).
- `passwords` is absent until a master password is set. The passwords are a JSON object of name to password
  (names in the order of their code points, no spaces, UTF-8 as it is), padded with PKCS#7 and encrypted
  with AES-256-CBC under the first 32 bytes of PBKDF2-HMAC-SHA256(master password in UTF-8, `salt` of 32
  bytes, `iterations`, 64 bytes); `mac` is HMAC-SHA256 under the last 32 bytes of `salt`, `iv` (16 bytes) and
  the ciphertext `data`, verified before anything is decrypted. A wrong master password and a changed byte give
  one message. A new salt and vector come at every save; `iterations` is at least 100000, 600000 by default.
  The master password is never kept on disk, and no password reaches the status bar, the logs or a script.
- The operations on this PC (`users::Plan`, run natively as administrator, no PowerShell): create the
  accounts that are missing with the password, the flags, the description and the full name, the group of
  `admin` only, and the profile folder with an access list of SYSTEM, Administrators and the account itself,
  registered in `ProfileList`; change the existing ones by their orders only (update existing also by the
  rest of the row, never the password); or set the passwords of the accounts that exist and nothing else. An
  account is never deleted, a profile registered elsewhere is never moved, and the account of the process is
  never removed from Administrators. One status an account: CREATED, UPDATED (with the changes),
  ALREADY_EXISTS, PASSWORD_SET, NOT_FOUND, ERROR. The operation runs in the process of the window, so the
  master password crosses no process boundary; without the rights the window says to run as administrator.
- The stand-alone script `Create-<name>.ps1` (`data/templates/Create-Users.runtime.ps1`) reads the users
  file it was made for next to it (the name in Base64 of its UTF-8, so any name works and the script stays
  ASCII), asks the master password itself and does the same with the cmdlets of
  Windows PowerShell 5.1; `-PasswordsOnly` sets the passwords only, `-UpdateExisting` and `-UserName` as in
  the window.
- The accounts of a users file go into the form Accounts of a profile, with their passwords when the master
  password is given, else without; the form Accounts also makes a users file of its accounts, whose
  passwords wait for a master password before the file can be saved.

## 12. Registry file

- A `.reg` file is read whole: the header (`Windows Registry Editor Version 5.00` or `REGEDIT4`), keys
  (`[path]`) and their deletions (`[-path]`), values of every type with the default value as `@` and their
  deletions (`"name"=-`), comments and blank lines. A value continued with `\` is one line; a line the
  program does not read is kept as it is, so a file opened and saved again loses no line.
- A save writes the lines back with CR LF: UTF-16 LE with its mark for version 5, the text as it is for
  REGEDIT4. A text of several lines is written as `hex(1)`.
- This PC saves `Apply-<profile>.reg` and `Revert-<profile>.reg` for a selection from the plan of a native
  apply or return: the default user profile at `HKEY_USERS\UnattendDefault` with the commands that load its
  hive in the comments, a list that is not additive as `[-key]` before its values, a comment
  `; Rule <id>` before the steps of each rule, and the steps a registry file cannot hold (programs,
  capabilities, apps, the skipped ones) as comments.

## 13. Theme

```json
{
  "format": "winweaver-theme",
  "version": 1,
  "name": "Latte",
  "dark": false,
  "font": "Segoe UI",
  "font_size": 13.0,
  "mono_font": "Consolas",
  "icon_font": "",
  "colors": { "accent": "#8a4b1c", "bg": "#efe4d2", "text": "#3e2c1f" }
}
```

`dark` names the base; a colour the theme leaves out keeps the base. The fonts are families of the fonts
folder of Windows (empty: Segoe UI, Consolas and Segoe UI Emoji), the size from 10 to 20 points. `colors`
holds the colours of the palette by name, and the colours a program adds to the themes by their names. A
file `themes/<id>.json` in the same layout is read too; a `.wwtheme` file of the same id wins.

## 14. MCP configuration

`mcp/<name>.wwmcp`: `format` `winweaver-mcp`, `version`, `transport` (`http` or `stdio`, for a start
without the window), `port` (8765 by default), `token` (the bearer token of HTTP, at least 32 characters;
the window makes one when it is empty), `mode` (`read`, `edit`, `files`), `read_pc`, `registry` (`off`,
`read`, `write`), `edit_library`, `profile` (the preset or profile file the workspace starts with, empty for
the first preset) and `detailed_log`. Every field but the format has a default, so a short file is enough.
The MCP tab saves its file at every change; `winweaver --mcp FILE` serves as the file says.

## 15. Plans, backups and reports of the native apply

- A native apply, return to defaults or undo writes `output/plan-<stamp>.json` (`format`
  `winweaver-apply-plan`, the operations in order, a random nonce so that no two plans are equal), and the
  elevated helper writes `output/backup-<stamp>.json` before the first change and
  `output/plan-<stamp>-report.json` after the last one.
- The backup has the format of the backup of the stand-alone apply script, so `Undo-Apply.ps1` restores a
  backup of the program and the program restores one of the script. It names the changes no undo takes back
  (`account-changed`, `file-deleted`, `capability-removed`, `app-removed`).
- An audit report is the JSON the stand-alone audit script writes on another PC: the result of every check
  (`ok`, `differs`, `unknown`) and the data of the forms. Opened, it is read into a new profile.
