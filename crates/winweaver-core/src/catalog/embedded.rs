//! The catalog data embedded in the executable from `data/catalog/` at build time.
//!
//! A new rules file is added to `RULE_FILES` here; the test `embedded_matches_data_folder` fails
//! when the folder and this list differ.

use super::{Catalog, CatalogError};
use crate::jsonfile;

/// The version of the catalog data (the predecessor's catalog 0.11 carried over).
pub const CATALOG_VERSION: &str = "0.11";

pub const GROUPS_FILE: &str = "groups.json";
pub const GROUPS: &str = include_str!("../../../../data/catalog/groups.json");

/// The rules files in catalog order: the file name and its content.
pub const RULE_FILES: [(&str, &str); 18] = [
    (
        "00-install.json",
        include_str!("../../../../data/catalog/00-install.json"),
    ),
    (
        "01-oobe.json",
        include_str!("../../../../data/catalog/01-oobe.json"),
    ),
    (
        "02-printing.json",
        include_str!("../../../../data/catalog/02-printing.json"),
    ),
    (
        "03-update.json",
        include_str!("../../../../data/catalog/03-update.json"),
    ),
    (
        "04-defender.json",
        include_str!("../../../../data/catalog/04-defender.json"),
    ),
    (
        "05-security.json",
        include_str!("../../../../data/catalog/05-security.json"),
    ),
    (
        "06-network.json",
        include_str!("../../../../data/catalog/06-network.json"),
    ),
    (
        "07-removable.json",
        include_str!("../../../../data/catalog/07-removable.json"),
    ),
    (
        "08-logging.json",
        include_str!("../../../../data/catalog/08-logging.json"),
    ),
    (
        "09-privacy.json",
        include_str!("../../../../data/catalog/09-privacy.json"),
    ),
    (
        "10-apps.json",
        include_str!("../../../../data/catalog/10-apps.json"),
    ),
    (
        "11-default-user.json",
        include_str!("../../../../data/catalog/11-default-user.json"),
    ),
    (
        "12-user-logon.json",
        include_str!("../../../../data/catalog/12-user-logon.json"),
    ),
    (
        "13-post-oobe.json",
        include_str!("../../../../data/catalog/13-post-oobe.json"),
    ),
    (
        "14-browsers.json",
        include_str!("../../../../data/catalog/14-browsers.json"),
    ),
    (
        "15-office.json",
        include_str!("../../../../data/catalog/15-office.json"),
    ),
    (
        "16-explorer.json",
        include_str!("../../../../data/catalog/16-explorer.json"),
    ),
    (
        "17-shell.json",
        include_str!("../../../../data/catalog/17-shell.json"),
    ),
];

/// The translations of the catalog: the language code and the file content.
pub const TEXT_FILES: [(&str, &str); 2] = [
    ("ru", include_str!("../../../../data/catalog/lang/ru.json")),
    ("uk", include_str!("../../../../data/catalog/lang/uk.json")),
];

/// The catalog built from the embedded data.
pub fn load() -> Result<Catalog, CatalogError> {
    let groups: super::GroupsFile = jsonfile::parse(GROUPS, GROUPS_FILE)?;
    let mut rules = Vec::new();
    for (name, text) in RULE_FILES {
        let file: super::RulesFile = jsonfile::parse(text, name)?;
        rules.extend(file.rules);
    }
    Catalog::from_parts(CATALOG_VERSION, groups.groups, rules)
}
