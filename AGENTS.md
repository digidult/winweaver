# AGENTS.md: map of the WinWeaver repository

For agents and developers: where things are, what to read first, which rules apply, the state of the work.
Updated with every change of structure, commands or task status. Last update: 14.10.2026 (the first commit:
specification draft 0.1 and four UI mockups).

Repository: https://github.com/digidult/winweaver (private, branch `main`). Commits are authored by
`digidult <271767060+digidult@users.noreply.github.com>` (a local `git config` of this clone, never global).
A commit made by an agent ends with the line `Co-Authored-By: <agent model> <noreply@anthropic.com>`.

## 1. The project in three sentences

WinWeaver is the successor of WinKickOff: a portable editor of Windows 11 installation profiles that builds
`autounattend.xml`, compares profiles, reads, applies and reverts the settings of this PC natively, imports
ADMX policy templates, generates stand-alone scripts and serves an MCP server. It is written in Rust with
eframe (egui + glow + winit) for Windows 10 and 11, one executable with data folders next to it. The data of
the predecessor's rule catalog and its whole functionality carry over; the file formats are new and no code is
ported (clean room specification first, then implementation).

## 2. What to read first

| Task | Start with |
|---|---|
| Understand the scope and the principles | `docs/spec/00-scope.md` |
| Design or change the interface | `docs/spec/01-ui.md`, then `docs/design/README.md` and the mockups |
| File types, formats, what lives next to the executable | `docs/spec/02-documents.md` |
| What the program must do (feature inventory) | `docs/spec/03-features.md` |
| Crates, modules, threads, build | `docs/spec/04-architecture.md` |

## 3. Folder structure

```
(repository root)
├── AGENTS.md              this file
├── README.md              purpose, status, links
├── LICENSE                MIT
├── .gitattributes         LF line endings for every text file
├── .gitignore
└── docs/
    ├── spec/              clean room specification (00 scope, 01 UI, 02 documents, 03 features, 04 architecture)
    └── design/            UI design variants: README.md (comparison) and mockups/ (HTML, a shared CSS)
```

Planned (not yet created): `crates/` (a Cargo workspace), `data/` (the rule catalog, translations, themes,
presets, documentation shipped next to the executable), `.github/workflows/` (CI build on Windows).

## 4. Project rules

1. Language: everything in the repository is English (code, comments, documentation, commit messages, issue
   texts). The interface, the catalog and the user documentation are English sources with Ukrainian and
   Russian translation files; other languages are files added next to the executable.
2. No em dash (U+2014) and no en dash (U+2013) in any text we create, in any language: documents, code,
   data, comments, interface strings, commit messages. Number ranges use a hyphen (08:00-20:00).
3. Clean room: no source code of WinKickOff is copied or translated into this repository. The specification
   describes behaviour; the implementation follows the specification. Only the data of the rule catalog
   (rules, groups, translations, presets) is imported, through a documented conversion.
4. Dependencies: Rust crates only, popular and well known, open source, pure Rust (no extra DLL next to the
   executable, no vendored C library that needs a runtime). The executable and its configuration stay
   portable: nothing is installed, nothing is written outside the program folder and the system temp folder.
5. No PowerShell inside the program: the audit, the apply and the revert of this PC use the Windows API.
   PowerShell appears only in the generated answer file where Windows Setup has no other mechanism, and in
   the stand-alone scripts the program generates for use without it.
6. The customer's work PC is untouchable: only reading there (registry, files, logs). Rust is not installed
   on it and must not be installed by an agent: builds and tests run in CI (GitHub Actions) or in a VM.
   Nothing that changes the system runs on it, not even "temporary" or "reversible" tests.
7. Text files are UTF-8 without a byte order mark with LF line endings (`.gitattributes`); `rustfmt` and
   `clippy` with the workspace settings, no warnings.
8. Untrusted input (profiles, catalog files, policy templates, MCP requests) is read with size, depth and
   character limits and turned into a message, never into a crash.
9. Every feature is developed in its own branch and reaches `main` through a pull request; `git pull
   --ff-only` before starting work, since other people push too.

## 5. Commands

None yet. The Cargo workspace comes with the first implementation milestone.

## 6. Work status

| Area | State | Date |
|---|---|---|
| Specification | Draft 0.1: scope, UI, documents, features, architecture | 14.10.2026 |
| UI design | Four HTML mockups (variants A, B, C and the secondary tabs), comparison and recommendation | 14.10.2026 |
| Implementation | Not started | |

Decisions awaited from the customer: the UI variant to build (`docs/design/README.md`), the file extensions
(`docs/spec/02-documents.md` section 1), the minimum Windows version for the executable (10 1809 or 10 21H2).

## 7. How to update this file

Structure changed: section 3. A command appeared: section 5. A task finished or started: section 6. A
decision was made or a fact that affects future changes was found: the specification and, when it is about
the process, section 4. The date in the header on every change.
