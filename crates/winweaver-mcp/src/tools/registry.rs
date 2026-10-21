//! The tools of the registry of this PC: off unless the user allows them, writing only when
//! allowed too.

use std::sync::atomic::{AtomicBool, AtomicUsize};

use serde_json::{Value, json};
use winweaver_core::registry::{self, Data, Query, Registry};
use winweaver_core::security::{self, Entry, Rights, Scope};

use super::{Args, Kind, Needs, Outcome, Tool, ToolError, fail, s, schema};
use crate::{Server, State};

/// The most places and keys a search of the registry gives and reads.
const SEARCH_LIMIT: usize = 500;
const SEARCH_KEYS: usize = 100_000;

pub(super) const READ_REGISTRY_KEY: Tool = Tool {
    name: "read_registry_key",
    description: "Reads a key of the registry of this PC: its subkeys and its values with their types (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY, REG_NONE). A path like HKEY_LOCAL_MACHINE\\Software or HKLM\\Software. Needs the option registry read or write.",
    needs: Needs::REGISTRY_READ,
    schema: || schema(json!({ "path": s("The key") }), &["path"]),
    run: read_registry_key,
};

pub(super) const SEARCH_REGISTRY: Tool = Tool {
    name: "search_registry",
    description: "Searches a key of the registry of this PC and every key under it, as Find of regedit: the names of keys, the names of values and the data of values (texts and numbers), without regard to case. Gives the keys and values found (at most limit: 100 by default, 500 at most, within 100000 keys read) and whether every key was read. Needs the option registry read or write.",
    needs: Needs::REGISTRY_READ,
    schema: || {
        schema(
            json!({
                "path": s("The key to search under"),
                "text": s("The text looked for"),
                "keys": { "type": "boolean", "description": "Look at the names of keys; true by default" },
                "values": { "type": "boolean", "description": "Look at the names of values; true by default" },
                "data": { "type": "boolean", "description": "Look at the data of values; true by default" },
                "whole": { "type": "boolean", "description": "Match the whole name or data only; false by default" },
                "limit": { "type": "integer", "minimum": 1, "maximum": SEARCH_LIMIT }
            }),
            &["path", "text"],
        )
    },
    run: search_registry,
};

pub(super) const SET_REGISTRY_VALUE: Tool = Tool {
    name: "set_registry_value",
    description: "Writes a value of a key that exists. The data is text: lines for REG_MULTI_SZ, a decimal or 0x hexadecimal number for REG_DWORD and REG_QWORD, hexadecimal pairs for REG_BINARY. Needs the option registry write.",
    needs: Needs::REGISTRY_WRITE,
    schema: || {
        schema(
            json!({
                "path": s("The key"),
                "name": s("The value; empty for the default value of the key"),
                "type": s("REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD or REG_BINARY"),
                "data": s("The data as text")
            }),
            &["path", "name", "type", "data"],
        )
    },
    run: set_registry_value,
};

pub(super) const DELETE_REGISTRY_VALUE: Tool = Tool {
    name: "delete_registry_value",
    description: "Deletes a value of a key. Needs the option registry write.",
    needs: Needs::REGISTRY_WRITE,
    schema: || {
        schema(
            json!({ "path": s("The key"), "name": s("The value") }),
            &["path", "name"],
        )
    },
    run: delete_registry_value,
};

pub(super) const CREATE_REGISTRY_KEY: Tool = Tool {
    name: "create_registry_key",
    description: "Creates a key and the keys above it that are missing. Needs the option registry write.",
    needs: Needs::REGISTRY_WRITE,
    schema: || schema(json!({ "path": s("The new key") }), &["path"]),
    run: create_registry_key,
};

pub(super) const DELETE_REGISTRY_KEY: Tool = Tool {
    name: "delete_registry_key",
    description: "Deletes a key with all its subkeys and values; a hive is never deleted. Needs the option registry write.",
    needs: Needs::REGISTRY_WRITE,
    schema: || schema(json!({ "path": s("The key") }), &["path"]),
    run: delete_registry_key,
};

pub(super) const READ_REGISTRY_RIGHTS: Tool = Tool {
    name: "read_registry_rights",
    description: "Reads the permissions of a key: its owner, whether it inherits the entries of the key above, and every entry of its access list (the account with its name, allow or deny, full, read or special access with its mask, what it applies to, inherited or its own). Needs the option registry read or write.",
    needs: Needs::REGISTRY_READ,
    schema: || schema(json!({ "path": s("The key") }), &["path"]),
    run: read_registry_rights,
};

pub(super) const SET_REGISTRY_RIGHTS: Tool = Tool {
    name: "set_registry_rights",
    description: "Replaces the entries of a key itself with the given ones; the inherited entries stay, and Windows adds them again. A key of the machine changes only when WinWeaver runs as administrator. Gives the permissions after the change. Needs the option registry write.",
    needs: Needs::REGISTRY_WRITE,
    schema: || {
        schema(
            json!({
                "path": s("The key"),
                "entries": {
                    "type": "array",
                    "maxItems": 64,
                    "description": "The entries of the key itself",
                    "items": {
                        "type": "object",
                        "properties": {
                            "account": s("An account or a group of this PC by its name, a security identifier (S-1-...) or an alias of SDDL (BA, BU, SY)"),
                            "type": s("allow (by default) or deny"),
                            "access": s("read (by default) or full"),
                            "applies_to": s("key_and_subkeys (by default), key or subkeys")
                        },
                        "required": ["account"],
                        "additionalProperties": false
                    }
                },
                "inherits": { "type": "boolean", "description": "Whether the key takes the entries of the key above; unchanged when left out" }
            }),
            &["path", "entries"],
        )
    },
    run: set_registry_rights,
};

/// The owner and the access list of a key.
fn read_rights(reg: &dyn Registry, path: &str) -> Result<Rights, ToolError> {
    let sddl = reg.security(path).map_err(|e| fail(Kind::ReadFailed, e))?;
    Rights::parse(&sddl).map_err(|e| fail(Kind::ReadFailed, e))
}

/// The permissions of a key as the tools give them, the name of each account beside it.
fn rights_json(reg: &dyn Registry, path: &str, rights: &Rights) -> Value {
    let name = |sid: &str| {
        reg.account_name(sid)
            .or_else(|| security::alias_title(sid).map(str::to_owned))
            .unwrap_or_default()
    };
    let entries: Vec<Value> = rights
        .entries
        .iter()
        .map(|e| {
            let access = if e.full() {
                "full"
            } else if e.read() {
                "read"
            } else {
                "special"
            };
            let applies_to = match e.scope {
                Scope::Key => "key",
                Scope::KeyAndSubkeys => "key_and_subkeys",
                Scope::Subkeys => "subkeys",
            };
            json!({
                "account": e.sid,
                "name": name(&e.sid),
                "type": if e.deny { "deny" } else { "allow" },
                "access": access,
                "mask": format!("0x{:x}", e.mask),
                "applies_to": applies_to,
                "inherited": e.inherited,
            })
        })
        .collect();
    json!({
        "path": path,
        "owner": rights.owner,
        "owner_name": name(&rights.owner),
        "inherits": !rights.protected,
        "entries": entries,
    })
}

/// An entry of `set_registry_rights`. The account is a name Windows knows, or an identifier whose
/// characters are checked, so no text of the caller reaches the access list unchecked.
fn entry_arg(reg: &dyn Registry, item: &Value) -> Result<Entry, ToolError> {
    let field = |name: &str| item.get(name).and_then(Value::as_str).unwrap_or_default();
    let account = field("account").trim();
    let digits =
        |rest: &str| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '-');
    let alias = account.len() == 2 && account.chars().all(|c| c.is_ascii_uppercase());
    let sid = match account.strip_prefix("S-1-") {
        Some(rest) if digits(rest) => account.to_owned(),
        _ if alias => account.to_owned(),
        _ => match reg.account_sid(account) {
            Some(sid) => sid,
            None => {
                let why = format!("no account of this PC: {account}");
                return Err(fail(Kind::Invalid, why));
            }
        },
    };
    let (kind, access, scope) = (field("type"), field("access"), field("applies_to"));
    let deny = match kind {
        "" | "allow" => false,
        "deny" => true,
        other => {
            let why = format!("type is allow or deny, not {other}");
            return Err(fail(Kind::Invalid, why));
        }
    };
    let full = match access {
        "" | "read" => false,
        "full" => true,
        other => {
            let why = format!("access is read or full, not {other}");
            return Err(fail(Kind::Invalid, why));
        }
    };
    let scope = match scope {
        "" | "key_and_subkeys" => Scope::KeyAndSubkeys,
        "key" => Scope::Key,
        "subkeys" => Scope::Subkeys,
        other => {
            let why = format!("applies_to is key_and_subkeys, key or subkeys, not {other}");
            return Err(fail(Kind::Invalid, why));
        }
    };
    Ok(Entry::new(&sid, deny, full, scope))
}

fn read_registry_key(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    let key = server
        .registry
        .read(&path)
        .map_err(|e| fail(Kind::ReadFailed, e))?;
    Ok(json!(key))
}

fn search_registry(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    if registry::normalize(&path).is_none() {
        return Err(fail(
            Kind::Invalid,
            format!("not a key of the registry: {path}"),
        ));
    }
    let query = Query {
        text: args.text("text")?.trim().to_owned(),
        keys: args.flag_or("keys", true),
        values: args.flag_or("values", true),
        data: args.flag_or("data", true),
        whole: args.flag_or("whole", false),
    };
    if query.text.is_empty() {
        return Err(fail(Kind::Invalid, "the argument text must not be empty"));
    }
    let limit = args.limit("limit", 100, SEARCH_LIMIT);
    let (stop, read) = (AtomicBool::new(false), AtomicUsize::new(0));
    let found = registry::search(
        &*server.registry,
        &path,
        &query,
        limit,
        SEARCH_KEYS,
        &stop,
        &read,
    );
    Ok(json!(found))
}

fn set_registry_value(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    let name = args.optional_text("name");
    let kind = args.text("type")?;
    let data =
        Data::parse(&kind, &args.optional_text("data")).map_err(|e| fail(Kind::Invalid, e))?;
    server
        .registry
        .set(&path, &name, &data)
        .map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "written": path, "name": name, "type": kind }))
}

fn delete_registry_value(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    let name = args.optional_text("name");
    server
        .registry
        .delete_value(&path, &name)
        .map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "deleted": path, "name": name }))
}

fn create_registry_key(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    server
        .registry
        .create_key(&path)
        .map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "created": path }))
}

fn delete_registry_key(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    server
        .registry
        .delete_key(&path)
        .map_err(|e| fail(Kind::WriteFailed, e))?;
    Ok(json!({ "deleted": path }))
}

fn read_registry_rights(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    let rights = read_rights(&*server.registry, &path)?;
    Ok(rights_json(&*server.registry, &path, &rights))
}

fn set_registry_rights(server: &Server, _state: &mut State, args: &Args<'_>) -> Outcome {
    let path = args.text("path")?;
    let mut rights = read_rights(&*server.registry, &path)?;
    if let Some(inherit) = args.get("inherits").and_then(Value::as_bool) {
        rights.protected = !inherit;
    }
    let list = args
        .list("entries")
        .ok_or_else(|| fail(Kind::Invalid, "the argument entries is a list"))?;
    let mut own = Vec::new();
    for item in list.iter().take(64) {
        own.push(entry_arg(&*server.registry, item)?);
    }
    if rights.protected && own.is_empty() {
        let why = "a key that does not inherit needs at least one entry of its own";
        return Err(fail(Kind::Invalid, why));
    }
    rights.entries.retain(|e| e.inherited);
    rights.entries.extend(own);
    server
        .registry
        .set_security(&path, &rights.dacl(), rights.protected)
        .map_err(|e| fail(Kind::WriteFailed, e))?;
    let after = read_rights(&*server.registry, &path)?;
    Ok(rights_json(&*server.registry, &path, &after))
}
