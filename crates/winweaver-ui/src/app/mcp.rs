//! The MCP server: its configuration file, its start, its tab.

use crate::*;

impl WinWeaver {
    /// Brings the MCP tab to the front, opening it when there is none.
    pub(crate) fn open_mcp(&mut self, shell: &mut Shell<WinWeaver>) {
        show_single(shell, DocKind::Mcp, || Doc::Mcp(Box::default()));
    }

    /// Reads the configuration of the MCP tab at the start: `mcp/default.wwmcp`, made from the port
    /// and the token an earlier version kept in the settings when it does not exist yet.
    pub(crate) fn load_mcp_config(&mut self, shell: &mut Shell<WinWeaver>) {
        let path = self.full.mcp.path.clone();
        if path.exists() {
            match winweaver_mcp::config::Config::load(&path) {
                Ok(config) => {
                    self.full.mcp.take(&config);
                    self.full.mcp.saved = Some(config);
                }
                Err(e) => {
                    self.full.mcp.error =
                        trf("The MCP configuration was not read: {e}", &[("e", &e)]);
                    // A file that cannot be read is not written over.
                    self.full.mcp.saved = Some(self.full.mcp.config());
                }
            }
            return;
        }
        let mut config = winweaver_mcp::config::Config::default();
        if shell.settings.extra.mcp_port >= 1024 {
            config.port = shell.settings.extra.mcp_port;
        }
        config.token = shell.settings.extra.mcp_token.clone();
        if config.check().is_err() {
            config.token.clear();
        }
        self.full.mcp.take(&config);
        // Written at the next frame by `persist_mcp`.
        self.full.mcp.saved = None;
    }

    /// Writes the configuration of the MCP tab once it changed; a failure is shown once.
    pub(crate) fn persist_mcp(&mut self) {
        let config = self.full.mcp.config();
        if self.full.mcp.saved.as_ref() == Some(&config) {
            return;
        }
        if let Err(e) = config.save(&self.full.mcp.path) {
            self.full.mcp.error = trf("The MCP configuration was not saved: {e}", &[("e", &e)]);
        }
        self.full.mcp.saved = Some(config);
    }

    /// Opens a configuration file in the MCP tab; a running server keeps the one it has.
    pub(crate) fn open_mcp_config(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        if self.full.mcp.run.is_some() {
            shell.status =
                tr("Stop the MCP server before you open another configuration").to_owned();
            return;
        }
        match winweaver_mcp::config::Config::load(&path) {
            Ok(config) => {
                self.full.mcp.take(&config);
                self.full.mcp.path = path;
                self.full.mcp.saved = Some(config);
                self.full.mcp.error.clear();
                self.apply(shell, Act::Mcp(McpAct::Open));
            }
            Err(e) => shell.status = trf("Not opened: {e}", &[("e", &e)]),
        }
    }

    /// Saves the configuration of the MCP tab into another file, which the tab edits from then on.
    pub(crate) fn save_mcp_as(&mut self, shell: &mut Shell<WinWeaver>) {
        let dir = winweaver_mcp::config::Config::folder(&exe_dir());
        let _ = std::fs::create_dir_all(&dir);
        let ext = winweaver_mcp::config::EXTENSION;
        let Some(mut path) = rfd::FileDialog::new()
            .set_title(tr("Save the MCP configuration as"))
            .add_filter(tr("MCP configurations"), &[ext])
            .set_directory(&dir)
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        let config = self.full.mcp.config();
        match config.save(&path) {
            Ok(()) => {
                shell.saved(&path);
                self.full.mcp.path = path;
                self.full.mcp.saved = Some(config);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// Starts the HTTP server of the window with the configuration of the MCP tab.
    pub(crate) fn start_mcp(&mut self, shell: &mut Shell<WinWeaver>) {
        if self.full.mcp.token.is_empty() {
            match winweaver_mcp::new_token() {
                Ok(t) => self.full.mcp.token = t,
                Err(e) => {
                    self.full.mcp.error = e;
                    return;
                }
            }
        }
        let server = std::sync::Arc::new(winweaver_mcp::Server::new(
            self.catalog.clone(),
            &PRESETS,
            winweaver_mcp::Options {
                mode: self.full.mcp.mode,
                registry: self.full.mcp.registry,
                edit_library: self.full.mcp.edit_library,
                marks_path: self.full.marks_path.clone(),
                imports_dir: self.full.imports_dir.clone(),
                read_pc: self.full.mcp.read_pc,
                profiles_dir: profiles_dir(),
                output_dir: output_dir(),
                detailed_log: self.full.mcp.detailed_log,
            },
        ));
        if !self.full.mcp.profile.is_empty()
            && let Err(e) = server.select(&self.full.mcp.profile)
        {
            self.full.mcp.error = trf(
                "The profile {name}: {e}",
                &[("name", &self.full.mcp.profile), ("e", &e)],
            );
            return;
        }
        match winweaver_mcp::http::HttpServer::start(
            server.clone(),
            self.full.mcp.port,
            self.full.mcp.token.clone(),
        ) {
            Ok(http) => {
                shell.status = trf("The MCP server runs on {url}", &[("url", &http.url())]);
                self.full.mcp.error.clear();
                self.full.mcp.run = Some(mcpview::McpRun { server, http });
            }
            Err(e) => {
                self.full.mcp.error = trf("The server did not start: {e}", &[("e", &e)]);
            }
        }
    }

    /// Runs an act of the acts of the MCP server and its tab.
    pub(crate) fn apply_mcp(&mut self, shell: &mut Shell<WinWeaver>, act: McpAct) {
        match act {
            McpAct::Open => self.open_mcp(shell),
            McpAct::Start => self.start_mcp(shell),
            McpAct::Stop => {
                self.full.mcp.run = None;
                shell.status = tr("The MCP server stopped").to_owned();
            }
            McpAct::RenewToken => match winweaver_mcp::new_token() {
                Ok(t) => self.full.mcp.token = t,
                Err(e) => self.full.mcp.error = e,
            },
            McpAct::OpenProfile(name) => {
                let profile = self
                    .full
                    .mcp
                    .run
                    .as_ref()
                    .and_then(|r| r.server.workspace_profile(&name));
                if let Some(profile) = profile {
                    let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                    doc.dirty = true;
                    shell.open_tab(Doc::Profile(Box::new(doc)));
                }
            }
        }
    }
}
