# 00. Scope and principles

## 1. What WinWeaver is

WinWeaver is a portable Windows desktop program for people who install and maintain Windows 11 Pro on the PCs
of a small workgroup without a domain, where the users are not IT professionals and the organisation is under
constant cyber attack. The priorities are security and updatability; cosmetics and third-party programs are
out of scope.

The program:

1. shows a catalog of installation rules as a searchable tree, grouped by the mechanism that carries them
   (the answer file and its scripts, the registry, the rest) and by topic, and resolves
   dependencies and conflicts between rules automatically;
2. edits profiles: which rules are on, with which parameters, plus the forms of the installation (edition
   and key, computer name, workgroup, disk, accounts, languages, time zone);
3. checks a profile and builds `autounattend.xml` from it, with the profile embedded so that the file opens
   again as a profile;
4. compares a profile with a file, a preset or its own saved state, saves the differences as a diff file
   with comments and applies a diff, or one difference of it, to a profile;
5. reads the settings of this PC into a profile or checks a selection of rules on it, applies a selection
   or returns it to the Windows defaults natively through the Windows API with a backup and an undo, and
   saves the same as stand-alone scripts (Apply, Revert, Audit, Undo) and registry files for a PC without
   the program;
6. imports ADMX policy templates (of this Windows, of a folder, of a zip archive) as catalogs of rules,
   exchanges them as catalog files, compares and edits them;
7. shows and edits the registry of this PC (keys, values, permissions, search) and registry files (`.reg`);
8. keeps users files: the local accounts of a PC with their passwords encrypted, read from this PC,
   created on it natively or through a stand-alone script, and passed to the accounts of a profile;
9. serves an MCP server (stdio and loopback HTTP) through which AI agents read and, when allowed, edit
   profiles, bookmarks, catalog files and the registry, and read this PC;
10. shows its guide (at the first start), the reference of the parameters, its licence and any Markdown
    page inside the window, and keeps bookmarks of options and of registry keys;
11. ships with presets (Office, Strict, Laptop, Home) and in nine languages (English, Ukrainian, Russian,
    German, Spanish, French, Indonesian, Italian and Turkish).

The rule catalog, its translations, the presets, the time zones and the runtime templates (the PowerShell
of the answer file and of the stand-alone scripts in `data/templates/`) are data of the project; the program
code is its own.

The window is built on W#, the framework of supakov/ww (the crates `ww-base` and `ww`, described in
its `docs/ww.md`), a git dependency of its branch `main`. W# also carries the text editor wedit and
the viewer wview, in supakov/wview.

## 2. Non-goals

- Domain join, Intune, Autopilot, provisioning packages.
- Third-party programs, drivers from the internet, cosmetics of the desktop.
- Running on anything but Windows (the program reads and writes the Windows registry).
- An installer: the program is a zip with one executable and data folders.

## 3. Stack

- Rust, stable toolchain, edition 2024 (rust-version 1.88); the Windows target `x86_64-pc-windows-msvc`
  with the C runtime linked statically.
- UI: eframe (egui on winit) through W#, with two renderers: OpenGL through glow and DirectX 12 through
  wgpu with its `dx12` backend alone. Immediate mode, one window, painted widgets of W# where egui has
  none (the tree with check boxes, the lists with columns, the breadcrumb, the tabs, the cards).
- Target systems: Windows 10 and Windows 11, x64, the systems the Rust toolchain supports. The program
  starts on a clean PC without any redistributable: static CRT, no extra DLL. Without a driver of the
  graphics card OpenGL is not available, and DirectX 12 draws on the software rasterizer of Windows
  (`04-architecture.md` section 9).
- The browser edition of wwlite: the same window built for `wasm32-unknown-unknown` with WebGL (glow),
  run by the page `index.html` of its archive from any static web server. It opens files through the
  browser, downloads what it saves and keeps its settings and drafts in the storage of the page
  (`04-architecture.md` section 7, `docs/wasm.md`).

## 4. Principles

- P1 Portability. One executable and its data folders next to it. The catalog, the translations of the
  built-in languages, the presets, the themes, the guide, the reference and the licence are embedded, so a
  bare `winweaver.exe` runs. Settings, bookmarks, profiles, output, imports, drafts, MCP configurations and
  logs are written next to the executable (`02-documents.md` section 10); nothing goes to the registry,
  `%APPDATA%` or the Start menu, and nothing is written outside the program folder and the system temp
  folder. The extensions of the program are registered nowhere. A file that cannot be written gives a
  message, never a crash. The browser edition of wwlite has no folder: its settings and drafts live in
  the storage of the page, and a saved file is a download of the browser.
- P2 No PowerShell in the program. Reading this PC uses the Windows API in the process of the window.
  Applying, returning to defaults and undoing run in an elevated helper process of the same executable,
  also through the Windows API (registry, services, DISM, the package API, the network management API,
  programs of the system folder); the operations of a users file run natively in the window started as
  administrator. PowerShell appears only inside the generated answer file and in the generated stand-alone
  scripts; the program never runs a script.
- P3 Languages. English is the source language of the interface, the catalog and the documentation.
  Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish ship built in. A folder
  `lang/<code>/` next to the executable (`ui.json`, optionally `guide.md`) adds a language of the
  interface or overrides a built-in one; a missing text falls back to English. Every text of the interface
  is written as `tr` or `trf` and has a translation in every built-in language; a test fails otherwise.
- P4 Dependencies. Popular, well known, open source Rust crates that need no extra DLL next to the
  executable and no runtime to install. A new crate is named in `04-architecture.md` with the reason.
- P5 Untrusted input. Profiles, diffs, bookmark files, catalog files, policy templates and their archives,
  registry files, users files, answer files, audit reports, theme files and MCP requests are read with
  limits (size, depth, characters), and every defect becomes a short message, never a crash or a hang of
  the window. Texts that reach the registry, the answer file or a script are checked by the rules of the
  place they go to.
- P6 Keyboard. The tabs, the command bar, the trees, the lists, the forms and the dialogs work with the
  keyboard (`01-ui.md` section 12).
- P7 One window, documents as tabs. The window holds tabs; a tab is a document (a profile, a diff, a
  catalog, the editor of a catalog, the bookmarks, a Markdown page, the registry, a registry file, a users
  file, the editor of a theme) or a function (MCP, settings). Profiles, diffs, catalog files, bookmark
  files, users files, themes and MCP configurations are plain files with their own extensions; presets are
  embedded in the executable.
- P8 Own code. The specification is the source of truth for the behaviour; the program code is written
  from it. The data named in section 1 are data of the project.

## 5. Licence

MIT (`LICENSE` in the root, also built into the executable and shown by Menu, License). The ADMX templates
of Microsoft are not redistributed by the program; the user imports them from their Windows.

## 6. Development process

- English everywhere in the repository. No em dash and no en dash in any text the project creates; number
  ranges use a hyphen. Text files are UTF-8 without a byte order mark, with LF line endings; `rustfmt`
  (width 100) and `clippy -D warnings`.
- Commit messages: an imperative first line up to 72 characters, a body when the why is not obvious.
  Author `digidult`.
- A feature is a branch and a pull request into `main`, which the customer merges. CI runs for a pull
  request that is ready for review, for `main` after a merge and for a tag (`AGENTS.md` section 5).
- The version of the workspace grows with every pull request that reaches `main`: the second number for a
  feature, the third for fixes only. Every build names its commit (`<version>+<12 digits of the commit>`),
  shown by `--version`, About, the logs, the MCP server, the answer file and the scripts. A release has its
  notes in `docs/releases/v<version>.md`.
- The customer's work PC has no Rust toolchain and gets none: builds, tests and any run of the program
  happen in CI or in a VM. On that PC agents only read.
- Documentation changes go in the same pull request as the code they describe: this specification, the
  guide in every language and `package/README.md` for a change the user sees. `AGENTS.md` is the map and
  is updated with every change of structure or status.
