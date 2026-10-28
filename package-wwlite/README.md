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
| `imports/` | Made by you when wanted: catalog files `.wwcatalog`, every one read at the start, whose rules join the tree and the answer file |
| `themes/` | Made by you when wanted: theme files `.wwtheme` to choose in Settings |

The program writes next to itself: `wwlite.settings.json`, `drafts/` (unsaved profiles) and the
answer files you build where you choose.

## First steps

1. Run `wwlite.exe`. The guide opens at the first start (Menu, Guide or F1 later).
2. Menu, New, Profile from the preset Office.
3. Check the rules in the tree and fill the forms Installation, Accounts and Languages.
4. Check (F7), then Build (F9): `autounattend.xml` is written where you choose.
5. Copy `autounattend.xml` to the root of the installation USB stick.

## Fonts

Settings, General: the font of the text, of the code and the size, kept in a theme file in `themes/`.

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

## The browser edition

`wwlite-web-<version>.zip` is the same program for a browser: `index.html`, `wwlite.js` and
`wwlite_bg.wasm`. Put the folder on any web server and open `index.html` over http (a page opened
from `file://` cannot load the module). It runs in a current Chromium, Firefox or Safari with WebGL.

What it does: opens a profile, an answer file with the profile it embeds or an older profile through
the file dialog of the browser (Menu, Open or the button Open) or by a drop of the file on the page;
edits, checks and builds as on Windows; Save and Save as download the profile, Build downloads
`autounattend.xml` (the scripts are embedded in it, as always). The settings, the unsaved drafts and
the fonts you choose stay in the storage of the page of that browser.

What it does not: no scripts beside the answer file (Windows alone writes them), no catalogs from a
folder `imports`, no themes from a folder `themes`, no languages from a folder `lang` (English and
Ukrainian are built in), no recent files, no tabs of the last session, no log file (the console of the
browser shows it). The names of the input languages are in English, from a list of Windows 11 built
in. Ctrl+N, Ctrl+W and Ctrl+Tab belong to the browser: use the menu and the buttons instead.
