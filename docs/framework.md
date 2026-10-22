# W#, the framework WW (October 2026)

Part 2 of the refactoring (part 1 is `docs/refactoring.md`, merged as 0.23.0). The window of
WinWeaver (the tab row, the command bar, the navigation row, the message and status bars, the
settings tab, the themes and fonts, the translations, the painted tree and list, the modal windows,
the drafts and the session) knows nothing of profiles, catalogs or the registry. It leaves the crate
`winweaver-ui` and becomes a framework for programs of the kind "a window with tabs" that analyse and
manage a system: `ww` in code, WW in documents, the brand W#. WinWeaver is its first user and keeps
everything of its own (the documents `.ww*`, the menu items that open them, the technical documents
of its features); the portable editor `wedit` is its second user and the proof that the line is cut
right. The code and the documents of WW are English only.

The survey behind this plan was made on 21.10.2026 on 0.23.0: of the 64 files of `winweaver-ui`,
fourteen (4 700 lines) name no type of WinWeaver, eleven touch it in one or two places, the rest are
WinWeaver.

## 1. Goals, measurable

- `ww` depends on `eframe`, `egui_commonmark`, `rfd`, `serde` and `ww-base`, never on
  `winweaver-core`, `winweaver-win` or `winweaver-mcp`; `cargo build -p ww` and `cargo build -p wedit`
  succeed without the WinWeaver crates in the graph.
- `wedit` is under 800 lines of its own code and shows every part of the shell: tabs, the menu, the
  command bar, the search of a tab, the settings tab, the two themes with their fonts, English built
  in and Ukrainian as a file, the drafts, the restored tabs, the recent files, the log.
- WinWeaver does what 0.23.0 does: every test passes, the screenshots look the same, the files next
  to the executable keep their names and formats (`settings.json`, the drafts, the logs, the themes).
- A text of WW appears once in `tr` or `trf` and is translated in the eight languages of WinWeaver;
  the test `every_text_of_the_interface_is_translated` reads the sources of `ww` too.
- No file of `ww` over 800 lines; the shell itself (`shell/`) in files by theme like `app/` of part 1.

## 2. The line between WW and an application

| WW holds | The application holds |
|---|---|
| the window: `run`, `smoke_window`, the renderers and `--renderer`, the title, the colours of the title bar | its name, its icon, its version text, the pages it opens (guide, license, reference) |
| the tab row with the menu button, a drag of a tab, the menu of a tab, the new button | the items of the menu, what New makes, what Open accepts (`open_path`) |
| the command bar drawn from data (`Item`, `Command`, `Menu`), the keys of the shell commands | the commands of each tab (`commands`), the menus of the bar (`menu_popup`), its own keys |
| the navigation row (history, breadcrumb, star, the search of the tab) drawn from data (`Nav`) | the crumbs, whether the tab has a history or a star, what a click does |
| the message bar (a list of messages with links) and the status bar (the status text, the file button, the right part) | the messages of the active tab and their links, the right part of the status bar |
| the settings tab with its General section (language, theme, fonts, renderer, restore tabs) and the file `settings.json` | its own sections and fields (`Extra`), drawn by it |
| the themes Light and Dark, `.wwtheme` files next to the executable, the editor of a theme, the fonts | more embedded themes (Latte, Matrix) |
| `tr` and `trf`, the dictionaries the application embeds and the files `lang/<code>/ui.json` | the dictionaries themselves (WinWeaver eight, wedit none) |
| the painted tree and list, the modal window, the side panel, the file dialog in a folder, the Markdown viewer with its search and anchors, the search of a tab (`Search`, `matches_words`) | the views of its documents |
| the drafts of unsaved documents (a text a document gives), the restored tabs, the recent files, the log, the question over the window, the acts of the shell | the documents (`Document`), the acts of the application and `apply` |
| the files of the program: `jsonfile` (strict JSON, the trait `Document`), `paths` (a file inside the program folder kept relative), `clock` (`ww-base`) | its file formats |

The window of WinWeaver keeps the look of variant A (`docs/spec/01-ui.md`); WW is that look.

## 3. The crates

```
crates/
├── ww-base/        jsonfile (strict reading with limits, the BOMs, the trait Document, write_atomic),
│                   paths, clock: moved from winweaver-core, which re-exports them unchanged
├── ww/             the framework: style, tree, list, widgets, icons, themes (light, dark), i18n,
│                   renderer, mdview, settings (generic over the application's fields), session (the
│                   log, the drafts), dialogs, folders, question, commands, doc (Document, Search),
│                   shell/ (Shell, Act, the frame, the chrome by theme: tabs, bar, nav, status,
│                   settings, themes, session, apply), run; data/themes/{light,dark}.wwtheme;
│                   tests with a tiny application of two document kinds
├── winweaver-ui/   WinWeaver on WW: the documents, their views, the acts by subsystem, app/ (the
│                   methods of WinWeaver with the shell as a parameter), its icons, its commands, its
│                   menu, its settings sections, its pages, its eight languages, its two extra themes
├── wedit/          the editor: TextDoc (a .txt in an egui text editor), MdDoc of WW for .md with
│                   "Edit as text" in its command bar; main.rs
└── winweaver/      unchanged
```

`ww` and `ww-base` carry the workspace version. `wedit.exe` is built by the Windows job and packaged
as its own archive `wedit-<version>.zip` (the exe, `README.md`, `lang/uk/ui.json`), beside the
WinWeaver archive; a tag releases both.

## 4. The interface

```rust
pub trait Document {
    fn title(&self) -> String;                 // the default: the name of the file, else name()
    fn name(&self) -> String { String::new() }
    fn dirty(&self) -> bool { false }
    fn path(&self) -> Option<&Path> { None }
    fn search(&mut self) -> Option<&mut Search> { None }
    fn icon(&self) -> &'static str;
    fn search_hint(&self) -> &'static str { "" }
    fn has_tools(&self) -> bool { true }       // a command bar
    fn has_nav(&self) -> bool { false }        // a navigation row
    fn scrolled(&mut self, how: Scrolled) {}   // Revealed, Follow: the selection came into view
    fn draft(&self) -> Option<String> { None } // the text a draft keeps while the file is unsaved
}

pub trait App: Sized + 'static {
    type Doc: Document;
    type Act;                                   // the acts of the application
    type Extra: Serialize + DeserializeOwned + Default + Clone + PartialEq;   // its settings
    const NAME: &'static str;                   // "WinWeaver", "wedit"
    const SETTINGS_FORMAT: &'static str;        // "winweaver-settings"
    fn languages() -> &'static [(&'static str, &'static str)];   // the embedded ui.json files
    fn themes() -> &'static [(&'static str, &'static str)];      // the embedded themes beyond light and dark
    fn new_document(&mut self, shell: &mut Shell<Self>);
    fn open_path(&mut self, shell: &mut Shell<Self>, path: PathBuf);
    fn save(&mut self, shell: &mut Shell<Self>, ask: bool);
    fn restore_draft(&mut self, shell: &mut Shell<Self>, path: Option<PathBuf>, text: String) -> bool;
    fn draw(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act<Self>>);
    fn commands(&self, shell: &Shell<Self>) -> Vec<Item<Self>>;
    fn menu_popup(&self, shell: &Shell<Self>, menu: usize, ui: &mut egui::Ui, acts: &mut Vec<Act<Self>>);
    fn menu(&self, shell: &Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act<Self>>);
    fn nav(&self, shell: &Shell<Self>) -> Option<Nav<Self>>;
    fn messages(&self, shell: &Shell<Self>) -> Vec<Message<Self>>;
    fn status_right(&self, shell: &Shell<Self>) -> Vec<String>;
    fn settings_sections(&self) -> Vec<&'static str>;
    fn settings_section(&mut self, shell: &mut Shell<Self>, section: usize, ui: &mut egui::Ui);
    fn shortcuts(&self, shell: &Shell<Self>, input: &mut egui::InputState) -> Vec<Act<Self>>;
    fn apply(&mut self, shell: &mut Shell<Self>, act: Self::Act);
    fn dialogs(&mut self, shell: &mut Shell<Self>, ctx: &egui::Context, acts: &mut Vec<Act<Self>>);
    fn after_frame(&mut self, shell: &mut Shell<Self>, ctx: &egui::Context);
    fn about(&self, shell: &Shell<Self>) -> String;
}

pub struct Shell<A: App> {
    pub docs: Vec<A::Doc>, pub active: usize,
    pub settings: Settings<A::Extra>, settings_dirty: bool,
    pub status: String, shown_file: Option<(String, PathBuf)>,
    question: Option<Question<A>>, messages_open: bool, search_history: Vec<String>,
    themes: Vec<Theme>, applied_theme, title_colors, last_title, exit, graphics: String,
    drafts_dir: PathBuf, logs_dir: PathBuf, pub pending: Vec<Act<A>>, last_logged: String,
}

pub enum Act<A: App> { Shell(ShellAct<A>), App(A::Act) }
pub enum ShellAct<A: App> {
    Activate(usize), MoveTab(usize, usize), CloseTab(usize, bool), New, Open, OpenPath(PathBuf),
    Save, SaveAs, ForgetRecent, OpenSettings, OpenSection(usize), EditTheme(String),
    OpenThemeFile, UseTheme(String), ToggleMessages, SearchCleared, Revealed, Follow,
    Message(String), ShowInFolder(PathBuf), OpenLink(String), Exit, Ask(Box<Question<A>>), Answer(bool),
}
```

The shell owns the generic state and the application struct (`shell.app`); the methods of the
application take `shell: &mut Shell<Self>` where they took `&mut self` fields of `App` before, so
`self.docs` becomes `shell.docs`, `self.status` becomes `shell.status` and the borrow of the
application's own fields (the catalog, the imports, the machine) stays apart from the shell's. The
shell's methods (`show_or_open`, `focus_tab`, `open_tab`, `saved`, `opened`, `save_failed`,
`open_failed`, `ask`, `discard_act`, `active_search`, `open_file`) move with it. `Question` holds an
`Act<A>`; its generic constructors (`close_discard`, `forget_recent`) move with it, the ones of
WinWeaver stay. `Item`, `Command`, `Menu` take the act type; the keys of the shell (`keys::NEW`,
`OPEN`, `SAVE`, `SAVE_AS`, `CLOSE`, `SETTINGS`, `FIND`, `NEXT_TAB`, `PREVIOUS_TAB`, `TAB_LEFT`,
`TAB_RIGHT`) are WW's, the others (`CHECK`, `BUILD`, `BOOKMARK`, `FOLD`, `GUIDE`, `BACK`,
`FORWARD`) WinWeaver's. The settings file has the generic fields (format, version, language, theme,
custom theme, renderer, restore tabs, open tabs, active tab, recent, search history) and the
application's as `#[serde(flatten)] extra: A::Extra`, so `settings.json` of 0.23.0 reads unchanged.

The pages: `MdDoc::new(title, path, text)` is WW's; the guide and the license are pages of
WinWeaver (`pages.rs`) with a marker `MdDoc::page(id, title, text)`, so a page opens once.

## 5. wedit

A portable editor for people who want a text file open fast and safe, in the look of WinWeaver.
`wedit.exe` next to `settings.json`, `drafts/`, `logs/`, `themes/` and `lang/` of its own.

- Tabs of `.txt` files (and any text file): a monospace editor (`egui::TextEdit::multiline` with its
  undo), the search of the tab (Find, F3 next, Shift+F3 back), the line and column in the status bar,
  Save and Save as, a draft two seconds after a change, the tabs of the last session restored.
- A `.md` file opens in the Markdown viewer of WW with the search of a page; its command bar offers
  Edit as text, which opens the same file in a text tab (one tab a file).
- The menu: New, Open, Open recent, Save, Save as, Close tab, Close tab discarding changes,
  Settings, About, Exit. The command bar: New, Open, Save, Save as, Find.
- Settings: General of WW alone (language, theme, fonts, renderer, restore tabs).
- Two themes, Light and Dark, built in; more as `.wwtheme` files. English built in; `lang/uk/ui.json`
  in the archive as the example of a language file, every text of wedit and of WW in it.
- Drop a file on the window to open it; the command line `wedit <file>...`.
- Under 800 lines: `main.rs`, `textdoc.rs`, `app.rs`.

## 6. Steps

### W1. `ww-base`

`jsonfile.rs`, `paths.rs`, `clock.rs` move from `winweaver-core/src` to `crates/ww-base/src` (with
their tests); `winweaver-core` depends on `ww-base` and keeps `pub use ww_base::{clock, jsonfile,
paths};`, so no caller changes. Check: Linux.

### W2. `ww`, the parts that move as they are

`crates/ww/src`: `style.rs`, `tree.rs`, `list.rs`, `widgets.rs`, `themes.rs` (Light and Dark embedded
from `crates/ww/data/themes`, `EMBEDDED` becomes `embedded(extra)`), `i18n.rs` (the dictionaries
handed by the application at the start: `i18n::install(&[(code, json)])`; `tr`, `trf`, `languages`,
`set_language`, `current` unchanged), `renderer.rs`, `mdview.rs` (without the pages), `settings.rs`
(generic), `session.rs` (the log, the drafts of a text), `dialogs.rs` (the open dialog takes the
filters from the application), `folders.rs` (`exe_dir`, the folders by name), `question.rs` (the
type and the generic constructors), `commands.rs` (the types and the shell keys), `icons.rs` (the
icons of the shell; WinWeaver keeps its own file for the rest), `doc.rs` (`Document`, `Search`,
`matches_words`, `Scrolled`). `winweaver-ui` depends on `ww` and re-exports what its views name
(`pub use ww::{style, tree, list, widgets, ...}`), so the views do not change. Check: Linux.

### W3. `ww::shell`

`shell/mod.rs` (`Shell`, `Act`, `ShellAct`, `new`, the accessors), `shell/frame.rs`
(`eframe::App`: the chrome, `app.draw`, the question, `app.dialogs`, the acts, `app.after_frame`,
the drafts, the session), `shell/apply.rs` (the shell acts, `ask`, `discard_act`, the shortcuts of
the shell keys), `shell/tabs.rs`, `shell/bar.rs`, `shell/nav.rs`, `shell/status.rs` (from `chrome/`,
drawn from the data the application gives), `shell/settings.rs` (the settings tab, the General
section), `shell/themes.rs`, `shell/session.rs`, `shell/files.rs` (`open_file`, the recent files),
`run.rs` (`run::<A>`, `smoke_window::<A>`). WinWeaver becomes `impl ww::App for WinWeaver`: `App`
loses the shell's fields, its methods take `shell`, `Doc` implements `Document`, `Act` keeps
`Profile`, `Pc`, `Users`, `Diff`, `Marks`, `Mcp`, `Catalog`, `Import`, `Registry`, `RegFile` and
loses `Nav`, `File`, `Settings`, `Theme` (the shell's), `chrome/` goes, `commands.rs` keeps
`commands` and the extra keys, `app/frame.rs` becomes `draw` by kind, `app/apply.rs` the `apply` of
the application acts. The tests build `Shell::new(WinWeaver::new(..))` and keep their steps. Check:
Linux, then Windows (the window tests, the screenshots).

### W4. `wedit`

`crates/wedit`: `app.rs` (`Wedit`, `Doc { Text(TextDoc), Page(MdDoc) }`, `Act { Find, FindNext(bool),
EditAsText }`, `Extra {}`), `textdoc.rs`, `main.rs` (the arguments, `ww::run`). `package-wedit/`
(`README.md`, `lang/uk/ui.json`). The Windows job builds and packages it. Check: Linux, then Windows.

### W5. The documents

`docs/ww.md`: the framework for its users (the interface of section 4 as it is, how to start an
application, the files next to the executable, the themes, the languages), English. AGENTS.md
sections 2, 3, 6 and 7; `docs/spec/04-architecture.md`; `README.md` names W# and wedit;
`docs/releases/v0.24.0.md`. Version 0.24.0.

## 7. Behaviour settled on purpose

- The drafts of WinWeaver change their file format: `drafts/<key>.json` holds `{"path", "text"}`
  where the text is the profile as JSON (0.23.0 held the profile as a JSON object). An old draft is
  read once by WinWeaver's `restore_draft` (both shapes), then written in the new one.
- The open dialog of WinWeaver (`pick_any`) lists the same filters, given by WinWeaver to the shell.
- The About text of WinWeaver and the title of the window keep their words; `wedit` has its own.
- `settings.json` of 0.23.0 reads unchanged (the generic fields and WinWeaver's are the same keys).

## 8. Order and the CI runs

| Group | Steps | Check |
|---|---|---|
| 1 | W1, W2 | Linux |
| 2 | W3 | Linux, then Windows |
| 3 | W4, W5, the version, the pull request ready | Linux, then Windows (both archives) |

One branch, `ww`, one pull request, version 0.24.0; the Linux cross check after every commit group,
Windows twice.

## 9. Left out on purpose

- WW has no document of its own but the Markdown page; a text editor document is wedit's.
- The MCP server, This PC, the registry and the catalogs stay WinWeaver's; WW offers no hook for a
  server or a background read beyond `after_frame`.
- No change of the look, the keys or the texts; no new feature of WinWeaver.
- A crate of its own for `tree` and `list` (W# widgets) only if a third user asks.

## 10. Done (22.10.2026)

W1 to W5 as planned, on the branch `ww`, version 0.24.0. What differs from the plan above: `shell/frame.rs`,
`shell/apply.rs`, `shell/session.rs` and `shell/files.rs` are `shell/window.rs` (the frame, the acts of
the window, the keys, the session restored) with the session helpers as methods of `Shell`; the theme
module of the shell is `shell/theme.rs`; `App::started` (the first tab, at every construction), `load`
(the files of the program at a start) and `restored` are three hooks, and `settings_opened` lets the
program read its lists again when the settings tab opens; `Window<A>` holds the shell and the program
apart, so the tests build `Window::new(WinWeaver::new(catalog, texts), settings)`. The drafts of
WinWeaver keep their format (the profile as a JSON object) under the name `ww-draft`, and a draft of
0.23.0 reads too. `ww` re-exports `eframe` and `rfd`, so wedit depends on `ww` alone.
