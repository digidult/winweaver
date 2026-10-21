//! The computer name template: its parts for `section-computer-name.ps1` and the example
//! the form shows.

/// The parts of the computer name template for `section-computer-name.ps1`.
pub fn computer_name_parts(template: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('{')
            && let Some(end) = after.find('}')
        {
            if !text.is_empty() {
                parts.push(format!("text:{text}"));
                text.clear();
            }
            let (name, length) = match after[..end].split_once(':') {
                Some((n, l)) => (n, l.parse::<usize>().ok()),
                None => (&after[..end], None),
            };
            let default_len = match name {
                "serial" => 15,
                "mac" => 12,
                _ => 6,
            };
            parts.push(format!("{name}:{}", length.unwrap_or(default_len)));
            rest = &after[end + 1..];
        } else {
            let c = rest.chars().next().unwrap_or('?');
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !text.is_empty() {
        parts.push(format!("text:{text}"));
    }
    parts
}

/// The serial number the example of a computer name template takes.
pub const EXAMPLE_SERIAL: &str = "5CG1234XYZ";

/// The MAC address the example of a computer name template takes.
pub const EXAMPLE_MAC: &str = "00155D3E5A01";

/// What a computer name template gives with sample values, the way `section-computer-name.ps1`
/// expands it: the last characters of the serial number `5CG1234XYZ` and of the MAC address
/// `00155D3E5A01` (a shorter value stays whole), fixed random characters, the name cut at 15.
pub fn example_name(template: &str) -> String {
    const RANDOM: &str = "K7Q2MX9H4TRW3N5";
    let mut name = String::new();
    for part in computer_name_parts(template) {
        let Some((kind, arg)) = part.split_once(':') else {
            continue;
        };
        if kind == "text" {
            name.push_str(arg);
            continue;
        }
        let length = arg.parse::<usize>().unwrap_or(0).min(RANDOM.len());
        let value = match kind {
            "serial" => EXAMPLE_SERIAL,
            "mac" => EXAMPLE_MAC,
            _ => &RANDOM[..length],
        };
        name.push_str(&value[value.len().saturating_sub(length)..]);
    }
    name.chars().take(15).collect()
}
