# WinWeaver

A portable Windows desktop program that prepares unattended installations of Windows 11 Pro for small
workgroups and keeps the installed PCs configured: a searchable catalog of installation rules, profiles,
`autounattend.xml` built from a profile, comparison and diff files, the settings of this PC read, applied
and returned to defaults natively, imported policy templates, stand-alone scripts, and an MCP server for
AI agents.

WinWeaver is the successor of WinKickOff. It carries over the data of the built-in rule catalog and the
whole functionality; the file formats are new and no code is ported (clean room).

Stack: Rust, [eframe](https://github.com/emilk/egui) (egui + glow + winit). Runs on Windows 10 and 11,
one executable with its data folders next to it, nothing installed.

Licence: [MIT](LICENSE).

## Status

Alpha (October 2026): the window builds in CI (`WinWeaver-<version>.zip` in the artifacts of every run; a
tag `v<version>` publishes a release). Profiles, the check, the forms, the build of `autounattend.xml`,
comparison and diff files, the native read of this PC with the stand-alone apply, revert and audit scripts,
bookmarks, the guide, the MCP server (stdio and loopback HTTP) and the interface in English, Ukrainian and
Russian work; the native apply and the import of policy templates come next. Nothing has been accepted in a
virtual machine yet. The specification is the plan; `AGENTS.md` keeps the state.

| What | Where |
|---|---|
| Map of the repository, rules for agents and developers | [AGENTS.md](AGENTS.md) |
| Specification (scope, UI, documents, features, architecture) | [docs/spec/](docs/spec/README.md) |
| UI design variants (HTML mockups, open in a browser) | [docs/design/](docs/design/README.md) |

## Languages

The development language is English: code, comments, specification, commit messages. The interface, the
rule catalog and the user documentation ship in English, Ukrainian and Russian; another language is added
by dropping its files into the folders next to the executable.
