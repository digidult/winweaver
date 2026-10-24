//! Reference data: the time zones embedded from `data/resources/`, the editions, and the input
//! languages and keyboard layouts that Windows lists (`Locales`, read by `winweaver_win`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::jsonfile;

const TIMEZONES: &str = include_str!("../../../data/resources/timezones.json");

/// The editions a profile can name with their generic installation keys (Pro and Education) or
/// KMS client keys (the rest), as Microsoft publishes them; a key selects the edition and does not
/// activate Windows.
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
    (
        "Enterprise LTSC 2024 / 2021 / 2019",
        "M7XTQ-FN8P6-TTKYV-9D4CC-J462D",
    ),
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
    /// The titles in the other languages of the interface, by language code.
    #[serde(default)]
    pub titles: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub recommended: bool,
}

impl TimeZone {
    /// The title in a language, English when the language has none, the id when nothing has.
    pub fn title_for(&self, language: &str) -> &str {
        let text = match language {
            "ru" => &self.title,
            "uk" => &self.title_uk,
            other => self.titles.get(other).unwrap_or(&self.title_en),
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

/// A language Windows offers for input, as `winweaver_win::locales` reads it from this Windows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputLanguage {
    /// The tag, such as `uk-UA`, or `uk` for a language without a region.
    pub tag: String,
    /// The locale id in hex (`0422`); empty for a language Windows numbers only at the first
    /// sign-in, such as `ru-UA`.
    pub lcid: String,
    /// The name in the language of Windows.
    pub name: String,
    /// The language it belongs to (`ru` for `ru-UA`), the fallback of one without a locale id.
    pub parent: String,
    /// The input profile Windows gives it first (`0422:00020422`), empty when it has none.
    pub default: String,
}

/// A keyboard layout of Windows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardLayout {
    /// The id in hex, such as `00020422`.
    pub klid: String,
    /// The name in the language of Windows.
    pub name: String,
}

/// The input languages and the keyboard layouts of Windows, read from the system and never
/// embedded. An item of the input languages of a profile is a tag (the language with the layout
/// Windows gives it first) or a layout id such as `0422:00000422` (that language, that layout).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Locales {
    pub languages: Vec<InputLanguage>,
    pub layouts: Vec<KeyboardLayout>,
}

static READER: OnceLock<fn() -> Locales> = OnceLock::new();
static LOCALES: OnceLock<Locales> = OnceLock::new();

/// Names the function that reads the languages and layouts of Windows; the first one named is
/// called at the first use. The window and the MCP server name `winweaver_win::locales`, the
/// tests a list of their own.
pub fn set_locale_reader(read: fn() -> Locales) {
    let _ = READER.set(read);
}

/// The input languages and keyboard layouts, read once; empty when no reader was named.
pub fn locales() -> &'static Locales {
    LOCALES.get_or_init(|| READER.get().map_or_else(Locales::default, |read| read()))
}

/// Whether a locale id is one Windows hands at the first sign-in to a language without its own
/// (0x2000 to 0x4C00 by steps of 0x400, as in `2000:00000419` of `ru-UA`).
fn transient(lcid: &str) -> bool {
    let id = u32::from_str_radix(lcid, 16).unwrap_or(0);
    (0x2000..=0x4C00).contains(&id) && id.is_multiple_of(0x400)
}

impl Locales {
    /// The list of a JSON text, the form tests write it in.
    pub fn parse(text: &str) -> Self {
        jsonfile::parse(text, "locales.json").unwrap_or_default()
    }

    /// The language of a tag, without regard to case.
    pub fn language(&self, tag: &str) -> Option<&InputLanguage> {
        self.languages
            .iter()
            .find(|l| l.tag.eq_ignore_ascii_case(tag))
    }

    /// The language of a locale id in hex.
    pub fn language_of(&self, lcid: &str) -> Option<&InputLanguage> {
        let id = u32::from_str_radix(lcid, 16).ok()?;
        self.languages
            .iter()
            .find(|l| u32::from_str_radix(&l.lcid, 16) == Ok(id))
    }

    /// The keyboard layout of an id in hex.
    pub fn layout(&self, klid: &str) -> Option<&KeyboardLayout> {
        self.layouts
            .iter()
            .find(|l| l.klid.eq_ignore_ascii_case(klid))
    }

    /// The name of the layout of an input profile such as `0422:00020422`, the profile itself
    /// when the layout is not known.
    pub fn layout_name(&self, profile: &str) -> String {
        let klid = profile.split_once(':').map_or(profile, |(_, k)| k);
        match self.layout(klid) {
            Some(layout) => layout.name.clone(),
            None => profile.to_owned(),
        }
    }

    /// The language and the input profile an item of the input languages of a profile names: a
    /// tag gives the profile Windows gives the language first (empty for one without a locale
    /// id, which Windows numbers at the first sign-in), a layout id gives itself.
    pub fn input(&self, item: &str) -> Option<(&InputLanguage, String)> {
        if crate::validate::is_layout(item) {
            let (lcid, _) = item.split_once(':')?;
            return Some((self.language_of(lcid)?, item.to_owned()));
        }
        let language = self.language(item)?;
        if language.lcid.is_empty() {
            return Some((language, String::new()));
        }
        Some((language, language.default.clone()))
    }

    /// The items of a profile for an input language as Windows lists it: a tag it may shorten
    /// (`uk` for `uk-UA`) and its input profiles. The profile Windows gives the language first is
    /// its tag, another one a layout id, a profile of a locale id handed at the first sign-in
    /// the tag as read; `None` when Windows does not know the language.
    pub fn read_items(&self, tag: &str, tips: &[String]) -> Option<Vec<String>> {
        let own = self.language(tag);
        let mut items = Vec::new();
        for tip in tips {
            let lcid = tip.split_once(':').map_or("", |(l, _)| l);
            let item = if transient(lcid) {
                own.map(|l| l.tag.clone())
            } else if crate::validate::is_layout(tip) {
                self.language_of(lcid).map(|l| self.layout_item(l, tip))
            } else {
                own.map(|l| self.specific(l))
            };
            items.push(item?);
        }
        if items.is_empty() {
            items.push(self.specific(own?));
        }
        Some(items)
    }

    /// The item of a language with one of its input profiles: its tag for the profile Windows
    /// gives it first, else the profile.
    fn layout_item(&self, language: &InputLanguage, tip: &str) -> String {
        if language.default.eq_ignore_ascii_case(tip) {
            self.specific(language)
        } else {
            tip.to_uppercase()
        }
    }

    /// The tag of a language with its region: a tag without one names the language of the
    /// profile Windows gives it first (`uk` gives `uk-UA`).
    fn specific(&self, language: &InputLanguage) -> String {
        if language.tag.contains('-') {
            return language.tag.clone();
        }
        let lcid = language.default.split_once(':').map_or("", |(l, _)| l);
        match self.language_of(lcid) {
            Some(l) => l.tag.clone(),
            None => language.tag.clone(),
        }
    }
}

pub fn time_zones() -> &'static [TimeZone] {
    static ZONES: OnceLock<Vec<TimeZone>> = OnceLock::new();
    ZONES.get_or_init(|| jsonfile::parse(TIMEZONES, "timezones.json").unwrap_or_default())
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
        assert!(kyiv.title_for("de").contains("Kiew"));
        assert_eq!(
            kyiv.title_for("pl"),
            kyiv.title_en,
            "English without a title"
        );
        for zone in zones {
            for code in ["de", "es", "fr", "id", "it", "tr"] {
                assert!(
                    zone.titles.contains_key(code),
                    "{} has no {code} title",
                    zone.id
                );
            }
        }
        assert_eq!(generic_key("Pro"), Some("VK7JG-NPHTM-C97JM-9MPGT-3V66T"));
        assert!(generic_key("Home").is_none());
        assert_eq!(editions().count(), 14);
    }

    #[test]
    fn input_languages_are_items_as_windows_lists_them() {
        let l = Locales::parse(include_str!("../tests/data/locales.json"));
        assert_eq!(l.language("UK-ua").unwrap().lcid, "0422");
        assert_eq!(l.language_of("422").unwrap().tag, "uk-UA");
        assert!(l.language_of("").is_none());
        assert_eq!(l.layout_name("0422:00000422"), "Ukrainian");
        let input = |item: &str| l.input(item).map(|(x, p)| (x.tag.clone(), p));
        let uk = ("uk-UA".to_owned(), "0422:00020422".to_owned());
        assert_eq!(input("uk-UA"), Some(uk));
        let standard = ("uk-UA".to_owned(), "0422:00000422".to_owned());
        assert_eq!(input("0422:00000422"), Some(standard));
        let ru = ("ru-UA".to_owned(), String::new());
        assert_eq!(input("ru-UA"), Some(ru));
        assert!(input("xx-XX").is_none());
        assert!(input("0999:00000409").is_none());
        let tips = |t: &str| vec![t.to_owned()];
        let read = |tag: &str, tips: &[String]| l.read_items(tag, tips);
        let one = |item: &str| Some(vec![item.to_owned()]);
        assert_eq!(read("uk", &tips("0422:00020422")), one("uk-UA"));
        assert_eq!(read("uk", &tips("0422:00000422")), one("0422:00000422"));
        assert_eq!(read("uk", &[]), one("uk-UA"));
        assert_eq!(read("UK-ua", &[]), one("uk-UA"));
        assert_eq!(read("ru-UA", &tips("2000:00000419")), one("ru-UA"));
        assert_eq!(read("ru", &[]), one("ru-RU"));
        assert_eq!(read("en-US", &[]), one("en-US"));
        assert_eq!(read("xx", &[]), None);
        assert_eq!(read("uk-XX", &[]), None);
        let two = vec!["0422:00020422".to_owned(), "0422:00000422".to_owned()];
        let both = Some(vec!["uk-UA".to_owned(), "0422:00000422".to_owned()]);
        assert_eq!(read("uk", &two), both);
    }
}
