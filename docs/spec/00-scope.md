# 00. Scope and principles

Draft 0.1, 14.10.2026.

## 1. What WinWeaver is

WinWeaver is a portable Windows desktop program for people who install and maintain Windows 11 Pro on the PCs
of a small workgroup without a domain, where the users are not IT professionals and the organisation is under
constant cyber attack. The priorities are security and updatability; cosmetics and third-party programs are
out of scope.

The program:

1. shows a catalog of installation rules as a searchable tree, grouped by the mechanism that carries them
   (answer file settings, policies, other registry values and scripts) and by topic, and resolves
   dependencies and conflicts between rules automatically;
2. edits profiles: which rules are on, with which parameters, plus the forms of the installation (edition
   and key, computer name, workgroup, disk, accounts, languages, time zone);
3. checks a profile and builds `autounattend.xml` from it, with the profile embedded so that the file opens
   again as a profile; imports older answer files by their actions;
4. compares two profiles, saves the differences as a diff file, merges a diff into a profile;
5. reads the settings of this PC into a profile, applies a profile or a selection of rules to this PC and
   returns settings to Windows defaults, all natively through the Windows API, with a backup and an undo;
6. imports ADMX policy templates (of this Windows or of a folder) as rules, with a trust order between
   imports, and exchanges them as catalog files;
7. generates stand-alone scripts (Apply, Undo, Audit) for use on a PC without the program;
8. serves an MCP server (stdio and loopback HTTP) through which AI agents read and, when allowed, edit
   profiles and read this PC;
9. shows its own documentation, a guide at the first start and a catalog of bookmarks inside the window;
10. ships with presets (Office, Strict, Laptop, Home) and in nine languages (English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish).

The predecessor is WinKickOff (Python 3.14, tkinter). WinWeaver carries over the data of its built-in rule
catalog and its whole functionality. The file formats are new and need no compatibility. No code is ported.

## 2. Non-goals

- Domain join, Intune, Autopilot, provisioning packages.
- Third-party programs, drivers from the internet, cosmetics of the desktop.
- Running on anything but Windows (the program reads and writes the Windows registry).
- An installer: the program is a zip with one executable and data folders.
- Compatibility with the file formats of the predecessor.

## 3. Stack

- Rust, stable toolchain, edition 2024, x64 Windows target with the MSVC toolchain in CI (`x86_64-pc-windows-msvc`).
- UI: eframe (egui with the glow renderer on winit). Immediate mode, one window, our own widgets where egui
  has none (tree with check boxes, breadcrumb, tabs with close buttons).
- Target systems: Windows 10 (21H2 or later) and Windows 11. The program must start on a clean Windows 10 PC
  without any redistributable: static CRT, no extra DLL (the OpenGL driver of Windows is enough for glow;
  a software fallback is a known limitation, see `04-architecture.md`).

## 4. Principles

- P1 Portability. One executable and its data folders next to it (`lang/`, `themes/`, `catalogs/`, `docs/`,
  `profiles/`). Settings, logs, drafts and output are written next to the executable; nothing goes to the
  registry, `%APPDATA%` or the Start menu. When the folder is read-only, the program says so and runs without
  saving settings.
- P2 No PowerShell in the program. Reading, applying and reverting settings of this PC use the Windows API
  (registry, services, scheduled tasks, DISM where needed) from an elevated helper process of the same
  executable. PowerShell survives only inside the generated answer file where Windows Setup has no other
  mechanism (the embedded scripts of the passes), and in the stand-alone scripts generated for use without
  the program.
- P3 Languages. English is the source language of the interface, the catalog and the documentation.
  Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish ship built in. A language is a set of files in `lang/<code>/` next to the
  executable; a missing text falls back to English; a new language is added by adding its folder.
- P4 Dependencies. Popular, well known, open source Rust crates; pure Rust, no extra DLL, no runtime to
  install. A new crate is named in `04-architecture.md` with the reason.
- P5 Untrusted input. Profiles, diffs, catalog files, policy templates, answer files and MCP requests are
  read with limits (size, depth, characters) and every defect becomes a short message, never a crash or a
  hang of the window. Texts that reach the registry, the answer file or a script are checked by the rules
  of the place they go to.
- P6 Keyboard. Every function is reachable without a mouse: tabs, tree, search, menu, forms and dialogs
  have focus order and shortcuts (`01-ui.md` section 12).
- P7 One window, documents as tabs. The window holds tabs; a tab is a document (a profile, a diff, a
  catalog, a bookmark file, a Markdown page) or a function (MCP, settings). Profiles, diffs, catalogs and
  bookmark files are plain files with their own extensions; presets are embedded in the executable.
- P8 Clean room. The specification is the source of truth; the implementation is written from it.

## 5. Licence

MIT (`LICENSE` in the root). Imported ADMX templates of Microsoft are not redistributed by the program;
the user imports them from their Windows.

## 6. Development process

- English everywhere in the repository. Commit messages: an imperative first line up to 72 characters, a
  body when the why is not obvious. Author `digidult`.
- A feature is a branch and a pull request into `main`; CI builds and tests every pull request.
- The customer's work PC has no Rust toolchain and gets none: builds, tests and any run of the program
  happen in CI or in a VM. On that PC agents only read.
- Documentation changes go in the same pull request as the code they describe; `AGENTS.md` is the map and
  is updated with every change of structure or status.

## Decisions

- 14.10.2026: the supported systems are Windows 10 and 11, the systems the Rust toolchain supports;
  among the Windows desktops in use online these are almost all (the customer left the choice to the
  agent, guided by the versions in use).
- 14.10.2026: the runtime templates of the predecessor (the PowerShell of the answer file and of the
  stand-alone scripts) are carried over as data in `data/templates/`.

## Open questions

1. Should the executable be signed (a code signing certificate is needed) or is SmartScreen's warning
   acceptable for an internal tool?
