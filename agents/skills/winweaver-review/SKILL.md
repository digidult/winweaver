---
name: winweaver-review
description: Review a WinWeaver profile for weak protection and mistakes, and explain it to the person. Use when asked what a profile does, whether it is good, or what to improve.
---

# Review a profile

1. `get_status`; `get_profile`: the forms (edition, key mode, disk, accounts, languages, time zone), `rules_in_effect`,
   `parameters`, `changed`.
2. `check_profile`: errors stop the build, warnings name what to look at. Report errors first.
3. Protection that is off: `list_rules` {group, on: false, limit: 50} for the groups `defender`, `security.policies`,
   `security`, `update`, `network`, `browsers`, `privacy`. Every row has its level: `baseline` (basic protection) and
   `recommended` matter most; `optional` is a choice; `risky` may disturb programs.
4. For each finding worth a change, `get_rule` {id}: `risk`, `requires`, `conflicts`. A rule `blocked_by` another one
   is chosen on but waits for it; a rule `linked_by` a built-in rule is already written by it.
5. Compare with a stricter preset when useful: `compare_profiles` {a: <current>, b: "Strict"}.

## Report

- Plain words, the language of the person; each rule as its title and id in backticks.
- Order: errors, basic protection off, recommended protection off, then the rest in one line.
- Say what each proposal costs (`risk`) and what it needs. Propose changes; make none without a yes
  (skill `winweaver-change`).
- Not faults: the accounts Admin and User without passwords, the display language of the image, a disk mode chosen on
  purpose.
- End with what has not run: the build in the window (F9) and an installation in a virtual machine.
