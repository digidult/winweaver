//! Find of regedit: a key and every key under it, depth first.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::Serialize;

use super::{Data, Registry, normalize};

/// What a search of the registry looks at, as Find of regedit does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The text looked for, without regard to case.
    pub text: String,
    /// The names of keys.
    pub keys: bool,
    /// The names of values.
    pub values: bool,
    /// The data of values: texts and numbers (decimal or hexadecimal), not bytes.
    pub data: bool,
    /// The whole name or data, not a part of it.
    pub whole: bool,
}

impl Query {
    fn matches(&self, text: &str) -> bool {
        let (text, wanted) = (text.to_lowercase(), self.text.to_lowercase());
        if self.whole {
            text == wanted
        } else {
            text.contains(&wanted)
        }
    }

    fn matches_data(&self, data: &Data) -> bool {
        match data {
            Data::Sz(s) | Data::ExpandSz(s) => self.matches(s),
            Data::MultiSz(items) => items.iter().any(|s| self.matches(s)),
            Data::Dword(n) => self.matches(&n.to_string()) || self.matches(&format!("0x{n:x}")),
            Data::Qword(n) => self.matches(&n.to_string()) || self.matches(&format!("0x{n:x}")),
            Data::Binary(_) | Data::None(_) | Data::Other { .. } => false,
        }
    }
}

/// A place a search found: a key whose name matches, or a value of a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hit {
    pub path: String,
    /// The value found; `None` when the name of the key matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// The end of a search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Found {
    pub hits: Vec<Hit>,
    /// The keys read.
    pub keys: usize,
    /// The keys that could not be read, as without the rights.
    pub unreadable: usize,
    /// Whether every key under the root was read: false when stopped or at a limit.
    pub complete: bool,
}

/// Searches the key `root` and every key under it, depth first in the order regedit goes, until
/// `limit` hits or `max_keys` keys read. Another thread ends it at the next key with `stop`, and
/// `read` counts the keys read for a progress.
pub fn search(
    reg: &dyn Registry,
    root: &str,
    query: &Query,
    limit: usize,
    max_keys: usize,
    stop: &AtomicBool,
    read: &AtomicUsize,
) -> Found {
    let mut found = Found::default();
    let nothing = query.text.is_empty() || !(query.keys || query.values || query.data);
    let Some(root) = normalize(root).filter(|_| !nothing) else {
        return found;
    };
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        if stop.load(Ordering::Relaxed) || found.hits.len() >= limit || found.keys >= max_keys {
            return found;
        }
        let Ok(key) = reg.read(&path) else {
            found.unreadable += 1;
            continue;
        };
        found.keys += 1;
        read.store(found.keys, Ordering::Relaxed);
        let name = path.rsplit('\\').next().unwrap_or(&path);
        if query.keys && query.matches(name) {
            found.hits.push(Hit {
                path: path.clone(),
                value: None,
            });
        }
        for v in &key.values {
            let by_name = query.values && !v.name.is_empty() && query.matches(&v.name);
            let by_data = query.data && query.matches_data(&v.data);
            if (by_name || by_data) && found.hits.len() < limit {
                found.hits.push(Hit {
                    path: path.clone(),
                    value: Some(v.name.clone()),
                });
            }
        }
        // The first subkey is read first, as regedit goes.
        stack.extend(key.subkeys.iter().rev().map(|s| format!("{path}\\{s}")));
    }
    found.complete = true;
    found
}
