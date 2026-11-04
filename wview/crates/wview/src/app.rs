//! wview on the window of W#: a tab a file, drawn by the plugin that opens its format; without a
//! file, the page of the plugins; the guide; the section Plugins of the settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wwrt::egui;
use wwrt::plugin::{self, Host, Plugins, Request, Step, View};
use wwrt::serde::{Deserialize, Serialize};
use wwrt::serde_json::Value;
use wwrt::ww::commands::{Item, cmd, keyed};
use wwrt::ww::i18n::{self, tr, trf};
use wwrt::ww::keys;
use wwrt::ww::mdview::{self, MdDoc};
use wwrt::ww::shell::menu_item;
use wwrt::ww::{App, Document, Draft, Search, Shell, ShellAct, ShellDoc, icons};

/// The version of the program, for About and the log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The translations of the window: the texts of wview and of the shell of W#.
pub(crate) static LANGUAGES: [(&str, &str); 8] = [
    ("uk", include_str!("../lang/uk/ui.json")),
    ("ru", include_str!("../lang/ru/ui.json")),
    ("de", include_str!("../lang/de/ui.json")),
    ("es", include_str!("../lang/es/ui.json")),
    ("fr", include_str!("../lang/fr/ui.json")),
    ("id", include_str!("../lang/id/ui.json")),
    ("it", include_str!("../lang/it/ui.json")),
    ("tr", include_str!("../lang/tr/ui.json")),
];

/// The guide, English and its translations.
pub(crate) const GUIDE: &str = include_str!("../docs/guide.md");
pub(crate) static GUIDES: [(&str, &str); 8] = [
    ("uk", include_str!("../lang/uk/guide.md")),
    ("ru", include_str!("../lang/ru/guide.md")),
    ("de", include_str!("../lang/de/guide.md")),
    ("es", include_str!("../lang/es/guide.md")),
    ("fr", include_str!("../lang/fr/guide.md")),
    ("id", include_str!("../lang/id/guide.md")),
    ("it", include_str!("../lang/it/guide.md")),
    ("tr", include_str!("../lang/tr/guide.md")),
];

/// The fields of wview in the settings file: the settings of every plugin, by the name of its DLL.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(crate = "wwrt::serde")]
pub struct Extra {
    #[serde(default)]
    pub plugins: BTreeMap<String, Value>,
}

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
    /// The guide.
    Page(MdDoc),
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
            Doc::Page(d) => d.title(),
            Doc::Shell(d) => d.title(),
        }
    }

    fn dirty(&self) -> bool {
        match self {
            Doc::Shell(d) => d.dirty(),
            _ => false,
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Doc::File(d) => Some(&d.path),
            Doc::Page(d) => d.path(),
            Doc::Shell(d) => d.path(),
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        match self {
            Doc::File(d) => d.view.search(),
            Doc::Page(d) => d.search(),
            Doc::Shell(_) => None,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Doc::File(d) if d.view.browses() => "\u{1F5BC}",
            Doc::File(_) => icons::PAGE,
            Doc::Page(_) => icons::BOOK,
            Doc::Shell(d) => d.icon(),
        }
    }

    fn search_hint(&self) -> &'static str {
        match self {
            Doc::Shell(_) => "",
            _ => "Search the text (Ctrl+F)",
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
            Doc::Shell(d) => d.draft_key(),
            _ => None,
        }
    }

    fn draft_due(&mut self, all: bool) -> Option<Draft> {
        match self {
            Doc::Shell(d) => d.draft_due(all),
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
    Shell(ShellAct<Wview>),
    /// A command of the bar of the view in front (`View::tools`).
    Tool(&'static str),
    /// Another file of the folder of the one in front, of a format its plugin opens.
    Browse(Step),
    /// The guide in a tab.
    Guide,
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

/// The settings of a plugin in the settings of the window, `Null` before the first change.
fn plugin_settings(shell: &Shell<Wview>, plugins: &Plugins, plugin: usize) -> Value {
    shell
        .settings
        .extra
        .plugins
        .get(&plugins.loaded[plugin].id)
        .cloned()
        .unwrap_or(Value::Null)
}

/// The view of a file by the first plugin that opens its format.
fn open_view(
    plugins: &Plugins,
    settings: &Value,
    plugin: usize,
    path: &Path,
) -> Result<FileDoc, String> {
    let view = plugins.loaded[plugin]
        .plugin
        .open(path, settings)
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
        let count = files.len();
        if next == at {
            shell.status = trf("{n} of {count}", &[("n", &(at + 1)), ("count", &count)]);
            return;
        }
        let settings = plugin_settings(shell, &self.plugins, plugin);
        match open_view(&self.plugins, &settings, plugin, &files[next]) {
            Ok(doc) => {
                let active = shell.active;
                shell.docs[active] = Doc::File(doc);
                shell.status = trf("{n} of {count}", &[("n", &(next + 1)), ("count", &count)]);
            }
            Err(e) => shell.status = e,
        }
    }

    /// The guide in the language of the window, in a tab of its own.
    fn guide(&self, shell: &mut Shell<Self>) {
        if shell.focus_tab(|d| matches!(d, Doc::Page(p) if p.page == Some("guide"))) {
            return;
        }
        let code = i18n::current();
        let text = GUIDES
            .iter()
            .find(|(c, _)| *c == code)
            .map_or(GUIDE, |(_, t)| *t);
        shell.open_tab(Doc::Page(MdDoc::page(
            "guide",
            tr("Guide"),
            text.to_owned(),
        )));
    }

    /// The plugins, what they open and the DLLs that did not load.
    fn plugins_list(&self, ui: &mut egui::Ui) {
        for loaded in &self.plugins.loaded {
            ui.strong(tr(loaded.plugin.name()));
            ui.weak(loaded.file.display().to_string());
            for format in loaded.plugin.formats() {
                ui.label(format!(
                    "{}: {}",
                    tr(format.name),
                    format.extensions.join(", ")
                ));
            }
            ui.add_space(6.0);
        }
        for (file, why) in &self.plugins.failed {
            ui.colored_label(
                ui.visuals().error_fg_color,
                trf(
                    "Not loaded: {file}: {why}",
                    &[("file", &file.display()), ("why", why)],
                ),
            );
        }
        ui.add_space(6.0);
        ui.weak(plugin::runtime());
    }

    /// The page without a file.
    fn plugins_page(&self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("wview");
                ui.label(tr(
                    "Open a file (Ctrl+O) or drop one on the window: the plugin of its format shows it.",
                ));
                ui.add_space(8.0);
                self.plugins_list(ui);
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

    fn languages() -> &'static [(&'static str, &'static str)] {
        &LANGUAGES
    }

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
        let Some(plugin) = self.plugins.for_path(&path) else {
            shell.status = trf("No plugin opens {name}", &[("name", &file_name(&path))]);
            return;
        };
        let plugins = self.plugins.clone();
        let settings = plugin_settings(shell, &plugins, plugin);
        shell.open_file(
            &path,
            |d| matches!(d, Doc::File(f) if f.path == path),
            |p| open_view(&plugins, &settings, plugin, p),
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
                dialog = dialog.add_filter(tr(format.name), format.extensions);
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

    fn draw(&mut self, shell: &mut Shell<Self>, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let (wrap_lines, page_width) = (shell.settings.wrap_lines, shell.settings.page_width);
        let plugin = match shell.active_doc() {
            Some(Doc::File(doc)) => Some(doc.plugin),
            _ => None,
        };
        let settings = plugin
            .map(|p| plugin_settings(shell, &self.plugins, p))
            .unwrap_or(Value::Null);
        match shell.active_doc_mut() {
            Some(Doc::File(doc)) => {
                let mut host = Host {
                    wrap_lines,
                    page_width,
                    settings: &settings,
                    requests: Vec::new(),
                };
                doc.view.ui(ui, &mut host);
                for request in host.requests {
                    match request {
                        Request::Browse(step) => acts.push(Act::Browse(step)),
                    }
                }
            }
            Some(Doc::Page(doc)) => {
                egui::CentralPanel::default()
                    .show(ui, |ui| mdview::md_view(ui, doc, wrap_lines, page_width));
            }
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
                    items.push(cmd(tool.icon, tr(tool.label), Act::Tool(tool.id)));
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
            Some(Doc::Page(_)) | None => vec![open],
            Some(Doc::Shell(_)) => Vec::new(),
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
        menu_item(ui, icons::BOOK, tr("Guide"), "", Act::Guide, acts);
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
            tr(self.plugins.loaded[doc.plugin].plugin.name()).to_owned(),
            doc.view.status(),
        ]
    }

    fn settings_sections(&self) -> Vec<&'static str> {
        vec!["Plugins"]
    }

    /// The plugins with their settings, each in a card, then the DLLs that did not load.
    fn settings_section(
        &mut self,
        shell: &mut Shell<Self>,
        _section: usize,
        ui: &mut egui::Ui,
        _acts: &mut Vec<Act>,
    ) {
        for loaded in &self.plugins.loaded {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.strong(tr(loaded.plugin.name()));
                ui.weak(loaded.file.display().to_string());
                ui.add_space(4.0);
                if loaded.plugin.has_settings() {
                    let mut settings = shell
                        .settings
                        .extra
                        .plugins
                        .get(&loaded.id)
                        .cloned()
                        .unwrap_or(Value::Null);
                    if loaded.plugin.settings_ui(ui, &mut settings) {
                        shell
                            .settings
                            .extra
                            .plugins
                            .insert(loaded.id.clone(), settings);
                        shell.settings_dirty = true;
                    }
                } else {
                    ui.weak(tr("The plugin has no settings."));
                }
            });
            ui.add_space(6.0);
        }
        for (file, why) in &self.plugins.failed {
            ui.colored_label(
                ui.visuals().error_fg_color,
                trf(
                    "Not loaded: {file}: {why}",
                    &[("file", &file.display()), ("why", why)],
                ),
            );
        }
        ui.add_space(6.0);
        ui.weak(plugin::runtime());
    }

    fn apply(&mut self, shell: &mut Shell<Self>, act: Act) {
        match act {
            Act::Tool(id) => {
                if let Some(Doc::File(doc)) = shell.active_doc_mut() {
                    doc.view.act(id);
                }
            }
            Act::Browse(step) => self.browse(shell, step),
            Act::Guide => self.guide(shell),
            // The window runs its own acts (`shell_act`); one that comes here goes back to it.
            Act::Shell(a) => shell.pending.push(Act::Shell(a)),
        }
    }
}
