//! The MCP server of WinWeaver (`docs/spec/03-features.md` section 7): protocol 2025-06-18 on
//! JSON-RPC 2.0, over stdio or over HTTP on 127.0.0.1 with a bearer token.
//!
//! The server keeps its own workspace of profiles, independent of the window. The mode decides
//! what the tools may do: `read` (nothing changes, the mode of every start), `edit` (workspace
//! profiles change in memory) and `files` (profiles saved and the answer file written, only into
//! the folders of the program). Reading this PC needs the option `read_pc`. Nothing through MCP
//! ever applies settings, elevates, deletes or imports.

pub mod http;
pub mod stdio;
mod tools;

use std::path::PathBuf;
use std::sync::Mutex;

use indexmap::IndexMap;
use serde_json::{Value, json};
use winweaver_core::catalog::Catalog;
use winweaver_core::clock;
use winweaver_core::profile::Profile;

/// The protocol version the server speaks.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// The largest message the server reads.
pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;
const JOURNAL_SIZE: usize = 500;

/// What the tools may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Read,
    Edit,
    Files,
}

impl Mode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "read" => Some(Mode::Read),
            "edit" => Some(Mode::Edit),
            "files" => Some(Mode::Files),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Read => "read",
            Mode::Edit => "edit",
            Mode::Files => "files",
        }
    }
}

/// The options of a server; none of them is ever saved.
#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    /// Whether the tool read_this_pc may read the settings of this PC.
    pub read_pc: bool,
    /// The folder of the profiles the tools load and save.
    pub profiles_dir: PathBuf,
    /// The folder the answer file is written into.
    pub output_dir: PathBuf,
}

/// A profile of the workspace.
#[derive(Debug, Clone)]
pub struct WsDoc {
    pub profile: Profile,
    pub file: Option<PathBuf>,
    pub dirty: bool,
}

/// One request in the journal of the server.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub time: String,
    pub client: String,
    pub method: String,
    /// The arguments that are identifiers, numbers or switches; other texts are not shown.
    pub detail: String,
    pub result: String,
}

pub(crate) struct State {
    pub options: Options,
    pub docs: IndexMap<String, WsDoc>,
    pub current: String,
    pub journal: Vec<Entry>,
    pub client: String,
}

/// The server: the catalog, the presets and the state behind a lock, shared by the transports.
pub struct Server {
    pub catalog: Catalog,
    pub presets: Vec<(String, Profile)>,
    state: Mutex<State>,
}

impl Server {
    /// A server whose workspace starts with a copy of the first preset.
    pub fn new(catalog: Catalog, presets: &[(&str, &str)], options: Options) -> Self {
        let presets: Vec<(String, Profile)> = presets
            .iter()
            .filter_map(|(name, text)| {
                Profile::parse(text, name)
                    .ok()
                    .map(|p| ((*name).to_owned(), p))
            })
            .collect();
        let first = presets
            .first()
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| Profile::from_defaults("Office", &catalog));
        let mut docs = IndexMap::new();
        let current = first.name.clone();
        docs.insert(
            current.clone(),
            WsDoc {
                profile: first,
                file: None,
                dirty: false,
            },
        );
        Self {
            catalog,
            presets,
            state: Mutex::new(State {
                options,
                docs,
                current,
                journal: Vec::new(),
                client: String::new(),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn options(&self) -> Options {
        self.lock().options.clone()
    }

    pub fn set_mode(&self, mode: Mode) {
        self.lock().options.mode = mode;
    }

    pub fn set_read_pc(&self, on: bool) {
        self.lock().options.read_pc = on;
    }

    /// The last requests, the newest last.
    pub fn journal(&self) -> Vec<Entry> {
        self.lock().journal.clone()
    }

    /// The name of the client that sent `initialize`.
    pub fn client(&self) -> String {
        self.lock().client.clone()
    }

    /// The profiles of the workspace: the name, whether it changed, the file name.
    pub fn workspace(&self) -> Vec<(String, bool, String)> {
        let state = self.lock();
        state
            .docs
            .iter()
            .map(|(name, d)| {
                let file = d
                    .file
                    .as_ref()
                    .and_then(|f| f.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (name.clone(), d.dirty, file)
            })
            .collect()
    }

    /// A copy of a workspace profile, for the window to open it in a tab.
    pub fn workspace_profile(&self, name: &str) -> Option<Profile> {
        self.lock().docs.get(name).map(|d| d.profile.clone())
    }

    /// Makes a preset or a profile file of the profiles folder the current profile, whatever
    /// the mode: the choice of the person who starts the server.
    pub fn select(&self, name: &str) -> Result<(), String> {
        let mut state = self.lock();
        tools::select(self, &mut state, name).map_err(|e| format!("{}: {}", e.kind, e.message))
    }

    /// Handles one JSON-RPC message given as text; `None` for a notification.
    pub fn handle_text(&self, text: &str) -> Option<String> {
        if text.len() > MAX_MESSAGE {
            return Some(error(Value::Null, -32600, "the message is too large").to_string());
        }
        let value: Value = match winweaver_core::jsonfile::parse(text, "message") {
            Ok(v) => v,
            Err(e) => return Some(error(Value::Null, -32700, &e.to_string()).to_string()),
        };
        self.handle(&value).map(|v| v.to_string())
    }

    /// Handles one JSON-RPC message; `None` for a notification.
    pub fn handle(&self, message: &Value) -> Option<Value> {
        let Some(object) = message.as_object() else {
            return Some(error(
                Value::Null,
                -32600,
                "a message is one JSON object; batches are not supported",
            ));
        };
        let id = object.get("id").cloned();
        let method = object.get("method").and_then(Value::as_str).unwrap_or("");
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || method.is_empty() {
            return id.map(|id| error(id, -32600, "not a JSON-RPC 2.0 request"));
        }
        let params = object.get("params").cloned().unwrap_or(json!({}));
        let Some(id) = id else {
            // A notification (initialized, cancelled) needs no answer.
            return None;
        };
        let result = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools::definitions() })),
            "tools/call" => Ok(self.call_tool(&params)),
            "resources/list" => Ok(json!({ "resources": tools::resources() })),
            "resources/read" => self.read_resource(&params),
            _ => Err((-32601, format!("unknown method {method}"))),
        };
        Some(match result {
            Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
            Err((code, text)) => error(id, code, &text),
        })
    }

    fn initialize(&self, params: &Value) -> Value {
        let client = params
            .pointer("/clientInfo/name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .take(60)
            .collect::<String>();
        {
            let mut state = self.lock();
            state.client = client;
        }
        self.record("initialize", String::new(), "ok".to_owned());
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false }, "resources": { "listChanged": false } },
            "serverInfo": { "name": "winweaver", "version": winweaver_core::APP_VERSION },
            "instructions": INSTRUCTIONS,
        })
    }

    fn call_tool(&self, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let detail = tools::journal_detail(&args);
        let outcome = {
            let mut state = self.lock();
            tools::call(self, &mut state, name, &args)
        };
        match outcome {
            Ok(value) => {
                self.record(name, detail, "ok".to_owned());
                let text = serde_json::to_string_pretty(&value).unwrap_or_default();
                json!({
                    "content": [{ "type": "text", "text": tools::fit(&text) }],
                    "structuredContent": value,
                    "isError": false,
                })
            }
            Err(e) => {
                self.record(name, detail, e.kind.to_owned());
                json!({
                    "content": [{ "type": "text", "text": format!("{}: {}", e.kind, e.message) }],
                    "isError": true,
                })
            }
        }
    }

    fn read_resource(&self, params: &Value) -> Result<Value, (i64, String)> {
        let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
        let text = {
            let state = self.lock();
            tools::resource(self, &state, uri)
        };
        match text {
            Some(text) => {
                self.record("resources/read", uri.chars().take(80).collect(), "ok".to_owned());
                Ok(json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }] }))
            }
            None => Err((-32002, format!("unknown resource {uri}"))),
        }
    }

    fn record(&self, method: &str, detail: String, result: String) {
        let mut state = self.lock();
        let client = state.client.clone();
        state.journal.push(Entry {
            time: clock::now_utc_iso(),
            client,
            method: method.to_owned(),
            detail,
            result,
        });
        let excess = state.journal.len().saturating_sub(JOURNAL_SIZE);
        state.journal.drain(..excess);
    }
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The instructions the server gives the client at `initialize`.
pub const INSTRUCTIONS: &str = "WinWeaver holds a catalog of rules for unattended Windows 11 installations and a workspace of profiles that choose them. Start with get_status. The mode decides what the tools may do: read (nothing changes), edit (workspace profiles change in memory), files (profiles are saved and the answer file is written into the folders of the program). An error text starts with its kind: not_allowed, not_found, invalid, write_failed or read_failed. Passwords and product keys are never shown.";

/// A token for the HTTP transport: 40 letters and digits from the random source of the system,
/// starting with a letter.
pub fn new_token() -> Result<String, String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut token = String::with_capacity(40);
    for _ in 0..16 {
        let mut bytes = [0u8; 64];
        getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
        for b in bytes {
            // The first character is a letter (52), the others letters or digits (62). A byte
            // at or above the largest multiple of the size below 256 is skipped, so that every
            // character is equally likely.
            let size = if token.is_empty() { 52 } else { 62 };
            if usize::from(b) < 256 - 256 % size {
                token.push(ALPHABET[usize::from(b) % size] as char);
            }
            if token.len() == 40 {
                return Ok(token);
            }
        }
    }
    Err("the random source gave too few usable bytes".to_owned())
}
