# WinWeaver

WinWeaver prepares unattended installations of Windows 10 and 11. You choose the settings of a computer in a profile,
and the program builds the answer file `autounattend.xml` with its scripts: put it on the installation USB stick, and
Windows installs and configures itself without questions. The same profile can be compared with others, checked
against a running PC and applied to it. The rule catalog is written for small offices without a domain: security and
updates first.

## Start

Windows 10 or 11, 64-bit. Nothing is installed: unpack the archive into a folder and run `winweaver.exe`. The program
keeps its settings and files in that folder. The guide opens at the first start and with F1.

## What it does

- **Profiles** (`.wwprofile`): a tree of about 280 rules in three parts (Installation: the answer file and its
  scripts; Policies: every rule that only writes the registry; Other: services, features, apps and scripts), each
  with its description, risk, what it writes and its parameters, and the forms of the installation: edition and
  product key, computer name, workgroup, disk, accounts, languages, time zone. Presets: Office, Strict, Laptop, Home.
- **Check and build**: Check (F7) lists errors and warnings; Build (F9) writes `autounattend.xml` and its scripts into
  `output`.
- **Compare** two profiles (`.wwdiff`): the differences side by side with a comment on each; apply one or all.
- **This PC**: read its settings into a profile (the features and capabilities of Windows as administrator) and see
  what is in effect; check the selected rules; apply them or
  return them to the Windows defaults as administrator with a backup of every value, and undo a change from its backup.
  The script steps of the rules (.NET 3.5 and drivers from the media, services, accounts) run natively too.
  For other PCs it saves stand-alone scripts that apply, revert, audit and undo; their audit report opens as a profile.
  The registry values of an apply or a return also go to a registry file (`.reg`) for regedit.
- **Policy templates**: import the ADMX templates of this Windows, of a folder, of a zip archive (as Chrome, Edge and
  Firefox ship them, read without unpacking) or a catalog file, also zipped; their policies
  become rules. Only the imports checked in the settings are loaded. Options of a profile whose catalog is not
  loaded stay in the profile under Unknown options and are left out of the build; Load catalogs finds and loads
  their imports. Compare sets two catalogs side by side; Edit opens a catalog in an editor, New catalog starts
  one of your own. An `.admx` or `.wwcatalog` file also opens as a tab without importing.
- **Group Policy files** (`Registry.pol`): the files of this Windows or any `.pol` file in a tab, their entries
  edited, the root chosen for a file outside Group Policy, the entries of another `.pol` or `.reg` file added, the
  file saved as `.reg`. **Catalogs compared** in a tab, their report saved as Markdown; policies and groups copied
  into a catalog of your own. **Only on** in the bar of a profile shows the rules that are on; the work in the
  background shows in a panel of its own.
- **Registry** of this PC in a tab: keys, values of every type of regedit, edits and deletions that ask
  first, the permissions of keys, bookmarks of keys; every list of what a rule writes opens its key there. A registry file (`.reg`) opens
  as a tab of its keys, values and comments, with the rules that write each key, and is saved without losing a line.
- **Links** to everything a view names: the rules a rule needs, the ones that need it or conflict with it, the rules
  named in the messages, the groups above and inside a group, the same policy in other catalogs, the rules that write
  a registry key; a click shows the item where it lives.
- **Bookmarks** of rules with comments (`.wwmarks`), **themes** with their colours and fonts (`.wwtheme`) and an editor.
- **Languages**: English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish built in,
  Belarusian as a file, more as files (below); "Follow Windows" takes the language of Windows when it is one of them.
- **MCP server** for AI agents: they read and, when you allow it, change profiles. Its settings are a file of
  `mcp/` (`default.wwmcp`); the journal shows every call with its details. See `agents/README.md`.
- **Users files** (`.wwusers`): the local accounts to make on a PC (name, group, state, profile folder, what to do
  with an account that exists) with their passwords encrypted under a master password in the same file. Read
  the accounts of this PC into one, create the accounts on this PC or set their passwords as administrator,
  save a stand-alone script that does the same on another PC, or load the accounts into a profile.

Every new answer file goes to a virtual machine first, never straight to working computers.

## The way of work

1. Start from a preset (Menu, New) or read this PC into a profile (This PC).
2. Adjust the rules, their parameters and the forms of the installation; import policy templates when you need
   more rules.
3. Check (F7) and build (F9): `output/autounattend.xml` goes to the root of the installation USB stick.
4. Install in a virtual machine first, then read that PC and compare it with the profile.
5. Keep the installed PCs in line with This PC (applied as administrator with a backup) or with the stand-alone
   scripts for other PCs.

The guide (F1) tells the same in more detail and in the language of the program.

## Editions and keys

The edition form offers these editions with the generic key of each, "Ask during Setup" or a key of your own. A
generic key selects the edition during the installation; it does not activate Windows.

| Edition | Generic key |
|---|---|
| Pro | VK7JG-NPHTM-C97JM-9MPGT-3V66T |
| Pro N | MH37W-N47XK-V7XM9-C7227-GCQG9 |
| Pro for Workstations | NRG8B-VKK3Q-CXVCJ-9G2XF-6Q84J |
| Pro for Workstations N | 9FNHH-K3HBT-3W4TD-6383H-6XYWF |
| Pro Education | 6TP4R-GNPTD-KYYHQ-7B7DP-J447Y |
| Pro Education N | YVWGF-BXNMC-HTQYQ-CPQ99-66QFC |
| Education | YNMGQ-8RYV3-4PGQ3-C8XTP-7CFBY |
| Education N | 2WH4N-8QGBV-H22JP-CT43Q-MDWWJ |
| Enterprise | NPPR9-FWDCX-D2C8J-H872K-2YT43 |
| Enterprise N | DPH2V-TTNVB-4X9Q3-TJR4H-KHJW4 |
| Enterprise G | YYVX9-NTFWV-6MDM3-9PT4T-4M68B |
| Enterprise G N | 44RPN-FTY23-9VTTB-MP9BX-T84FV |
| Enterprise LTSC 2024 / 2021 / 2019 | M7XTQ-FN8P6-TTKYV-9D4CC-J462D |
| Enterprise N LTSC | 92NFX-8DJQP-P6BBQ-THF9C-7CG2H |

## The folder of the program

| Item | What it is |
|---|---|
| `winweaver.exe` | The program |
| `profiles/` | Your profiles; the presets are there as files too |
| `lang/be/` | The Belarusian translation, the example of a language |
| `agents/` | Instructions, skills and a container for AI agents |
| `winweaver.settings.json`, `bookmarks.wwmarks` | Created when needed; the settings are named after the executable, so a renamed copy keeps its own, and a `settings.json` of an earlier version is read and kept |
| `output/` | Answer files, scripts, plans and backups of changes of this PC |
| `imports/` | Imported policy templates |
| `themes/` | Your themes (`<name>.wwtheme`) |
| `drafts/`, `logs/` | Unsaved changes; the logs of each day: `winweaver-<date>.log` of the window, `mcp-<date>.jsonl` of the MCP server (one JSON object a line), with the reason when the program fails |

The settings, bookmarks, drafts and diffs name a file inside this folder relative to it, so the whole folder can
move or go to another PC; a file elsewhere keeps its full path.

## Keys

F1 guide, F7 check, F9 build, Ctrl+N new profile, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+, settings,
Ctrl+F search in the tab, Ctrl+B bookmark,
Ctrl+D description, Ctrl+W close the tab, Ctrl+Tab next tab, Ctrl+Shift+PgUp and PgDn move the tab, Alt+Left and
Alt+Right back and forward. In the tree and the lists: the arrows, Home, End, Page Up and Down, Space to switch, Enter
to open.

## Renderer

The window draws with OpenGL or with DirectX 12. Automatic (Settings, General, Window) tries OpenGL first, which takes
less memory, and switches to DirectX 12 when OpenGL does not start: a virtual machine without 3D acceleration, a fresh
Windows without the driver of the graphics card, some remote sessions. DirectX 12 draws there on the software
rasterizer of Windows. If the window draws wrongly with OpenGL (an old driver), choose DirectX 12; a change takes
effect at the next start. For one start only: `winweaver.exe --renderer directx` (or `opengl`, `auto`). The log
`logs/winweaver-<date>.log` names what the window draws with.

## Command line

`winweaver.exe` without parameters opens the window. A value follows its parameter after a space or `=`:
`--renderer=directx`.

| Parameters | What they do |
|---|---|
| `--renderer NAME` | Opens the window and tries the renderer NAME first, for this start only: `auto`, `opengl` or `directx` |
| `--mcp stdio` | Serves MCP on the standard input and output, without the window; an MCP client starts it so |
| `--mcp http --token TOKEN` | Serves MCP on `http://127.0.0.1:8765/mcp` without the window; the token has at least 32 characters |
| `--port N` | With `--mcp http`: another port |
| `--mode NAME` | With `--mcp`: `read` (the default) only reads, `edit` changes profiles in memory, `files` also saves them and writes the answer file |
| `--read-pc` | With `--mcp`: allows reading the settings of this PC |
| `--registry NAME` | With `--mcp`: `off` (the default), `read` lets the tools read the registry, `write` change it too |
| `--edit-library` | With `--mcp`: the tools may change the bookmarks and the catalog files; reading them is always allowed |
| `--profile NAME` | With `--mcp`: a preset or a profile of `profiles/` becomes the current one |
| `--mcp FILE` | Serves MCP without the window as a configuration file says: a path to a `.wwmcp` file, or a name of the folder `mcp` next to the program |
| `--config FILE` | With `--mcp stdio` or `--mcp http`: the settings of a configuration file, the token too; the other options change them for this start |
| `--detailed-log` | With `--mcp`: the log keeps every request and answer whole, without passwords, keys and tokens |
| `--version` | Prints the version |
| `--check-catalog` | Loads the built-in catalog with its translations and prints their problems |
| `--build PROFILE` | Checks a profile and builds its answer file without the window: `autounattend.xml` in `output`, with the scripts it embeds beside it; PROFILE is a `.wwprofile` file, a preset (`Office`, `Strict`, `Laptop`, `Home`) or the name of a profile of `profiles/`; an error of the check stops the build |
| `--import-admx SOURCE` | Imports policy templates as Settings, Policy templates does: `system` (the templates of this Windows), a folder of ADMX files, an `.admx` file with the ADML of its culture folders, a `.zip` archive or a `.wwcatalog` file; the catalog file goes into `imports/`, and the window and the MCP server load it at their next start (run it while the window is closed); a catalog whose rules run code of their own needs `--code allow` |
| `--export-catalog SOURCE` | Writes a catalog file (`.wwcatalog`) as Export in Settings, Policy templates does: of an import by its id (the name of its file in `imports/`), or of a source of `--import-admx` read without importing it; `<id>.wwcatalog` in `output` |
| `--read-pc` | Without `--mcp`: reads this PC into a new profile named after the computer, as This PC, Read this PC does, and saves it in `profiles/`; with an `.xml` file in `--out` it writes the answer file of that profile instead; the features and capabilities of Windows are read only when the program runs as administrator |
| `--base PROFILE` | With `--read-pc`: the profile the read starts from, given as for `--build`; the preset Office by default |
| `--diff A B` | Writes the differences from the profile A to the profile B as a diff file, `A-vs-B.wwdiff` in `profiles/`; A and B are given as for `--build` |
| `--script KIND FILE` | Writes the stand-alone script of a profile (`.wwprofile`) or a diff (`.wwdiff`) without the window: `apply`, `revert` (back to the defaults of Windows) or `audit`; an apply or a return gets `Undo-Apply.ps1` beside it |
| `--reg KIND FILE` | Writes the registry file of the apply (`apply`) or of the return to the defaults (`revert`) of a profile or a diff |
| `--only IDS` | With `--script` or `--reg`: only these rules or groups, their ids separated by commas |
| `--out FILE` | With `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` or `--reg`: the file to write when the value ends with the extension the command writes (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`); any other value is a folder, made when missing, for the file of its usual name; `Undo-Apply.ps1` and the scripts of the answer file go beside the file; without `--out` the files go to `output` next to the program (`profiles/` for `--read-pc` and `--diff`) |
| `--help` | Prints the list of parameters |
| `--smoke-window` | Opens a small window, draws two frames and prints what drew them (also with `--renderer`): a check of the graphics |
| `--elevated-apply PLAN DIGEST` | The window starts itself so to apply a confirmed plan as administrator; not for starting by hand |

A command that writes files names every file it writes and ends with the code 0; a wrong use of the parameters
ends with the code 2, a failure with 1, and the reason is printed.

The mode and the permission to read this PC are given at every start and never saved. The messages go to
the console the program was started from and to `logs/`.

## Add a language

1. Make a folder `lang/<code>` next to `winweaver.exe`, with the code of the language: `be`, `pl`, `de`.
2. Put `ui.json` there: a JSON object in UTF-8 whose keys are the English texts of the interface and whose values are
   their translations. `_language` is the name of the language in itself. A text left out stays English.
3. Optionally `guide.md`: the guide in that language.
4. Restart the program and choose the language in Settings, General.

`lang/be/ui.json` holds every text of the interface: copy it as the start of a new language and translate the values.
Its beginning:

```json
{
  "_language": "Беларуская",
  "Bookmarks": "Закладкі",
  "Settings": "Налады",
  "Open failed: {e}": "Не ўдалося адкрыць: {e}",
  "Catalog {version}, {n} rules": "Каталог {version}, правіл: {n}"
}
```

Keep the words in braces (`{e}`, `{n}`) and the keys in brackets (`(Ctrl+F)`) as they are. The titles and descriptions
of the rules are built in for the built-in languages; another language shows them in English.

## License

MIT. The text is built into `winweaver.exe`: Menu, License; it stays with the program when only the executable
is moved.
