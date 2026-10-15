//! The MCP tab (`docs/spec/01-ui.md` section 11): the HTTP server of the window, its mode, the
//! option to read this PC, the token, the configuration for clients, the workspace of the server
//! and the journal, a list under them that the search of the tab filters. The server does not
//! depend on the open tabs.

use std::sync::Arc;

use eframe::egui;
use winweaver_mcp::http::HttpServer;
use winweaver_mcp::{Mode, Server};

use crate::i18n::{tr, trf};
use crate::{Act, list, style, tree};

/// A server the window runs.
pub struct McpRun {
    pub server: Arc<Server>,
    pub http: HttpServer,
}

/// What the tab shows when no server runs, and the token the next start uses.
pub struct McpPanel {
    pub port: u16,
    pub token: String,
    pub mode: Mode,
    pub read_pc: bool,
    pub run: Option<McpRun>,
    pub error: String,
    /// The selected entry of the journal, by its place in the journal.
    pub journal_sel: Option<usize>,
    /// The selection moved with the keys: it comes into view at the next frame.
    pub journal_follow: bool,
    /// The search was cleared: the selection comes to the middle of the list.
    pub journal_reveal: bool,
}

/// The configuration a client needs for the HTTP server of the window.
pub fn http_config(url: &str, token: &str) -> String {
    format!(
        "{{\n  \"mcpServers\": {{\n    \"winweaver\": {{\n      \"type\": \"http\",\n      \"url\": \"{url}\",\n      \"headers\": {{ \"Authorization\": \"Bearer {token}\" }}\n    }}\n  }}\n}}"
    )
}

/// The configuration a client needs to start the program itself on stdio.
pub fn stdio_config(exe: &str) -> String {
    let exe = exe.replace('\\', "\\\\");
    format!(
        "{{\n  \"mcpServers\": {{\n    \"winweaver\": {{\n      \"command\": \"{exe}\",\n      \"args\": [\"--mcp\", \"stdio\", \"--mode\", \"read\"]\n    }}\n  }}\n}}"
    )
}

fn masked(token: &str) -> String {
    if token.len() < 8 {
        return "***".to_owned();
    }
    format!("{}...{}", &token[..4], &token[token.len() - 4..])
}

pub fn mcp_view(ui: &mut egui::Ui, panel: &mut McpPanel, acts: &mut Vec<Act>) {
    ui.heading(tr("MCP server"));
    ui.weak(tr("AI agents read and, when the mode allows, edit profiles through this server. It has its own workspace of profiles and never applies settings to this PC."));
    ui.add_space(8.0);
    let running = panel.run.is_some();
    egui::Grid::new("mcp-grid")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
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
                    tr("Allow reading the settings of this PC (off at every start)"),
                )
                .changed()
                && let Some(run) = &panel.run
            {
                run.server.set_read_pc(panel.read_pc);
            }
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
                    acts.push(Act::McpRenewToken);
                }
            });
            ui.end_row();
        });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if running {
            if ui.button(tr("Stop")).clicked() {
                acts.push(Act::McpStop);
            }
        } else if ui.button(tr("Start")).clicked() {
            acts.push(Act::McpStart);
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
            ui.ctx().copy_text(stdio_config(&exe));
        }
    });
    if !panel.error.is_empty() {
        ui.colored_label(ui.visuals().warn_fg_color, &panel.error);
    }
    ui.weak(tr("The copied configuration holds the token; the clipboard history of Windows keeps what is copied."));
    let Some(run) = &panel.run else {
        return;
    };
    ui.add_space(10.0);
    ui.strong(tr("Workspace of the server"));
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
                acts.push(Act::McpOpen(name.clone()));
            }
        });
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(1));
}

/// The journal of the running server, the newest call first, as the search of the tab filters it.
pub fn journal_view(ui: &mut egui::Ui, panel: &mut McpPanel, search: &crate::Search) {
    let Some(run) = &panel.run else {
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(18, 8))
        .show(ui, |ui| style::section_title(ui, tr("Journal")));
    let words = search.words();
    let entries = run.server.journal();
    let rows: Vec<list::Row<usize>> = entries
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, e)| {
            let hay = format!("{} {} {} {} {}", e.time, e.client, e.method, e.detail, e.result)
                .to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|(i, e)| {
            let result = list::Cell::new(&e.result);
            list::Row {
                key: i,
                cells: vec![
                    list::Cell::new(e.time.get(11..19).unwrap_or(&e.time)).muted(),
                    list::Cell::new(&e.client).muted(),
                    list::Cell::new(&e.method),
                    list::Cell::new(&e.detail).muted(),
                    if e.result == "ok" { result } else { result.warn() },
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
        let stdio: serde_json::Value =
            serde_json::from_str(&stdio_config("C:\\W\\winweaver.exe")).unwrap();
        assert_eq!(
            stdio["mcpServers"]["winweaver"]["command"],
            "C:\\W\\winweaver.exe"
        );
        assert_eq!(masked("abcdefghijkl"), "abcd...ijkl");
    }
}
