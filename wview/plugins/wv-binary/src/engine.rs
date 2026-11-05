//! The engine: a file read by a description into a tree of nodes. A node is its name, its place
//! and size in the file and its value; the bytes stay in the file (a `Source`, the `BlockFile` of
//! wwrt for a file on the disk), and the limits of the tree keep a file made to be large or deep
//! from taking the memory: what does not fit is named in the node where it stopped.

use std::fmt::Write as _;

use wwrt::blocks::BlockFile;

use crate::expr::{self, Expr, Part, Scope};
use crate::format::{Endian, Field, Format, Kind, Place, Show};

use wwrt::ww::i18n::{tr, trf};

/// An offset as the notes show it.
fn hex(v: u64) -> String {
    format!("0x{v:X}")
}

/// The bytes a file is read from.
pub trait Source {
    fn len(&self) -> u64;
    /// Copies the bytes from `at` into `out`, as many as the file has there; gives the count.
    fn read(&mut self, at: u64, out: &mut [u8]) -> usize;
}

impl Source for BlockFile {
    fn len(&self) -> u64 {
        BlockFile::len(self)
    }

    fn read(&mut self, at: u64, out: &mut [u8]) -> usize {
        self.read_at(at, out).unwrap_or(0)
    }
}

impl Source for Vec<u8> {
    fn len(&self) -> u64 {
        self.as_slice().len() as u64
    }

    fn read(&mut self, at: u64, out: &mut [u8]) -> usize {
        let Ok(at) = usize::try_from(at) else {
            return 0;
        };
        let n = out.len().min(self.as_slice().len().saturating_sub(at));
        out[..n].copy_from_slice(&self[at..at + n]);
        n
    }
}

/// How large a tree may grow.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub nodes: usize,
    /// The items of one array.
    pub items: u64,
    /// The structures within structures.
    pub depth: usize,
    /// The bytes of a text that ends in a zero.
    pub text: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            nodes: 500_000,
            items: 100_000,
            depth: 32,
            text: 4096,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    /// A structure of its type.
    Struct(String),
    /// An array of its items.
    Array(u64),
    Int(i128),
    Float(f64),
    Text(String),
    Raw,
    /// A field that could not be read; the note says why.
    Missing,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    /// The name of an item of an array by the key of its field.
    pub key: Option<String>,
    pub offset: u64,
    pub size: u64,
    pub data: Data,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// The names of an enum or of flags, a time, or why the node stops.
    pub note: String,
    pub error: bool,
    pub show: Show,
}

/// A file read by a description.
pub struct Tree {
    pub nodes: Vec<Node>,
    /// The nodes that could not be read or stopped at a limit.
    pub errors: usize,
}

impl Tree {
    #[cfg(test)]
    pub fn root(&self) -> &Node {
        &self.nodes[0]
    }

    /// The child of a node by its name.
    #[cfg(test)]
    pub fn child(&self, node: usize, name: &str) -> Option<usize> {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .find(|c| self.nodes[*c].name == name)
    }

    /// The smallest node that holds the byte at `offset`, the deepest of the ones of a size.
    pub fn at(&self, offset: u64) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.size > 0 && offset >= n.offset && offset - n.offset < n.size {
                let better = match best {
                    None => true,
                    Some(b) => n.size <= self.nodes[b].size,
                };
                if better {
                    best = Some(i);
                }
            }
        }
        best
    }

    /// The names of a node from the root down to it.
    pub fn path(&self, node: usize) -> Vec<usize> {
        let mut out = vec![node];
        while let Some(p) = self.nodes[*out.last().unwrap_or(&0)].parent {
            out.push(p);
        }
        out.reverse();
        out
    }

    /// The value of a node as the tree shows it: `0x8664 AMD64`, `"KERNEL32.dll"`, `16 bytes`.
    pub fn value(&self, node: usize) -> String {
        let n = &self.nodes[node];
        let mut out = match &n.data {
            Data::Struct(t) => t.clone(),
            Data::Array(count) => format!("[{count}]"),
            Data::Int(v) => number(*v, n.show),
            Data::Float(f) => format!("{f}"),
            Data::Text(t) => format!("{t:?}"),
            Data::Raw => trf("{n} bytes", &[("n", &n.size)]),
            Data::Missing => String::new(),
        };
        if !n.note.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&n.note);
        }
        out
    }

    /// The name of a node with the key of an item: `[0] .text`.
    pub fn label(&self, node: usize) -> String {
        let n = &self.nodes[node];
        match &n.key {
            Some(k) => format!("{} {k}", n.name),
            None => n.name.clone(),
        }
    }
}

/// A number as it shows.
pub fn number(v: i128, show: Show) -> String {
    match show {
        Show::Hex if v >= 0 => format!("0x{v:X}"),
        Show::Time => match u64::try_from(v) {
            Ok(0) => "0".to_owned(),
            Ok(s) => time(s),
            Err(_) => v.to_string(),
        },
        _ if (0..10).contains(&v) => v.to_string(),
        _ if v < 0 => v.to_string(),
        _ => format!("{v} (0x{v:X})"),
    }
}

/// Seconds since 1970 as a date and a time of UTC.
pub fn time(seconds: u64) -> String {
    let days = (seconds / 86400) as i64;
    let rest = seconds % 86400;
    // The civil date of a count of days (the algorithm of Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

struct Reader<'a> {
    format: &'a Format,
    source: &'a mut dyn Source,
    nodes: Vec<Node>,
    limits: Limits,
    depth: usize,
    full: bool,
}

/// The scope of an expression: the structure being read and the place of its next field.
struct At<'r, 'a> {
    reader: &'r mut Reader<'a>,
    node: usize,
    here: u64,
}

impl Scope for At<'_, '_> {
    fn path(&self, parts: &[Part]) -> Result<i128, String> {
        self.reader.path(self.node, self.here, parts)
    }

    fn here(&self) -> i128 {
        i128::from(self.here)
    }

    fn read(&mut self, width: u8, at: u64) -> Result<i128, String> {
        let endian = self.reader.format.endian;
        self.reader.int(at, width, false, endian)
    }
}

fn offset(value: i128) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| trf("an offset of {value}", &[("value", &value)]))
}

impl Reader<'_> {
    fn push(&mut self, node: Node) -> usize {
        let index = self.nodes.len();
        if let Some(p) = node.parent {
            self.nodes[p].children.push(index);
        }
        self.nodes.push(node);
        index
    }

    fn node(name: &str, offset: u64, data: Data, parent: Option<usize>) -> Node {
        Node {
            name: name.to_owned(),
            key: None,
            offset,
            size: 0,
            data,
            parent,
            children: Vec::new(),
            note: String::new(),
            error: false,
            show: Show::Number,
        }
    }

    fn missing(&mut self, parent: usize, name: &str, at: u64, why: String) -> usize {
        let mut node = Self::node(name, at, Data::Missing, Some(parent));
        node.note = why;
        node.error = true;
        self.push(node)
    }

    /// An integer of the file, `None` past its end.
    fn int(&mut self, at: u64, width: u8, signed: bool, endian: Endian) -> Result<i128, String> {
        let mut bytes = [0u8; 8];
        let width = usize::from(width).min(8);
        if self.source.read(at, &mut bytes[..width]) != width {
            return Err(trf("past the end of the file at {at}", &[("at", &hex(at))]));
        }
        let mut v: u64 = 0;
        for i in 0..width {
            let b = match endian {
                Endian::Little => bytes[i],
                Endian::Big => bytes[width - 1 - i],
            };
            v |= u64::from(b) << (8 * i);
        }
        if signed && width < 8 && v >> (8 * width - 1) == 1 {
            v |= u64::MAX << (8 * width);
        }
        Ok(if signed {
            i128::from(v as i64)
        } else {
            i128::from(v)
        })
    }

    /// The value of a path seen from `node`: its first name in the node or the nearest parent
    /// that has it, the rest down from there.
    fn path(&self, node: usize, here: u64, parts: &[Part]) -> Result<i128, String> {
        let shown = || {
            parts
                .iter()
                .map(|p| match p {
                    Part::Name(n) => n.clone(),
                    Part::Offset => "@offset".to_owned(),
                    Part::Size => "@size".to_owned(),
                    Part::End => "@end".to_owned(),
                    Part::Count => "@count".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(".")
        };
        let (mut at, rest) = match parts.first() {
            Some(Part::Name(first)) => {
                let mut scope = Some(node);
                let mut found = None;
                while let Some(s) = scope {
                    if let Some(c) = self.nodes[s]
                        .children
                        .iter()
                        .copied()
                        .find(|c| self.nodes[*c].name == *first)
                    {
                        found = Some(c);
                        break;
                    }
                    scope = self.nodes[s].parent;
                }
                (
                    found.ok_or_else(|| trf("no field {path}", &[("path", &shown())]))?,
                    &parts[1..],
                )
            }
            _ => (node, parts),
        };
        for (i, part) in rest.iter().enumerate() {
            let n = &self.nodes[at];
            // The structure being read has the size of the fields read so far.
            let size = if at == node && i == 0 && rest.len() == parts.len() {
                here.saturating_sub(n.offset)
            } else {
                n.size
            };
            match part {
                Part::Name(name) => {
                    at = n
                        .children
                        .iter()
                        .copied()
                        .find(|c| self.nodes[*c].name == *name)
                        .ok_or_else(|| trf("no field {path}", &[("path", &shown())]))?;
                }
                Part::Offset => return Ok(i128::from(n.offset)),
                Part::Size => return Ok(i128::from(size)),
                Part::End => return Ok(i128::from(n.offset + size)),
                Part::Count => return Ok(i128::from(n.children.len() as u64)),
            }
        }
        match self.nodes[at].data {
            Data::Int(v) => Ok(v),
            _ => Err(trf("{path} is not a number", &[("path", &shown())])),
        }
    }

    fn eval(&mut self, node: usize, here: u64, e: &Expr) -> Result<i128, String> {
        let mut scope = At {
            reader: self,
            node,
            here,
        };
        expr::eval(e, &mut scope)
    }

    /// An address of the space `rva` as an offset of the file, by its table (the sections of a
    /// PE): within the virtual range of an item, or before the first item (the headers).
    fn rva(&self, rva: i128) -> Result<u64, String> {
        let address = offset(rva)?;
        let Some(space) = &self.format.rva else {
            return Err(tr("no space rva").to_owned());
        };
        let table = self.nodes[0]
            .children
            .iter()
            .copied()
            .find(|c| self.nodes[*c].name == space.table)
            .ok_or_else(|| trf("no table {table} for an rva", &[("table", &space.table)]))?;
        let value = |item: usize, name: &str| -> Option<u64> {
            let c = self.nodes[item]
                .children
                .iter()
                .copied()
                .find(|c| self.nodes[*c].name == name)?;
            match self.nodes[c].data {
                Data::Int(v) => u64::try_from(v).ok(),
                _ => None,
            }
        };
        let mut lowest = u64::MAX;
        for &item in &self.nodes[table].children {
            let (Some(va), Some(vs), Some(file)) = (
                value(item, &space.r#virtual),
                value(item, &space.size),
                value(item, &space.file),
            ) else {
                continue;
            };
            let fs = space
                .file_size
                .as_deref()
                .and_then(|f| value(item, f))
                .unwrap_or(vs);
            lowest = lowest.min(va);
            if address >= va && address - va < vs.max(fs) {
                if address - va >= fs {
                    return Err(trf(
                        "the rva {rva} has no bytes in the file",
                        &[("rva", &hex(address))],
                    ));
                }
                return Ok(file + (address - va));
            }
        }
        if address < lowest {
            return Ok(address);
        }
        Err(trf(
            "the rva {rva} is in no section",
            &[("rva", &hex(address))],
        ))
    }

    fn names(&self, list: &[(u64, String)], v: u64, flags: bool) -> String {
        if !flags {
            return list
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
        }
        let mut out = Vec::new();
        let mut rest = v;
        for (bit, name) in list {
            if *bit != 0 && v & bit == *bit {
                out.push(name.clone());
                rest &= !bit;
            }
        }
        let mut text = out.join(" | ");
        if rest != 0 && !out.is_empty() {
            let _ = write!(text, " | 0x{rest:X}");
        }
        text
    }

    /// One value of a kind at `at`: its node and the end of its bytes.
    fn item(
        &mut self,
        parent: usize,
        name: &str,
        kind: &Kind,
        field: &Field,
        at: u64,
        scope: (usize, u64),
    ) -> (usize, u64) {
        match kind {
            Kind::Int { width, signed } => match self.int(at, *width, *signed, field.endian) {
                Ok(v) => {
                    let mut node = Self::node(name, at, Data::Int(v), Some(parent));
                    node.size = u64::from(*width);
                    node.show = field.show;
                    let n = u64::try_from(v).unwrap_or(0);
                    if let Some(e) = &field.enumeration {
                        node.note = self.names(&self.format.enums[e], n, false);
                    } else if let Some(f) = &field.flags {
                        node.note = self.names(&self.format.flags[f], n, true);
                    }
                    (self.push(node), at + u64::from(*width))
                }
                Err(e) => (self.missing(parent, name, at, e), at),
            },
            Kind::Float { width } => match self.int(at, *width, false, field.endian) {
                Ok(v) => {
                    let f = if *width == 4 {
                        f64::from(f32::from_bits(v as u32))
                    } else {
                        f64::from_bits(v as u64)
                    };
                    let mut node = Self::node(name, at, Data::Float(f), Some(parent));
                    node.size = u64::from(*width);
                    (self.push(node), at + u64::from(*width))
                }
                Err(e) => (self.missing(parent, name, at, e), at),
            },
            Kind::Text { wide } => self.text(parent, name, field, at, *wide, scope),
            Kind::Raw => {
                let size = match field.size.as_ref().map(|s| self.eval(scope.0, scope.1, s)) {
                    Some(Ok(s)) => offset(s),
                    Some(Err(e)) => Err(e),
                    None => Err(tr("raw bytes without a size").to_owned()),
                };
                match size {
                    Ok(size) if at.checked_add(size).is_some_and(|e| e <= self.source.len()) => {
                        let mut node = Self::node(name, at, Data::Raw, Some(parent));
                        node.size = size;
                        (self.push(node), at + size)
                    }
                    Ok(size) => {
                        let why = trf(
                            "{size} bytes at {at} past the end of the file",
                            &[("size", &size), ("at", &hex(at))],
                        );
                        (self.missing(parent, name, at, why), at)
                    }
                    Err(e) => (self.missing(parent, name, at, e), at),
                }
            }
            Kind::Struct(t) => {
                let index = self.structure(name, t, at, Some(parent));
                if !field.key.is_empty() {
                    let key = field.key.iter().find_map(|k| self.key(index, k));
                    self.nodes[index].key = key;
                }
                let end = at + self.nodes[index].size;
                (index, end)
            }
        }
    }

    /// The text a key names in an item.
    fn key(&self, item: usize, path: &[Part]) -> Option<String> {
        let mut at = item;
        for part in path {
            let Part::Name(name) = part else {
                return None;
            };
            at = self.nodes[at]
                .children
                .iter()
                .copied()
                .find(|c| self.nodes[*c].name == *name)?;
        }
        match &self.nodes[at].data {
            Data::Text(t) => Some(t.clone()),
            Data::Int(v) => Some(number(*v, self.nodes[at].show)),
            _ => None,
        }
    }

    fn text(
        &mut self,
        parent: usize,
        name: &str,
        field: &Field,
        at: u64,
        wide: bool,
        scope: (usize, u64),
    ) -> (usize, u64) {
        let unit: u64 = if wide { 2 } else { 1 };
        let fixed = match field.size.as_ref().map(|s| self.eval(scope.0, scope.1, s)) {
            Some(Ok(s)) => Some(offset(s).map(|s| s.min(self.limits.text as u64 * unit))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        };
        let length = match fixed {
            Some(Err(e)) => return (self.missing(parent, name, at, e), at),
            Some(Ok(size)) => size,
            None => self.limits.text as u64 * unit,
        };
        let mut bytes = vec![0u8; length as usize];
        let got = self.source.read(at, &mut bytes);
        bytes.truncate(got);
        let units: Vec<u16> = if wide {
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| match field.endian {
                    Endian::Little => u16::from_le_bytes(*c),
                    Endian::Big => u16::from_be_bytes(*c),
                })
                .collect()
        } else {
            bytes.iter().map(|b| u16::from(*b)).collect()
        };
        let zero = units.iter().position(|u| *u == 0);
        let (shown, size) = match (fixed, zero) {
            // A text of a size: its bytes, the zeros at its end left out.
            (Some(_), z) => (&units[..z.unwrap_or(units.len())], got as u64),
            (None, Some(z)) => (&units[..z], (z as u64 + 1) * unit),
            (None, None) => {
                let why = if (got as u64) < length {
                    trf(
                        "a text past the end of the file at {at}",
                        &[("at", &hex(at))],
                    )
                } else {
                    trf(
                        "a text longer than {n} characters",
                        &[("n", &self.limits.text)],
                    )
                };
                return (self.missing(parent, name, at, why), at);
            }
        };
        let text = if wide {
            String::from_utf16_lossy(shown)
        } else {
            shown
                .iter()
                .map(|u| match *u as u8 {
                    b @ 0x20..=0x7e => b as char,
                    _ => '.',
                })
                .collect()
        };
        let mut node = Self::node(name, at, Data::Text(text), Some(parent));
        node.size = size;
        (self.push(node), at + size)
    }

    /// The kind of a field: its own, or the case of its switch.
    fn kind(&mut self, field: &Field, scope: (usize, u64)) -> Result<Kind, String> {
        if let Some(k) = &field.kind {
            return Ok(k.clone());
        }
        let Some((e, cases)) = &field.switch else {
            return Err(tr("a field without a kind").to_owned());
        };
        let v = self.eval(scope.0, scope.1, e)?;
        cases
            .iter()
            .find(|(c, _)| *c == v)
            .map(|(_, k)| k.clone())
            .ok_or_else(|| {
                trf(
                    "no case of the switch for {value}",
                    &[("value", &format!("0x{v:X}"))],
                )
            })
    }

    /// Whether the bytes of a range are all zero.
    fn zero(&mut self, at: u64, size: u64) -> bool {
        let mut bytes = vec![0u8; size.min(4096) as usize];
        let n = self.source.read(at, &mut bytes);
        n == bytes.len() && bytes.iter().all(|b| *b == 0)
    }

    /// The items of an array field from `at`.
    fn array(
        &mut self,
        parent: usize,
        field: &Field,
        kind: &Kind,
        at: u64,
        scope: (usize, u64),
    ) -> (usize, u64) {
        let count = match field.count.as_ref().map(|c| self.eval(scope.0, scope.1, c)) {
            Some(Ok(c)) => match u64::try_from(c) {
                Ok(c) => Some(c),
                Err(_) => {
                    let why = trf("a count of {n}", &[("n", &c)]);
                    return (self.missing(parent, &field.name, at, why), at);
                }
            },
            Some(Err(e)) => return (self.missing(parent, &field.name, at, e), at),
            None => None,
        };
        let mut node = Self::node(&field.name, at, Data::Array(0), Some(parent));
        node.show = field.show;
        let array = self.push(node);
        let mut here = at;
        let mut n: u64 = 0;
        loop {
            if count.is_some_and(|c| n >= c) {
                break;
            }
            if n >= self.limits.items || self.nodes.len() >= self.limits.nodes {
                self.nodes[array].note = match count {
                    Some(c) => trf(
                        "the first {n} of {count} items",
                        &[("n", &n), ("count", &c)],
                    ),
                    None => trf("the first {n} items", &[("n", &n)]),
                };
                self.nodes[array].error = true;
                break;
            }
            if here >= self.source.len() {
                if count.is_some() {
                    self.nodes[array].note = trf("{n} items to the end of the file", &[("n", &n)]);
                    self.nodes[array].error = true;
                }
                break;
            }
            let first = self.nodes.len();
            let (item, end) = self.item(array, &format!("[{n}]"), kind, field, here, (array, here));
            if field.terminated && end > here && self.zero(here, end - here) {
                // The item of zeros ends the array and is not one of it.
                self.nodes.truncate(first);
                self.nodes[array].children.pop();
                here = end;
                break;
            }
            if self.nodes[item].error || end == here {
                break;
            }
            here = end;
            n += 1;
        }
        self.nodes[array].data = Data::Array(n);
        self.nodes[array].size = here - at;
        (array, here)
    }

    /// A structure of a type at `at`, its fields read in their order.
    fn structure(&mut self, name: &str, type_name: &str, at: u64, parent: Option<usize>) -> usize {
        let me = self.push(Self::node(
            name,
            at,
            Data::Struct(type_name.to_owned()),
            parent,
        ));
        if self.depth >= self.limits.depth {
            self.nodes[me].note = trf("deeper than {n} structures", &[("n", &self.limits.depth)]);
            self.nodes[me].error = true;
            return me;
        }
        let format = self.format;
        let Some(fields) = format.types.get(type_name) else {
            return me;
        };
        self.depth += 1;
        let mut here = at;
        for field in fields {
            if self.nodes.len() >= self.limits.nodes {
                if !self.full {
                    self.full = true;
                    self.nodes[me].note = trf(
                        "the tree holds {n} nodes at most",
                        &[("n", &self.limits.nodes)],
                    );
                    self.nodes[me].error = true;
                }
                break;
            }
            if let Some(when) = &field.when
                && !matches!(self.eval(me, here, when), Ok(v) if v != 0)
            {
                continue;
            }
            if let Some(value) = &field.value {
                match self.eval(me, here, value) {
                    Ok(v) => {
                        let mut node = Self::node(&field.name, here, Data::Int(v), Some(me));
                        node.show = field.show;
                        self.push(node);
                    }
                    Err(e) => {
                        self.missing(me, &field.name, here, e);
                    }
                }
                continue;
            }
            let place = match &field.place {
                Place::Next => Ok(here),
                Place::At(e) => self.eval(me, here, e).and_then(offset),
                Place::Rva(e) => self.eval(me, here, e).and_then(|r| self.rva(r)),
            };
            let place = place.and_then(|p| self.kind(field, (me, here)).map(|k| (p, k)));
            let (from, kind) = match place {
                Ok(p) => p,
                Err(e) => {
                    self.missing(me, &field.name, here, e);
                    continue;
                }
            };
            let end = if field.count.is_some() || field.terminated {
                self.array(me, field, &kind, from, (me, here)).1
            } else {
                self.item(me, &field.name, &kind, field, from, (me, here)).1
            };
            if field.place == Place::Next {
                here = end.max(here);
            }
        }
        self.depth -= 1;
        self.nodes[me].size = here - at;
        me
    }
}

/// A file read by a description into its tree.
pub fn read(format: &Format, source: &mut dyn Source, limits: Limits) -> Tree {
    let mut reader = Reader {
        format,
        source,
        nodes: Vec::new(),
        limits,
        depth: 0,
        full: false,
    };
    reader.structure("file", &format.root, 0, None);
    let errors = reader.nodes.iter().filter(|n| n.error).count();
    Tree {
        nodes: reader.nodes,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(description: &str, bytes: Vec<u8>) -> Tree {
        let format = Format::parse(description).unwrap();
        let mut source = bytes;
        read(&format, &mut source, Limits::default())
    }

    /// The node of a path of names from the root.
    fn find(t: &Tree, path: &[&str]) -> usize {
        path.iter().fold(0, |at, name| {
            t.child(at, name)
                .unwrap_or_else(|| panic!("no {name} in {}", t.label(at)))
        })
    }

    const SHAPES: &str = r#"{
        "format": "wwformat", "version": 1, "name": "Shapes", "extensions": ["shp"],
        "root": "file",
        "spaces": { "rva": { "table": "blocks", "virtual": "address", "size": "size",
                             "file": "offset" } },
        "types": {
            "file": { "fields": [
                { "name": "kind", "type": "u8", "enum": "kinds" },
                { "name": "flags", "type": "u8", "flags": "bits" },
                { "name": "count", "type": "u16" },
                { "name": "body", "switch": "kind", "cases": { "1": "small", "2": "large" } },
                { "name": "blocks", "type": "block", "count": "count", "at": "16", "key": "name" },
                { "name": "pointed", "type": "text", "rva": "0x1002" },
                { "name": "list", "type": "u16", "terminated": true, "at": "40" },
                { "name": "after", "value": "body.@end * 2" },
                { "name": "never", "type": "u8", "when": "kind == 9" }
            ] },
            "small": { "fields": [ { "name": "a", "type": "u8" } ] },
            "large": { "fields": [
                { "name": "a", "type": "u16", "endian": "big" },
                { "name": "b", "type": "i8" }
            ] },
            "block": { "fields": [
                { "name": "name", "type": "text", "size": "4" },
                { "name": "address", "type": "u16" },
                { "name": "size", "type": "u8" },
                { "name": "offset", "type": "u8" }
            ] }
        },
        "enums": { "kinds": { "1": "Small", "2": "Large" } },
        "flags": { "bits": { "0x1": "A", "0x4": "C" } }
    }"#;

    fn shapes() -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[..4].copy_from_slice(&[2, 0x07, 2, 0]);
        b[4..7].copy_from_slice(&[0x01, 0x02, 0xFF]);
        // Two blocks: "one" at 0x1000 (8 bytes at 32), "two" at 0x2000.
        b[16..24].copy_from_slice(&[b'o', b'n', b'e', 0, 0x00, 0x10, 8, 32]);
        b[24..32].copy_from_slice(&[b't', b'w', b'o', 0, 0x00, 0x20, 8, 48]);
        b[32..40].copy_from_slice(b"..hi!\0..");
        b[40..46].copy_from_slice(&[7, 0, 9, 0, 0, 0]);
        b
    }

    #[test]
    fn a_file_is_read_by_its_description() {
        let t = tree(SHAPES, shapes());
        assert_eq!(
            t.errors,
            0,
            "{:?}",
            t.nodes.iter().filter(|n| n.error).collect::<Vec<_>>()
        );
        assert_eq!(t.value(find(&t, &["kind"])), "2 Large");
        assert_eq!(
            t.value(find(&t, &["flags"])),
            "7 A | C | 0x2",
            "a bit of no name stays"
        );
        let body = find(&t, &["body"]);
        assert_eq!(
            t.nodes[body].data,
            Data::Struct("large".to_owned()),
            "the switch took 2"
        );
        assert_eq!(
            t.nodes[find(&t, &["body", "a"])].data,
            Data::Int(0x0102),
            "big-endian"
        );
        assert_eq!(t.nodes[find(&t, &["body", "b"])].data, Data::Int(-1));
        let blocks = find(&t, &["blocks"]);
        assert_eq!(t.nodes[blocks].data, Data::Array(2));
        assert_eq!(t.label(t.nodes[blocks].children[1]), "[1] two");
        assert_eq!(
            t.nodes[find(&t, &["pointed"])].data,
            Data::Text("hi!".to_owned())
        );
        assert_eq!(
            t.nodes[find(&t, &["pointed"])].offset,
            34,
            "0x1002 is 2 into the block at 32"
        );
        let list = find(&t, &["list"]);
        assert_eq!(t.nodes[list].data, Data::Array(2), "the zero ends the list");
        assert_eq!(t.nodes[list].size, 6, "with the zero");
        assert_eq!(t.nodes[find(&t, &["after"])].data, Data::Int(14));
        assert!(t.child(0, "never").is_none());
        assert_eq!(
            t.root().size,
            7,
            "the fields in place, not the ones elsewhere"
        );
        assert_eq!(t.at(5), Some(find(&t, &["body", "a"])));
        assert_eq!(t.at(35), Some(find(&t, &["pointed"])));
    }

    #[test]
    fn a_file_made_to_break_the_reader_is_named_where_it_stops() {
        let mut b = shapes();
        b[2] = 0xFF; // 255 blocks from 16: past the end of the file.
        b[0] = 3; // a kind of no case.
        let t = tree(SHAPES, b);
        let blocks = find(&t, &["blocks"]);
        assert!(t.nodes[blocks].error, "{}", t.value(blocks));
        let body = find(&t, &["body"]);
        assert!(t.value(body).contains("no case"), "{}", t.value(body));
        // A structure that holds itself stops at the depth.
        let deep = r#"{ "format": "wwformat", "version": 1, "name": "Deep", "extensions": [],
            "root": "node", "types": { "node": { "fields": [
                { "name": "next", "type": "node", "at": "0" } ] } } }"#;
        let t = tree(deep, vec![0; 4]);
        assert!(t.nodes.len() <= 40, "{}", t.nodes.len());
        assert!(t.nodes.iter().any(|n| n.note.contains("deeper")));
        // An array of a count too large stops at the limit of its items.
        let many = r#"{ "format": "wwformat", "version": 1, "name": "Many", "extensions": [],
            "root": "file", "types": { "file": { "fields": [
                { "name": "items", "type": "u8", "count": "1000000000000" } ] } } }"#;
        let format = Format::parse(many).unwrap();
        let limits = Limits {
            items: 100,
            ..Limits::default()
        };
        let t = read(&format, &mut vec![1u8; 1000], limits);
        assert_eq!(t.nodes[1].data, Data::Array(100));
        assert!(t.value(1).contains("the first 100"), "{}", t.value(1));
    }

    #[test]
    fn times_and_numbers_show_as_people_read_them() {
        assert_eq!(time(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(time(1_700_000_000), "2023-11-14 22:13:20 UTC");
        assert_eq!(time(951_782_400), "2000-02-29 00:00:00 UTC");
        assert_eq!(number(7, Show::Number), "7");
        assert_eq!(number(332, Show::Number), "332 (0x14C)");
        assert_eq!(number(332, Show::Hex), "0x14C");
        assert_eq!(number(-5, Show::Number), "-5");
    }
}
