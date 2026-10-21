//! The MCP server of WinWeaver (`docs/spec/03-features.md` section 7): protocol 2025-06-18 on
//! JSON-RPC 2.0, over stdio or over HTTP on 127.0.0.1 with a bearer token.
//!
//! The server keeps its own workspace of profiles, independent of the window. The mode decides
//! what the tools may do: `read` (nothing changes, the mode of every start), `edit` (workspace
//! profiles change in memory) and `files` (profiles saved and the answer file written, only into
//! the folders of the program). Reading this PC needs the option `read_pc`. Nothing through MCP
//! ever applies settings, elevates, deletes or imports.

pub mod config;
pub mod http;
pub mod log;
pub mod stdio;
mod tools;

use std::path::PathBuf;
use std::sync::Mutex;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use winweaver_core::catalog::Catalog;
use winweaver_core::clock;
use winweaver_core::profile::Profile;

/// The protocol version the server speaks.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// The largest message the server reads.
pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;
const JOURNAL_SIZE: usize = 500;
/// The longest answer the journal keeps for the panel of the MCP tab and the detailed log.
const ANSWER_KEPT: usize = 16 * 1024;
/// The fields whose values never reach the journal or the log.
const SECRETS: [&str; 4] = ["password", "product_key", "token", "authorization"];

/// What the tools may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

/// What the tools of the registry may do: off unless the configuration of the server allows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistryAccess {
    #[default]
    Off,
    Read,
    /// Reading and writing.
    Write,
}

impl RegistryAccess {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Self::Off),
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// The options of a server, from its configuration file (`config::Config`) and the folders of the
/// program.
#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    /// What the tools of the registry may do.
    pub registry: RegistryAccess,
    /// Whether the tools may change the bookmarks and the catalog files; reading them is always
    /// allowed, like reading profiles.
    pub edit_library: bool,
    /// The bookmark file and the folder of the catalog files of the imports.
    pub marks_path: PathBuf,
    pub imports_dir: PathBuf,
    /// Whether the tool read_this_pc may read the settings of this PC.
    pub read_pc: bool,
    /// The folder of the profiles the tools load and save.
    pub profiles_dir: PathBuf,
    /// The folder the answer file is written into.
    pub output_dir: PathBuf,
    /// Whether the log keeps every request and answer whole, without secrets.
    pub detailed_log: bool,
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
    /// The whole arguments and the answer, without passwords, keys and tokens, for the panel of
    /// the MCP tab; a long answer is cut to its first characters.
    pub arguments: Value,
    pub answer: Value,
    /// How long the call took, in milliseconds.
    pub ms: u64,
}

/// A copy of a value without the values of the secret fields (passwords, the product key, tokens).
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let secret = SECRETS.iter().any(|s| k.eq_ignore_ascii_case(s));
                    let v = if secret && !v.is_null() {
                        Value::from("***")
                    } else {
                        redact(v)
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        other => other.clone(),
    }
}

/// An answer for the journal: without secrets, and as the start of its text when it is long.
fn kept(value: &Value) -> Value {
    let value = redact(value);
    let text = value.to_string();
    if text.len() <= ANSWER_KEPT {
        return value;
    }
    let mut end = ANSWER_KEPT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Value::from(format!("{}... ({} bytes in all)", &text[..end], text.len()))
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
    /// The registry the tools of the registry read and change: of this PC, or in memory.
    pub(crate) registry: std::sync::Arc<dyn winweaver_core::registry::Registry>,
    state: Mutex<State>,
}

impl Server {
    /// A server whose workspace starts with a copy of the first preset.
    pub fn new(catalog: Catalog, presets: &[(&str, &str)], options: Options) -> Self {
        // The input languages and layouts of this Windows, read at the first build or check.
        winweaver_core::resources::set_locale_reader(winweaver_win::locales);
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
            registry: winweaver_win::this_registry(),
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

    pub fn set_registry(&self, access: RegistryAccess) {
        self.lock().options.registry = access;
    }

    pub fn set_edit_library(&self, on: bool) {
        self.lock().options.edit_library = on;
    }

    /// The server with another registry, as the tests give one in memory.
    pub fn with_registry(
        mut self,
        registry: std::sync::Arc<dyn winweaver_core::registry::Registry>,
    ) -> Self {
        self.registry = registry;
        self
    }

    pub fn set_detailed_log(&self, on: bool) {
        self.lock().options.detailed_log = on;
    }

    /// The last requests, the newest last, without their arguments and answers (the list of the
    /// MCP tab reads it at every frame).
    pub fn journal(&self) -> Vec<Entry> {
        self.lock()
            .journal
            .iter()
            .map(|e| Entry {
                time: e.time.clone(),
                client: e.client.clone(),
                method: e.method.clone(),
                detail: e.detail.clone(),
                result: e.result.clone(),
                arguments: Value::Null,
                answer: Value::Null,
                ms: e.ms,
            })
            .collect()
    }

    /// One request of the journal whole, by its place in the journal.
    pub fn journal_entry(&self, index: usize) -> Option<Entry> {
        self.lock().journal.get(index).cloned()
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
        tools::select(self, &mut state, name)
            .map_err(|e| format!("{}: {}", e.kind.name(), e.message))
    }

    /// Handles one JSON-RPC message given as text; `None` for a notification.
    pub fn handle_text(&self, text: &str) -> Option<String> {
        if text.len() > MAX_MESSAGE {
            log::line(
                "WARN",
                &format!("a message of {} bytes was refused: too large", text.len()),
            );
            return Some(error(Value::Null, -32600, "the message is too large").to_string());
        }
        let value: Value = match winweaver_core::jsonfile::parse(text, "message") {
            Ok(v) => v,
            Err(e) => {
                log::line(
                    "WARN",
                    &format!("a message that is not JSON was refused: {e}"),
                );
                return Some(error(Value::Null, -32700, &e.to_string()).to_string());
            }
        };
        // A failure inside the program answers this request with an error and keeps the
        // process and the other requests going; the panic hook has written the details.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.handle(&value))) {
            Ok(answer) => answer.map(|v| v.to_string()),
            Err(payload) => {
                let method = value.get("method").and_then(Value::as_str).unwrap_or("");
                log::line(
                    "ERROR",
                    &format!(
                        "the request {method} failed inside the program: {}; the server goes on",
                        log::panic_text(payload.as_ref())
                    ),
                );
                let id = value.get("id").cloned().unwrap_or(Value::Null);
                Some(
                    error(
                        id,
                        -32603,
                        "internal error: the request failed inside WinWeaver; the MCP log of the program has the details",
                    )
                    .to_string(),
                )
            }
        }
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
            #[cfg(test)]
            "test/panic" => panic!("a failure inside a request"),
            _ => {
                log::line("WARN", &format!("unknown method {method}"));
                Err((-32601, format!("unknown method {method}")))
            }
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
        let protocol = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .filter(char::is_ascii_graphic)
            .take(20)
            .collect::<String>();
        log::line(
            "INFO",
            &format!("client '{client}' connected (protocol {protocol})"),
        );
        {
            let mut state = self.lock();
            state.client = client;
        }
        self.record(Call {
            method: "initialize",
            detail: String::new(),
            result: "ok",
            arguments: params,
            answer: &Value::Null,
            ms: 0,
        });
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false }, "resources": { "listChanged": false } },
            "serverInfo": { "name": "winweaver", "version": winweaver_core::BUILD },
            "instructions": INSTRUCTIONS,
        })
    }

    fn call_tool(&self, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let detail = tools::journal_detail(&args);
        let started = std::time::Instant::now();
        log::line("INFO", &format!("tool {name} {detail}"));
        let outcome = {
            let mut state = self.lock();
            tools::call(self, &mut state, name, &args)
        };
        let ms = started.elapsed().as_millis();
        match &outcome {
            Ok(_) => log::line("INFO", &format!("tool {name} ok in {ms} ms")),
            Err(e) => log::line(
                "WARN",
                &format!(
                    "tool {name} {} in {ms} ms: {}",
                    e.kind.name(),
                    e.message.chars().take(200).collect::<String>()
                ),
            ),
        }
        let answer = match &outcome {
            Ok(value) => value.clone(),
            Err(e) => Value::from(format!("{}: {}", e.kind.name(), e.message)),
        };
        let result = match &outcome {
            Ok(_) => "ok",
            Err(e) => e.kind.name(),
        };
        self.record(Call {
            method: name,
            detail,
            result,
            arguments: &args,
            answer: &answer,
            ms: u64::try_from(ms).unwrap_or(u64::MAX),
        });
        match outcome {
            Ok(value) => {
                let text = serde_json::to_string_pretty(&value).unwrap_or_default();
                json!({
                    "content": [{ "type": "text", "text": tools::fit(&text) }],
                    "structuredContent": value,
                    "isError": false,
                })
            }
            Err(e) => {
                json!({
                    "content": [{ "type": "text", "text": format!("{}: {}", e.kind.name(), e.message) }],
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
                self.record(Call {
                    method: "resources/read",
                    detail: uri.chars().take(80).collect(),
                    result: "ok",
                    arguments: params,
                    answer: &Value::from(format!("{} characters", text.chars().count())),
                    ms: 0,
                });
                let mime = if uri == "winweaver://skill" {
                    "text/markdown"
                } else {
                    "application/json"
                };
                Ok(json!({ "contents": [{ "uri": uri, "mimeType": mime, "text": text }] }))
            }
            None => Err((-32002, format!("unknown resource {uri}"))),
        }
    }

    /// Puts a call into the journal, and into the log when the log is detailed.
    fn record(&self, call: Call<'_>) {
        let mut state = self.lock();
        let client = state.client.clone();
        let entry = Entry {
            time: clock::now_utc_iso(),
            client,
            method: call.method.to_owned(),
            detail: call.detail,
            result: call.result.to_owned(),
            arguments: redact(call.arguments),
            answer: kept(call.answer),
            ms: call.ms,
        };
        if state.options.detailed_log {
            log::event(
                "DETAIL",
                json!({
                    "client": entry.client,
                    "call": entry.method,
                    "result": entry.result,
                    "ms": entry.ms,
                    "arguments": entry.arguments,
                    "answer": entry.answer,
                }),
            );
        }
        state.journal.push(entry);
        let excess = state.journal.len().saturating_sub(JOURNAL_SIZE);
        state.journal.drain(..excess);
    }
}

/// One call for the journal.
struct Call<'a> {
    method: &'a str,
    detail: String,
    result: &'a str,
    arguments: &'a Value,
    answer: &'a Value,
    ms: u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A failure inside a request answers it with an error; the lock of the state is still
    /// usable and the next request is served.
    #[test]
    fn a_failure_inside_a_request_keeps_the_server_going() {
        let catalog = Catalog::embedded().unwrap();
        let server = Server::new(
            catalog,
            &[],
            Options {
                mode: Mode::Read,
                registry: RegistryAccess::Off,
                edit_library: false,
                marks_path: std::env::temp_dir().join("winweaver-no-marks.wwmarks"),
                imports_dir: std::env::temp_dir().join("winweaver-no-imports"),
                read_pc: false,
                profiles_dir: std::env::temp_dir(),
                output_dir: std::env::temp_dir(),
                detailed_log: false,
            },
        );
        let failed = server
            .handle_text(r#"{"jsonrpc": "2.0", "id": 7, "method": "test/panic"}"#)
            .unwrap();
        let failed: Value = serde_json::from_str(&failed).unwrap();
        assert_eq!(failed["id"], 7);
        assert_eq!(failed["error"]["code"], -32603);
        let ping = server
            .handle_text(r#"{"jsonrpc": "2.0", "id": 8, "method": "ping"}"#)
            .unwrap();
        assert!(ping.contains("\"result\""), "{ping}");
        assert_eq!(server.options().mode, Mode::Read);
    }
}
