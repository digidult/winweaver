//! Tests of diff files on the presets.

use std::path::Path;

use winweaver_core::diff::{Diff, Side};
use winweaver_core::profile::Profile;

fn preset(name: &str) -> Profile {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/presets/{name}.wwprofile"));
    Profile::load(&path).unwrap()
}

fn sides() -> (Side, Side) {
    (
        Side {
            name: "Office".into(),
            ..Side::default()
        },
        Side {
            name: "Strict".into(),
            ..Side::default()
        },
    )
}

#[test]
fn a_profile_differs_from_itself_in_nothing() {
    let office = preset("office");
    let (a, b) = sides();
    let diff = Diff::between(&office, &office, a, b);
    assert!(diff.is_empty(), "{diff:?}");
}

#[test]
fn office_and_strict_differ_and_the_diff_merges() {
    let office = preset("office");
    let strict = preset("strict");
    let (a, b) = sides();
    let diff = Diff::between(&office, &strict, a, b);
    assert!(!diff.rules.is_empty(), "Strict turns rules on that Office leaves off");
    for d in diff.rules.values() {
        assert_ne!(d.base, d.other);
    }
    let mut merged = office.clone();
    diff.apply_to(&mut merged);
    assert_eq!(merged.rules, strict.rules);
    assert_eq!(merged.install, strict.install);
    assert_eq!(merged.languages, strict.languages);
    assert_eq!(merged.accounts, strict.accounts);
    let (a, b) = sides();
    assert!(Diff::between(&merged, &strict, a, b).is_empty());
}

#[test]
fn forms_and_accounts_are_compared_field_by_field() {
    let office = preset("office");
    let mut other = office.clone();
    other.install.workgroup = "LAB".into();
    other.install.disk_mode = "partition".into();
    other.languages.input.push("en-GB".into());
    other.accounts[0].display_name = "Administrator".into();
    let (a, b) = sides();
    let diff = Diff::between(&office, &other, a, b);
    assert_eq!(diff.install.len(), 2, "{:?}", diff.install.keys().collect::<Vec<_>>());
    assert_eq!(diff.install["workgroup"].other, serde_json::json!("LAB"));
    assert_eq!(diff.languages.len(), 1);
    assert!(diff.accounts.is_some());
    assert_eq!(diff.count(), 4);
    let mut merged = office.clone();
    diff.apply_to(&mut merged);
    assert_eq!(merged, other);
}

#[test]
fn diff_round_trip_and_wrong_files() {
    let office = preset("office");
    let home = preset("home");
    let (a, b) = sides();
    let diff = Diff::between(&office, &home, a, b);
    let dir = std::env::temp_dir().join(format!("winweaver-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("office-home.wwdiff");
    diff.save(&path).unwrap();
    assert_eq!(Diff::load(&path).unwrap(), diff);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(Diff::parse(r#"{"format": "winweaver-profile", "version": 1, "catalog": "0", "base": {}, "other": {}}"#, "x").is_err());
    assert!(Diff::parse(r#"{"format": "winweaver-diff", "version": 9, "catalog": "0", "base": {}, "other": {}}"#, "x").is_err());
}
