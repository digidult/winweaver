# 02. Documents and files

Draft 0.1, 14.10.2026.

## 1. Document types

| Document | Extension | Content | Tab |
|---|---|---|---|
| Profile | `.wwprofile` | The choices of rules with parameters and the installation forms | Tree and detail panel |
| Diff | `.wwdiff` | The differences between two profiles | Two-sided tree |
| Catalog file | `.wwcatalog` | Imported policy templates (records of policies), exchangeable between PCs | Tree without check boxes |
| Bookmark file | `.wwmarks` | Positions with comments | List |
| MCP configuration | `.wwmcp` | The transport, port, token, mode and switches of an MCP server, its first profile, the detailed log | The MCP tab |
| Markdown page | `.md` | Documentation, the guide, any Markdown file | Rendered page |
| Registry file | `.reg` | The format of regedit (version 5 in UTF-16 with its mark, or REGEDIT4): keys, values and their deletions, comments; every line is kept, the ones the program does not read too. Also saved from This PC for the apply or the return of a selection | Outline of keys and comments, the values of the selected key |
| Answer file | `autounattend.xml` | The build; opened as a profile when it embeds one, imported by its actions otherwise | Opens as a profile |

The extensions are registered nowhere (portability); the program opens a file by its extension and, for an
unknown extension, by its content.

## 2. Common format

- JSON, UTF-8 without a byte order mark, LF, two-space indentation, keys in a fixed order (canonical layout,
  so that diffs in version control are readable).
- Every file starts with the fields `format` (the document type, such as `winweaver-profile`) and
  `version` (an integer). A newer version than the program knows is refused with a message; an older one
  is migrated on load with a warning listed in the message bar.
- Reading is strict: duplicate keys, `NaN`, lone surrogates, nesting deeper than 32 levels, a file larger
  than 16 MB (64 MB for a catalog file, which may be xz or gzip compressed) are errors. A field of a wrong
  type falls back to its default with a warning; an unknown field is kept and written back unchanged
  (forward compatibility between minor versions).
- Writing goes to a temporary file next to the target that is renamed over it, so a failed save leaves the
  old file whole.
- A path to a file inside the program folder (the folder of the executable) is kept relative to it and
  starts with `.`, such as `.\profiles\Office.wwprofile`, so the folder can move or go to another PC with
  its settings, bookmarks, drafts and diffs; a file elsewhere keeps its absolute path. The recent files and
  the open tabs of the settings, the profiles of bookmarks and of diffs, the file of a draft and its key
  follow this rule, and so do the ids of imports made of a folder or a file; in memory every path is
  absolute (`winweaver_core::paths`).

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
    "admx.windows.AllowTelemetry": { "on": true, "params": { "level": 0 }, "source": "system" }
  },
  "unknown": { }
}
```

- An item of `languages.input` is a language tag (`uk-UA`: the language with the layout Windows gives it
  first) or a layout id (`0422:00000422`: the language of that locale id with that layout); the first item
  is the default. The languages and layouts are the ones Windows lists, never a list of the program.
- `rules` holds only the rules whose state differs from "off with default parameters"; a rule not listed
  is off. Parameters are written only when they differ from the defaults.
- `source` of an imported policy records the most trusted kind of import the choice was made in
  (bundled, system, folder, package); a choice from a less trusted kind than the one that now owns the
  policy is held in `unknown` and not applied.
- `unknown` keeps the entries of rules the loaded catalog does not have, unchanged, so that a profile
  survives a round trip through another catalog version or a catalog that is not loaded; they are left
  out of the build with a warning.
- Passwords of accounts are stored in the profile as the user typed them (the answer file needs them);
  the MCP server and the logs redact them.

## 4. Diff

```json
{
  "format": "winweaver-diff",
  "version": 1,
  "base": { "name": "Office", "path": "profiles/office.wwprofile", "modified": "..." },
  "other": { "name": "Strict", "path": "profiles/strict.wwprofile", "modified": "..." },
  "rules": { "asr.usb-untrusted": { "base": { "on": false }, "other": { "on": true } } },
  "install": { "key_mode": { "base": "generic", "other": "ask" } },
  "accounts": { }, "languages": { },
  "comments": { "rule:asr.usb-untrusted": "Strict blocks USB sticks\nask the customer first" }
}
```

A diff applied to a profile sets the `other` side of every entry; the resolver then disables what the new
states make impossible and the message bar lists it. One item can be applied alone. `comments` keeps an
optional comment of several lines by the key of an item: `rule:<id>`, `install:<field>`,
`languages:<field>` or `accounts`.

## 5. Catalog file

An envelope around the records of one import of policy templates: `format` `winweaver-catalog`, `version`,
`kind` (`admx`), `name`, `windows` (the build the templates came from), `created`, `comment`, `templates`
(the records: categories, policies with elements, registry keys, value names and values, texts by
culture). The records are checked on load for safe characters and keys (`03-features.md` section 7).
Plain, gzip or xz; the compression is detected by the first bytes.

## 6. Bookmark file

```json
{
  "format": "winweaver-bookmarks",
  "version": 1,
  "items": [
    { "document": "profile", "path": "profiles/office.wwprofile", "item": "defender.pua", "comment": "Ask the customer about Audit mode\nand note the answer", "created": "..." }
  ]
}
```

A bookmark holds the id of an option (a rule, a group or a form) and, as a link, the profile it was made
in (`path`, empty for an unsaved profile or a catalog tab); the title is taken from the catalog when it
is shown, so an option is one bookmark in every profile. The comment may have several lines. Opening a
bookmark opens its profile (loading the file when no tab has it) and selects the option; without a
profile file it selects the option in the profile tab in front. Files may also hold positions of other
documents with a `title` (`"document": "markdown"`, `"item": "#heading"`); they are read and kept.

## 7. Settings

`settings.json` next to the executable: language, theme, font size, the open tabs and the active one, the
recent files, the search history, the layout (splitter positions, the description panel), the inline
toggles, the draft interval, the MCP defaults and token, the logging level, the imports that are loaded
(`loaded_imports`; an earlier version named the hidden ones, `hidden_imports`, read once) and whether the
unknown options of a profile are hidden. The window is the only writer;
a headless MCP process never writes it. When the folder is read-only the settings are kept in memory.

## 8. Draft

The unsaved state of every modified document is written to `drafts/<hash of the path as it is kept>.json` every 30 s and
on close; at the next start the program offers to restore the drafts it finds. A save or a discard deletes
the draft.

## 9. Presets

The presets Office, Strict, Laptop and Home are profiles embedded in the executable at build time (from
`data/presets/*.wwprofile`, generated from the catalog defaults and the preset definitions). They open
from the New menu as unsaved profiles named after the preset.

## 10. What lives next to the executable

```
WinWeaver/
├── winweaver.exe          the window and the MCP server (stdio and HTTP, also headless with --mcp)
├── settings.json          written by the window
├── bookmarks.wwmarks      created at the first bookmark
├── lang/<code>/           ui.json, catalog.json, docs/*.md (a language is a folder; en is embedded)
├── themes/*.wwtheme       themes: colours and fonts (Light, Dark, Latte and Matrix are embedded)
├── catalogs/*.wwcatalog   catalog files shipped with the program (none until the licence check)
├── imports/<id>/          saved imports of policy templates (created by the program)
├── profiles/              the user's profiles (the default folder of Open and Save)
├── docs/                  the user documentation in English (Markdown, served in the Markdown tabs)
├── drafts/                drafts of unsaved documents
├── mcp/*.wwmcp           MCP configurations; default.wwmcp is the one of the MCP tab
├── logs/                  winweaver-<date>.log (the window), mcp-<date>.jsonl (the MCP server, its crashes)
└── output/                autounattend.xml and the generated scripts
```

Everything embedded (the catalog, English texts, presets, the four themes, the guide) is in the
executable, so a bare `winweaver.exe` runs; the folders add languages, themes, catalogs and persistence.

## Open questions

1. Extensions: `.wwprofile`, `.wwdiff`, `.wwcatalog`, `.wwmarks` as proposed, or shorter (`.wwp`, `.wwd`,
   `.wwc`, `.wwb`)?
2. Should a profile store the passwords at all, or should the answer file be built with the password typed
   at build time (then "Open profile from autounattend.xml" cannot restore it either)?
