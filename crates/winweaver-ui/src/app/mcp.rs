//! The MCP server: its configuration file, its start, its tab.

use crate::*;

/// The largest log of the server shown as a page.
const MCP_LOG_MAX_BYTES: u64 = 32 * 1024 * 1024;

impl WinWeaver {
    /// Brings the MCP tab to the front, opening it when there is none.
    pub(crate) fn open_mcp(&mut self, shell: &mut Shell<WinWeaver>) {
        show_single(shell, DocKind::Mcp, || Doc::Mcp(Box::default()));
    }

    /// The tools of WinWeaver with the catalog and the folders of the window.
    fn mcp_tools(&self) -> winweaver_mcp::Tools {
        mcpview::tools_of(
            &self.catalog,
            self.full.marks_path.clone(),
            self.full.imports_dir.clone(),
        )
    }

    /// Reads the configuration of the MCP tab at the start: `mcp/default.wwmcp` (a file of an
    /// earlier version written in the format of W# first), made from the port and the token an
    /// earlier version kept in the settings when it does not exist yet.
    pub(crate) fn load_mcp_config(&mut self, shell: &mut Shell<WinWeaver>) {
        let path = wwmcp::Config::default_path(&exe_dir());
        let existed = path.exists();
        self.full.mcp = mcpview::McpPanel::new(path, self.mcp_tools());
        if existed {
            return;
        }
        let config = &mut self.full.mcp.panel.config;
        if shell.settings.extra.mcp_port >= 1024 {
            config.port = shell.settings.extra.mcp_port;
        }
        config.token = shell.settings.extra.mcp_token.clone();
        if config.check().is_err() {
            config.token.clear();
        }
        // Written at the next frame by `persist_mcp` when it differs from the defaults.
    }

    /// Writes the configuration of the MCP tab once it changed; a failure is shown once.
    pub(crate) fn persist_mcp(&mut self) {
        self.full.mcp.panel.persist();
    }

    /// Opens a configuration file in the MCP tab; a running server keeps the one it has.
    pub(crate) fn open_mcp_config(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        if let Err(e) = winweaver_mcp::legacy::migrate(&path) {
            shell.status = trf("Not opened: {e}", &[("e", &e)]);
            return;
        }
        match self.full.mcp.panel.open_config(&path) {
            Ok(()) => self.apply(shell, Act::Mcp(McpAct::Open)),
            Err(e) => shell.status = trf("Not opened: {e}", &[("e", &e)]),
        }
    }

    /// Saves the configuration of the MCP tab into another file, which the tab edits from then on.
    pub(crate) fn save_mcp_as(&mut self, shell: &mut Shell<WinWeaver>) {
        if let Some(line) = self.full.mcp.panel.apply(wwmcp::ui::McpAct::SaveAs) {
            shell.status = line;
        }
    }

    /// Starts the HTTP server of the window with the configuration of the MCP tab, on tools of
    /// a new workspace.
    pub(crate) fn start_mcp(&mut self, shell: &mut Shell<WinWeaver>) {
        let tools = self.mcp_tools();
        if let Err(e) = self.full.mcp.renew(tools) {
            let name = self
                .full
                .mcp
                .panel
                .config
                .source(winweaver_mcp::SOURCE)
                .options
                .get("profile")
                .cloned()
                .unwrap_or_default();
            self.full.mcp.panel.error =
                trf("The profile {name}: {e}", &[("name", &name), ("e", &e)]);
            return;
        }
        shell.status = self.full.mcp.panel.start();
    }

    /// Runs an act of the acts of the MCP server and its tab.
    pub(crate) fn apply_mcp(&mut self, shell: &mut Shell<WinWeaver>, act: McpAct) {
        use wwmcp::ui::McpAct as Panel;
        match act {
            McpAct::Open => self.open_mcp(shell),
            McpAct::Panel(Panel::Start) => self.start_mcp(shell),
            // A new token cuts the clients off: the user says so first.
            McpAct::Panel(Panel::RenewToken) => shell.ask(crate::question::renew_token()),
            McpAct::RenewConfirmed => {
                self.full.mcp.panel.apply(Panel::RenewToken);
            }
            // The log of the server, JSON Lines, as a page of text.
            McpAct::Panel(Panel::OpenLog(path)) => {
                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                shell.open_file(
                    &path,
                    |d| matches!(d, Doc::Markdown(m) if m.path.as_ref() == Some(&path)),
                    |p| winweaver_core::jsonfile::read_text(p, MCP_LOG_MAX_BYTES),
                    |text, path| Doc::Markdown(Box::new(MdDoc::plain(title, Some(path), text))),
                );
            }
            McpAct::Panel(act) => {
                if let Some(line) = self.full.mcp.panel.apply(act) {
                    shell.status = line;
                }
            }
            McpAct::OpenProfile(name) => {
                if let Some(profile) = self.full.mcp.tools.workspace_profile(&name) {
                    let mut doc = ProfileDoc::new(profile, None, &self.catalog);
                    doc.dirty = true;
                    shell.open_tab(Doc::Profile(Box::new(doc)));
                }
            }
        }
    }
}
