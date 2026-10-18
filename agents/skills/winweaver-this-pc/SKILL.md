---
name: winweaver-this-pc
description: Read the settings of the PC that runs WinWeaver and compare them with a profile, for example after an installation. Use when asked what is in effect on this computer or why it differs from a profile.
---

# Read this PC

1. `get_status`: `read_pc` must be true (the person allows it in the MCP tab: "Allow reading the settings of this PC";
   off at every start) and the mode at least edit. Otherwise say what to switch and wait.
2. Ask before the read. `load_profile` the profile to compare with first: the read takes the current profile as its
   base.
3. `read_this_pc`: reads only, changes nothing and makes a new workspace profile named after the computer. The result
   has `summary` (counts by verdict), `forms`, `notes`, `partly` and `not_read`.
4. `compare_profiles` {a: <the profile>, b: <the read profile>}.

## Reading the result

- in effect: every value matches. Not in effect: none does. Partly in effect (read as off): some values differ;
  `partly` names each value read and what the rule wants. Usually Windows or a program changed it after the
  installation; tell the person which value, never guess more.
- as on a clean Windows: the values equal the defaults of Windows, so nothing proves the rule; the read keeps its state.
- not readable, `not_read`: rules that act only during the installation, at the first sign-in or on the sign-in
  screen are not read; features and capabilities are read only when WinWeaver runs as administrator.
- Rules of the default user profile are read in the profile of the signed-in user.
- Windows shows uk-UA as `uk`; the read maps it back.

Report differences in plain words with rule titles and ids. Never apply settings: WinWeaver does it in the window, not
through you.
