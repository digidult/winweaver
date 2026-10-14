//! Reference data embedded from `data/resources/`: time zones, keyboards and the editions.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::jsonfile;

const TIMEZONES: &str = include_str!("../../../data/resources/timezones.json");
const KEYBOARDS: &str = include_str!("../../../data/resources/keyboards.json");

/// The editions a profile can name with their generic installation keys (Pro and Education) or
/// KMS client keys (the rest), as Microsoft publishes them; a key selects the edition and does not
/// activate Windows. The names are the ones the predecessor's customer aligned.
pub const EDITION_KEYS: [(&str, &str); 14] = [
    ("Pro", "VK7JG-NPHTM-C97JM-9MPGT-3V66T"),
    ("Enterprise", "NPPR9-FWDCX-D2C8J-H872K-2YT43"),
    ("Education", "YNMGQ-8RYV3-4PGQ3-C8XTP-7CFBY"),
    ("Pro N", "MH37W-N47XK-V7XM9-C7227-GCQG9"),
    ("Pro for Workstations", "NRG8B-VKK3Q-CXVCJ-9G2XF-6Q84J"),
    ("Pro for Workstations N", "9FNHH-K3HBT-3W4TD-6383H-6XYWF"),
    ("Pro Education", "6TP4R-GNPTD-KYYHQ-7B7DP-J447Y"),
    ("Pro Education N", "YVWGF-BXNMC-HTQYQ-CPQ99-66QFC"),
    ("Education N", "2WH4N-8QGBV-H22JP-CT43Q-MDWWJ"),
    ("Enterprise N", "DPH2V-TTNVB-4X9Q3-TJR4H-KHJW4"),
    ("Enterprise G", "YYVX9-NTFWV-6MDM3-9PT4T-4M68B"),
    ("Enterprise G N", "44RPN-FTY23-9VTTB-MP9BX-T84FV"),
    ("Enterprise LTSC 2024 / 2021 / 2019", "M7XTQ-FN8P6-TTKYV-9D4CC-J462D"),
    ("Enterprise N LTSC", "92NFX-8DJQP-P6BBQ-THF9C-7CG2H"),
];

/// The edition names, in the order of `EDITION_KEYS`.
pub fn editions() -> impl Iterator<Item = &'static str> {
    EDITION_KEYS.iter().map(|(name, _)| *name)
}

/// The generic key of an edition.
pub fn generic_key(edition: &str) -> Option<&'static str> {
    EDITION_KEYS
        .iter()
        .find(|(name, _)| *name == edition)
        .map(|(_, key)| *key)
}

/// A Windows time zone: the id Windows uses and its titles.
#[derive(Debug, Clone, Deserialize)]
pub struct TimeZone {
    pub id: String,
    /// The Russian title (the data came with it first).
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub title_uk: String,
    #[serde(default)]
    pub title_en: String,
    #[serde(default)]
    pub recommended: bool,
}

impl TimeZone {
    /// The title in a language, English when the language has none, the id when nothing has.
    pub fn title_for(&self, language: &str) -> &str {
        let text = match language {
            "ru" => &self.title,
            "uk" => &self.title_uk,
            _ => &self.title_en,
        };
        if !text.is_empty() {
            text
        } else if !self.title_en.is_empty() {
            &self.title_en
        } else {
            &self.id
        }
    }
}

/// An input language with its keyboard layout.
#[derive(Debug, Clone, Deserialize)]
pub struct Keyboard {
    /// The language tag, such as `uk-UA`.
    pub tag: String,
    /// The locale id in hex, `None` for a language without one (`ru-UA`).
    #[serde(default)]
    pub lcid: Option<String>,
    /// The keyboard layout id in hex.
    pub klid: String,
    #[serde(default)]
    pub title: String,
    /// Windows assigns the id at the first sign-in; the answer file gets the fallback.
    #[serde(default)]
    pub transient: bool,
    #[serde(default)]
    pub fallback: Option<String>,
}

pub fn time_zones() -> &'static [TimeZone] {
    static ZONES: OnceLock<Vec<TimeZone>> = OnceLock::new();
    ZONES.get_or_init(|| jsonfile::parse(TIMEZONES, "timezones.json").unwrap_or_default())
}

pub fn keyboards() -> &'static [Keyboard] {
    static KEYBOARDS_PARSED: OnceLock<Vec<Keyboard>> = OnceLock::new();
    KEYBOARDS_PARSED
        .get_or_init(|| jsonfile::parse(KEYBOARDS, "keyboards.json").unwrap_or_default())
}

/// The keyboard of a language tag.
pub fn keyboard(tag: &str) -> Option<&'static Keyboard> {
    keyboards().iter().find(|k| k.tag == tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_load() {
        let zones = time_zones();
        assert!(
            zones
                .iter()
                .any(|z| z.id == "FLE Standard Time" && z.recommended)
        );
        let kyiv = zones.iter().find(|z| z.id == "FLE Standard Time").unwrap();
        assert!(kyiv.title_for("en").contains("Kyiv"));
        assert!(!kyiv.title_for("uk").is_empty());
        assert!(keyboard("en-US").is_some_and(|k| k.klid == "00000409"));
        assert!(keyboard("ru-UA").is_some_and(|k| k.transient && k.lcid.is_none()));
        assert_eq!(generic_key("Pro"), Some("VK7JG-NPHTM-C97JM-9MPGT-3V66T"));
        assert!(generic_key("Home").is_none());
        assert_eq!(editions().count(), 14);
    }
}
