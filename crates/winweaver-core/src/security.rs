//! The rights of a registry key as the registry tab shows and changes them: the owner and the
//! access list of its security descriptor, read from and written to the security descriptor
//! language of Windows (SDDL), the form `ConvertSecurityDescriptorToStringSecurityDescriptorW`
//! gives. The entries a key inherits are shown, not changed: a change writes the entries of the key
//! itself, and Windows adds the inherited ones again.

/// The rights regedit calls "Full control" (`KEY_ALL_ACCESS`) and "Read" (`KEY_READ`).
pub const FULL: u32 = 0xF003F;
pub const READ: u32 = 0x20019;
/// The longest descriptor the program reads.
pub const SDDL_MAX: usize = 64 * 1024;

/// What an entry applies to, as regedit names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Key,
    KeyAndSubkeys,
    Subkeys,
}

impl Scope {
    /// The flags of SDDL of the scope.
    fn flags(self) -> &'static str {
        match self {
            Scope::Key => "",
            Scope::KeyAndSubkeys => "CI",
            Scope::Subkeys => "CIIO",
        }
    }
}

/// One entry of the access list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub deny: bool,
    /// The account: a security identifier or an alias of SDDL (`BA` for Administrators).
    pub sid: String,
    /// The rights, the generic ones taken as the rights of a key.
    pub mask: u32,
    /// The rights as SDDL writes them.
    pub rights: String,
    pub scope: Scope,
    /// Given by a key above: shown, not changed.
    pub inherited: bool,
    /// An entry of another kind (an object entry, a callback), kept as SDDL wrote it.
    pub other: Option<String>,
}

impl Entry {
    /// A new entry of the key itself.
    pub fn new(sid: &str, deny: bool, full: bool, scope: Scope) -> Self {
        let (mask, rights) = if full { (FULL, "KA") } else { (READ, "KR") };
        Self {
            deny,
            sid: sid.to_owned(),
            mask,
            rights: rights.to_owned(),
            scope,
            inherited: false,
            other: None,
        }
    }

    pub fn full(&self) -> bool {
        self.mask & FULL == FULL
    }

    pub fn read(&self) -> bool {
        self.mask & READ == READ
    }

    /// Full control or Read, as a change of the tab sets it.
    pub fn set_full(&mut self, full: bool) {
        let (mask, rights) = if full { (FULL, "KA") } else { (READ, "KR") };
        self.mask = mask;
        self.rights = rights.to_owned();
    }

    /// The entry in SDDL.
    fn text(&self) -> String {
        if let Some(other) = &self.other {
            return other.clone();
        }
        let kind = if self.deny { "D" } else { "A" };
        let mut flags = self.scope.flags().to_owned();
        if self.inherited {
            flags.push_str("ID");
        }
        format!("({kind};{flags};{};;;{})", self.rights, self.sid)
    }
}

/// The owner and the access list of a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rights {
    /// The owner: a security identifier or an alias of SDDL; empty when not given.
    pub owner: String,
    /// The key does not inherit the entries of the key above.
    pub protected: bool,
    pub entries: Vec<Entry>,
}

/// The rights of the two letters of SDDL, the generic ones as the rights of a key.
fn right(code: &str) -> Option<u32> {
    Some(match code {
        "GA" | "KA" => FULL,
        "GR" | "KR" | "GX" | "KX" => READ,
        "GW" | "KW" => 0x20006,
        "RC" => 0x20000,
        "SD" => 0x10000,
        "WD" => 0x40000,
        "WO" => 0x80000,
        "CC" => 0x1,
        "DC" => 0x2,
        "LC" => 0x4,
        "SW" => 0x8,
        "RP" => 0x10,
        "WP" => 0x20,
        "DT" => 0x40,
        "LO" => 0x80,
        "CR" => 0x100,
        _ => return None,
    })
}

/// The mask of the rights of an entry: a number after `0x`, or codes of two letters.
fn mask_of(rights: &str) -> Result<u32, String> {
    if let Some(hex) = rights
        .strip_prefix("0x")
        .or_else(|| rights.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).map_err(|_| format!("not a mask of rights: {rights}"));
    }
    if !rights.is_ascii() || !rights.len().is_multiple_of(2) {
        return Err(format!("not a mask of rights: {rights}"));
    }
    let mut mask = 0;
    for i in (0..rights.len()).step_by(2) {
        let code = &rights[i..i + 2];
        mask |= right(code).ok_or_else(|| format!("a right SDDL does not know: {code}"))?;
    }
    Ok(mask)
}

/// Whether the flags of an entry (`CIIO`) hold a flag of two letters.
fn has_flag(flags: &str, flag: &str) -> bool {
    (0..flags.len())
        .step_by(2)
        .any(|i| flags.get(i..i + 2) == Some(flag))
}

/// An entry of the access list from its text inside the brackets.
fn entry_of(text: &str) -> Result<Entry, String> {
    let fields: Vec<&str> = text.split(';').collect();
    if fields.len() < 6 {
        return Err(format!("not an entry of an access list: ({text})"));
    }
    let flags = fields[1];
    let has = |f: &str| has_flag(flags, f);
    let scope = match (has("CI"), has("IO")) {
        (true, true) => Scope::Subkeys,
        (true, false) => Scope::KeyAndSubkeys,
        _ => Scope::Key,
    };
    let other = !matches!(fields[0], "A" | "D") || fields.len() > 6;
    Ok(Entry {
        deny: fields[0] == "D",
        sid: fields[5].to_owned(),
        mask: mask_of(fields[2]).unwrap_or(0),
        rights: fields[2].to_owned(),
        scope,
        inherited: has("ID"),
        other: other.then(|| format!("({text})")),
    })
}

impl Rights {
    /// Reads the owner and the access list of a descriptor in SDDL; the group and the audit list
    /// are left out.
    pub fn parse(sddl: &str) -> Result<Self, String> {
        if sddl.len() > SDDL_MAX || sddl.chars().any(char::is_control) {
            return Err("not a security descriptor the program reads".to_owned());
        }
        // The parts O:, G:, D: and S: begin outside the brackets of the entries.
        let bytes = sddl.as_bytes();
        let mut starts = Vec::new();
        let mut depth = 0usize;
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                b'O' | b'G' | b'D' | b'S' if depth == 0 && bytes.get(i + 1) == Some(&b':') => {
                    starts.push(i);
                }
                _ => {}
            }
        }
        let mut rights = Rights {
            owner: String::new(),
            protected: false,
            entries: Vec::new(),
        };
        for (n, &start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(sddl.len());
            let part = &sddl[start + 2..end];
            match bytes[start] {
                b'O' => rights.owner = part.to_owned(),
                b'D' => {
                    let flags = &part[..part.find('(').unwrap_or(part.len())];
                    rights.protected = flags.contains('P');
                    let mut rest = &part[flags.len()..];
                    while let Some(open) = rest.find('(') {
                        let close = rest[open..]
                            .find(')')
                            .ok_or("an entry of the access list has no end")?;
                        rights.entries.push(entry_of(&rest[open + 1..open + close])?);
                        rest = &rest[open + close + 1..];
                    }
                }
                _ => {}
            }
        }
        Ok(rights)
    }

    /// The access list of the entries of the key itself in SDDL, as a change writes it: the
    /// inherited entries are left out, Windows adds them again, and the denials come first, the
    /// order Windows keeps.
    pub fn dacl(&self) -> String {
        let mut out = String::from("D:");
        if self.protected {
            out.push('P');
        }
        let own: Vec<&Entry> = self.entries.iter().filter(|e| !e.inherited).collect();
        let denials = own.iter().filter(|e| e.deny && e.other.is_none());
        let others = own.iter().filter(|e| !e.deny || e.other.is_some());
        for e in denials.chain(others) {
            out.push_str(&e.text());
        }
        out
    }
}

/// The names of the accounts SDDL writes as two letters, when Windows does not give one.
pub fn alias_title(sid: &str) -> Option<&'static str> {
    Some(match sid {
        "BA" => "Administrators",
        "BU" => "Users",
        "SY" => "SYSTEM",
        "CO" => "CREATOR OWNER",
        "AU" => "Authenticated Users",
        "WD" => "Everyone",
        "AC" => "ALL APPLICATION PACKAGES",
        "RC" => "RESTRICTED",
        "PU" => "Power Users",
        "IU" => "INTERACTIVE",
        "NS" => "NETWORK SERVICE",
        "LS" => "LOCAL SERVICE",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rights of a key of HKEY_LOCAL_MACHINE\SOFTWARE as Windows gives them.
    const SOFTWARE: &str = "O:BAG:SYD:PAI(A;CIIO;KA;;;CO)(A;CI;KA;;;SY)(A;CI;KA;;;BA)(A;CI;KR;;;BU)\
(A;CI;KR;;;AC)(A;CI;0x20019;;;S-1-15-3-1024-1065365936-1281604716-3511738428-1654721687-432734479-3232135806-4053264122-3456934681)";

    #[test]
    fn the_rights_of_a_key_are_read_and_written() {
        let rights = Rights::parse(SOFTWARE).unwrap();
        assert_eq!(rights.owner, "BA");
        assert!(rights.protected);
        assert_eq!(rights.entries.len(), 6);
        let creator = &rights.entries[0];
        assert_eq!(creator.sid, "CO");
        assert_eq!(creator.scope, Scope::Subkeys);
        assert!(creator.full() && !creator.deny && !creator.inherited);
        assert!(rights.entries[3].read() && !rights.entries[3].full());
        assert!(rights.entries[5].read(), "a mask in hexadecimal");
        // Written and read again, the same.
        let again = Rights::parse(&rights.dacl()).unwrap();
        assert_eq!(again.entries, rights.entries);
        assert!(again.protected);
        // An inherited entry is shown and never written.
        let inherited = Rights::parse("D:AI(A;CIID;KA;;;BA)(D;;KR;;;WD)").unwrap();
        assert!(inherited.entries[0].inherited);
        assert!(inherited.entries[1].deny);
        assert_eq!(inherited.dacl(), "D:(D;;KR;;;WD)");
        // A new entry and a change of an entry.
        let mut changed = inherited.clone();
        let added = Entry::new("S-1-5-21-1-2-3-1001", false, true, Scope::KeyAndSubkeys);
        changed.entries.push(added);
        changed.entries[1].set_full(true);
        let expected = "D:(D;;KA;;;WD)(A;CI;KA;;;S-1-5-21-1-2-3-1001)";
        assert_eq!(changed.dacl(), expected);
        assert_eq!(alias_title("BA"), Some("Administrators"));
        assert!(Rights::parse("D:(A;CI;KA;;").is_err());
    }
}
