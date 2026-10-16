---
name: winweaver
description: Use WinWeaver over MCP to read and edit Windows 11 installation profiles, check them and build autounattend.xml. Start with get_status; respect the mode.
---

# WinWeaver over MCP

WinWeaver holds a catalog of 284 rules for unattended installations of Windows 10 and 11 in small workgroups:
answer file settings, policies, other registry values, services, app removals and scripts. A profile says
which rules are on, with which parameters, and fills the forms of the installation (edition and key,
computer name, workgroup, disk, accounts, languages, time zone). The MCP server of WinWeaver keeps its own
workspace of profiles; it never applies anything to a computer.

Tool names may carry a prefix of the client, such as `mcp__winweaver__get_status`.

## First steps

1. `get_status`: the mode, whether reading this PC is allowed, the current profile and the workspace.
2. `get_profile`: the current profile (passwords and the product key appear as `***`).
3. `list_groups` (the three roots: Installation, Standard policies, Other), then `list_rules` with a group,
   or `search_rules` with words.
4. `get_rule` before changing a rule: it says what the rule writes, its risk, its parameters and what it
   needs or conflicts with.

## Modes

The person running the server chooses the mode; ask them to switch it, never work around it.

| Mode | What the tools may do |
|---|---|
| read | Read the catalog and the profiles, check, preview a build. The mode of every start. |
| edit | Change the workspace profiles in memory: `set_rule`, `set_param`, `set_group`, `set_profile_info`, `new_profile`, `load_profile`; `read_this_pc` when the option is on. |
| files | Also `save_profile` (into the profiles folder) and `write_answer_file` (into the output folder). |

`read_this_pc` needs the option "Allow reading the settings of this PC" of the MCP tab (or `--read-pc`) and
the mode edit; it reads only and puts the result into a new workspace profile. Its `partly` lists the rules
of which some values differ, with each value read and what the rule wants.

## Errors

An error text starts with its kind:

- `not_allowed`: the mode or the option does not allow the tool; tell the person what to switch.
- `not_found`: no such rule, group, preset, profile or tool; list or search first.
- `invalid`: an argument is wrong (an option that does not exist, a number out of range, a path in a
  name), or the profile has errors that stop the build; `check_profile` lists them.
- `write_failed`, `read_failed`: a file could not be written or read; report the text.

## Working well

- Change one thing, then `check_profile`. Errors stop the build; warnings name what to look at (a baseline
  rule of the protection that is off, a blocked rule, the disk mode partition).
- A rule may be blocked: it is chosen on but needs another rule that is off (`blocked_by` in `get_rule`).
- An imported policy may be linked: a built-in rule in effect writes its values (`linked_by` in
  `get_rule`). It stays off and needs no change; the built-in rule is the one to switch.
- Parameters: an enum takes one of its `options` values, an int a whole number in its range, a string a
  line of text (ASCII where it reaches the scripts), a list a JSON array of texts (for a list of value
  names, `name=value` items).
- Compare before you save: `compare_profiles` with a preset (Office, Strict, Laptop, Home) shows what a
  profile changes.
- The starter accounts Admin and User have no passwords by the decision of the customer; do not propose to
  add passwords.
- The display language of Windows is the language of the installation image; do not change it.
