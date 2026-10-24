# WinWeaver guide

WinWeaver prepares unattended installations of Windows 10 and 11 and keeps the installed PCs configured.
A profile says which rules of the catalog are on, with which parameters, and what the installation forms
hold; from it the program builds the answer file `autounattend.xml` with its scripts, compares profiles,
reads the settings of a running PC and applies them to it. The catalog is written for small offices
without a domain: security and updates first.

This page opens at the first start and later from the menu or with F1. It is a Markdown file; the
documentation pages open the same way, each in its own tab.

## How it is organized

- **Profiles** (`.wwprofile`) hold your choices; the presets Office, Strict, Laptop and Home are starting
  points.
- **The catalog** is the built-in rules in three parts (Installation, Standard policies, Other) and the
  policy templates (ADMX) you import, which become rules too.
- **Tabs**: every document opens in its own tab: profiles, comparisons, catalogs, bookmarks, the MCP
  server, pages like this one.
- **This PC** reads the running Windows, checks rules against it, applies them with a backup and makes
  scripts for other PCs.
- **The folder of the program** keeps everything next to `winweaver.exe` (see the end of this page);
  nothing is installed.

## First steps

1. Open a preset: **Menu, New, Profile from the preset Office**. Office holds the catalog defaults;
   Strict adds restrictions, Laptop suits portable PCs, Home is an allowlist for a home PC.
2. Walk the tree on the left. A check box turns a rule on or off; a group check box turns its rules
   on or off at once; a greyed rule waits for a rule it needs. The right panel shows the parameters
   of the selected rule and its description.
3. Fill the forms **Installation**, **Accounts** and **Languages** under the first root.
4. Check the profile with F7. The messages appear under the navigation row; a click on a message
   selects the rule or the form it is about.
5. Build the answer file with F9: `autounattend.xml` lands in the folder `output` next to the
   executable, with the scripts it embeds beside it for a look. Copy the file to the root of the
   installation USB stick.
6. Save the profile with Ctrl+S. Profiles are `.wwprofile` files in the folder `profiles` next to
   the executable, or anywhere you choose with Save as.
7. Try the answer file in a virtual machine before any real PC: install, then read that PC with
   **This PC** and compare it with the profile.
8. Keep the installed PCs in line: **This PC** applies the selected rules as administrator with a
   backup, or saves stand-alone scripts that apply, revert, audit and undo on other PCs.

## The name of the computer

In the form **Installation** the computer name is random (as Windows does by default), a fixed name or a
template that the answer file expands on each PC during Setup. A template holds Latin letters, digits and
hyphens around these placeholders:

| Placeholder | What it gives |
|---|---|
| `{serial}`, `{serial:N}` | The serial number of the computer from its firmware, its letters and digits in capitals; `:N` keeps its last N characters, all of them (up to 15) without it |
| `{mac}`, `{mac:N}` | The MAC address of the first physical network adapter, 12 hexadecimal digits in capitals; `:N` keeps its last N digits, all 12 without it |
| `{random}`, `{random:N}` | Random capitals and digits without the look-alikes I, O, 0 and 1; `:N` gives N of them, 6 without it |

N is from 1 to 15. A serial number or an address that cannot be read (there is none, or the maker left a
placeholder such as `Default string`) becomes random characters of the same length; a shorter one stays
whole. The name with the full length of each placeholder fits 15 characters, so `PC-{serial}` is too long
and `PC-{serial:12}` fits; keep a letter in it, since Windows refuses a name of digits alone.
`PC-{random}` gives `PC-` and six random characters, such as `PC-K7Q2MX`; with the serial number
`5CG1234XYZ` and the address `00155D3E5A01`, `WS-{serial:6}` gives `WS-234XYZ` and `{mac:4}-LAB` gives
`5A01-LAB`. Below the field the form shows what the template gives.

## Input languages

The form **Languages** lists the input languages in their order; the first one is the default. Each
language shows its name and a list of keyboard layouts: the layout Windows gives the language first, then
every layout of Windows by its name. **Add** finds a language by a part of its name or tag among all the
languages Windows knows and adds it with the chosen layout; the same language may come twice with two
layouts, such as Ukrainian (Enhanced) and Ukrainian. A language Windows numbers only at the first
sign-in, such as Russian (Ukraine), gets the layout Windows gives it then, and the answer file names the
language it belongs to until that sign-in. The languages and layouts are the ones of the Windows
WinWeaver runs on, by their names in its language.

## Users files

A users file (`.wwusers`) lists the local accounts to make on a PC with their passwords, encrypted under
a master password in the same file. **Menu, New, Users file** starts an empty one, **Users file of the
accounts of this PC** one with every normal account of this PC (its group, state, password expiry and
profile folder; no password). The tab shows the accounts in a list and the selected one in a form: the
name, the full name, the description, the group, the profile folder, and the orders for an account that
already exists on a PC (set the password of the file, enable or disable it, exactly this group); without
them an account that exists is left as it is. **Unlock** asks the master password, or sets it when the
file has none; **Password** sets the password of the selected account; **Lock** forgets them. Without the
master password the passwords cannot be read again. **Check** lists the problems, **Read this PC** adds
the accounts of this PC the file lacks. **Create on this PC** and **Apply passwords** change this PC
after a question and need WinWeaver run as administrator; the result stands under the form, one line an
account. **Script** saves `Create-<name>.ps1` next to the users file: it asks the master password itself
and does the same on another PC from a command prompt opened as administrator (`-PasswordsOnly` sets the
passwords only). In the form **Accounts** of a profile, **From a users file** loads its accounts, with
their passwords when you give the master password.

While the file is unlocked, the form shows the password of the selected account as dots, with **Show**
and **Copy** (the clipboard history of Windows keeps what is copied). In the form **Accounts** of a
profile, **To a users file** makes a users file of its accounts; their passwords wait for a master
password before the first save.

## The window

- **Tabs**: every open document is a tab: a profile, a diff, this guide, the bookmarks, the MCP
  server, the settings. The menu button at the left end of the tab row holds New, Open, Save,
  Compare, the guide, the MCP server, the bookmarks, Settings and Exit.
- **About and License** in the menu: the version with the commit of the build, and the license (MIT),
  built into the program, so it stays with `winweaver.exe` moved alone.
- **Command bar**: New, Open, Save, Check, Build, This PC, Compare. In a narrow window it shows icons
  only, in the navigation row.
- **Navigation row**: Back and Forward move through the items you selected; the breadcrumb shows
  where you are and jumps to any level; Bookmark marks the position; Search filters the tree as
  you type and remembers what you searched.
- **Tree and detail panel**: the tree on the left, the selected item on the right. The description
  folds with Ctrl+D. The rules of a selected group show a switch at the end of their rows.
- **Links**: every rule, group, catalog, profile file and registry key a view names leads to it: the
  rules a rule needs, the ones that need it and the ones it conflicts with; the rules named in the
  messages; the groups above and inside a group; the same policy in other catalogs; the rules that write
  a registry key, in the registry tab and in a registry file. A link in a profile opens its tree down to
  the item; from another tab it shows the rule in the first profile tab.
- **Status bar**: the state of the document, the catalog version, the MCP server and the last message.
- **Open the folder**: after a build, a saved script, registry file or profile, a button at the end of
  the status bar shows the file in its folder in Explorer; Settings, Files opens the folders of the
  program.

## This PC

**This PC** in the command bar reads the settings of this computer through Windows, without changing
anything:

- **Read this PC into a new profile**: every rule that can be judged on a running Windows becomes on
  (in effect), off (partly or not in effect) or keeps its state (not readable, or as on a clean
  Windows); the forms take the edition, the time zone and the languages.
- **Read the selection into this profile** and **Check the selection on this PC** work on the
  selected rule or group.
- **Save an apply script**, **a return-to-defaults script** or **an audit script** writes a PowerShell
  script for this or another PC. Run an apply or return script as administrator, first on a test PC;
  it saves a backup that `Undo-Apply.ps1` next to it restores. The report of an audit script opens
  with **Open an audit report** and becomes a profile.
- **Save a registry file** or **a return-to-defaults registry file** writes the registry values of the
  selection as a `.reg` file for regedit or `reg import`; what a registry file cannot hold (script steps,
  apps, capabilities) stays in it as comments. The values of the default user profile go to
  `HKEY_USERS\UnattendDefault`, and the comment at the top of the file tells how to load its hive there.

Rules that act only during installation or at the first sign-in of a user are not read.

The optional features and the capabilities of Windows are read through DISM when WinWeaver runs as
administrator, which adds a few seconds to the read; without the rights they stay not readable.
The read runs in the background: the window goes on working, and a spinner in the status bar shows it.

Applied as administrator, a rule that removes apps or capabilities of Windows removes them from the window
as well: capabilities through DISM, apps for every user and for the accounts created later. The backup names
them, and Undo does not bring them back.

The script steps of the rules are applied natively too, without PowerShell: .NET Framework 3.5 and the
drivers from the installation media, the start types of services, NetBIOS, the removable drives in the
navigation pane, "password never expires" for the accounts of the profile, the built-in Administrator and
Guest disabled (never the account in use) and the copies of the answer file deleted. Undo does not take
back programs that ran, accounts that changed and deleted files; the confirmation says how many steps of
a plan are such.

## Comparing profiles

**Menu, Compare with a file** or **Compare with a preset** opens a diff tab: every difference between
the active profile and the other one, in three columns. "Apply the right side" sets those values in
the profile the diff was made from; "Save as a diff file" keeps the differences as a `.wwdiff`
file that applies later to any profile.

**Compare, Unsaved changes** opens a diff tab of the profile against its file, or against the preset it
came from: the left side is the profile as it is now, the right side as it was saved.

## Opening other files

Open accepts profiles (`.wwprofile`), diffs (`.wwdiff`), bookmark files (`.wwmarks`), Markdown pages,
answer files with an embedded profile (the profile they embed), older profiles in JSON and audit reports
(`.json`).

A registry file (`.reg`) opens in a tab of its own, as the section on the registry describes.

## Policy templates

**Settings, Policy templates** imports the ADMX templates of this Windows, of a folder or a catalog file;
their policies become rules under Standard policies, and the imports are kept in `imports/`. Only the
checked imports are read and loaded; the others take no memory. A profile can hold options of an import
that is not loaded: they stay in the profile unchanged, are listed under **Unknown options** at the end of
the tree and are left out of the build, with a warning when the profile opens and when the answer file is
built. **Load catalogs** in that group finds the imports that hold them and loads only those. A setting
(General, Window) hides the group.

A zip archive of templates, as Chrome, Edge and Firefox ship them, or of a catalog file is imported with
**Import an archive...** and read without unpacking it.

A policy two imports hold shows in both trees. **Compare** in the bar of a catalog tab sets the catalog
side by side with another loaded import or an open catalog file: the policies only in one of them and
those whose title, description, values or parameters differ. Removing an import, like every removal,
asks first.

The detail panel of an option links to the catalog tabs that hold it and to the same policy in the
trees of other imports; a bookmark of the option does the same. Settings, General choose which parts of
the description open at first.

**Edit** in the bar of a catalog tab opens the catalog in an editor, and **New catalog** in the menu of the
templates starts an empty one of your own. A policy gets its title, its description, the computer or the
default user profile, and the registry values it writes or removes; **Apply** checks the whole file, and
nothing changes when the check fails. Save (Ctrl+S) writes the file and loads it again.

## The registry

**Menu, Registry** opens the registry of this PC in a tab: the keys on the left, the values of the
selected key on the right with their types (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD,
REG_BINARY), the path in the row above. A double click edits a value; New key, New value, Edit and Delete
are in the bar, and a deletion asks first. Keys of the machine (HKEY_LOCAL_MACHINE) change only when
WinWeaver runs as administrator. The button next to a key in **What it writes** opens it here.
**Bookmark** keeps the key and the value with a comment, and the bookmark lists the rules that write it.
**Find** in the bar searches under the selected key as Find of regedit does: in the names of keys, the
names of values and the data, by the whole string only when asked. The search runs in the background,
**Stop** ends it, and a click on a place found shows it.

**Permissions** in the bar shows the owner and the access list of the selected key: the entries it
inherits are shown, its own entries change (allow or deny, full control or read, where they apply), an
account or a group of this PC gets a new entry, and the inheritance from the key above can be turned off.
Apply asks first; a key of the machine changes only with WinWeaver as administrator.

A registry file (`.reg`) opens in a tab of its own, as a structure rather than as text: on the left the
keys and the comments of the file in their order, on the right the selected key with its values in the
types of regedit and the comments between them, a link to the registry tab and the rules of the loaded
catalogs that write the key or the selected value. **New key**, **New value**, **Edit** and **Delete**
are in the bar, and a deletion asks first. **Save** writes every line back: the comments and the lines
the program does not read stay where they were. A file saved from **This PC** names its rules in
comments, and the arrow of such a comment shows the rule in the profile.

## The MCP server

**Menu, MCP server** opens the tab of the server through which AI agents read profiles and, when the mode allows, change them. The tab edits a configuration file, `mcp/default.wwmcp` next to the program, and saves it at every change: the port and the token, the mode (read only reads, edit changes profiles in memory, files also saves them and writes the answer file), the permissions to read this PC, to read or change the registry and to change the bookmarks and catalogs, the profile the server starts with, and the detailed log. **Open** takes another `.wwmcp` file and **Save as** makes a copy; `winweaver --mcp FILE` serves without the window as the file says. The journal of the running server lists every call; **Show the details** opens a panel at its right with the arguments and the answer of the selected call and buttons to the rules, profiles, catalogs and registry keys it names. The log of the server is `logs/mcp-<date>.jsonl`, one JSON object a line; the detailed log, off by default, keeps every request and answer whole without passwords, keys and tokens.

## The renderer

The window draws with OpenGL or with DirectX 12. **Automatic** (Settings, General, Window) tries OpenGL
first, which takes less memory, and switches to DirectX 12 when OpenGL does not start: in a virtual
machine without 3D acceleration, on a fresh installation without the driver of the graphics card, in
some remote sessions. DirectX 12 draws there too, on the software rasterizer of Windows. If the window
draws wrongly with OpenGL (an old driver), choose DirectX 12. A change takes effect at the next start;
`winweaver.exe --renderer directx` (or `opengl`, `auto`) chooses for one start only. The log in `logs/`
names what the window draws with.

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
| `--script KIND FILE` | Writes the stand-alone script of a profile (`.wwprofile`) or a diff (`.wwdiff`) without the window: `apply`, `revert` (back to the defaults of Windows) or `audit`; an apply or a return gets `Undo-Apply.ps1` beside it |
| `--reg KIND FILE` | Writes the registry file of the apply (`apply`) or of the return to the defaults (`revert`) of a profile or a diff |
| `--only IDS` | With `--script` or `--reg`: only these rules or groups, their ids separated by commas |
| `--out FOLDER` | With `--script` or `--reg`: the folder of the files, `output` next to the program by default |
| `--help` | Prints the list of parameters |
| `--smoke-window` | Opens a small window, draws two frames and prints what drew them (also with `--renderer`): a check of the graphics |
| `--elevated-apply PLAN DIGEST` | The window starts itself so to apply a confirmed plan as administrator; not for starting by hand |

The mode and the permission to read this PC are given at every start and never saved. The messages go to
the console the program was started from and to `logs/`.

## Keyboard

| Keys | Action |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | New, Open, Save, Save as |
| F7, F9 | Check the profile, build the answer file |
| Ctrl+F, Esc | Search, clear and close the search |
| Alt+Left, Alt+Right | Back, Forward |
| Ctrl+B | Bookmark the position |
| Ctrl+D | Fold or show the description |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Close the tab, next tab, previous tab |
| Esc | Cancel the confirmation of a change of this PC |
| Ctrl+, | Settings |
| F1 | This guide |

## Files next to the executable

`winweaver.settings.json` (language, theme, recent files, the tabs of the last session), `profiles/` (your
profiles), `output/` (the answer file and the scripts), `drafts/` (unsaved changes, restored at the
next start), `mcp/` (the configurations of the MCP server), `logs/` (what the program did), `bookmarks.wwmarks`. Nothing is written anywhere else.

The settings, bookmarks, drafts and diffs name a file inside this folder relative to it, so the whole
folder can move or go to another PC; a file elsewhere keeps its full path.

## Not there yet

Nothing of the plan for now: what the program needs next comes from its use.
