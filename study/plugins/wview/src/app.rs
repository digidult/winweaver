//! wview on the window of W#: a tab a file, drawn by the plugin that opens its format; without a
//! file, the page of the plugins.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wwrt::egui::{self, Key, Modifiers};
use wwrt::plugin::{self, Host, Plugins, View};
use wwrt::serde::{Deserialize, Serialize};
use wwrt::ww::commands::{Item, cmd, keyed};
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::keys;
use wwrt::ww::shell::menu_item;
use wwrt::ww::{App, Document, Draft, Search, Shell, ShellAct, ShellDoc, icons};

/// The version of the program, for About and the log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The fields of wview in the settings file: none yet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(crate = "wwrt::serde")]
pub struct Extra {}

impl wwrt::ww::Extra for Extra {
    const FORMAT: &'static str = "wview-settings";
}

/// A file in a tab: its plugin and the view the plugin made of it.
pub struct FileDoc {
    pub path: PathBuf,
    pub plugin: usize,
    pub view: Box<dyn View>,
}

/// A tab of wview.
pub enum Doc {
    File(FileDoc),
    /// The settings and the editor of a theme: documents of the window itself.
    Shell(ShellDoc),
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl Document for Doc {
    fn title(&self) -> String {
        match self {
            Doc::File(d) => file_name(&d.path),
            Doc::Shell(d) => d.title(),
        }
    }

    fn dirty(&self) -> bool {
        match self {
            Doc::File(_) => false,
            Doc::Shell(d) => d.dirty(),
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Doc::File(d) => Some(&d.path),
            Doc::Shell(d) => d.path(),
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        match self {
            Doc::File(d) => d.view.search(),
            Doc::Shell(_) => None,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Doc::File(d) if d.view.browses() => "\u{1F5BC}",
            Doc::File(_) => icons::PAGE,
            Doc::Shell(d) => d.icon(),
        }
    }

    fn search_hint(&self) -> &'static str {
        match self {
            Doc::File(_) => "Search the text (Ctrl+F)",
            Doc::Shell(_) => "",
        }
    }

    fn has_tools(&self) -> bool {
        match self {
            Doc::Shell(d) => d.has_tools(),
            Doc::File(_) => true,
        }
    }

    fn draft_key(&self) -> Option<String> {
        match self {
            Doc::Shell(d) => d.draft_key(),
            Doc::File(_) => None,
        }
    }

    fn draft_due(&mut self, all: bool) -> Option<Draft> {
        match self {
            Doc::Shell(d) => d.draft_due(all),
            Doc::File(_) => None,
        }
    }

    fn shell_doc(&self) -> Option<&ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            Doc::File(_) => None,
        }
    }

    fn shell_doc_mut(&mut self) -> Option<&mut ShellDoc> {
        match self {
            Doc::Shell(d) => Some(d),
            Doc::File(_) => None,
        }
    }
}

/// Where a step through the files of a folder goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    First,
    Previous,
    Next,
    Last,
}

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    /// An act of the window itself, run by the shell.
    Shell(ShellAct<Wview>),
    /// A command of the bar of the view in front (`View::tools`).
    Tool(&'static str),
    /// Another file of the folder of the one in front, of a format its plugin opens.
    Browse(Step),
}

impl From<ShellAct<Wview>> for Act {
    fn from(act: ShellAct<Wview>) -> Self {
        Act::Shell(act)
    }
}

/// The viewer.
pub struct Wview {
    plugins: Arc<Plugins>,
    /// The files named on the command line, opened after the session is restored.
    files: Vec<PathBuf>,
}

/// The view of a file by the first plugin that opens its format.
fn open_view(plugins: &Plugins, path: &Path) -> Result<FileDoc, String> {
    let plugin = plugins
        .for_path(path)
        .ok_or_else(|| trf("No plugin opens {name}", &[("name", &file_name(path))]))?;
    let view = plugins.loaded[plugin]
        .plugin
        .open(path)
        .map_err(|e| format!("{}: {e}", file_name(path)))?;
    Ok(FileDoc {
        path: path.to_path_buf(),
        plugin,
        view,
    })
}

impl Wview {
    pub fn new(plugins: Arc<Plugins>, files: Vec<PathBuf>) -> Self {
        Self { plugins, files }
    }

    /// The file a step leads to from the one in front, among the files of its folder its plugin
    /// opens, by their names; it takes the place of that file in its tab.
    fn browse(&mut self, shell: &mut Shell<Self>, step: Step) {
        let Some(Doc::File(doc)) = shell.active_doc() else {
            return;
        };
        let (path, plugin) = (doc.path.clone(), doc.plugin);
        let Some(dir) = path.parent() else {
            return;
        };
        let loaded = &self.plugins.loaded[plugin];
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && loaded.opens(p))
                    .collect()
            })
            .unwrap_or_default();
        files.sort_by_key(|p| file_name(p).to_lowercase());
        let Some(at) = files.iter().position(|p| *p == path) else {
            return;
        };
        let last = files.len() - 1;
        let next = match step {
            Step::First => 0,
            Step::Previous => at.saturating_sub(1),
            Step::Next => (at + 1).min(last),
            Step::Last => last,
        };
        if next == at {
            shell.status = trf(
                "{name} is the only one this way ({n} of {count})",
                &[
                    ("name", &file_name(&path)),
                    ("n", &(at + 1)),
                    ("count", &files.len()),
                ],
            );
            return;
        }
        match open_view(&self.plugins, &files[next]) {
            Ok(doc) => {
                let active = shell.active;
                shell.docs[active] = Doc::File(doc);
                shell.status = trf(
                    "{n} of {count}",
                    &[("n", &(next + 1)), ("count", &files.len())],
                );
            }
            Err(e) => shell.status = e,
        }
    }

    /// The page without a file: the plugins and what they open.
    fn plugins_page(&self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("wview");
                ui.label(tr(
                    "Open a file (Ctrl+O) or drop one on the window: the plugin of its format shows it.",
                ));
                ui.add_space(8.0);
                for loaded in &self.plugins.loaded {
                    ui.strong(loaded.plugin.name());
                    ui.weak(loaded.file.display().to_string());
                    for format in loaded.plugin.formats() {
                        ui.label(format!("{}: {}", format.name, format.extensions.join(", ")));
                    }
                    ui.add_space(6.0);
                }
                for (file, why) in &self.plugins.failed {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("{}: {why}", file.display()),
                    );
                }
                ui.add_space(6.0);
                ui.weak(plugin::runtime());
            });
        });
    }
}

impl App for Wview {
    type Doc = Doc;
    type Act = Act;
    type Extra = Extra;

    const NAME: &'static str = "wview";
    const NEW_HINT: &'static str = "Open a file (Ctrl+O)";

    fn shell_doc(doc: ShellDoc) -> Doc {
        Doc::Shell(doc)
    }

    fn shell_act(act: Act) -> Result<ShellAct<Self>, Act> {
        match act {
            Act::Shell(a) => Ok(a),
            other => Err(other),
        }
    }

    /// The files of the command line, in tabs.
    fn restored(&mut self, shell: &mut Shell<Self>) {
        for path in std::mem::take(&mut self.files) {
            self.open_path(shell, path);
        }
    }

    /// A viewer makes no file: New opens one.
    fn new_document(&mut self, shell: &mut Shell<Self>) {
        shell.pending.push(Act::Shell(ShellAct::Open));
    }

    fn open_path(&mut self, shell: &mut Shell<Self>, path: PathBuf) {
        let plugins = self.plugins.clone();
        shell.open_file(
            &path,
            |d| matches!(d, Doc::File(f) if f.path == path),
            |p| open_view(&plugins, p),
            |doc, _| Doc::File(doc),
        );
    }

    fn open_dialog(&self, _shell: &Shell<Self>) -> wwrt::ww::rfd::FileDialog {
        let mut every: Vec<&str> = Vec::new();
        let mut dialog = wwrt::ww::rfd::FileDialog::new().set_title(tr("Open"));
        for loaded in &self.plugins.loaded {
            for format in loaded.plugin.formats() {
                every.extend(format.extensions.iter().copied());
            }
        }
        if !every.is_empty() {
            dialog = dialog.add_filter(tr("Every format of the plugins"), every.as_slice());
        }
        for loaded in &self.plugins.loaded {
            for format in loaded.plugin.formats() {
                dialog = dialog.add_filter(format.name, format.extensions);
            }
        }
        dialog.add_filter(tr("All files"), &["*"])
    }

    fn save(&mut self, shell: &mut Shell<Self>, _ask: bool) {
        shell.status = tr("A viewer saves nothing").to_owned();
    }

    fn about(&self, shell: &Shell<Self>) -> Vec<String> {
        let mut lines = shell.about_lines(VERSION);
        lines.push(plugin::runtime().to_owned());
        lines.push(trf("{n} plugins", &[("n", &self.plugins.loaded.len())]));
        lines
    }

    fn draw(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui, _acts: &mut Vec<Act>) {
        let host = Host {
            wrap_lines: shell.settings.wrap_lines,
            page_width: shell.settings.page_width,
        };
        match shell.active_doc_mut() {
            Some(Doc::File(doc)) => doc.view.ui(ui, &host),
            Some(Doc::Shell(_)) => {}
            None => self.plugins_page(ui),
        }
    }

    fn commands(&self, shell: &Shell<Self>) -> Vec<Item<Act>> {
        let open = keyed(icons::OPEN, tr("Open"), &keys::OPEN, ShellAct::Open);
        match shell.active_doc() {
            Some(Doc::File(doc)) => {
                let mut items = vec![open];
                let tools = doc.view.tools();
                if !tools.is_empty() {
                    items.push(Item::Separator);
                }
                for tool in tools {
                    items.push(cmd(tool.icon, tool.label, Act::Tool(tool.id)));
                }
                if doc.view.browses() {
                    items.push(Item::Separator);
                    items.push(cmd(
                        icons::BACK,
                        tr("Previous"),
                        Act::Browse(Step::Previous),
                    ));
                    items.push(cmd(icons::FORWARD, tr("Next"), Act::Browse(Step::Next)));
                }
                items
            }
            Some(Doc::Shell(_)) => Vec::new(),
            None => vec![open],
        }
    }

    fn menu(&self, shell: &Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        menu_item(
            ui,
            icons::OPEN,
            tr("Open..."),
            keys::OPEN.text,
            Act::Shell(ShellAct::Open),
            acts,
        );
        menu_item(
            ui,
            icons::CLOSE,
            tr("Close tab"),
            keys::CLOSE.text,
            Act::Shell(ShellAct::CloseTab(shell.active, false)),
            acts,
        );
        ui.separator();
        menu_item(
            ui,
            icons::GEAR,
            tr("Settings"),
            keys::SETTINGS.text,
            Act::Shell(ShellAct::OpenSettings),
            acts,
        );
        menu_item(
            ui,
            icons::INFO,
            tr("About"),
            "",
            Act::Shell(ShellAct::About),
            acts,
        );
        menu_item(
            ui,
            icons::EXIT,
            tr("Exit"),
            "",
            Act::Shell(ShellAct::Exit),
            acts,
        );
    }

    /// The plugin of the tab and what its view says.
    fn status_left(&self, shell: &Shell<Self>) -> Vec<String> {
        let Some(Doc::File(doc)) = shell.active_doc() else {
            return Vec::new();
        };
        vec![
            self.plugins.loaded[doc.plugin].plugin.name().to_owned(),
            doc.view.status(),
        ]
    }

    /// The arrows, the page keys, Home and End step through the folder of an image.
    fn shortcuts(&self, shell: &Shell<Self>, input: &mut egui::InputState) -> Vec<Act> {
        let browses = matches!(shell.active_doc(), Some(Doc::File(d)) if d.view.browses());
        if !browses {
            return Vec::new();
        }
        let keys = [
            (Key::ArrowLeft, Step::Previous),
            (Key::PageUp, Step::Previous),
            (Key::ArrowRight, Step::Next),
            (Key::PageDown, Step::Next),
            (Key::Home, Step::First),
            (Key::End, Step::Last),
        ];
        keys.iter()
            .filter(|(key, _)| input.consume_key(Modifiers::NONE, *key))
            .map(|(_, step)| Act::Browse(*step))
            .collect()
    }

    fn apply(&mut self, shell: &mut Shell<Self>, act: Act) {
        match act {
            Act::Tool(id) => {
                if let Some(Doc::File(doc)) = shell.active_doc_mut() {
                    doc.view.act(id);
                }
            }
            Act::Browse(step) => self.browse(shell, step),
            // The window runs its own acts (`shell_act`); one that comes here goes back to it.
            Act::Shell(a) => shell.pending.push(Act::Shell(a)),
        }
    }
}
