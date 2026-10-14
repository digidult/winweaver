//! Tests of the profile check on the presets and on broken profiles.

use std::path::Path;

use serde_json::Value;
use winweaver_core::catalog::Catalog;
use winweaver_core::profile::{Account, Profile};
use winweaver_core::resolver;
use winweaver_core::validate::{self, Severity, Target};

fn preset(name: &str) -> Profile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/presets/{name}.wwprofile"));
    Profile::load(&path).unwrap()
}

fn check(catalog: &Catalog, profile: &Profile) -> Vec<validate::Issue> {
    let resolution = resolver::resolve(catalog, profile);
    validate::check(catalog, profile, &resolution)
}

#[test]
fn presets_have_no_errors() {
    let catalog = Catalog::embedded().unwrap();
    for name in ["office", "strict", "laptop", "home"] {
        let issues = check(&catalog, &preset(name));
        let errors: Vec<&validate::Issue> = issues.iter().filter(|i| i.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }
    let home = check(&catalog, &preset("home"));
    assert!(
        home.iter().any(|i| i.severity == Severity::Warning && i.text.contains("baseline")),
        "Home leaves baseline rules off and the check says so"
    );
}

#[test]
fn broken_forms_are_errors() {
    let catalog = Catalog::embedded().unwrap();
    let mut profile = preset("office");
    profile.install.workgroup = "too long a workgroup".into();
    profile.install.disk_mode = "partition".into();
    profile.install.partition_id = "0".into();
    profile.install.computer_name_mode = "fixed".into();
    profile.install.computer_name = "12345".into();
    profile.install.product_key_mode = "key".into();
    profile.install.product_key = "short".into();
    profile.languages.ui_language = "Ukrainian".into();
    profile.languages.input.push("not a layout".into());
    profile.accounts.push(Account {
        name: "Користувач".into(),
        display_name: "x".into(),
        group: "Guests".into(),
        description: String::new(),
        password: String::new(),
    });
    profile.accounts.push(Account {
        name: "admin".into(),
        group: "Users".into(),
        ..Account::default()
    });
    let issues = check(&catalog, &profile);
    let texts: Vec<&str> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.text.as_str())
        .collect();
    for expected in [
        "workgroup",
        "partition number",
        "computer name is",
        "product key",
        "display language",
        "input language",
        "must be ASCII",
        "Administrators or Users",
        "listed twice",
    ] {
        assert!(texts.iter().any(|t| t.contains(expected)), "no error about {expected}: {texts:?}");
    }
    assert!(issues.iter().any(|i| i.target == Target::Install && i.severity == Severity::Warning));
    assert_eq!(issues[0].severity, Severity::Error, "errors come first");
}

#[test]
fn rule_parameters_and_dependencies_are_checked() {
    let catalog = Catalog::embedded().unwrap();
    let mut profile = preset("office");
    let with_enum = catalog
        .rules()
        .iter()
        .find(|r| r.params.values().any(|p| matches!(p, winweaver_core::catalog::Param::Enum { .. })))
        .expect("a rule with an enum parameter");
    let (name, _) = with_enum
        .params
        .iter()
        .find(|(_, p)| matches!(p, winweaver_core::catalog::Param::Enum { .. }))
        .unwrap();
    profile.set_on(&with_enum.id, true);
    for req in &with_enum.requires {
        profile.set_on(req, true);
    }
    profile
        .rules
        .entry(with_enum.id.clone())
        .or_default()
        .params
        .insert(name.clone(), Value::from("no such option"));
    let issues = check(&catalog, &profile);
    assert!(
        issues.iter().any(|i| i.target == Target::Rule(with_enum.id.clone())
            && i.text.contains("not one of its options")),
        "{issues:?}"
    );

    let dependent = catalog.rules().iter().find(|r| !r.requires.is_empty()).unwrap();
    let mut profile = preset("office");
    profile.set_on(&dependent.id, true);
    profile.set_on(&dependent.requires[0], false);
    let issues = check(&catalog, &profile);
    assert!(
        issues.iter().any(|i| i.target == Target::Rule(dependent.id.clone())
            && i.severity == Severity::Warning
            && i.text.contains("blocked")),
        "{issues:?}"
    );
}

#[test]
fn autologon_needs_an_account_without_a_password() {
    let catalog = Catalog::embedded().unwrap();
    if catalog.rule(validate::RULE_AUTOLOGON).is_none() {
        return;
    }
    let mut profile = preset("office");
    profile.set_on(validate::RULE_AUTOLOGON, true);
    profile
        .rules
        .entry(validate::RULE_AUTOLOGON.to_owned())
        .or_default()
        .params
        .insert("account".into(), Value::from("Nobody"));
    let issues = check(&catalog, &profile);
    assert!(
        issues.iter().any(|i| i.severity == Severity::Error && i.text.contains("does not list")),
        "{issues:?}"
    );
}
