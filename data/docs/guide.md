# WinWeaver guide

WinWeaver prepares unattended installations of Windows 11 Pro for small workgroups and keeps the
installed PCs configured. A profile says which rules of the catalog are on, with which parameters,
and what the installation forms hold; the program builds `autounattend.xml` from it.

This page opens at the first start and later from the menu or with F1. It is a Markdown file; the
documentation pages open the same way, each in its own tab.

## First steps

1. Open a preset: **Menu, New, Profile from the preset Office**. Office holds the catalog defaults;
   Strict adds restrictions, Laptop suits portable PCs, Home is an allowlist for a home PC.
2. Walk the tree on the left. A check box turns a rule on or off; a group check box turns its rules
   on or off at once; a greyed rule waits for a rule it needs. The right panel shows the parameters
   of the selected rule and its description.
3. Fill the forms **Installation**, **Accounts** and **Languages** under the first root.
4. Check the profile with F7. The messages appear under the navigation row; a click on a message
   selects the rule or the form it is about.
5. Save the profile with Ctrl+S. Profiles are `.wwprofile` files in the folder `profiles` next to
   the executable, or anywhere you choose with Save as.

## The window

- **Tabs**: every open document is a tab: a profile, a diff, this guide, the settings. The menu
  button at the left end of the tab row holds New, Open, Save, Compare, Settings and Exit.
- **Command bar**: New, Open, Save, Check, Build, This PC, Compare.
- **Navigation row**: Back and Forward move through the items you selected; the breadcrumb shows
  where you are and jumps to any level; Bookmark marks the position; Search filters the tree as
  you type and remembers what you searched.
- **Tree and detail panel**: the tree on the left, the selected item on the right. The description
  folds with Ctrl+D.
- **Status bar**: the state of the document, the catalog version and the last message.

## Comparing profiles

**Menu, Compare with a file** or **Compare with a preset** opens a diff tab: every difference between
the active profile and the other one, in three columns. "Apply the right side" sets those values in
the profile the diff was made from; "Save as a diff file" keeps the differences as a `.wwdiff`
file that applies later to any profile.

## Keyboard

| Keys | Action |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | New, Open, Save, Save as |
| F7 | Check the profile |
| Ctrl+F, Esc | Search, clear and close the search |
| Alt+Left, Alt+Right | Back, Forward |
| Ctrl+B | Bookmark the position |
| Ctrl+D | Fold or show the description |
| Ctrl+W, Ctrl+Tab | Close the tab, next tab |
| Ctrl+, | Settings |
| F1 | This guide |

## Files next to the executable

`settings.json` (language, theme, recent files, search history), `profiles/` (your profiles),
`output/` (the built answer file and scripts). Nothing is written anywhere else.

## Not there yet

This build is an early one. Build, This PC (read, apply, return to defaults), imported policy
templates, the MCP server, bookmark files and the translations of the interface come with the next
milestones; the commands say so when you use them.
