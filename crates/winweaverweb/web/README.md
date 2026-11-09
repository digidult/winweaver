# winweaverweb: WinWeaver in a browser

WinWeaver in a page: every file of WinWeaver opened and saved, the answer file, the stand-alone
scripts and the registry files built, without this PC and without the MCP server, which need
Windows.

## Two ways to open it

- `winweaverweb.html`: the whole edition in one file. Open it with a double click or put it on any
  web server. It has the built-in catalog and the built-in languages.
- `files/`: the page (`index.html`) with its module, for a web server. A page served so also reads
  the files of its folder: put catalog files into `files/imports/`, themes into `files/themes/`,
  translations into `files/lang/<code>/ui.json`, run `python manifest.py files` (it writes
  `files/files.json`, the list the page reads, since a browser cannot list a folder of a server),
  and serve the folder, for example `python -m http.server 8080 --directory files`. A page opened
  from the disk (`file://`) cannot load its module; use `winweaverweb.html` there.

## Files

- **Open** (or a drop on the page) takes a profile, an answer file, a diff, bookmarks, a catalog
  file, a zip archive of policy templates, a users file, a Group Policy file, a registry file or a
  Markdown page. An archive of ADMX templates opens as a catalog: **Save** gives its catalog file,
  which `files/imports/` loads at the next start.
- **Save**, **Save as**, **Build**, the scripts and the registry files of the menu This PC are
  downloads: the browser saves them where it saves downloads.
- The settings, the drafts of unsaved tabs and the bookmarks stay in the storage of the page of
  this browser, as long as its data of the site is kept.

## Not in a browser

This PC (reading it, checking it, applying to it, its accounts), the registry of a PC and the MCP
server need WinWeaver on Windows; their menus are not there, and the native apply is replaced by the
scripts and the registry files to run on the PC itself.
