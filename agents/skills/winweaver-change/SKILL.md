---
name: winweaver-change
description: Carry out a change of a WinWeaver profile safely, from finding the rule to saving the profile or writing the answer file. Use when the person asks to switch, set, create, save or build.
---

# Carry out a change

1. `get_status`: the mode, the current profile, `changed` profiles of the workspace.
2. Find the rule: `search_rules` {query} with the words of the person, or `list_rules` {group}. Unsure which one is
   meant: show two or three candidates and ask.
3. `get_rule` {id}: what it does, `risk`, `params` (enum `options`, int `min`/`max`, string, list), `requires`,
   `conflicts`, `blocked_by`, `linked_by`. A rule `linked_by` a built-in rule needs no change.
4. Say what will change and what it brings with it (needed rules, conflicts), then wait for a yes.
5. The call: `set_rule` {id, on}, `set_param` {id, name, value} (a list is an array of texts, `name=value` items for
   value names), `set_group` {group, on} (every rule below the group), `set_profile_info` {name?, author?, comment?}.
   A new profile: `new_profile` {name, preset?}; another one: `load_profile` {name}.
6. `check_profile`; report new errors and warnings in plain words.
7. Keep it only when asked: `save_profile` {name?} (mode files) writes `profiles/<name>.wwprofile` and replaces a file
   of that name, so confirm the name. The names Office, Strict, Laptop and Home are kept for the presets.
8. The answer file: `preview_build` to look at it; `write_answer_file` (mode files) after a yes, it replaces
   `output/autounattend.xml` and its scripts. Then tell the person: build in the window (F9) to check the scripts,
   install in a virtual machine before any real PC.

A refusal `not_allowed`: name the mode or option the person switches in the MCP tab and wait. `invalid`: fix the value
from `get_rule` and propose it again.
