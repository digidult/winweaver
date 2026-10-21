//! The pages of WinWeaver: the guide in the language of the interface, the license and the
//! reference of the parameters, embedded in the executable.

use crate::*;

/// The guide shown at the first start and from the menu (F1).
pub const GUIDE: &str = include_str!("../../../data/docs/guide.md");
/// The translated guides: the language code and the page.
pub const GUIDES: [(&str, &str); 8] = [
    ("uk", include_str!("../../../data/lang/uk/guide.md")),
    ("ru", include_str!("../../../data/lang/ru/guide.md")),
    ("de", include_str!("../../../data/lang/de/guide.md")),
    ("es", include_str!("../../../data/lang/es/guide.md")),
    ("fr", include_str!("../../../data/lang/fr/guide.md")),
    ("id", include_str!("../../../data/lang/id/guide.md")),
    ("it", include_str!("../../../data/lang/it/guide.md")),
    ("tr", include_str!("../../../data/lang/tr/guide.md")),
];

pub const GUIDE_TITLE: &str = "Guide.md";
/// The parameter reference of the catalog, carried over with its data (`data/reference/`): the
/// file name and the page. The `doc` field of a rule links into it, a heading as `#anchor`.
pub const REFERENCE: [(&str, &str); 22] = [
    (
        "00-architecture.md",
        include_str!("../../../data/reference/00-architecture.md"),
    ),
    (
        "01-windows-pe.md",
        include_str!("../../../data/reference/01-windows-pe.md"),
    ),
    (
        "02-specialize-xml.md",
        include_str!("../../../data/reference/02-specialize-xml.md"),
    ),
    (
        "03-oobe-accounts-languages.md",
        include_str!("../../../data/reference/03-oobe-accounts-languages.md"),
    ),
    (
        "04-users-and-media.md",
        include_str!("../../../data/reference/04-users-and-media.md"),
    ),
    (
        "05-printing.md",
        include_str!("../../../data/reference/05-printing.md"),
    ),
    (
        "06-windows-update.md",
        include_str!("../../../data/reference/06-windows-update.md"),
    ),
    (
        "07-defender.md",
        include_str!("../../../data/reference/07-defender.md"),
    ),
    (
        "08-accounts-uac-lsa-remote.md",
        include_str!("../../../data/reference/08-accounts-uac-lsa-remote.md"),
    ),
    (
        "09-network-smb-firewall.md",
        include_str!("../../../data/reference/09-network-smb-firewall.md"),
    ),
    (
        "10-removable-scripts-browser.md",
        include_str!("../../../data/reference/10-removable-scripts-browser.md"),
    ),
    (
        "11-logging-audit.md",
        include_str!("../../../data/reference/11-logging-audit.md"),
    ),
    (
        "12-telemetry-consumer-ai.md",
        include_str!("../../../data/reference/12-telemetry-consumer-ai.md"),
    ),
    (
        "13-services-apps.md",
        include_str!("../../../data/reference/13-services-apps.md"),
    ),
    (
        "14-default-user-profile.md",
        include_str!("../../../data/reference/14-default-user-profile.md"),
    ),
    (
        "15-per-user-script.md",
        include_str!("../../../data/reference/15-per-user-script.md"),
    ),
    (
        "16-post-oobe.md",
        include_str!("../../../data/reference/16-post-oobe.md"),
    ),
    (
        "17-cross-links.md",
        include_str!("../../../data/reference/17-cross-links.md"),
    ),
    (
        "18-browsers.md",
        include_str!("../../../data/reference/18-browsers.md"),
    ),
    (
        "19-more-privacy.md",
        include_str!("../../../data/reference/19-more-privacy.md"),
    ),
    (
        "20-explorer-namespaces.md",
        include_str!("../../../data/reference/20-explorer-namespaces.md"),
    ),
    (
        "README.md",
        include_str!("../../../data/reference/README.md"),
    ),
];

pub const GUIDE_ID: &str = "guide";
pub const LICENSE_ID: &str = "license";
const LICENSE_TITLE: &str = "LICENSE";

/// The guide in the language of the interface.
pub fn guide() -> MdDoc {
    MdDoc::page(GUIDE_ID, GUIDE_TITLE, guide_text(&i18n::current()))
}

/// The license of the program, built into the executable, so that it travels with the exe alone.
pub fn license() -> MdDoc {
    MdDoc::page(LICENSE_ID, LICENSE_TITLE, license_text())
}

/// The text of `LICENSE` as a Markdown page: its first line (the name of the license) a heading.
pub fn license_text() -> String {
    let text = include_str!("../../../LICENSE").replace("\r\n", "\n");
    match text.split_once('\n') {
        Some((first, rest)) if !first.starts_with('#') => format!("# {first}\n{rest}"),
        _ => text,
    }
}

/// The guide in a language: a file `lang/<code>/guide.md` next to the executable, the embedded
/// translation, or the English page.
pub fn guide_text(code: &str) -> String {
    let file = lang_dir().join(code).join("guide.md");
    if code != "en"
        && let Ok(text) = winweaver_core::jsonfile::read_text_bom(&file, MARKDOWN_MAX_BYTES)
    {
        return text;
    }
    GUIDES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(GUIDE, |(_, text)| text)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_license_is_built_in_as_a_page() {
        let text = license_text();
        assert!(text.starts_with("# MIT License\n"), "{text}");
        assert!(text.contains("Permission is hereby granted"));
        assert_eq!(license().page, Some(LICENSE_ID));
        assert_eq!(guide().page, Some(GUIDE_ID));
        let browsers = REFERENCE
            .iter()
            .find(|(n, _)| *n == "18-browsers.md")
            .unwrap()
            .1;
        assert!(browsers.contains("## Microsoft Edge"));
    }
}
