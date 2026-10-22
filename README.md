# WinWeaver

A portable Windows desktop program that prepares unattended installations of Windows 11 Pro for small
workgroups and keeps the installed PCs configured: a searchable catalog of installation rules, profiles,
`autounattend.xml` built from a profile, comparison and diff files, the settings of this PC read, applied
and returned to defaults natively, imported policy templates, stand-alone scripts, and an MCP server for
AI agents.

WinWeaver is the successor of WinKickOff. It carries over the data of the built-in rule catalog and the
whole functionality; the file formats are new and no code is ported (clean room).

Stack: Rust, [eframe](https://github.com/emilk/egui) (egui + glow and wgpu + winit). Runs on Windows 10 and 11,
one executable with its data folders next to it, nothing installed.

The window is built on W# (`ww` in the code, WW in the documents), a small framework of this repository for
programs that analyse and manage a system: a window with document tabs, a command bar, a navigation row,
themes, translations, drafts and the session, with the program bringing its documents and acts
([docs/ww.md](docs/ww.md)). Its second program is `wedit`, a portable text editor with a Markdown viewer,
shipped as `wedit-<version>.zip` beside WinWeaver.

Licence: [MIT](LICENSE).

## Status

Alpha (October 2026): the window builds in CI (`WinWeaver-<version>.zip` in the artifacts of every run; a
tag `v<version>` publishes a release). Profiles, the check, the forms, the build of `autounattend.xml`,
comparison and diff files, the native read of this PC and its native apply, return to defaults and undo
(with a backup the stand-alone scripts understand, the default user profile included), the stand-alone
apply, revert and audit scripts, the registry tab with the permissions of keys, registry files, the
import of policy templates (lists included) and catalog files with their editor,
bookmarks of options, diff files with comments, the guide and the reference, themes with their fonts and
an editor, the MCP server (stdio and loopback HTTP) and the interface in English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish
work; the window has the layout of variant A and is used with the keyboard as well. Builds 0.11.0, 0.16.1 and
0.18.1 were accepted in a virtual machine (the answer file, the stand-alone scripts, the native apply and
undo, This PC, the registry with the permissions of keys, registry files, the catalog editor, the MCP
server). The
specification is the plan; `AGENTS.md` keeps the state.

| What | Where |
|---|---|
| Map of the repository, rules for agents and developers | [AGENTS.md](AGENTS.md) |
| Specification (scope, UI, documents, features, architecture) | [docs/spec/](docs/spec/README.md) |
| W#, the framework of the window, for the people who write a program on it | [docs/ww.md](docs/ww.md) |

## Languages

The development language is English: code, comments, specification, commit messages. The interface, the
rule catalog and the user documentation ship in English, Ukrainian, Russian, German, Spanish, French, Indonesian, Italian and Turkish; another language is added
by dropping its files into the folders next to the executable.
