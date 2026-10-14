//! Tests of the embedded catalog data, the translations, the presets and the resolver.

use std::path::{Path, PathBuf};

use winweaver_core::catalog::{self, Catalog, TREE_ROOTS, embedded};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::profile::Profile;
use winweaver_core::resolver::{self, State};

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn embedded_catalog_loads() {
    let catalog = Catalog::embedded().expect("the embedded catalog loads");
    assert_eq!(catalog.rules().len(), 284);
    assert_eq!(catalog.groups().len(), 40);
    assert_eq!(catalog.version(), embedded::CATALOG_VERSION);
    for root in TREE_ROOTS {
        assert!(catalog.group(root).is_some(), "root {root}");
        assert!(catalog.group(root).unwrap().parent.is_none());
    }
}

#[test]
fn embedded_matches_data_folder() {
    let dir = data_dir().join("catalog");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json") && n != "groups.json")
        .collect();
    names.sort();
    let embedded: Vec<&str> = embedded::RULE_FILES.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names, embedded,
        "RULE_FILES in embedded.rs must list the folder"
    );
    let from_dir = Catalog::load_dir(&dir).expect("the folder loads");
    assert_eq!(
        from_dir.rules().len(),
        Catalog::embedded().unwrap().rules().len()
    );
}

#[test]
fn catalog_is_sound() {
    let catalog = Catalog::embedded().unwrap();
    let problems = catalog::check::check(&catalog);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn catalog_files_are_ascii_without_dashes() {
    for (name, text) in embedded::RULE_FILES
        .iter()
        .chain([(embedded::GROUPS_FILE, embedded::GROUPS)].iter())
    {
        assert!(text.is_ascii(), "{name} must be ASCII");
    }
    for (code, text) in embedded::TEXT_FILES {
        assert!(
            !text.contains('\u{2013}') && !text.contains('\u{2014}'),
            "{code}.json holds a dash"
        );
    }
}

#[test]
fn translations_cover_every_rule() {
    let catalog = Catalog::embedded().unwrap();
    let texts = CatalogTexts::embedded().expect("the translations load");
    assert_eq!(texts.len(), 2);
    for t in &texts {
        assert!(!t.language.is_empty(), "{}: _language", t.code);
        for rule in catalog.rules() {
            let entry = t.rules.get(&rule.id);
            assert!(
                entry.is_some_and(|e| e.title.is_some()),
                "{}: {} has no title",
                t.code,
                rule.id
            );
            assert!(
                !t.rule_title(rule).is_empty(),
                "{}: {} has an empty title",
                t.code,
                rule.id
            );
        }
        for id in t.rules.keys() {
            assert!(catalog.rule(id).is_some(), "{}: unknown rule {id}", t.code);
        }
        for id in t.groups.keys() {
            assert!(
                catalog.group(id).is_some(),
                "{}: unknown group {id}",
                t.code
            );
        }
    }
}

#[test]
fn presets_load_and_office_is_the_defaults() {
    let catalog = Catalog::embedded().unwrap();
    let dir = data_dir().join("presets");
    for name in ["office", "strict", "laptop", "home"] {
        let path = dir.join(format!("{name}.wwprofile"));
        let mut profile = Profile::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            profile.catalog,
            catalog.version(),
            "{name}: catalog version"
        );
        let unknown = profile.split_unknown(&catalog);
        assert!(unknown.is_empty(), "{name}: unknown rules {unknown:?}");
        assert_eq!(
            profile.to_json(),
            std::fs::read_to_string(&path).unwrap(),
            "{name}: canonical"
        );
    }
    let office = Profile::load(&dir.join("office.wwprofile")).unwrap();
    for rule in catalog.rules() {
        assert_eq!(
            office.is_on(&rule.id),
            rule.default,
            "Office differs from the default of {}",
            rule.id
        );
    }
}

#[test]
fn profile_round_trip() {
    let catalog = Catalog::embedded().unwrap();
    let mut profile = Profile::from_defaults("Round trip", &catalog);
    profile.set_on("install.bypass-tpm", false);
    let dir = std::env::temp_dir().join(format!("winweaver-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("round-trip.wwprofile");
    profile.save(&path).unwrap();
    let loaded = Profile::load(&path).unwrap();
    assert_eq!(loaded, profile);
    assert!(!loaded.is_on("install.bypass-tpm"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_of_another_format_is_refused() {
    let err = Profile::parse(
        r#"{"format": "something", "version": 1, "catalog": "0", "name": "x"}"#,
        "x.wwprofile",
    )
    .unwrap_err();
    assert!(err.to_string().contains("not a WinWeaver profile"), "{err}");
    let err = Profile::parse(
        r#"{"format": "winweaver-profile", "version": 99, "catalog": "0", "name": "x"}"#,
        "x",
    )
    .unwrap_err();
    assert!(err.to_string().contains("newer"), "{err}");
}

#[test]
fn resolver_blocks_dependents() {
    let catalog = Catalog::embedded().unwrap();
    let dependent = catalog
        .rules()
        .iter()
        .find(|r| !r.requires.is_empty())
        .expect("a rule with requires");
    let required = dependent.requires[0].clone();
    let mut profile = Profile::from_defaults("Resolver", &catalog);
    profile.set_on(&dependent.id, true);
    profile.set_on(&required, true);
    let resolution = resolver::resolve(&catalog, &profile);
    assert!(resolution.in_effect(&dependent.id));
    profile.set_on(&required, false);
    let resolution = resolver::resolve(&catalog, &profile);
    assert_eq!(
        *resolution.state(&dependent.id),
        State::Blocked {
            by: required.clone()
        }
    );
    assert!(!resolution.in_effect(&dependent.id));
}

#[test]
fn every_rule_sits_under_the_root_of_its_mechanism() {
    let catalog = Catalog::embedded().unwrap();
    let wrong: Vec<String> = catalog
        .rules()
        .iter()
        .filter(|r| catalog.root_of(&r.group) != Some(Catalog::mechanism(r).root()))
        .map(|r| format!("{} ({:?}) in {}", r.id, Catalog::mechanism(r), r.group))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
