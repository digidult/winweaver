# WinWeaver guide

WinWeaver prepares unattended installations of Windows 11 Pro for small workgroups and keeps the
installed PCs configured. A profile says which rules of the catalog are on, with which parameters,
and what the installation forms hold; the program builds `autounattend.xml` from it.

This page opens at the first start and later from the menu or with F1. It is a Markdown file; the
documentation pages open the same way, each in its own tab.

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

## The window

- **Tabs**: every open document is a tab: a profile, a diff, this guide, the bookmarks, the MCP
  server, the settings. The menu button at the left end of the tab row holds New, Open, Save,
  Compare, the guide, the MCP server, the bookmarks, Settings and Exit.
- **Command bar**: New, Open, Save, Check, Build, This PC, Compare. In a narrow window it shows icons
  only, in the navigation row.
- **Navigation row**: Back and Forward move through the items you selected; the breadcrumb shows
  where you are and jumps to any level; Bookmark marks the position; Search filters the tree as
  you type and remembers what you searched.
- **Tree and detail panel**: the tree on the left, the selected item on the right. The description
  folds with Ctrl+D. The rules of a selected group show a switch at the end of their rows.
- **Status bar**: the state of the document, the catalog version, the MCP server and the last message.

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

Rules that act only during installation or at the first sign-in of a user are not read.

## Comparing profiles

**Menu, Compare with a file** or **Compare with a preset** opens a diff tab: every difference between
the active profile and the other one, in three columns. "Apply the right side" sets those values in
the profile the diff was made from; "Save as a diff file" keeps the differences as a `.wwdiff`
file that applies later to any profile.

## Opening other files

Open accepts profiles (`.wwprofile`), diffs (`.wwdiff`), bookmark files (`.wwmarks`), Markdown pages,
answer files built by WinWeaver or WinKickOff (the profile they embed), WinKickOff profiles and audit
reports (`.json`).

## Policy templates

**Settings, Policy templates** imports the ADMX templates of this Windows, of a folder or a catalog file;
their policies become rules under Standard policies, and the imports are kept in `imports/`. Only the
checked imports are read and loaded; the others take no memory. A profile can hold options of an import
that is not loaded: they stay in the profile unchanged, are listed under **Unknown options** at the end of
the tree and are left out of the build, with a warning when the profile opens and when the answer file is
built. **Load catalogs** in that group finds the imports that hold them and loads only those. A setting
(General, Window) hides the group.

## The MCP server

**Menu, MCP server** opens the tab of the server through which AI agents read profiles and, when the
mode allows, change them. The server starts in the mode read; edit lets it change profiles in memory,
files lets it save them and write the answer file. Reading this PC needs its own permission, which is
off at every start.

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
| `--profile NAME` | With `--mcp`: a preset or a profile of `profiles/` becomes the current one |
| `--version` | Prints the version |
| `--check-catalog` | Loads the built-in catalog with its translations and prints their problems |
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

`settings.json` (language, theme, recent files, the tabs of the last session), `profiles/` (your
profiles), `output/` (the answer file and the scripts), `drafts/` (unsaved changes, restored at the
next start), `logs/` (what the program did), `bookmarks.wwmarks`. Nothing is written anywhere else.

## Not there yet

Reading the features and capabilities of Windows directly from the program: the audit script reads them
meanwhile.
