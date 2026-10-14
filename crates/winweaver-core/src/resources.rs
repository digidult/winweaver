//! Reference data embedded from `data/resources/`: time zones, keyboards and the editions.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::jsonfile;

const TIMEZONES: &str = include_str!("../../../data/resources/timezones.json");
const KEYBOARDS: &str = include_str!("../../../data/resources/keyboards.json");

/// The editions a profile can name; Pro and Education have generic installation keys, the
/// others are activated through KMS or a key typed later.
pub const EDITIONS: [&str; 14] = [
    "Pro",
    "Pro N",
    "Pro for Workstations",
    "Pro N for Workstations",
    "Pro Education",
    "Pro Education N",
    "Education",
    "Education N",
    "Enterprise",
    "Enterprise N",
    "Enterprise G",
    "Enterprise G N",
    "Enterprise LTSC",
    "Enterprise N LTSC",
];

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
    KEYBOARDS_PARSED.get_or_init(|| jsonfile::parse(KEYBOARDS, "keyboards.json").unwrap_or_default())
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
        assert!(zones.iter().any(|z| z.id == "FLE Standard Time" && z.recommended));
        let kyiv = zones.iter().find(|z| z.id == "FLE Standard Time").unwrap();
        assert!(kyiv.title_for("en").contains("Kyiv"));
        assert!(!kyiv.title_for("uk").is_empty());
        assert!(keyboard("en-US").is_some_and(|k| k.klid == "00000409"));
        assert!(keyboard("ru-UA").is_some_and(|k| k.transient && k.lcid.is_none()));
        assert!(EDITIONS.contains(&"Pro"));
    }
}
