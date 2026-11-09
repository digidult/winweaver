//! Tests of the tools of WinWeaver on the MCP server of W#: the protocol, the modes and the
//! options of the source `winweaver`, the tools and both transports (HTTP on a free loopback
//! port).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use winweaver_core::catalog::Catalog;
use winweaver_mcp::{Paths, SOURCE, Tools};
use wwmcp::http::HttpServer;
use wwmcp::{Config, Mode, Server};

const OFFICE: &str = include_str!("../../../data/presets/office.wwprofile");
const STRICT: &str = include_str!("../../../data/presets/strict.wwprofile");

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("winweaver-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The server of the tools of WinWeaver, with them at hand for their workspace.
struct S {
    server: Arc<Server>,
    tools: Arc<Tools>,
}

impl std::ops::Deref for S {
    type Target = Server;

    fn deref(&self) -> &Server {
        &self.server
    }
}

impl S {
    /// Sets an option of the source `winweaver`, as the tab or the command line does.
    fn set(&self, key: &str, value: &str) {
        let mut config = self.server.config();
        config
            .source_mut(SOURCE)
            .options
            .insert(key.to_owned(), value.to_owned());
        self.server.set_config(config);
    }

    fn set_registry(&self, access: &str) {
        self.set("registry", access);
    }

    fn set_edit_library(&self, on: bool) {
        self.set("edit_library", if on { "on" } else { "off" });
    }

    fn workspace(&self) -> Vec<(String, bool, String)> {
        self.tools.workspace()
    }
}

fn tools(dir: &std::path::Path) -> Tools {
    Tools::new(
        Catalog::embedded().unwrap(),
        &[("Office", OFFICE), ("Strict", STRICT)],
        Paths {
            marks_path: dir.join("bookmarks.wwmarks"),
            imports_dir: dir.join("imports"),
            profiles_dir: dir.join("profiles"),
            output_dir: dir.join("output"),
        },
    )
}

fn served(mode: Mode, tools: Tools) -> S {
    let tools = Arc::new(tools);
    let config = Config {
        mode,
        ..Config::default()
    };
    let server = Server::new(winweaver_mcp::info(), config).with(tools.clone());
    S {
        server: Arc::new(server),
        tools,
    }
}

fn server(mode: Mode, dir: &std::path::Path) -> S {
    served(mode, tools(dir))
}

/// The tools of the registry are off until allowed, and write only when writing is allowed; here on
/// a registry in memory.
#[test]
fn the_registry_tools_are_off_until_allowed() {
    let dir = folder("registry");
    let memory = Arc::new(winweaver_core::registry::MemoryRegistry::default());
    let s = served(Mode::Read, tools(&dir).with_registry(memory));
    let key = "HKCU\\Software\\WinWeaverTest";
    assert_eq!(
        error_kind(&s, "read_registry_key", json!({ "path": "HKCU" })),
        "not_allowed"
    );
    let any = json!({ "path": "HKCU", "text": "x" });
    assert_eq!(error_kind(&s, "search_registry", any), "not_allowed");
    s.set_registry("read");
    ok(&s, "read_registry_key", json!({ "path": "HKCU" }));
    assert_eq!(
        error_kind(&s, "create_registry_key", json!({ "path": key })),
        "not_allowed"
    );
    s.set_registry("write");
    ok(&s, "create_registry_key", json!({ "path": key }));
    ok(
        &s,
        "set_registry_value",
        json!({ "path": key, "name": "N", "type": "REG_DWORD", "data": "0x10" }),
    );
    let read = ok(&s, "read_registry_key", json!({ "path": key }));
    assert_eq!(
        read["values"][0]["data"],
        json!({ "type": "REG_DWORD", "data": 16 })
    );
    let find = json!({ "path": "HKCU\\Software", "text": "winweavertest" });
    let found = ok(&s, "search_registry", find);
    assert_eq!(
        found["hits"][0]["path"],
        json!("HKEY_CURRENT_USER\\Software\\WinWeaverTest")
    );
    assert_eq!(found["complete"], json!(true));
    let number = json!({ "path": key, "text": "0x10", "keys": false, "whole": true });
    assert_eq!(ok(&s, "search_registry", number)["hits"][0]["value"], "N");
    let empty = json!({ "path": key, "text": " " });
    assert_eq!(error_kind(&s, "search_registry", empty), "invalid");
    assert_eq!(
        error_kind(
            &s,
            "set_registry_value",
            json!({ "path": key, "name": "N", "type": "REG_DWORD", "data": "x" })
        ),
        "invalid"
    );
    // The permissions: read with the names of the accounts, the own entries replaced.
    let rights = ok(&s, "read_registry_rights", json!({ "path": key }));
    assert_eq!(rights["owner_name"], json!("Administrators"));
    let users = json!({ "path": key, "entries": [{ "account": "Users" }] });
    let after = ok(&s, "set_registry_rights", users);
    let own: Vec<&Value> = after["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["inherited"] == json!(false))
        .collect();
    assert_eq!(own.len(), 1, "{after}");
    assert_eq!(own[0]["account"], json!("BU"));
    assert_eq!(own[0]["access"], json!("read"));
    // An identifier is checked, so no text reaches the access list.
    let forged = json!({ "path": key, "entries": [{ "account": "S-1-5-32-544)(A;;KA;;;WD" }] });
    assert_eq!(error_kind(&s, "set_registry_rights", forged), "invalid");
    let nobody = json!({ "path": key, "entries": [], "inherits": false });
    assert_eq!(error_kind(&s, "set_registry_rights", nobody), "invalid");
    ok(&s, "delete_registry_key", json!({ "path": key }));
    assert_eq!(
        error_kind(&s, "read_registry_key", json!({ "path": key })),
        "read_failed"
    );
}

/// Bookmarks and catalogs are read always and changed only when the user allows it.
#[test]
fn bookmarks_and_catalogs_are_read_and_changed_only_when_allowed() {
    let dir = folder("library");
    let s = server(Mode::Read, &dir);
    assert_eq!(ok(&s, "list_bookmarks", json!({}))["bookmarks"], json!([]));
    let add =
        json!({ "kind": "registry", "path": "HKLM\\Software\\X", "item": "V", "comment": "check" });
    assert_eq!(error_kind(&s, "add_bookmark", add.clone()), "not_allowed");
    s.set_edit_library(true);
    ok(&s, "add_bookmark", add.clone());
    assert_eq!(error_kind(&s, "add_bookmark", add), "invalid", "only once");
    let list = ok(&s, "list_bookmarks", json!({}));
    assert_eq!(
        list["bookmarks"][0]["path"],
        json!("HKEY_LOCAL_MACHINE\\Software\\X")
    );
    ok(
        &s,
        "set_bookmark_comment",
        json!({ "index": 0, "comment": "done" }),
    );
    assert_eq!(
        ok(&s, "list_bookmarks", json!({}))["bookmarks"][0]["comment"],
        json!("done")
    );
    ok(&s, "remove_bookmark", json!({ "index": 0 }));
    assert_eq!(
        error_kind(&s, "remove_bookmark", json!({ "index": 0 })),
        "not_found"
    );

    let imports = dir.join("imports");
    std::fs::create_dir_all(&imports).unwrap();
    let file = winweaver_core::admx::to_catalog_file(
        &winweaver_core::admx::Templates::default(),
        "mine",
        "Mine",
        "",
        "now",
    );
    file.save(&imports.join("mine.wwcatalog")).unwrap();
    let catalogs = ok(&s, "list_catalogs", json!({}));
    assert_eq!(catalogs["catalogs"][0]["name"], json!("Mine"));
    let groups = ok(&s, "get_catalog", json!({ "id": "mine" }));
    let root = groups["groups"][0]["id"].as_str().unwrap().to_owned();
    let mut rule = file.new_policy(&root, "Custom").unwrap();
    rule.title = "A custom policy".to_owned();
    ok(
        &s,
        "set_catalog_policy",
        json!({ "id": "mine", "policy": rule }),
    );
    let read = ok(
        &s,
        "get_catalog_policy",
        json!({ "id": "mine", "policy": "admx.mine.custom" }),
    );
    assert_eq!(read["policy"]["title"], json!("A custom policy"));
    assert_eq!(
        error_kind(&s, "get_catalog", json!({ "id": "..\\x" })),
        "invalid"
    );
    ok(
        &s,
        "remove_catalog_policy",
        json!({ "id": "mine", "policy": "admx.mine.custom" }),
    );
    assert_eq!(
        error_kind(
            &s,
            "get_catalog_policy",
            json!({ "id": "mine", "policy": "admx.mine.custom" })
        ),
        "not_found"
    );
}

fn call(s: &Server, name: &str, args: Value) -> Value {
    let answer = s
        .handle(&json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": name, "arguments": args } }))
        .unwrap();
    answer["result"].clone()
}

fn ok(s: &Server, name: &str, args: Value) -> Value {
    let r = call(s, name, args);
    assert_eq!(
        r["isError"],
        json!(false),
        "{name}: {}",
        r["content"][0]["text"]
    );
    r["structuredContent"].clone()
}

fn error_kind(s: &Server, name: &str, args: Value) -> String {
    let r = call(s, name, args);
    assert_eq!(r["isError"], json!(true), "{name} should fail");
    r["content"][0]["text"]
        .as_str()
        .unwrap()
        .split(':')
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn the_protocol_answers_and_lists_the_tools() {
    let dir = folder("protocol");
    let s = server(Mode::Read, &dir);
    let init = s
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "clientInfo": { "name": "test-client" } } }))
        .unwrap();
    assert_eq!(init["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(init["result"]["serverInfo"]["name"], json!("winweaver"));
    assert!(
        s.handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .is_none()
    );
    let tools = s
        .handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
        .unwrap();
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 36, "{names:?}");
    for n in [
        "get_status",
        "set_rule",
        "write_answer_file",
        "read_this_pc",
        "compare_profiles",
    ] {
        assert!(names.contains(&n), "{n}");
    }
    let unknown = s
        .handle(&json!({ "jsonrpc": "2.0", "id": 3, "method": "nope" }))
        .unwrap();
    assert_eq!(unknown["error"]["code"], json!(-32601));
    let batch = s
        .handle_text("[{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"ping\"}]")
        .unwrap();
    assert!(batch.contains("-32600"));
    let broken = s.handle_text("{").unwrap();
    assert!(broken.contains("-32700"));
    let resources = s.handle(&json!({ "jsonrpc": "2.0", "id": 4, "method": "resources/read", "params": { "uri": "winweaver://catalog" } })).unwrap();
    assert!(
        resources["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("defender.pua")
    );
    assert_eq!(s.client(), "test-client");
    assert!(s.journal().iter().any(|e| e.method == "initialize"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_mode_read_changes_nothing() {
    let dir = folder("read");
    let s = server(Mode::Read, &dir);
    let status = ok(&s, "get_status", json!({}));
    assert_eq!(status["mode"], json!("read"));
    assert_eq!(status["catalog"]["rules"], json!(284));
    assert_eq!(
        error_kind(&s, "set_rule", json!({ "id": "defender.pua", "on": false })),
        "not_allowed"
    );
    assert_eq!(
        error_kind(&s, "write_answer_file", json!({})),
        "not_allowed"
    );
    assert_eq!(error_kind(&s, "read_this_pc", json!({})), "not_allowed");
    assert_eq!(
        error_kind(&s, "get_rule", json!({ "id": "no.such" })),
        "not_found"
    );
    let rule = ok(&s, "get_rule", json!({ "id": "defender.pua" }));
    assert_eq!(rule["id"], json!("defender.pua"));
    let found = ok(&s, "search_rules", json!({ "query": "telemetry" }));
    assert!(found["total"].as_u64().unwrap() > 0);
    let groups = ok(&s, "list_groups", json!({}));
    assert_eq!(groups["groups"].as_array().unwrap().len(), 3);
    let preview = ok(&s, "preview_build", json!({}));
    assert!(
        preview["beginning"]
            .as_str()
            .unwrap()
            .contains("<Key>***</Key>"),
        "the key is hidden"
    );
    assert!(!dir.join("output").exists(), "nothing was written");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn edit_and_files_change_the_workspace_and_write_only_into_the_folders() {
    let dir = folder("files");
    let s = server(Mode::Files, &dir);
    let set = ok(&s, "set_rule", json!({ "id": "defender.pua", "on": false }));
    assert_eq!(set["state"], json!("off"));
    assert_eq!(
        error_kind(
            &s,
            "set_param",
            json!({ "id": "update.automatic", "name": "start", "value": 99 })
        ),
        "invalid"
    );
    ok(
        &s,
        "set_param",
        json!({ "id": "update.automatic", "name": "start", "value": 7 }),
    );
    let profile = ok(&s, "get_profile", json!({}));
    assert_eq!(profile["changed"], json!(true));
    assert_eq!(
        error_kind(&s, "save_profile", json!({ "name": "Office" })),
        "not_allowed"
    );
    assert_eq!(
        error_kind(&s, "save_profile", json!({ "name": "../escape" })),
        "invalid"
    );
    let saved = ok(&s, "save_profile", json!({ "name": "Office edited" }));
    assert_eq!(saved["file"], json!("Office edited.wwprofile"));
    assert!(
        dir.join("profiles")
            .join("Office edited.wwprofile")
            .is_file()
    );
    let written = ok(&s, "write_answer_file", json!({}));
    assert_eq!(written["file"], json!("autounattend.xml"));
    assert!(dir.join("output").join("autounattend.xml").is_file());
    ok(
        &s,
        "new_profile",
        json!({ "name": "Second", "preset": "Strict" }),
    );
    let cmp = ok(
        &s,
        "compare_profiles",
        json!({ "a": "Office", "b": "Second" }),
    );
    assert!(cmp["differences"].as_u64().unwrap() > 0);
    let listed = ok(&s, "list_profiles", json!({}));
    assert!(
        listed["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "Office edited.wwprofile")
    );
    ok(&s, "load_profile", json!({ "name": "Office edited" }));
    assert_eq!(s.workspace().len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn post(port: u16, token: &str, host: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    let status = answer.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = answer.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
    (status, body)
}

#[test]
fn the_http_transport_needs_the_token_and_the_loopback_host() {
    let dir = folder("http");
    let s = server(Mode::Read, &dir);
    let token = wwmcp::new_token().unwrap();
    assert_eq!(token.len(), 40);
    assert!(token.chars().next().unwrap().is_ascii_alphabetic());
    let mut http = HttpServer::start(s.server.clone(), 0, token.clone()).unwrap();
    let host = format!("127.0.0.1:{}", http.port);
    let ping = r#"{"jsonrpc": "2.0", "id": 1, "method": "ping"}"#;
    let (status, body) = post(http.port, &token, &host, ping);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"result\""));
    assert_eq!(post(http.port, "wrong", &host, ping).0, 401);
    assert_eq!(post(http.port, &token, "evil.example", ping).0, 403);
    let note = r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#;
    assert_eq!(post(http.port, &token, &host, note).0, 202);
    http.stop();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_stdio_transport_answers_line_by_line() {
    let dir = folder("stdio");
    let s = server(Mode::Read, &dir);
    let input = "{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"ping\"}\n\n{\"jsonrpc\": \"2.0\", \"method\": \"notifications/initialized\"}\n";
    let mut out = Vec::new();
    wwmcp::stdio::serve_on(&s, input.as_bytes(), &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("\"id\":1"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The options of the source `winweaver` reach its tools, and a profile of the option `profile`
/// starts the workspace; a call of the program itself meets no option.
#[test]
fn the_options_of_the_source_decide_and_a_profile_starts_the_workspace() {
    let dir = folder("options");
    let mut config = Config::default();
    config
        .source_mut(SOURCE)
        .options
        .insert("profile".to_owned(), "Strict".to_owned());
    let server = winweaver_mcp::server(Arc::new(tools(&dir)), config).unwrap();
    let status = server
        .call("get_status", &json!({}), &wwmcp::Caller::Client)
        .unwrap();
    assert_eq!(status["current"], json!("Strict"));
    let pc = server.call("read_this_pc", &json!({}), &wwmcp::Caller::Client);
    let why = pc.unwrap_err();
    assert_eq!(why.kind, wwmcp::Kind::NotAllowed);
    assert!(why.message.contains("--read-pc"), "{why}");
    let mut wrong = Config::default();
    wrong
        .source_mut(SOURCE)
        .options
        .insert("profile".to_owned(), "No such".to_owned());
    assert!(winweaver_mcp::server(Arc::new(tools(&dir)), wrong).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
