//! The MCP tab (`docs/spec/01-ui.md` section 11): the configuration file it edits
//! (`mcp/<name>.wwmcp`, saved at every change), the HTTP server of the window, its mode and
//! switches, the token, the configuration for clients, the workspace of the server, the journal
//! that the search of the tab filters, and at its right the details of the selected call with the
//! places it names. The server does not depend on the open tabs.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_mcp::config::{Config, Transport};
use winweaver_mcp::http::HttpServer;
use winweaver_mcp::{Entry, Mode, RegistryAccess, Server};

use crate::i18n::{tr, trf};
use crate::ShellAct;
use crate::{
    Act, CatalogAct, McpAct, RegistryAct, list, matches_words, style, tree, views,
};

/// A server the window runs.
pub struct McpRun {
    pub server: Arc<Server>,
    pub http: HttpServer,
}

/// The MCP tab: its search of the journal. The server itself is the `McpPanel` of the
/// window, since it runs with the tab closed too.
#[derive(Default)]
pub struct McpDoc {
    pub search: crate::Search,
}

/// The configuration the tab edits, the server it runs and the state of its journal.
pub struct McpPanel {
    /// The configuration file, saved at every change.
    pub path: PathBuf,
    /// The configuration as last saved, so that a change is written once.
    pub saved: Option<Config>,
    /// How a start without the window serves (`winweaver --mcp FILE`); the window serves HTTP.
    pub transport: Transport,
    pub port: u16,
    pub token: String,
    pub mode: Mode,
    pub read_pc: bool,
    /// What the tools of the registry may do.
    pub registry: RegistryAccess,
    /// Whether the tools may change the bookmarks and the catalog files.
    pub edit_library: bool,
    /// The preset or profile file the workspace starts with; empty for the first preset.
    pub profile: String,
    /// Whether the log keeps every request and answer whole, without secrets.
    pub detailed_log: bool,
    pub run: Option<McpRun>,
    pub error: String,
    /// The selected entry of the journal, by its place in the journal.
    pub journal_sel: Option<usize>,
    /// The selection moved with the keys: it comes into view at the next frame.
    pub journal_follow: bool,
    /// The search was cleared: the selection comes to the middle of the list.
    pub journal_reveal: bool,
    /// Whether the panel of the details of the selected call is open.
    pub detail_open: bool,
}

impl McpPanel {
    /// The tab of a configuration file, with the defaults until it is read.
    pub fn new(path: PathBuf) -> Self {
        let mut panel = Self {
            path,
            saved: None,
            transport: Transport::Http,
            port: 0,
            token: String::new(),
            mode: Mode::Read,
            read_pc: false,
            registry: RegistryAccess::Off,
            edit_library: false,
            profile: String::new(),
            detailed_log: false,
            run: None,
            error: String::new(),
            journal_sel: None,
            journal_follow: false,
            journal_reveal: false,
            detail_open: true,
        };
        panel.take(&Config::default());
        panel
    }

    /// The configuration the fields hold.
    pub fn config(&self) -> Config {
        Config {
            transport: self.transport,
            port: self.port,
            token: self.token.clone(),
            mode: self.mode,
            read_pc: self.read_pc,
            registry: self.registry,
            edit_library: self.edit_library,
            profile: self.profile.clone(),
            detailed_log: self.detailed_log,
            ..Config::default()
        }
    }

    /// Puts a configuration into the fields.
    pub fn take(&mut self, config: &Config) {
        self.transport = config.transport;
        self.port = config.port;
        self.token = config.token.clone();
        self.mode = config.mode;
        self.read_pc = config.read_pc;
        self.registry = config.registry;
        self.edit_library = config.edit_library;
        self.profile = config.profile.clone();
        self.detailed_log = config.detailed_log;
    }
}

/// The configuration a client needs for the HTTP server of the window.
pub fn http_config(url: &str, token: &str) -> String {
    format!(
        "{{\n  \"mcpServers\": {{\n    \"winweaver\": {{\n      \"type\": \"http\",\n      \"url\": \"{url}\",\n      \"headers\": {{ \"Authorization\": \"Bearer {token}\" }}\n    }}\n  }}\n}}"
    )
}

/// The configuration a client needs to start the program itself on stdio with a configuration
/// file.
pub fn stdio_config(exe: &str, config: &str) -> String {
    let value = serde_json::json!({
        "mcpServers": {
            "winweaver": {
                "command": exe,
                "args": ["--mcp", "stdio", "--config", config],
            }
        }
    });
    serde_json::to_string_pretty(&value).unwrap_or_default()
}

fn masked(token: &str) -> String {
    if token.len() < 8 {
        return "***".to_owned();
    }
    format!("{}...{}", &token[..4], &token[token.len() - 4..])
}

pub fn mcp_view(ui: &mut egui::Ui, panel: &mut McpPanel, acts: &mut Vec<Act>) {
    let presets: Vec<&str> = crate::PRESETS.iter().map(|(name, _)| *name).collect();
    crate::views::header(ui, |ui| {
        ui.label(style::strong(ui, tr("MCP server"), 18.0));
        ui.label(style::muted(ui, tr("AI agents read and, when the mode allows, edit profiles through this server. It has its own workspace of profiles and never applies settings to this PC."), 12.0));
    });
    crate::views::block(ui, |ui| server_card(ui, panel, &presets, acts));
    if panel.run.is_some() {
        crate::views::block(ui, |ui| workspace_card(ui, panel, acts));
    }
    ui.add_space(12.0);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(1));
}

/// The server: its state, port, mode, the read of this PC, the token and the configurations.
fn server_card(ui: &mut egui::Ui, panel: &mut McpPanel, presets: &[&str], acts: &mut Vec<Act>) {
    style::section_title(ui, tr("Server"));
    ui.add_space(4.0);
    let running = panel.run.is_some();
    egui::Grid::new("mcp-grid")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("Configuration"));
            ui.horizontal_wrapped(|ui| {
                ui.monospace(panel.path.display().to_string());
                if ui
                    .add_enabled(!running, egui::Button::new(tr("Open...")))
                    .clicked()
                {
                    acts.push(Act::Shell(ShellAct::Open));
                }
                if ui.button(tr("Save as...")).clicked() {
                    acts.push(Act::Shell(ShellAct::SaveAs));
                }
            });
            ui.end_row();

            ui.label(tr("State"));
            match &panel.run {
                Some(run) => {
                    let client = run.server.client();
                    ui.label(trf(
                        "Running on {url}; client {client}",
                        &[
                            ("url", &run.http.url()),
                            (
                                "client",
                                &if client.is_empty() {
                                    "-".to_owned()
                                } else {
                                    client
                                },
                            ),
                        ],
                    ));
                }
                None => {
                    ui.label(tr("Stopped"));
                }
            }
            ui.end_row();

            ui.label(tr("Port"));
            ui.add_enabled(
                !running,
                egui::DragValue::new(&mut panel.port).range(1024..=65535),
            );
            ui.end_row();

            ui.label(tr("Mode"));
            ui.horizontal(|ui| {
                let before = panel.mode;
                ui.radio_value(&mut panel.mode, Mode::Read, tr("read: nothing changes"));
                ui.radio_value(
                    &mut panel.mode,
                    Mode::Edit,
                    tr("edit: profiles change in memory"),
                );
                ui.radio_value(
                    &mut panel.mode,
                    Mode::Files,
                    tr("files: profiles and the answer file are written"),
                );
                if before != panel.mode
                    && let Some(run) = &panel.run
                {
                    run.server.set_mode(panel.mode);
                }
            });
            ui.end_row();

            ui.label(tr("This PC"));
            if ui
                .checkbox(
                    &mut panel.read_pc,
                    tr("Allow reading the settings of this PC"),
                )
                .changed()
                && let Some(run) = &panel.run
            {
                run.server.set_read_pc(panel.read_pc);
            }
            ui.end_row();

            ui.label(tr("Registry"));
            ui.vertical(|ui| {
                use winweaver_mcp::RegistryAccess;
                let mut read = panel.registry != RegistryAccess::Off;
                let mut write = panel.registry == RegistryAccess::Write;
                let a = ui
                    .checkbox(&mut read, tr("Allow reading the registry of this PC"))
                    .changed();
                let b = ui
                    .add_enabled(
                        read,
                        egui::Checkbox::new(&mut write, tr("Allow changing the registry too")),
                    )
                    .changed();
                if a || b {
                    panel.registry = match (read, write) {
                        (false, _) => RegistryAccess::Off,
                        (true, false) => RegistryAccess::Read,
                        (true, true) => RegistryAccess::Write,
                    };
                    if let Some(run) = &panel.run {
                        run.server.set_registry(panel.registry);
                    }
                }
            });
            ui.end_row();

            ui.label(tr("Bookmarks and catalogs"));
            if ui
                .checkbox(
                    &mut panel.edit_library,
                    tr("Allow changing the bookmarks and the catalogs"),
                )
                .changed()
                && let Some(run) = &panel.run
            {
                run.server.set_edit_library(panel.edit_library);
            }
            ui.end_row();

            ui.label(tr("Profile at the start"));
            let first = tr("The first preset");
            let shown = if panel.profile.is_empty() {
                first.to_owned()
            } else {
                panel.profile.clone()
            };
            ui.add_enabled_ui(!running, |ui| {
                egui::ComboBox::from_id_salt("mcp-profile")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut panel.profile, String::new(), first);
                        for name in presets {
                            ui.selectable_value(&mut panel.profile, (*name).to_owned(), *name);
                        }
                    });
            });
            ui.end_row();

            ui.label(tr("Log"));
            if ui
                .checkbox(
                    &mut panel.detailed_log,
                    tr("Detailed: every request and answer whole, without passwords, keys and tokens"),
                )
                .changed()
                && let Some(run) = &panel.run
            {
                run.server.set_detailed_log(panel.detailed_log);
            }
            ui.end_row();

            ui.label(tr("Without the window"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut panel.transport, Transport::Http, "HTTP");
                ui.radio_value(&mut panel.transport, Transport::Stdio, "stdio");
                ui.weak(tr("for winweaver --mcp with this file"));
            });
            ui.end_row();

            ui.label(tr("Token"));
            ui.horizontal(|ui| {
                ui.monospace(masked(&panel.token));
                if ui.button(tr("Copy the client configuration")).clicked() {
                    let url = format!("http://127.0.0.1:{}/mcp", panel.port);
                    ui.ctx().copy_text(http_config(&url, &panel.token));
                }
                if ui
                    .add_enabled(!running, egui::Button::new(tr("Renew")))
                    .clicked()
                {
                    acts.push(crate::question::renew_token().into());
                }
            });
            ui.end_row();
        });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if running {
            if ui.button(tr("Stop")).clicked() {
                acts.push(Act::Mcp(McpAct::Stop));
            }
        } else if ui.button(tr("Start")).clicked() {
            acts.push(Act::Mcp(McpAct::Start));
        }
        if ui
            .button(tr("Copy the stdio configuration"))
            .on_hover_text(tr(
                "For clients that start the program themselves; it needs no running server",
            ))
            .clicked()
        {
            let exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "winweaver.exe".to_owned());
            let file = panel.path.display().to_string();
            ui.ctx().copy_text(stdio_config(&exe, &file));
        }
    });
    if !panel.error.is_empty() {
        ui.colored_label(ui.visuals().warn_fg_color, &panel.error);
    }
    ui.weak(tr("The copied configuration holds the token; the clipboard history of Windows keeps what is copied."));
    if let Some(file) = winweaver_mcp::log::file_of_today() {
        ui.weak(trf(
            "The log of the server: {path}",
            &[("path", &file.display())],
        ));
    }
}

/// The documents of the workspace of the running server.
fn workspace_card(ui: &mut egui::Ui, panel: &McpPanel, acts: &mut Vec<Act>) {
    let Some(run) = &panel.run else {
        return;
    };
    style::section_title(ui, tr("Workspace of the server"));
    ui.add_space(4.0);
    for (name, dirty, file) in run.server.workspace() {
        ui.horizontal(|ui| {
            ui.label(&name);
            if dirty {
                ui.weak(tr("changed"));
            }
            if !file.is_empty() {
                ui.weak(file);
            }
            if ui.small_button(tr("Open in a tab")).clicked() {
                acts.push(Act::Mcp(McpAct::OpenProfile(name.clone())));
            }
        });
    }
}

/// The journal of the running server, the newest call first, as the search of the tab filters it.
pub fn journal_view(ui: &mut egui::Ui, panel: &mut McpPanel, search: &crate::Search) {
    let Some(run) = &panel.run else {
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(18, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                style::section_title(ui, tr("Journal"));
                let label = if panel.detail_open {
                    tr("Hide the details")
                } else {
                    tr("Show the details")
                };
                if ui.small_button(label).clicked() {
                    panel.detail_open = !panel.detail_open;
                }
            });
        });
    let words = search.words();
    let entries = run.server.journal();
    let rows: Vec<list::Row<usize>> = entries
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, e)| {
            let hay = format!(
                "{} {} {} {} {}",
                e.time, e.client, e.method, e.detail, e.result
            );
            matches_words(&hay, &words)
        })
        .map(|(i, e)| {
            let result = list::Cell::new(e.result.name());
            list::Row {
                key: i,
                cells: vec![
                    list::Cell::new(e.time.get(11..19).unwrap_or(&e.time)).muted(),
                    list::Cell::new(&e.client).muted(),
                    list::Cell::new(&e.method).strong(),
                    list::Cell::new(&e.detail).muted(),
                    if e.result.is_ok() {
                        result
                    } else {
                        result.warn()
                    },
                ],
                button: None,
            }
        })
        .collect();
    let columns = [
        list::Column::new(tr("Time"), 90.0),
        list::Column::new(tr("Client"), 150.0),
        list::Column::new(tr("Call"), 170.0),
        list::Column::new(tr("Arguments"), 320.0),
        list::Column::new(tr("Result"), 120.0),
    ];
    let selected = panel
        .journal_sel
        .and_then(|s| rows.iter().position(|r| r.key == s));
    let scroll = if panel.journal_reveal {
        tree::Scroll::Center
    } else if panel.journal_follow {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    let out = list::show(
        ui,
        egui::Id::new("mcp-journal"),
        tr("Journal"),
        &columns,
        &rows,
        selected,
        scroll,
    );
    for event in out.events {
        if let list::Event::Select(i) | list::Event::Open(i) = event {
            panel.journal_sel = Some(i);
        }
    }
    if out.moved {
        panel.journal_follow = true;
    } else if out.scrolled {
        panel.journal_follow = false;
        panel.journal_reveal = false;
    }
}

/// A place a call names: a rule of the catalog, a profile of the workspace, a catalog file of
/// the imports or a key of the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Rule(String),
    Profile(String),
    Catalog(String),
    Key(String),
}

/// The places the arguments and the answer of a call name, each once, at most 40.
pub fn places(
    entry: &Entry,
    catalog: &Catalog,
    imports: &[String],
    profiles: &[String],
) -> Vec<Place> {
    fn texts<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
        if out.len() >= 4000 {
            return;
        }
        match value {
            Value::String(s) => out.push(s),
            Value::Array(items) => items.iter().for_each(|v| texts(v, out)),
            Value::Object(map) => map.values().for_each(|v| texts(v, out)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    texts(&entry.arguments, &mut found);
    texts(&entry.answer, &mut found);
    let mut out: Vec<Place> = Vec::new();
    for text in found {
        let place = if catalog.rule(text).is_some() {
            Place::Rule(text.to_owned())
        } else if profiles.iter().any(|p| p == text) {
            Place::Profile(text.to_owned())
        } else if imports.iter().any(|i| i == text) {
            Place::Catalog(text.to_owned())
        } else if text.contains('\\')
            && let Some(key) = winweaver_core::registry::normalize(text)
        {
            Place::Key(key)
        } else {
            continue;
        };
        if !out.contains(&place) {
            out.push(place);
        }
        if out.len() >= 40 {
            break;
        }
    }
    out
}

/// The details of the selected call at the right of the journal: its arguments and answer, and
/// a button for every place it names.
pub fn detail_view(
    ui: &mut egui::Ui,
    panel: &McpPanel,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[String],
    acts: &mut Vec<Act>,
) {
    let Some(run) = &panel.run else {
        return;
    };
    style::section_title(ui, tr("Details of the call"));
    ui.add_space(4.0);
    let Some(entry) = panel.journal_sel.and_then(|i| run.server.journal_entry(i)) else {
        ui.weak(tr("Select a call in the journal."));
        return;
    };
    let profiles: Vec<String> = run.server.workspace().into_iter().map(|w| w.0).collect();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Grid::new("mcp-detail")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    let rows = [
                        (tr("Call"), entry.method.clone()),
                        (tr("Time"), entry.time.clone()),
                        (tr("Client"), entry.client.clone()),
                        (tr("Result"), entry.result.to_string()),
                        (tr("Duration"), trf("{ms} ms", &[("ms", &entry.ms)])),
                    ];
                    for (name, value) in rows {
                        ui.weak(name);
                        ui.label(value);
                        ui.end_row();
                    }
                });
            let places = places(&entry, catalog, imports, &profiles);
            if !places.is_empty() {
                ui.add_space(8.0);
                style::section_title(ui, tr("Places it names"));
                ui.horizontal_wrapped(|ui| {
                    for place in places {
                        let (text, act) = match place {
                            // A rule shows its title, its id in the hint.
                            Place::Rule(id) => {
                                let title = catalog
                                    .rule(&id)
                                    .map_or(id.as_str(), |r| views::rtitle(texts, r));
                                let shown = ui.small_button(title).on_hover_text(&id);
                                if shown.clicked() {
                                    acts.push(views::RuleLink::Elsewhere.act(&id));
                                }
                                continue;
                            }
                            Place::Profile(name) => {
                                (name.clone(), Act::Mcp(McpAct::OpenProfile(name)))
                            }
                            Place::Catalog(id) => (id.clone(), Act::Catalog(CatalogAct::Open(id))),
                            Place::Key(key) => {
                                (key.clone(), Act::Registry(RegistryAct::Open(key, None)))
                            }
                        };
                        if ui.small_button(text).clicked() {
                            acts.push(act);
                        }
                    }
                });
            }
            for (title, value) in [
                (tr("Arguments"), &entry.arguments),
                (tr("Answer"), &entry.answer),
            ] {
                ui.add_space(8.0);
                style::section_title(ui, title);
                let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(true),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configurations_are_json() {
        let http: serde_json::Value =
            serde_json::from_str(&http_config("http://127.0.0.1:1/mcp", "T")).unwrap();
        assert_eq!(
            http["mcpServers"]["winweaver"]["headers"]["Authorization"],
            "Bearer T"
        );
        let config = stdio_config("C:\\W\\winweaver.exe", "C:\\W\\mcp\\a.wwmcp");
        let stdio: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            stdio["mcpServers"]["winweaver"]["command"],
            "C:\\W\\winweaver.exe"
        );
        assert_eq!(
            stdio["mcpServers"]["winweaver"]["args"][3],
            "C:\\W\\mcp\\a.wwmcp"
        );
        assert_eq!(masked("abcdefghijkl"), "abcd...ijkl");
    }

    #[test]
    fn a_panel_keeps_its_configuration_and_a_call_names_its_places() {
        let mut panel = McpPanel::new(PathBuf::from("x.wwmcp"));
        let config = Config {
            mode: Mode::Files,
            profile: "Strict".to_owned(),
            detailed_log: true,
            ..Config::default()
        };
        panel.take(&config);
        assert_eq!(panel.config(), config);
        let catalog = Catalog::embedded().unwrap();
        let rule = catalog.rules()[0].id.clone();
        let entry = Entry {
            time: String::new(),
            client: String::new(),
            method: "get_rule".to_owned(),
            detail: String::new(),
            result: winweaver_mcp::Outcome::Ok,
            arguments: serde_json::json!({ "id": rule, "path": "HKLM\\Software\\X" }),
            answer: serde_json::json!({ "profile": "Office", "catalog": "test" }),
            ms: 1,
        };
        let found = places(
            &entry,
            &catalog,
            &["test".to_owned()],
            &["Office".to_owned()],
        );
        assert_eq!(
            found,
            vec![
                Place::Rule(rule),
                Place::Key("HKEY_LOCAL_MACHINE\\Software\\X".to_owned()),
                Place::Profile("Office".to_owned()),
                Place::Catalog("test".to_owned()),
            ]
        );
    }
}
