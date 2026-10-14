# WinWeaver specification

Clean room specification of WinWeaver, draft 0.1 of 14.10.2026. It describes what the program does and how
it looks, written from the requirements and from the behaviour the predecessor showed to its users, not from
its code.

| File | Content |
|---|---|
| [00-scope.md](00-scope.md) | Purpose, non-goals, stack, principles, licence, process |
| [01-ui.md](01-ui.md) | The window: tabs, toolbar, breadcrumb and navigation, search, menu, views, description panel, documentation, bookmarks, MCP tab, keyboard |
| [02-documents.md](02-documents.md) | Document types and files: profiles, diffs, catalogs, bookmarks, Markdown, settings, presets, what lives next to the executable |
| [03-features.md](03-features.md) | Feature inventory: everything carried over from the predecessor and what is new |
| [04-architecture.md](04-architecture.md) | Crates, modules, threads, Windows API use, elevation, build and CI, testing |

The UI design variants that illustrate `01-ui.md` are in [../design/](../design/README.md).

Open questions to the customer are collected at the end of each file under "Open questions".
