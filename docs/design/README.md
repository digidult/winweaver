# UI design variants

14.10.2026. Three variants of the main window and one page with the secondary tabs, as static HTML mockups
(open `mockups/*.html` in a browser; they follow the light or dark mode of the system; a few controls react:
the search field expands, the description panel collapses, the sections fold). They illustrate
`../spec/01-ui.md`; the layout is the variable, the behaviour is the same in every variant.

| File | Variant | Shown state |
|---|---|---|
| `mockups/a-command-bar.html` | A, Command bar | A profile tab with a rule selected, the search expanded with its history open |
| `mockups/b-rail.html` | B, Rail | A profile tab with a group selected, the hamburger menu open, the description as a bottom drawer |
| `mockups/c-compact.html` | C, Compact | A diff tab and the profile tree with inline toggles, the description collapsed to a bar |
| `mockups/d-secondary-tabs.html` | A | The MCP tab, the Settings tab, the bookmark tab and a Markdown tab, one under another |

## Variant A: Command bar

The layout of the Windows 11 apps (Notepad, File Explorer): the hamburger button and the document tabs in
the title row, a command bar with icons and labels under it, then the navigation row (back, forward,
breadcrumb, star, search), then the split view. The description panel is the lower part of the detail
panel and folds to its header. Familiar, every function visible, the most vertical space used by chrome
(three rows before the content).

## Variant B: Rail

A vertical rail on the left (as in Windows Settings or the VS Code activity bar) holds the kinds of
documents and functions: Profiles, Diffs, Catalogs, Bookmarks, Docs, MCP, Settings; the hamburger menu at
its bottom. The tabs of the open documents sit across the top of the content; the toolbar is one row of
icons at the right of the navigation row. The description is a drawer under both the tree and the detail
panel, so it can be wide (tables of registry values fit) and collapses to one line. Best when many documents
of several kinds are open; the rail costs 48 px of width and hides the document commands behind icons.

## Variant C: Compact

One header row holds the hamburger button, the tabs and the search icon; the navigation row holds back,
forward, breadcrumb, star and the icon-only toolbar at the right. The tree shows inline toggles for plain
switches, the detail panel is an inspector with the description as folded sections inside it, and the
whole description collapses to a thin bar. The most content on a small screen (1024x600 laptops); less
discoverable, labels only in tips.

## Comparison

| Criterion | A Command bar | B Rail | C Compact |
|---|---|---|---|
| Rows of chrome above the content | 3 | 2 (+ rail) | 2 |
| Document commands visible with labels | yes | icons only | icons only |
| Description panel | in the detail panel, folds | wide drawer, folds | inside the inspector, folds to a bar |
| Many open documents of several kinds | tab row scrolls | rail groups them by kind | tab row scrolls |
| Works at 1024x600 | labels hide | rail stays | yes |
| Keyboard model | tabs, toolbar, nav, tree, detail | rail added to the cycle | tabs, nav, tree, inspector |
| Closest to Windows 11 apps | yes | Settings app | no |
| Effort in egui | medium | medium | low |

## Recommendation

Build variant A with two borrowings: the inline toggles of the tree from C (as a setting, on by default when
a group is selected) and the wide, foldable description drawer of B as an option for the registry tables
(the detail panel keeps the description by default). The command bar with labels matches the Windows 11
look the customer asked for and keeps the functions discoverable for the non-professional users of the
installed PCs who may open the program to read this PC; the compact header of C is the fallback below
1100 px of width (the labels hide, the toolbar moves into the navigation row).
