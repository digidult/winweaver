//! The data of a value in the types of regedit: its bytes, its text in the editor and in the
//! list, its number.

use serde::{Deserialize, Serialize};

use super::DATA_MAX;

/// The data of a value in the types of regedit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Data {
    #[serde(rename = "REG_SZ")]
    Sz(String),
    #[serde(rename = "REG_EXPAND_SZ")]
    ExpandSz(String),
    #[serde(rename = "REG_MULTI_SZ")]
    MultiSz(Vec<String>),
    #[serde(rename = "REG_DWORD")]
    Dword(u32),
    #[serde(rename = "REG_QWORD")]
    Qword(u64),
    #[serde(rename = "REG_BINARY")]
    Binary(Vec<u8>),
    #[serde(rename = "REG_NONE")]
    None(Vec<u8>),
    /// Another type (REG_LINK, REG_RESOURCE_LIST and the like): its number and bytes, shown, not
    /// edited.
    #[serde(rename = "OTHER")]
    Other { kind: u32, bytes: Vec<u8> },
}

fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let text = String::from_utf16_lossy(&units);
    text.trim_end_matches('\0').to_owned()
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

impl Data {
    /// The name of the type, as regedit shows it.
    pub fn type_name(&self) -> String {
        match self {
            Data::Sz(_) => "REG_SZ".to_owned(),
            Data::ExpandSz(_) => "REG_EXPAND_SZ".to_owned(),
            Data::MultiSz(_) => "REG_MULTI_SZ".to_owned(),
            Data::Dword(_) => "REG_DWORD".to_owned(),
            Data::Qword(_) => "REG_QWORD".to_owned(),
            Data::Binary(_) => "REG_BINARY".to_owned(),
            Data::None(_) => "REG_NONE".to_owned(),
            Data::Other { kind, .. } => match kind {
                6 => "REG_LINK".to_owned(),
                8 => "REG_RESOURCE_LIST".to_owned(),
                9 => "REG_FULL_RESOURCE_DESCRIPTOR".to_owned(),
                10 => "REG_RESOURCE_REQUIREMENTS_LIST".to_owned(),
                n => format!("0x{n:x}"),
            },
        }
    }

    /// Whether the editor of the tab changes it: every type but the rare ones.
    pub fn editable(&self) -> bool {
        !matches!(self, Data::Other { .. })
    }

    /// The data in one line, as regedit lists it.
    pub fn display(&self) -> String {
        match self {
            Data::Sz(s) | Data::ExpandSz(s) => s.clone(),
            Data::MultiSz(items) => items.join(" "),
            Data::Dword(n) => format!("0x{n:08x} ({n})"),
            Data::Qword(n) => format!("0x{n:016x} ({n})"),
            Data::Binary(b) | Data::None(b) | Data::Other { bytes: b, .. } => hex(b),
        }
    }

    /// The text the editor starts from: lines for a multi-string, the number for a dword or
    /// qword, hexadecimal pairs for bytes.
    pub fn edit_text(&self) -> String {
        match self {
            Data::Sz(s) | Data::ExpandSz(s) => s.clone(),
            Data::MultiSz(items) => items.join("\n"),
            Data::Dword(n) => n.to_string(),
            Data::Qword(n) => n.to_string(),
            Data::Binary(b) | Data::None(b) | Data::Other { bytes: b, .. } => hex(b),
        }
    }

    /// The data of a type from the text of the editor: a dword or qword in decimal or with `0x` in
    /// hexadecimal, bytes as hexadecimal pairs (spaces and commas ignored), a multi-string as lines.
    pub fn parse(type_name: &str, text: &str) -> Result<Self, String> {
        let number = |text: &str, max: u64| -> Result<u64, String> {
            let t = text.trim();
            let n = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                Some(h) => u64::from_str_radix(h, 16),
                None => t.parse::<u64>(),
            }
            .map_err(|_| format!("not a number: {t}"))?;
            if n > max {
                return Err(format!("{t} is larger than {max}"));
            }
            Ok(n)
        };
        let data = match type_name {
            "REG_SZ" => Data::Sz(text.to_owned()),
            "REG_EXPAND_SZ" => Data::ExpandSz(text.to_owned()),
            "REG_MULTI_SZ" => Data::MultiSz(
                text.lines()
                    .map(str::to_owned)
                    .filter(|l| !l.is_empty())
                    .collect(),
            ),
            "REG_DWORD" => Data::Dword(number(text, u64::from(u32::MAX))? as u32),
            "REG_QWORD" => Data::Qword(number(text, u64::MAX)?),
            "REG_BINARY" => Data::Binary(unhex(text)?),
            "REG_NONE" => Data::None(unhex(text)?),
            other => return Err(format!("the type {other} is not edited here")),
        };
        if data.to_raw().1.len() > DATA_MAX {
            return Err(format!("larger than {} KB", DATA_MAX / 1024));
        }
        Ok(data)
    }

    /// The data of a type number and its bytes as the registry gives them.
    pub fn from_raw(kind: u32, bytes: &[u8]) -> Self {
        match kind {
            0 => Data::None(bytes.to_vec()),
            1 => Data::Sz(utf16_text(bytes)),
            2 => Data::ExpandSz(utf16_text(bytes)),
            3 => Data::Binary(bytes.to_vec()),
            4 if bytes.len() == 4 => {
                Data::Dword(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            }
            7 => Data::MultiSz(
                utf16_text(bytes)
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            11 if bytes.len() == 8 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(bytes);
                Data::Qword(u64::from_le_bytes(b))
            }
            other => Data::Other {
                kind: other,
                bytes: bytes.to_vec(),
            },
        }
    }

    /// The type number and the bytes to write.
    pub fn to_raw(&self) -> (u32, Vec<u8>) {
        match self {
            Data::None(b) => (0, b.clone()),
            Data::Sz(s) => (1, utf16_bytes(s)),
            Data::ExpandSz(s) => (2, utf16_bytes(s)),
            Data::Binary(b) => (3, b.clone()),
            Data::Dword(n) => (4, n.to_le_bytes().to_vec()),
            Data::MultiSz(items) => {
                let mut bytes: Vec<u8> = Vec::new();
                for item in items {
                    bytes.extend(item.encode_utf16().flat_map(u16::to_le_bytes));
                    bytes.extend([0, 0]);
                }
                bytes.extend([0, 0]);
                (7, bytes)
            }
            Data::Qword(n) => (11, n.to_le_bytes().to_vec()),
            Data::Other { kind, bytes } => (*kind, bytes.clone()),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn unhex(text: &str) -> Result<Vec<u8>, String> {
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err("an odd count of hexadecimal digits".to_owned());
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&digits[i..i + 2], 16)
                .map_err(|_| format!("not hexadecimal: {}", &digits[i..i + 2]))
        })
        .collect()
}
