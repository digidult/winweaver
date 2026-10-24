# wwlite

wwlite is the light edition of WinWeaver: it edits profiles of Windows 11 installation rules and
builds the answer file `autounattend.xml` from them. It reads nothing from this PC and changes
nothing on it: no apply, no scripts, no registry, no MCP server.

Nothing is installed: unzip the archive anywhere and run `wwlite.exe` on Windows 10 or 11.

## What is in the folder

| Path | What it is |
|---|---|
| `wwlite.exe` | The program |
| `profiles/` | The presets Office, Strict, Laptop and Home as profiles to open and change |
| `lang/de/` | German: `ui.json` (the interface) and `guide-lite.txt` (the guide) |
| `imports/` | Made by you when wanted: catalog files `.wwcatalog` whose rules join the tree and the answer file |
| `themes/` | Made by you when wanted: theme files `.wwtheme` to choose in Settings |

The program writes next to itself: `wwlite.settings.json`, `drafts/` (unsaved profiles) and the
answer files you build where you choose.

## First steps

1. Run `wwlite.exe`. The guide opens at the first start (Menu, Guide or F1 later).
2. Menu, New, Profile from the preset Office.
3. Check the rules in the tree and fill the forms Installation, Accounts and Languages.
4. Check (F7), then Build (F9): `autounattend.xml` is written where you choose.
5. Copy `autounattend.xml` to the root of the installation USB stick.

## Languages

English and Ukrainian are built in. Another language is a folder `lang/<code>` with its `ui.json`
and, if wanted, `guide-lite.txt`; German comes this way. Choose it in Settings, General.

## Command line

| Parameters | What they do |
|---|---|
| `--renderer NAME` | Opens the window and tries the renderer NAME first: `auto`, `opengl` or `directx` |
| `--smoke-window` | Opens a small window, draws two frames and prints what drew them |
| `--version` | Prints the version |
| `--help` | Prints the list of parameters |
