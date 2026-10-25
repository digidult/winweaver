//! Group Policy files (`Registry.pol`, the format PReg): the registry values the policies of
//! Group Policy set. A file of the computer (the folder `Machine`) applies to HKEY_LOCAL_MACHINE,
//! a file of the users (the folder `User`) to HKEY_CURRENT_USER; a file elsewhere has no hive
//! until one is chosen. A file is read, changed entry by entry, written back in the same format,
//! turned into a registry file, and takes the entries of another Group Policy or registry file.

use std::path::{Path, PathBuf};

use crate::regfile::{Change, Line, RegFile};
use crate::registry::Data;

pub const EXTENSION: &str = "pol";
const SIGNATURE: &[u8; 4] = b"PReg";
/// The largest file read.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
/// The most entries a file may hold.
const ENTRIES_MAX: usize = 500_000;

/// Whom the policies of a file are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Computer,
    User,
    /// A file outside the folders of Group Policy: its keys have no hive.
    Unknown,
}

impl Scope {
    /// The hive the keys of the file are in.
    pub fn hive(self) -> Option<&'static str> {
        match self {
            Scope::Computer => Some("HKEY_LOCAL_MACHINE"),
            Scope::User => Some("HKEY_CURRENT_USER"),
            Scope::Unknown => None,
        }
    }

    /// The name of the scope in a draft.
    pub fn name(self) -> &'static str {
        match self {
            Scope::Computer => "computer",
            Scope::User => "user",
            Scope::Unknown => "unknown",
        }
    }

    /// The scope of a name of [`Scope::name`]; an unknown name is no hive.
    pub fn from_name(name: &str) -> Self {
        match name {
            "computer" => Scope::Computer,
            "user" => Scope::User,
            _ => Scope::Unknown,
        }
    }

    /// The scope of a file by its folder, as Group Policy keeps them: `Machine` or `User`.
    pub fn of_path(path: &Path) -> Self {
        // Both separators, so a path of Windows reads the same everywhere.
        let text = path.to_string_lossy();
        let mut parts = text.rsplit(['/', '\\']).filter(|p| !p.is_empty());
        parts.next();
        let folder = parts.next().map(str::to_ascii_lowercase);
        match folder.as_deref() {
            Some("machine") => Scope::Computer,
            Some("user") => Scope::User,
            _ => Scope::Unknown,
        }
    }
}

/// What an entry does with its key or value.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// The value is set.
    Set(Data),
    /// `**soft.<name>`: the value is set only when it is not there.
    Soft(Data),
    /// `**del.<name>`: the value is deleted.
    Delete,
    /// `**delvals.`: every value of the key is deleted.
    DeleteAll,
    /// `**DeleteValues`: these values of the key are deleted.
    DeleteValues(Vec<String>),
    /// `**DeleteKeys`: these keys under the key are deleted.
    DeleteKeys(Vec<String>),
    /// `**SecureKey`: the key gets the rights of the administrators only (true) or its own back.
    SecureKey(bool),
    /// A key with no value: the key is made.
    CreateKey,
}

impl Action {
    /// Whether the entry sets or deletes one value, so another entry of the same key and name
    /// replaces it.
    pub fn of_value(&self) -> bool {
        matches!(self, Action::Set(_) | Action::Soft(_) | Action::Delete)
    }
}

/// An entry of a file: the key (without a hive), the name of the value and what is done.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: String,
    pub name: String,
    pub action: Action,
}

/// The type REG_SZ of an entry that only names what it deletes.
const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;

/// A text in UTF-16 LE with its zero.
fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

impl Entry {
    /// The name, the type and the data the file writes for the entry.
    fn raw(&self) -> (String, u32, Vec<u8>) {
        // The data of an entry that deletes: a space, as Group Policy writes it.
        let space = || wide(" ");
        match &self.action {
            Action::Set(d) => {
                let (kind, bytes) = d.to_raw();
                (self.name.clone(), kind, bytes)
            }
            Action::Soft(d) => {
                let (kind, bytes) = d.to_raw();
                (format!("**soft.{}", self.name), kind, bytes)
            }
            Action::Delete => (format!("**del.{}", self.name), REG_SZ, space()),
            Action::DeleteAll => ("**delvals.".to_owned(), REG_SZ, space()),
            Action::DeleteValues(list) => {
                ("**DeleteValues".to_owned(), REG_SZ, wide(&list.join(";")))
            }
            Action::DeleteKeys(list) => ("**DeleteKeys".to_owned(), REG_SZ, wide(&list.join(";"))),
            Action::SecureKey(on) => (
                "**SecureKey".to_owned(),
                REG_DWORD,
                u32::from(*on).to_le_bytes().to_vec(),
            ),
            Action::CreateKey => (String::new(), 0, Vec::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolFile {
    pub path: PathBuf,
    pub scope: Scope,
    pub entries: Vec<Entry>,
}

/// The two bytes of a character of UTF-16 at `i`.
fn char_at(bytes: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]))
}

fn u32_at(bytes: &[u8], i: usize) -> Option<u32> {
    let b = bytes.get(i..i + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// A text of UTF-16 up to its zero, and the index after the zero.
fn text_at(bytes: &[u8], mut i: usize) -> Option<(String, usize)> {
    let mut units = Vec::new();
    loop {
        let c = char_at(bytes, i)?;
        i += 2;
        if c == 0 {
            break;
        }
        units.push(c);
    }
    Some((String::from_utf16_lossy(&units), i))
}

/// The index after the character `c` at `i`, or an error naming the place.
fn expect(bytes: &[u8], i: usize, c: char) -> Result<usize, String> {
    match char_at(bytes, i) {
        Some(found) if found == c as u16 => Ok(i + 2),
        _ => Err(format!("'{c}' expected at byte {i}")),
    }
}

/// The names of a list value of `**DeleteValues` or `**DeleteKeys`, separated by `;`.
fn names(data: &Data) -> Vec<String> {
    match data {
        Data::Sz(s) | Data::ExpandSz(s) => s
            .split(';')
            .map(|n| n.trim_end_matches('\0').trim().to_owned())
            .filter(|n| !n.is_empty())
            .collect(),
        Data::MultiSz(list) => list.clone(),
        _ => Vec::new(),
    }
}

/// What an entry does, from the name of its value and its data.
fn action_of(name: &str, kind: u32, data: &[u8]) -> (String, Action) {
    let value = Data::from_raw(kind, data);
    let lower = name.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("**del.") {
        return (name[name.len() - rest.len()..].to_owned(), Action::Delete);
    }
    if lower.starts_with("**delvals") {
        return (String::new(), Action::DeleteAll);
    }
    if lower == "**deletevalues" {
        return (String::new(), Action::DeleteValues(names(&value)));
    }
    if lower == "**deletekeys" {
        return (String::new(), Action::DeleteKeys(names(&value)));
    }
    if let Some(rest) = lower.strip_prefix("**soft.") {
        return (
            name[name.len() - rest.len()..].to_owned(),
            Action::Soft(value),
        );
    }
    if lower == "**securekey" {
        return (String::new(), Action::SecureKey(value == Data::Dword(1)));
    }
    if name.is_empty() && kind == 0 && data.is_empty() {
        return (String::new(), Action::CreateKey);
    }
    (name.to_owned(), Action::Set(value))
}

/// The entry at `i`, `[key;name;type;size;data]`, and the index after it.
fn entry_at(bytes: &[u8], i: usize) -> Result<(Entry, usize), String> {
    let at = expect(bytes, i, '[')?;
    let (key, at) = text_at(bytes, at).ok_or("a key without its end")?;
    let at = expect(bytes, at, ';')?;
    let (value, at) = text_at(bytes, at).ok_or("a name without its end")?;
    let at = expect(bytes, at, ';')?;
    let kind = u32_at(bytes, at).ok_or("a type without its bytes")?;
    let at = expect(bytes, at + 4, ';')?;
    let size = u32_at(bytes, at).ok_or("a size without its bytes")? as usize;
    let at = expect(bytes, at + 4, ';')?;
    let data = bytes
        .get(at..at.saturating_add(size))
        .ok_or("data beyond the end of the file")?;
    let at = expect(bytes, at + size, ']')?;
    let (name, action) = action_of(&value, kind, data);
    Ok((Entry { key, name, action }, at))
}

impl PolFile {
    /// Reads the bytes of a file: the signature `PReg`, the version 1, then the entries
    /// `[key;name;type;size;data]` in UTF-16.
    pub fn parse(bytes: &[u8], path: &Path) -> Result<Self, String> {
        let name = path.display();
        if bytes.len() < 8 || &bytes[..4] != SIGNATURE {
            return Err(format!("{name}: not a Group Policy file (PReg)"));
        }
        let version = u32_at(bytes, 4).unwrap_or(0);
        if version != 1 {
            return Err(format!("{name}: version {version} of PReg is not known"));
        }
        let mut entries = Vec::new();
        let mut i = 8;
        while i < bytes.len() {
            if entries.len() >= ENTRIES_MAX {
                return Err(format!("{name}: more than {ENTRIES_MAX} entries"));
            }
            let entry = entry_at(bytes, i).map_err(|e| format!("{name}: {e}"))?;
            entries.push(entry.0);
            i = entry.1;
        }
        Ok(Self {
            path: path.to_path_buf(),
            scope: Scope::of_path(path),
            entries,
        })
    }

    /// An empty file at a path, of the scope of its folder.
    pub fn empty(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            scope: Scope::of_path(path),
            entries: Vec::new(),
        }
    }

    /// The bytes of the file: the signature, the version 1 and the entries, as Group Policy
    /// writes them.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = SIGNATURE.to_vec();
        out.extend(1u32.to_le_bytes());
        let ch = |c: char| (c as u16).to_le_bytes();
        for e in &self.entries {
            let (name, kind, data) = e.raw();
            out.extend(ch('['));
            out.extend(wide(&e.key));
            out.extend(ch(';'));
            out.extend(wide(&name));
            out.extend(ch(';'));
            out.extend(kind.to_le_bytes());
            out.extend(ch(';'));
            out.extend((data.len() as u32).to_le_bytes());
            out.extend(ch(';'));
            out.extend(data);
            out.extend(ch(']'));
        }
        out
    }

    /// Writes the file through a temporary one beside it, so a failed write leaves the old file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let io = |e: std::io::Error| format!("{}: {e}", path.display());
        let tmp = path.with_extension("pol.tmp");
        std::fs::write(&tmp, self.to_bytes()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    /// Sets an entry: one of the same key and value name that sets or deletes the value is
    /// replaced (any case), else the entry goes at the end; the index of the entry.
    pub fn put(&mut self, entry: Entry) -> usize {
        let same = self.entries.iter().position(|e| {
            entry.action.of_value()
                && e.action.of_value()
                && e.key.eq_ignore_ascii_case(&entry.key)
                && e.name.eq_ignore_ascii_case(&entry.name)
        });
        match same {
            Some(i) => {
                self.entries[i] = entry;
                i
            }
            None => {
                self.entries.push(entry);
                self.entries.len() - 1
            }
        }
    }

    /// Takes the entries of another Group Policy file, an entry of a value replacing the one of
    /// the same value; the count taken.
    pub fn merge(&mut self, other: &PolFile) -> usize {
        for e in &other.entries {
            self.put(e.clone());
        }
        other.entries.len()
    }

    /// The key of a path of regedit inside the hive of the file, without the hive; `None` for a
    /// path of another hive or a file without one.
    fn own_key(&self, path: &str) -> Option<String> {
        let hive = self.scope.hive()?;
        let full = crate::registry::normalize(path)?;
        let (head, rest) = full.split_once('\\').unwrap_or((full.as_str(), ""));
        (head.eq_ignore_ascii_case(hive) && !rest.is_empty()).then(|| rest.to_owned())
    }

    /// Takes the changes of a registry file whose keys are in the hive of the file: a value set
    /// or deleted, a key made, a key deleted (as `**DeleteKeys` of its parent). The counts of
    /// the changes taken and of the ones left out (another hive, a file without a hive).
    pub fn merge_reg(&mut self, reg: &RegFile) -> (usize, usize) {
        let (mut taken, mut left) = (0, 0);
        for change in reg.changes(None) {
            let path = match &change {
                Change::CreateKey(p) | Change::DeleteKey(p) => p,
                Change::Set { key, .. } | Change::DeleteValue { key, .. } => key,
            };
            let Some(key) = self.own_key(path) else {
                left += 1;
                continue;
            };
            taken += 1;
            match change {
                Change::CreateKey(_) => {
                    if !self
                        .entries
                        .iter()
                        .any(|e| e.key.eq_ignore_ascii_case(&key))
                    {
                        self.put(Entry {
                            key,
                            name: String::new(),
                            action: Action::CreateKey,
                        });
                    }
                }
                Change::DeleteKey(_) => {
                    let (parent, last) = key.rsplit_once('\\').unwrap_or(("", key.as_str()));
                    self.put(Entry {
                        key: parent.to_owned(),
                        name: String::new(),
                        action: Action::DeleteKeys(vec![last.to_owned()]),
                    });
                }
                Change::Set { name, data, .. } => {
                    self.put(Entry {
                        key,
                        name,
                        action: Action::Set(data),
                    });
                }
                Change::DeleteValue { name, .. } => {
                    self.put(Entry {
                        key,
                        name,
                        action: Action::Delete,
                    });
                }
            }
        }
        (taken, left)
    }

    /// The file as a registry file in the hive of its scope: the keys in the order of their
    /// first entry, a value set or deleted as a line, the values and keys deleted by a list as
    /// lines, and what a registry file cannot hold (every value of a key deleted, the rights of
    /// a key, a value set only when absent) said in comments. `None` without a hive.
    pub fn to_reg(&self) -> Option<RegFile> {
        let hive = self.scope.hive()?;
        let mut file = RegFile::default();
        let mut keys: Vec<&str> = Vec::new();
        for e in &self.entries {
            if !keys.iter().any(|k| k.eq_ignore_ascii_case(&e.key)) {
                keys.push(&e.key);
            }
        }
        for key in keys {
            let path = if key.is_empty() {
                hive.to_owned()
            } else {
                format!("{hive}\\{key}")
            };
            file.lines.push(Line::Blank);
            file.lines.push(Line::Key {
                path: path.clone(),
                delete: false,
            });
            let mut after: Vec<Line> = Vec::new();
            for e in self
                .entries
                .iter()
                .filter(|e| e.key.eq_ignore_ascii_case(key))
            {
                match &e.action {
                    Action::Set(d) => file.lines.push(Line::Value {
                        name: e.name.clone(),
                        data: Some(d.clone()),
                    }),
                    Action::Soft(d) => {
                        let note =
                            format!("; Group Policy sets {} only when it is not there", e.name);
                        file.lines.push(Line::Comment(note));
                        file.lines.push(Line::Value {
                            name: e.name.clone(),
                            data: Some(d.clone()),
                        });
                    }
                    Action::Delete => file.lines.push(Line::Value {
                        name: e.name.clone(),
                        data: None,
                    }),
                    Action::DeleteValues(list) => {
                        for name in list {
                            file.lines.push(Line::Value {
                                name: name.clone(),
                                data: None,
                            });
                        }
                    }
                    Action::DeleteAll => file.lines.push(Line::Comment(
                        "; Group Policy deletes every value of this key first".to_owned(),
                    )),
                    Action::SecureKey(on) => {
                        let note = if *on {
                            "; Group Policy lets only the administrators change this key"
                        } else {
                            "; Group Policy gives this key its own rights back"
                        };
                        file.lines.push(Line::Comment(note.to_owned()));
                    }
                    Action::DeleteKeys(list) => {
                        for name in list {
                            after.push(Line::Key {
                                path: format!("{path}\\{name}"),
                                delete: true,
                            });
                        }
                    }
                    Action::CreateKey => {}
                }
            }
            if !after.is_empty() {
                file.lines.push(Line::Blank);
                file.lines.extend(after);
            }
        }
        file.lines.push(Line::Blank);
        Some(file)
    }

    /// Reads a file, at most [`MAX_BYTES`].
    pub fn load(path: &Path) -> Result<Self, String> {
        let size = std::fs::metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len();
        if size > MAX_BYTES {
            return Err(format!(
                "{}: larger than {} MB",
                path.display(),
                MAX_BYTES / 1024 / 1024
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes, path)
    }

    /// A key of the file in the form of regedit, with the hive of the scope when it has one.
    pub fn full_key(&self, key: &str) -> String {
        match self.scope.hive() {
            Some(hive) => format!("{hive}\\{key}"),
            None => key.to_owned(),
        }
    }

    /// The entries that set or delete a value of a key (any case; the key in the form of
    /// regedit, with its hive), or every entry of the key without a value name.
    pub fn find(&self, full_key: &str, value: Option<&str>) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| self.full_key(&e.key).eq_ignore_ascii_case(full_key))
            .filter(|e| match value {
                None => true,
                Some(v) => match &e.action {
                    Action::DeleteAll => true,
                    Action::DeleteValues(list) => list.iter().any(|n| n.eq_ignore_ascii_case(v)),
                    _ => e.name.eq_ignore_ascii_case(v),
                },
            })
            .collect()
    }
}

/// The Group Policy files of this Windows, the computer's and the users', with their scope;
/// a file is there only when a policy was set.
pub fn system_files() -> Vec<(Scope, PathBuf)> {
    let windows = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let root = windows.join("System32").join("GroupPolicy");
    vec![
        (Scope::Computer, root.join("Machine").join("Registry.pol")),
        (Scope::User, root.join("User").join("Registry.pol")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entry of a file as Group Policy writes it.
    fn entry(key: &str, name: &str, kind: u32, data: &[u8]) -> Vec<u8> {
        let wide = |s: &str| -> Vec<u8> {
            s.encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect()
        };
        let ch = |c: char| (c as u16).to_le_bytes().to_vec();
        let mut out = ch('[');
        out.extend(wide(key));
        out.extend(ch(';'));
        out.extend(wide(name));
        out.extend(ch(';'));
        out.extend(kind.to_le_bytes());
        out.extend(ch(';'));
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(ch(';'));
        out.extend(data);
        out.extend(ch(']'));
        out
    }

    #[test]
    fn a_file_of_the_computer_is_read_with_its_actions() {
        let mut bytes = b"PReg".to_vec();
        bytes.extend(1u32.to_le_bytes());
        let key = "Software\\Policies\\Microsoft\\Windows\\DataCollection";
        bytes.extend(entry(key, "AllowTelemetry", 4, &0u32.to_le_bytes()));
        bytes.extend(entry(key, "**del.Old", 1, &[0, 0]));
        let list: Vec<u8> = "A;B\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
        bytes.extend(entry(key, "**DeleteValues", 1, &list));
        let path = Path::new("C:\\Windows\\System32\\GroupPolicy\\Machine\\Registry.pol");
        let pol = PolFile::parse(&bytes, path).unwrap();
        assert_eq!(pol.scope, Scope::Computer);
        assert_eq!(pol.entries.len(), 3);
        assert_eq!(pol.entries[0].action, Action::Set(Data::Dword(0)));
        assert_eq!(pol.entries[1].name, "Old");
        assert_eq!(pol.entries[1].action, Action::Delete);
        assert_eq!(
            pol.entries[2].action,
            Action::DeleteValues(vec!["A".to_owned(), "B".to_owned()])
        );
        let full = format!("HKEY_LOCAL_MACHINE\\{key}");
        assert_eq!(pol.find(&full, Some("allowtelemetry")).len(), 1);
        assert_eq!(
            pol.find(&full, Some("B")).len(),
            1,
            "a value of DeleteValues"
        );
        assert!(pol.find(&full, Some("Other")).is_empty());
    }

    #[test]
    fn a_file_is_written_back_turned_into_a_registry_file_and_merged() {
        let path = Path::new("D:\\Policies\\Registry.pol");
        let mut pol = PolFile::empty(path);
        assert_eq!(pol.scope, Scope::Unknown);
        assert!(pol.to_reg().is_none(), "no hive, no registry file");
        let key = "Software\\Policies\\Test";
        pol.put(Entry {
            key: key.to_owned(),
            name: "On".to_owned(),
            action: Action::Set(Data::Dword(1)),
        });
        pol.put(Entry {
            key: key.to_owned(),
            name: "Old".to_owned(),
            action: Action::Delete,
        });
        pol.put(Entry {
            key: key.to_owned(),
            name: String::new(),
            action: Action::DeleteKeys(vec!["Sub".to_owned()]),
        });
        pol.put(Entry {
            key: key.to_owned(),
            name: String::new(),
            action: Action::SecureKey(true),
        });
        // The same value again replaces its entry.
        let i = pol.put(Entry {
            key: key.to_lowercase(),
            name: "on".to_owned(),
            action: Action::Soft(Data::Sz("x".to_owned())),
        });
        assert_eq!(i, 0);
        assert_eq!(pol.entries.len(), 4);
        let again = PolFile::parse(&pol.to_bytes(), path).unwrap();
        assert_eq!(again.entries, pol.entries, "what is written reads the same");
        pol.scope = Scope::Computer;
        // The keys as the first entry of each writes them, here in lower case.
        let reg = pol.to_reg().unwrap().to_text().to_lowercase();
        assert!(reg.contains("[hkey_local_machine\\software\\policies\\test]"));
        assert!(reg.contains("\"old\"=-"));
        assert!(reg.contains("[-hkey_local_machine\\software\\policies\\test\\sub]"));
        assert!(reg.contains("; group policy lets only the administrators"));
        // A registry file of two hives: the values of the hive of the file come in.
        let text = "Windows Registry Editor Version 5.00\r\n\r\n\
            [HKEY_LOCAL_MACHINE\\Software\\Policies\\Test]\r\n\
            \"New\"=dword:00000002\r\n\
            \"Gone\"=-\r\n\r\n\
            [HKEY_CURRENT_USER\\Software\\Other]\r\n\
            \"X\"=\"y\"\r\n";
        let reg = RegFile::parse(text).unwrap();
        let (taken, left) = pol.merge_reg(&reg);
        assert_eq!(
            (taken, left),
            (3, 2),
            "the key and two values in, the other hive out"
        );
        assert!(
            pol.entries
                .iter()
                .any(|e| e.name == "New" && e.action == Action::Set(Data::Dword(2)))
        );
        assert!(
            pol.entries
                .iter()
                .any(|e| e.name == "Gone" && e.action == Action::Delete)
        );
        let mut other = PolFile::empty(path);
        other.put(Entry {
            key: key.to_owned(),
            name: "New".to_owned(),
            action: Action::Set(Data::Dword(3)),
        });
        let count = pol.entries.len();
        assert_eq!(pol.merge(&other), 1);
        assert_eq!(
            pol.entries.len(),
            count,
            "a value of the other file replaces its entry"
        );
        assert_eq!(Scope::from_name(Scope::User.name()), Scope::User);
    }

    #[test]
    fn a_broken_file_is_a_message() {
        let path = Path::new("x.pol");
        assert!(PolFile::parse(b"PRe", path).is_err());
        assert!(PolFile::parse(b"PReg\x02\0\0\0", path).is_err());
        let mut cut = b"PReg".to_vec();
        cut.extend(1u32.to_le_bytes());
        cut.extend([b'[', 0, b'a', 0]);
        assert!(PolFile::parse(&cut, path).is_err());
    }
}
