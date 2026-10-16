# WinWeaver

WinWeaver prepares unattended installations of Windows 10 and 11. You choose the settings of a computer in a profile,
and the program builds the answer file `autounattend.xml` with its scripts: put it on the installation USB stick, and
Windows installs and configures itself without questions. The same profile can be compared with others, checked
against a running PC and applied to it. The rule catalog is written for small offices without a domain: security and
updates first.

## Start

Windows 10 or 11, 64-bit. Nothing is installed: unpack the archive into a folder and run `winweaver.exe`. The program
keeps its settings and files in that folder. The guide opens at the first start and with F1.

## What it does

- **Profiles** (`.wwprofile`): a tree of about 280 rules in three parts (Installation, Standard policies, Other), each
  with its description, risk, what it writes and its parameters, and the forms of the installation: edition and
  product key, computer name, workgroup, disk, accounts, languages, time zone. Presets: Office, Strict, Laptop, Home.
- **Check and build**: Check (F7) lists errors and warnings; Build (F9) writes `autounattend.xml` and its scripts into
  `output`.
- **Compare** two profiles (`.wwdiff`): the differences side by side with a comment on each; apply one or all.
- **This PC**: read its settings into a profile and see what is in effect; check the selected rules; apply them or
  return them to the Windows defaults as administrator with a backup of every value, and undo a change from its backup.
  For other PCs it saves stand-alone scripts that apply, revert, audit and undo; their audit report opens as a profile.
- **Policy templates**: import the ADMX templates of this Windows, of a folder or a catalog file; their policies
  become rules. An `.admx` or `.wwcatalog` file also opens as a tab without importing.
- **Bookmarks** of rules with comments (`.wwmarks`), **themes** with their colours and fonts (`.wwtheme`) and an editor.
- **Languages**: English, Ukrainian and Russian built in, Belarusian as a file, more as files (below).
- **MCP server** for AI agents: they read and, when you allow it, change profiles. See `agents/README.md`.

Every new answer file goes to a virtual machine first, never straight to working computers.

## Editions and keys

The edition form offers these editions with the generic key of each, "Ask during Setup" or a key of your own. A
generic key selects the edition during the installation; it does not activate Windows.

| Edition | Generic key |
|---|---|
| Pro | VK7JG-NPHTM-C97JM-9MPGT-3V66T |
| Pro N | MH37W-N47XK-V7XM9-C7227-GCQG9 |
| Pro for Workstations | NRG8B-VKK3Q-CXVCJ-9G2XF-6Q84J |
| Pro for Workstations N | 9FNHH-K3HBT-3W4TD-6383H-6XYWF |
| Pro Education | 6TP4R-GNPTD-KYYHQ-7B7DP-J447Y |
| Pro Education N | YVWGF-BXNMC-HTQYQ-CPQ99-66QFC |
| Education | YNMGQ-8RYV3-4PGQ3-C8XTP-7CFBY |
| Education N | 2WH4N-8QGBV-H22JP-CT43Q-MDWWJ |
| Enterprise | NPPR9-FWDCX-D2C8J-H872K-2YT43 |
| Enterprise N | DPH2V-TTNVB-4X9Q3-TJR4H-KHJW4 |
| Enterprise G | YYVX9-NTFWV-6MDM3-9PT4T-4M68B |
| Enterprise G N | 44RPN-FTY23-9VTTB-MP9BX-T84FV |
| Enterprise LTSC 2024 / 2021 / 2019 | M7XTQ-FN8P6-TTKYV-9D4CC-J462D |
| Enterprise N LTSC | 92NFX-8DJQP-P6BBQ-THF9C-7CG2H |

## The folder of the program

| Item | What it is |
|---|---|
| `winweaver.exe` | The program |
| `profiles/` | Your profiles; the presets are there as files too |
| `lang/be/` | The Belarusian translation, the example of a language |
| `agents/` | Instructions, skills and a container for AI agents |
| `settings.json`, `bookmarks.wwmarks` | Created when needed |
| `output/` | Answer files, scripts, plans and backups of changes of this PC |
| `imports/` | Imported policy templates |
| `themes/` | Your themes (`<name>.wwtheme`) |
| `drafts/`, `logs/` | Unsaved changes and the log of each day |

## Keys

F1 guide, F7 check, F9 build, Ctrl+N new profile, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+, settings,
Ctrl+F search in the tab, Ctrl+B bookmark,
Ctrl+D description, Ctrl+W close the tab, Ctrl+Tab next tab, Ctrl+Shift+PgUp and PgDn move the tab, Alt+Left and
Alt+Right back and forward. In the tree and the lists: the arrows, Home, End, Page Up and Down, Space to switch, Enter
to open.

## Add a language

1. Make a folder `lang/<code>` next to `winweaver.exe`, with the code of the language: `be`, `pl`, `de`.
2. Put `ui.json` there: a JSON object in UTF-8 whose keys are the English texts of the interface and whose values are
   their translations. `_language` is the name of the language in itself. A text left out stays English.
3. Optionally `guide.md`: the guide in that language.
4. Restart the program and choose the language in Settings, General.

`lang/be/ui.json` holds every text of the interface: copy it as the start of a new language and translate the values.
Its beginning:

```json
{
  "_language": "Беларуская",
  "Bookmarks": "Закладкі",
  "Settings": "Налады",
  "Open failed: {e}": "Не ўдалося адкрыць: {e}",
  "Catalog {version}, {n} rules": "Каталог {version}, правіл: {n}"
}
```

Keep the words in braces (`{e}`, `{n}`) and the keys in brackets (`(Ctrl+F)`) as they are. The titles and descriptions
of the rules are built in for English, Ukrainian and Russian; another language shows them in English.

## License

MIT, see `LICENSE`.
