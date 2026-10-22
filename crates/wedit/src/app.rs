//! wedit on the window of WW: text tabs, Markdown pages, the menu, the command bar, the
//! settings of the window alone.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ww::commands::{Item, cmd, keyed};
use ww::eframe::egui::{self, Key, Modifiers};
use ww::i18n::{tr, trf};
use ww::keys::{self, Keys};
use ww::mdview::{self, MdDoc};
use ww::shell::menu_item;
use ww::{App, Document, Draft, Search, Shell, ShellAct, ShellDoc, icons, session};

use crate::textdoc::TextDoc;

/// The version of the program, for About and the log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The largest file a tab opens.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const FIND_NEXT: Keys = Keys::new("F3", Modifiers::NONE, Key::F3);
const FIND_BACK: Keys = Keys::new("Shift+F3", Modifiers::SHIFT, Key::F3);

/// The fields of wedit in the settings file: none beyond the ones of the window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Extra {}

impl ww::Extra for Extra {
    const FORMAT: &'static str = "wedit-settings";
}

/// A tab of wedit.
pub enum Doc {
    Text(TextDoc),
    /// A Markdown file shown as a page.
    Page(MdDoc),
    /// The settings and the editor of a theme: documents of the window itself.
    Shell(ShellDoc),
}

impl Document for Doc {
    fn title(&self) -> String {
        match self {
            Doc::Text(d) => d.title(),
            Doc::Page(d) => d.title(),
            Doc::Shell(d) => d.title(),
        }
    }

    fn dirty(&self) -> bool {
        match self {
            Doc::Text(d) => d.dirty,
            Doc::Page(_) => false,
            Doc::Shell(d) => d.dirty(),
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Doc::Text(d) => d.path.as_deref(),
            Doc::Page(d) => d.path(),
            Doc::Shell(d) => d.path(),
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        match self {
            Doc::Text(d) => Some(&mut d.search),
            Doc::Page(d) => d.search(),
            Doc::Shell(_) => None,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Doc::Text(_) => icons::PAGE,
            Doc::Page(_) => icons::BOOK,
            Doc::Shell(d) => d.icon(),
        }
    }

    fn search_hint(&self) -> &'static str {
        match self {
            Doc::Text(_) => "Find (Ctrl+F, F3 next)",
            Doc::Page(_) => "Search the text (Ctrl+F)",
            Doc::Shell(_) => "",
        }
    }

    fn has_tools(&self) -> bool {
        match self {
            Doc::Shell(d) => d.has_tools(),
            _ => true,
        }
    }

    fn draft_key(&self) -> Option<String> {
        match self {
            Doc::Text(d) => Some(d.draft_key.clone()),
            _ => None,
        }
    }

    fn draft_due(&mut self, all: bool) -> Option<Draft> {
        match self {
            Doc::Text(d) => d.draft_due(all),
            _ => None,
        }
    }

    fn shell_doc(&self) -> Option<&ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            _ => None,
        }
    }

    fn shell_doc_mut(&mut self) -> Option<&mut ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            _ => None,
        }
    }
}

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    /// An act of the window itself, run by the shell.
    Shell(ShellAct<Wedit>),
    /// Opens the search field of the tab.
    Find,
    /// Selects the next (`true`) or the previous match of the search in a text tab.
    FindNext(bool),
    /// Opens the file of a page in a text tab.
    EditAsText,
}

impl From<ShellAct<Wedit>> for Act {
    fn from(act: ShellAct<Wedit>) -> Self {
        Act::Shell(act)
    }
}

/// The editor.
pub struct Wedit {
    /// The files named on the command line, opened after the session is restored.
    files: Vec<PathBuf>,
}

/// A text file read whole: UTF-8, with or without its mark, up to `MAX_BYTES`.
fn read_file(path: &Path) -> Result<String, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_BYTES {
        return Err(trf(
            "{path} is larger than {mb} MB",
            &[("path", &path.display()), ("mb", &(MAX_BYTES / 1024 / 1024))],
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| trf("{path} is not UTF-8 text", &[("path", &path.display())]))
}

/// The name of a file for the title of its tab.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl Wedit {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self { files }
    }

    fn open_text(&mut self, shell: &mut Shell<Self>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Text(t) if t.path.as_ref() == Some(&path)),
            read_file,
            |text, path| Doc::Text(TextDoc::new(text, Some(path))),
        );
    }

    fn open_page(&mut self, shell: &mut Shell<Self>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Page(p) if p.path.as_deref() == Some(path.as_path())),
            read_file,
            |text, path| Doc::Page(MdDoc::new(file_name(&path), Some(path), text)),
        );
    }

    /// The next or the previous match of the search of the text tab in front.
    fn find_next(&mut self, shell: &mut Shell<Self>, forward: bool) {
        let Some(Doc::Text(doc)) = shell.active_doc_mut() else {
            return;
        };
        let needle = doc.search.text.trim().to_owned();
        if needle.is_empty() {
            doc.search.open = true;
            doc.search.focus = true;
            return;
        }
        if doc.find(&needle, forward).is_none() {
            shell.status = trf("{text} was not found", &[("text", &needle)]);
        }
    }

    /// The file of the page in front in a text tab: the one that has it, or a new one.
    fn edit_as_text(&mut self, shell: &mut Shell<Self>) {
        let Some(Doc::Page(page)) = shell.active_doc() else {
            return;
        };
        let (path, text) = (page.path.clone(), page.text.clone());
        if let Some(p) = &path
            && shell.focus_tab(|d| matches!(d, Doc::Text(t) if t.path.as_ref() == Some(p)))
        {
            return;
        }
        shell.open_tab(Doc::Text(TextDoc::new(text, path)));
    }

    fn about(&self, shell: &Shell<Self>) -> String {
        trf(
            "wedit {version}, settings {path}",
            &[
                ("version", &VERSION),
                ("path", &ww::Settings::<Extra>::path().display()),
            ],
        )
        .replace("{graphics}", &shell.graphics)
    }
}

impl App for Wedit {
    type Doc = Doc;
    type Act = Act;
    type Extra = Extra;

    const NAME: &'static str = "wedit";
    const NEW_HINT: &'static str = "New file (Ctrl+N)";

    fn shell_doc(doc: ShellDoc) -> Doc {
        Doc::Shell(doc)
    }

    fn shell_act(act: Act) -> Result<ShellAct<Self>, Act> {
        match act {
            Act::Shell(a) => Ok(a),
            other => Err(other),
        }
    }

    /// The first tab: an empty text.
    fn started(&mut self, shell: &mut Shell<Self>) {
        shell.open_tab(Doc::Text(TextDoc::new(String::new(), None)));
    }

    /// The files of the command line, in tabs; an empty first tab makes room for them.
    fn restored(&mut self, shell: &mut Shell<Self>) {
        let files = std::mem::take(&mut self.files);
        if files.is_empty() {
            return;
        }
        let before = shell.docs.len();
        for path in files {
            self.open_path(shell, path);
        }
        let untouched = matches!(shell.docs.first(), Some(Doc::Text(t)) if t.text.is_empty() && !t.dirty && t.path.is_none());
        if shell.docs.len() > before && untouched && shell.docs.len() > 1 {
            shell.docs.remove(0);
            shell.active = shell.active.saturating_sub(1).max(0);
            shell.active = shell.docs.len() - 1;
        }
    }

    fn new_document(&mut self, shell: &mut Shell<Self>) {
        shell.open_tab(Doc::Text(TextDoc::new(String::new(), None)));
    }

    fn open_path(&mut self, shell: &mut Shell<Self>, path: PathBuf) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "md" => self.open_page(shell, path),
            ww::themes::EXTENSION => shell.open_theme_path(path),
            _ => self.open_text(shell, path),
        }
    }

    fn open_dialog(&self, _shell: &Shell<Self>) -> ww::rfd::FileDialog {
        ww::rfd::FileDialog::new()
            .set_title(tr("Open"))
            .add_filter(tr("Text files"), &["txt"])
            .add_filter(tr("Markdown pages"), &["md"])
            .add_filter(tr("All files"), &["*"])
    }

    fn save(&mut self, shell: &mut Shell<Self>, ask: bool) {
        let Some(Doc::Text(doc)) = shell.active_doc() else {
            shell.status =
                tr("Nothing to save: a page is only viewed; Edit as text changes it").to_owned();
            return;
        };
        let path = match (&doc.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let proposed = doc
                    .path
                    .as_deref()
                    .map(file_name)
                    .unwrap_or_else(|| format!("{}.txt", tr("Untitled")));
                let dialog = ww::rfd::FileDialog::new()
                    .set_title(tr("Save as"))
                    .add_filter(tr("Text files"), &["txt"])
                    .add_filter(tr("All files"), &["*"])
                    .set_file_name(proposed);
                let dialog = match doc.path.as_deref().and_then(Path::parent) {
                    Some(dir) => dialog.set_directory(dir),
                    None => dialog,
                };
                let Some(p) = dialog.save_file() else {
                    return;
                };
                p
            }
        };
        let Some(Doc::Text(doc)) = shell.active_doc_mut() else {
            return;
        };
        let old_key = doc.draft_key.clone();
        match doc.save_to(&path) {
            Ok(()) => {
                doc.draft_key = session::key_for_path(&path);
                let new_key = doc.draft_key.clone();
                session::remove_draft(&shell.drafts_dir, &old_key);
                session::remove_draft(&shell.drafts_dir, &new_key);
                shell.saved(&path);
                shell.remember_recent(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// A draft of the last session: the text into the tab of its file, or a new tab.
    fn restore_draft(
        &mut self,
        shell: &mut Shell<Self>,
        key: String,
        path: Option<PathBuf>,
        document: serde_json::Value,
    ) -> bool {
        let serde_json::Value::String(text) = document else {
            return false;
        };
        let same = path.as_ref().and_then(|p| {
            shell
                .docs
                .iter()
                .position(|d| matches!(d, Doc::Text(t) if t.path.as_ref() == Some(p)))
        });
        let mut doc = match same {
            Some(i) => match shell.docs.remove(i) {
                Doc::Text(t) => t,
                _ => return false,
            },
            None => TextDoc::new(String::new(), path.filter(|p| p.exists())),
        };
        doc.text = text;
        doc.dirty = true;
        doc.changed = None;
        doc.draft_key = key;
        shell.open_tab(Doc::Text(doc));
        true
    }

    fn draw(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui, _acts: &mut Vec<Act>) {
        match shell.active_doc_mut() {
            Some(Doc::Text(doc)) => doc.view(ui),
            Some(Doc::Page(doc)) => {
                egui::CentralPanel::default().show(ui, |ui| mdview::md_view(ui, doc));
            }
            _ => {}
        }
    }

    fn commands(&self, shell: &Shell<Self>) -> Vec<Item<Act>> {
        let new = keyed(icons::NEW, tr("New"), &keys::NEW, ShellAct::New);
        let open = keyed(icons::OPEN, tr("Open"), &keys::OPEN, ShellAct::Open);
        let save = keyed(icons::SAVE, tr("Save"), &keys::SAVE, ShellAct::Save);
        let save_as = keyed(icons::SAVE, tr("Save as"), &keys::SAVE_AS, ShellAct::SaveAs);
        match shell.active_doc() {
            Some(Doc::Text(_)) => vec![
                new,
                open,
                save,
                save_as,
                Item::Separator,
                keyed(icons::FIND, tr("Find"), &keys::FIND, Act::Find),
            ],
            Some(Doc::Page(_)) => vec![
                new,
                open,
                Item::Separator,
                cmd(icons::EDIT, tr("Edit as text"), Act::EditAsText),
            ],
            Some(Doc::Shell(ShellDoc::Theme(_))) => vec![
                keyed(icons::OPEN, tr("Open"), &keys::OPEN, ShellAct::OpenThemeFile),
                save,
                save_as,
            ],
            Some(Doc::Shell(ShellDoc::Settings(_))) => Vec::new(),
            None => vec![new, open],
        }
    }

    fn menu(&self, shell: &Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        menu_item(ui, tr("New"), keys::NEW.text, Act::Shell(ShellAct::New), acts);
        menu_item(ui, tr("Open..."), keys::OPEN.text, Act::Shell(ShellAct::Open), acts);
        ui.add_enabled_ui(!shell.settings.recent.is_empty(), |ui| {
            ui.menu_button(tr("Open recent"), |ui| {
                for path in &shell.settings.recent {
                    menu_item(
                        ui,
                        &path.display().to_string(),
                        "",
                        Act::Shell(ShellAct::OpenPath(path.clone())),
                        acts,
                    );
                }
            });
        });
        menu_item(ui, tr("Save"), keys::SAVE.text, Act::Shell(ShellAct::Save), acts);
        menu_item(
            ui,
            tr("Save as..."),
            keys::SAVE_AS.text,
            Act::Shell(ShellAct::SaveAs),
            acts,
        );
        menu_item(
            ui,
            tr("Close tab"),
            keys::CLOSE.text,
            Act::Shell(ShellAct::CloseTab(shell.active, false)),
            acts,
        );
        menu_item(
            ui,
            tr("Close tab, discard changes"),
            "",
            shell.discard_act(shell.active),
            acts,
        );
        ui.separator();
        menu_item(
            ui,
            tr("Settings"),
            keys::SETTINGS.text,
            Act::Shell(ShellAct::OpenSettings),
            acts,
        );
        menu_item(
            ui,
            tr("About"),
            "",
            Act::Shell(ShellAct::Message(self.about(shell))),
            acts,
        );
        menu_item(ui, tr("Exit"), "", Act::Shell(ShellAct::Exit), acts);
    }

    /// The position of the cursor and the state of the text.
    fn status_left(&self, shell: &Shell<Self>) -> Vec<String> {
        let Some(Doc::Text(doc)) = shell.active_doc() else {
            return Vec::new();
        };
        let state = if doc.dirty {
            tr("Modified")
        } else if doc.path.is_some() {
            tr("Saved")
        } else {
            tr("Not saved yet")
        };
        vec![
            trf(
                "Ln {line}, Col {col}",
                &[("line", &doc.line), ("col", &doc.col)],
            ),
            state.to_owned(),
        ]
    }

    fn shortcuts(&self, _shell: &Shell<Self>, input: &mut egui::InputState) -> Vec<Act> {
        let mut acts = Vec::new();
        if input.consume_shortcut(&FIND_BACK.shortcut()) {
            acts.push(Act::FindNext(false));
        }
        if input.consume_shortcut(&FIND_NEXT.shortcut()) {
            acts.push(Act::FindNext(true));
        }
        acts
    }

    fn apply(&mut self, shell: &mut Shell<Self>, act: Act) {
        match act {
            Act::Find => {
                if let Some(search) = shell.active_search() {
                    search.open = true;
                    search.focus = true;
                }
            }
            Act::FindNext(forward) => self.find_next(shell, forward),
            Act::EditAsText => self.edit_as_text(shell),
            // The window runs its own acts (`shell_act`); one that comes here goes back to it.
            Act::Shell(a) => shell.pending.push(Act::Shell(a)),
        }
    }
}
