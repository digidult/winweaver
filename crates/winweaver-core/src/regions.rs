//! The regions (countries) Windows lists for the user region of a profile: the two-letter code,
//! the GeoID and the name in the language of Windows. They are read once from the system by the
//! reader the window and the MCP server name (`set_region_reader`), a list of their own in the
//! tests; the program embeds no list of them.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::jsonfile;

/// A region Windows knows, as `winweaver_win::regions` reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Region {
    /// The two-letter code (`UA`).
    pub code: String,
    /// The GeoID (241 for Ukraine), the value of `Nation` in the registry.
    pub id: u32,
    /// The name in the language of Windows.
    pub name: String,
}

impl Region {
    /// The GeoID as the registry holds it.
    pub fn id_text(&self) -> String {
        self.id.to_string()
    }
}

/// The regions of Windows, by their names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Regions {
    pub regions: Vec<Region>,
}

static READER: OnceLock<fn() -> Regions> = OnceLock::new();
static REGIONS: OnceLock<Regions> = OnceLock::new();

/// Names the function that reads the regions of Windows; the first one named is called at the
/// first use. The window and the MCP server name `winweaver_win::regions`, the tests a list of
/// their own.
pub fn set_region_reader(read: fn() -> Regions) {
    let _ = READER.set(read);
}

/// The regions of Windows, read once; empty when no reader was named or Windows is not there.
pub fn regions() -> &'static Regions {
    REGIONS.get_or_init(|| READER.get().map_or_else(Regions::default, |read| read()))
}

impl Regions {
    /// The list of a JSON text, the form tests write it in.
    pub fn parse(text: &str) -> Self {
        jsonfile::parse(text, "regions.json").unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// The region of a two-letter code, without regard to case.
    pub fn by_code(&self, code: &str) -> Option<&Region> {
        let code = code.trim();
        self.regions
            .iter()
            .find(|r| r.code.eq_ignore_ascii_case(code))
    }

    /// The region of a GeoID written as a number.
    pub fn by_id(&self, id: &str) -> Option<&Region> {
        let id: u32 = id.trim().parse().ok()?;
        self.regions.iter().find(|r| r.id == id)
    }

    /// The region a text names: by its code or by its GeoID.
    pub fn find(&self, text: &str) -> Option<&Region> {
        self.by_code(text).or_else(|| self.by_id(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_are_found_by_code_and_by_id() {
        let r = Regions::parse(include_str!("../tests/data/regions.json"));
        assert!(r.regions.len() >= 12);
        assert_eq!(r.by_code("ua").unwrap().id, 241);
        assert_eq!(r.by_id("241").unwrap().code, "UA");
        assert_eq!(r.by_id(" 94 ").unwrap().name, "Germany");
        assert_eq!(r.find("DE").unwrap().id_text(), "94");
        assert_eq!(r.find("94").unwrap().code, "DE");
        assert!(r.find("XX").is_none() && r.find("999").is_none());
        assert!(r.by_id("UA").is_none());
        assert!(Regions::parse("not json").is_empty());
    }
}
