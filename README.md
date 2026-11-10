# WinWeaver

A portable Windows desktop program that prepares unattended installations of Windows 11 Pro for small
workgroups and keeps the installed PCs configured: a searchable catalog of installation rules, profiles,
`autounattend.xml` built from a profile, comparison and diff files, the settings of this PC read, applied
and returned to defaults natively, imported policy templates, stand-alone scripts, and an MCP server for
AI agents.

Stack: Rust, [eframe](https://github.com/emilk/egui) (egui + glow and wgpu + winit). Runs on Windows 10 and 11,
one executable with its data folders next to it, nothing installed.

The window is built on W# (`ww` in the code, WW in the documents), a small framework for programs
that analyse and manage a system: a window with document tabs, a command bar, a navigation row, themes,
translations, drafts and the session, with the program bringing its documents and acts. W# and the
system of its plugins live in [supakov/ww](https://github.com/supakov/ww), a git dependency of this
repository. The light edition `wwlite` (`wwlite-<version>.zip`)
keeps the profile editor and the build of `autounattend.xml` alone, and runs in a browser too
(`wwlite-web-<version>.zip`, a page for any static web server).

Licence: [MIT](LICENSE).

## Status

Alpha: the window builds in CI (`WinWeaver-<version>.zip` and `wwlite-<version>.zip` in the artifacts
of every run; a tag `v<version>` publishes a release). On a computer of Windows with Rust, `build-all.cmd`
makes the same archives in `dist/` (`AGENTS.md`, section 5). Profiles, the check, the forms, the build of
`autounattend.xml`, comparison and diff files, the native read of this PC and its native apply, return to
defaults and undo (with a backup the stand-alone scripts understand, the default user profile included),
the stand-alone apply, revert and audit scripts, the registry tab with the permissions of keys, registry
files, users files, the import of policy templates and catalog files with their editor, bookmarks of
options, diff files with comments, the guide and the reference, themes with their fonts and icons, the MCP
server (stdio and loopback HTTP) and the interface in English, Ukrainian, Russian, German, Spanish, French,
Indonesian, Italian and Turkish work, with the keyboard as well. The builds are accepted in a virtual
machine by the steps of an acceptance page. The specification describes the program; `AGENTS.md` keeps the
state of the work.

| What | Where |
|---|---|
| Map of the repository, rules for agents and developers | [AGENTS.md](AGENTS.md) |
| Specification (scope, UI, documents, features, architecture) | [docs/spec/](docs/spec/README.md) |
| W#, the framework of the window, for the people who write a program on it | `docs/ww.md` of [supakov/ww](https://github.com/supakov/ww) |

## Languages

The development language is English: code, comments, specification, commit messages. The interface, the
rule catalog and the user documentation ship in English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish; another language is added
by dropping its files into the folders next to the executable.
