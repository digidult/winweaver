//! The MCP tab (`docs/spec/01-ui.md` section 11): the tab of the MCP server of W# (`wwmcp::ui`:
//! the configuration file it edits, `mcp/<name>.wwmcp` saved at every change, the HTTP server of
//! the window, the mode, the tools with their switches, the options of WinWeaver, the token, the
//! configurations for clients, a call of a tool by hand, the journal that the search of the tab
//! filters and the details of the call selected) with the parts of WinWeaver: the workspace of the
//! server, and the places a call names in its details. The server does not depend on the open
//! tabs.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_mcp::{Paths, Tools};
use wwmcp::{Config, Entry, Server};

use crate::i18n::tr;
use crate::{Act, CatalogAct, McpAct, RegistryAct, style, views};

/// The MCP tab: its search of the journal. The server itself is the `McpPanel` of the window,
/// since it runs with the tab closed too.
#[derive(Default)]
pub struct McpDoc {
    pub search: crate::Search,
}

/// The tab of the server of W# and the tools of WinWeaver it serves, for their workspace.
pub struct McpPanel {
    pub panel: wwmcp::ui::McpPanel,
    pub tools: Arc<Tools>,
}

/// The tools of WinWeaver with the folders of the window.
pub fn tools_of(catalog: &Catalog, marks_path: PathBuf, imports_dir: PathBuf) -> Tools {
    Tools::new(
        catalog.clone(),
        &crate::PRESETS,
        Paths {
            marks_path,
            imports_dir,
            profiles_dir: crate::profiles_dir(),
            output_dir: crate::output_dir(),
        },
    )
}

/// A server of the tools of WinWeaver; one whose profile does not load starts with the first
/// preset and the reason.
fn served(tools: Tools, config: Config) -> (Arc<Server>, Arc<Tools>, Option<String>) {
    let tools = Arc::new(tools);
    let server = Server::new(winweaver_mcp::info(), config).with(tools.clone());
    let profile = server
        .config()
        .source(winweaver_mcp::SOURCE)
        .options
        .get("profile")
        .cloned()
        .unwrap_or_default();
    let failed = if profile.is_empty() {
        None
    } else {
        tools.select(&profile).err()
    };
    (Arc::new(server), tools, failed)
}

impl McpPanel {
    /// The tab of a configuration file, a file of an earlier version written in the format of W#
    /// first.
    pub fn new(path: PathBuf, tools: Tools) -> Self {
        let migrated = winweaver_mcp::legacy::migrate(&path);
        let (server, tools, _) = served(tools, Config::default());
        let mut panel = wwmcp::ui::McpPanel::new(path, "winweaver", server);
        if let Err(e) = migrated {
            panel.error = e;
        }
        Self { panel, tools }
    }

    /// A server of new tools for a start: a workspace of its own, from the profile of the
    /// option `profile`; the reason when that profile does not load.
    pub fn renew(&mut self, tools: Tools) -> Result<(), String> {
        let (server, tools, failed) = served(tools, self.panel.config.clone());
        self.panel.set_server(server);
        self.tools = tools;
        match failed {
            Some(why) => Err(why),
            None => Ok(()),
        }
    }

    pub fn running(&self) -> bool {
        self.panel.running()
    }
}

/// The documents of the workspace of the server.
pub fn workspace_card(ui: &mut egui::Ui, tools: &Tools, acts: &mut Vec<Act>) {
    style::block(ui, |ui| {
        style::section_title(ui, tr("Workspace of the server"));
        ui.add_space(4.0);
        ui.weak(tr("AI agents read and, when the mode allows, edit profiles through this server. It has its own workspace of profiles and never applies settings to this PC."));
        for (name, dirty, file) in tools.workspace() {
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
    });
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

/// The buttons of the places a call names, in its details.
pub fn places_view(
    ui: &mut egui::Ui,
    entry: &Entry,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[String],
    profiles: &[String],
    acts: &mut Vec<Act>,
) {
    let places = places(entry, catalog, imports, profiles);
    if places.is_empty() {
        return;
    }
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
                Place::Profile(name) => (name.clone(), Act::Mcp(McpAct::OpenProfile(name))),
                Place::Catalog(id) => (id.clone(), Act::Catalog(CatalogAct::Open(id))),
                Place::Key(key) => (key.clone(), Act::Registry(RegistryAct::Open(key, None))),
            };
            if ui.small_button(text).clicked() {
                acts.push(act);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_names_its_places() {
        let catalog = Catalog::embedded().unwrap();
        let rule = catalog.rules()[0].id.clone();
        let entry = Entry {
            time: String::new(),
            client: String::new(),
            method: "get_rule".to_owned(),
            detail: String::new(),
            result: wwmcp::Status::Ok,
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

    #[test]
    fn a_panel_reads_an_old_configuration_and_starts_from_its_profile() {
        let dir = std::env::temp_dir().join(format!("winweaver-mcpview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("old.wwmcp");
        std::fs::write(
            &path,
            r#"{"format": "winweaver-mcp", "mode": "edit", "profile": "Strict"}"#,
        )
        .unwrap();
        let catalog = Catalog::embedded().unwrap();
        let tools = || tools_of(&catalog, dir.join("m.wwmarks"), dir.join("imports"));
        let mut panel = McpPanel::new(path.clone(), tools());
        assert!(panel.panel.error.is_empty(), "{}", panel.panel.error);
        assert_eq!(panel.panel.config.mode, wwmcp::Mode::Edit);
        panel.renew(tools()).unwrap();
        assert_eq!(panel.tools.workspace()[1].0, "Strict");
        assert!(Config::load(&path).is_ok(), "written in the format of W#");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
