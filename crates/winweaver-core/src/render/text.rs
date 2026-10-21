//! The escapes of the texts the answer file and the scripts hold: PowerShell literals, XML,
//! ASCII and Base64.

/// A PowerShell single-quoted literal: quotes doubled, also the typographic ones PowerShell
/// treats as quotes.
pub fn ps_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        if c == '\'' || ('\u{2018}'..='\u{201B}').contains(&c) {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// Text inside an XML element.
pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// JSON text with every character outside ASCII escaped as `\uXXXX`.
pub fn ascii_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// A text for a comment of the answer file or of a script: a character outside ASCII or a control
/// character becomes `?`, so the file stays ASCII; the profile embedded in the answer file keeps the
/// text whole.
pub fn ascii_text(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '?'
            }
        })
        .collect()
}

/// A text for an XML comment: ASCII, and no two hyphens in a row.
pub(super) fn comment_text(text: &str) -> String {
    let mut text = ascii_text(text);
    while text.contains("--") {
        text = text.replace("--", "- -");
    }
    text
}

/// Base64 of the UTF-8 bytes of a text (standard alphabet with padding).
pub fn base64(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[((v >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((v >> 12) & 63) as usize] as char);
        out.push(if n > 1 {
            ALPHABET[((v >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if n > 2 {
            ALPHABET[(v & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub(crate) fn ps_list(items: &[String]) -> String {
    items
        .iter()
        .map(|s| ps_quote(s))
        .collect::<Vec<_>>()
        .join(",")
}
