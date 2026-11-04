//! The tests of the window of wview: its translations and its guides.

use std::path::Path;

use wwrt::testing;

use crate::app::{GUIDE, GUIDES, LANGUAGES};

#[test]
fn every_text_of_the_window_is_translated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut wanted = testing::texts(&root.join("src"));
    wanted.extend(testing::texts(&root.join("../../../crates/ww/src")));
    // The texts the shell of W# translates without a literal of its own: the hint of New, the
    // hint of the search of a tab, the name of the section of the settings.
    wanted.extend(
        [
            "Open a file (Ctrl+O)",
            "Search the text (Ctrl+F)",
            "Plugins",
        ]
        .map(String::from),
    );
    let missing = testing::missing(&LANGUAGES, &wanted);
    assert!(missing.is_empty(), "{missing:#?}");
}

#[test]
fn every_language_has_its_guide() {
    assert!(GUIDE.starts_with("# wview\n"));
    let codes: Vec<&str> = LANGUAGES.iter().map(|(c, _)| *c).collect();
    let guides: Vec<&str> = GUIDES.iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, guides);
    for (code, text) in GUIDES {
        assert!(text.starts_with("# wview\n"), "{code}");
        assert_eq!(
            text.matches("\n## ").count(),
            GUIDE.matches("\n## ").count(),
            "{code}: the sections of the guide"
        );
    }
}
