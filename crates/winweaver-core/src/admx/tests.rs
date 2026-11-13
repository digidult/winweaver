use super::*;
use crate::catalog::{Action, Catalog, ListNames, Param, Phase, ROOT_POLICIES, RegKind, values};
use crate::i18n::CatalogTexts;
use serde_json::Value;

use super::rules::safe_key;
use super::templates::read_pair;
use super::xml::parse_xml;

const ADMX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions" revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target prefix="sample" namespace="Test.Policies.Sample" />
    <using prefix="windows" namespace="Microsoft.Policies.Windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="Sample" displayName="$(string.Sample)">
      <parentCategory ref="windows:WindowsComponents" />
    </category>
    <category name="Inner" displayName="$(string.Inner)">
      <parentCategory ref="Sample" />
    </category>
  </categories>
  <policies>
    <policy name="TurnOff" class="Machine" displayName="$(string.TurnOff)" explainText="$(string.TurnOff_Help)" key="Software\Policies\Test" valueName="Off">
      <parentCategory ref="Inner" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
    <policy name="Level" class="User" displayName="$(string.Level)" explainText="$(string.Level_Help)" key="Software\Policies\Test" presentation="$(presentation.Level)">
      <parentCategory ref="Sample" />
      <elements>
        <enum id="Mode" valueName="Mode">
          <item displayName="$(string.Low)"><value><decimal value="1" /></value></item>
          <item displayName="$(string.High)"><value><decimal value="2" /></value></item>
        </enum>
        <decimal id="Size" valueName="Size" minValue="1" maxValue="50" />
        <text id="Name" valueName="Name" maxLength="40" />
      </elements>
    </policy>
    <policy name="Listed" class="Machine" displayName="$(string.Listed)" key="Software\Policies\Test">
      <elements><list id="L" key="Software\Policies\Test\L" /></elements>
    </policy>
    <policy name="Lines" class="Machine" displayName="$(string.Listed)" key="Software\Policies\Test">
      <elements>
        <multiText id="M" valueName="Lines" />
        <list id="N" key="Software\Policies\Test\N" valuePrefix="" />
      </elements>
    </policy>
    <policy name="Unsafe" class="Machine" displayName="$(string.Listed)" key="Software\Policies\..\Test" valueName="X" />
  </policies>
</policyDefinitions>"#;

const ADML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <resources>
    <stringTable>
      <string id="Sample">Sample &amp; tests</string>
      <string id="Inner">Inner</string>
      <string id="TurnOff">Turn the sample off</string>
      <string id="TurnOff_Help">Explains &#x41; lot.</string>
      <string id="Level">Level</string>
      <string id="Level_Help">Sets the level.</string>
      <string id="Low">Low</string>
      <string id="High">High</string>
      <string id="Listed">Listed</string>
    </stringTable>
    <presentationTable>
      <presentation id="Level">
        <dropdownList refId="Mode" defaultItem="1">Mode of work</dropdownList>
        <decimalTextBox refId="Size" defaultValue="10">Size</decimalTextBox>
        <textBox refId="Name"><label>Name</label></textBox>
      </presentation>
    </presentationTable>
  </resources>
</policyDefinitionResources>"#;

const ADML_UK: &str = r#"<policyDefinitionResources><resources><stringTable>
      <string id="TurnOff">Вимкнути зразок</string><string id="Inner">Усередині</string>
    </stringTable></resources></policyDefinitionResources>"#;

fn sample() -> CatalogFile {
    to_catalog_file(
        &sample_templates(),
        "test",
        "Sample templates",
        "26300",
        "now",
    )
}

fn sample_templates() -> Templates {
    let mut t = Templates::default();
    read_pair(
        ADMX,
        &[
            ("en-US".into(), ADML.into()),
            ("uk-UA".into(), ADML_UK.into()),
        ],
        "sample.admx",
        &mut t,
    )
    .unwrap();
    t
}

/// Two templates of one prefix in two namespaces (as Zoom has its user and machine ones) and a
/// copy of one of them (as Office has its x64 and x86 ones), in UTF-16 as most vendors write
/// them: two policies with ids of their namespaces, the copy left out, the categories told
/// apart.
#[test]
fn vendor_templates_in_utf16_with_a_copy_and_a_shared_prefix() {
    use std::io::Write;
    let admx = |ns: &str, class: &str| {
        format!(
            r#"<?xml version="1.0" encoding="utf-16"?>
<policyDefinitions revision="1.0" schemaVersion="1.0">
  <policyNamespaces><target prefix="fullarmor" namespace="{ns}" /></policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories><category name="Zoom" displayName="$(string.Zoom)" /></categories>
  <policies>
    <policy name="Mute" class="{class}" displayName="$(string.Mute)" key="Software\Policies\Zoom" valueName="Mute">
      <parentCategory ref="Zoom" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
  </policies>
</policyDefinitions>"#
        )
    };
    let adml = r#"<?xml version="1.0" encoding="utf-16"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0"><resources><stringTable>
<string id="Zoom">Zoom Meetings</string><string id="Mute">Mute the microphone</string>
</stringTable></resources></policyDefinitionResources>"#;
    let utf16 = |text: &str| {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes
    };
    let dir = std::env::temp_dir().join(format!("winweaver-vendors-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vendors.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let user = admx("Zoom.Meetings.HKCU", "User");
    let machine = admx("Zoom.Meetings.HKLM", "Machine");
    for (name, text) in [
        ("zoom/user.admx", user.as_str()),
        ("zoom/en-US/user.adml", adml),
        ("zoom/machine.admx", machine.as_str()),
        ("zoom/en-US/machine.adml", adml),
        ("copy/user.admx", user.as_str()),
        ("copy/en-US/user.adml", adml),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&utf16(text)).unwrap();
    }
    zip.finish().unwrap();
    let Archive::Templates(t) = read_archive(&path).unwrap() else {
        panic!("templates");
    };
    let f = to_catalog_file(&t, "vendors", "Vendors", "", "now");
    let ids: Vec<&str> = f
        .rules
        .iter()
        .map(|r| r.id.as_str())
        .filter(|id| !id.ends_with(".off"))
        .collect();
    assert_eq!(
        ids,
        vec![
            "admx.zoom-meetings-hkcu.mute",
            "admx.zoom-meetings-hklm.mute"
        ],
        "{:?}",
        f.skipped
    );
    assert!(
        f.skipped
            .iter()
            .any(|(_, why)| why.starts_with("a second copy"))
    );
    let titles: Vec<&str> = f.groups.iter().skip(1).map(|g| g.title.as_str()).collect();
    assert_eq!(titles, vec!["Zoom Meetings (HKCU)", "Zoom Meetings (HKLM)"]);
    f.check().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The same policies in two imports show in both trees (the second holds aliases), and the
/// comparison of two catalog files names what differs.
#[test]
fn a_policy_of_two_imports_shows_in_both_trees_and_they_compare() {
    let first = sample();
    let mut second = to_catalog_file(&sample_templates(), "again", "Again", "", "now");
    let base = Catalog::embedded().unwrap();
    let (merged, notes) = merge(&base, &[first.clone(), second.clone()]).unwrap();
    assert_eq!(merged.rules().len(), base.rules().len() + first.rules.len());
    assert_eq!(notes.len(), 1);
    assert_eq!(merged.rules_under("admx.test").len(), first.rules.len());
    assert_eq!(
        merged.rules_under("admx.again").len(),
        first.rules.len(),
        "the second tree shows the same policies"
    );
    let policies = first
        .rules
        .iter()
        .filter(|r| !r.id.ends_with(".off"))
        .count();
    let same = compare(&first, &second);
    assert_eq!(same.same, policies);
    assert!(same.only_first.is_empty() && same.only_second.is_empty() && same.changed.is_empty());
    second
        .rules
        .retain(|r| r.id != "admx.sample.level" && r.id != "admx.sample.level.off");
    for r in &mut second.rules {
        if r.id == "admx.sample.turnoff" {
            r.title = "Another title".to_owned();
        }
        if r.id == "admx.sample.turnoff.off" {
            r.actions.clear();
        }
    }
    let d = compare(&first, &second);
    let only: Vec<&str> = d.only_first.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(only, vec!["admx.sample.level"]);
    assert!(d.only_second.is_empty());
    assert_eq!(d.changed.len(), 1);
    assert_eq!(d.changed[0].id, "admx.sample.turnoff");
    assert_eq!(d.changed[0].what, vec!["title", "values when off"]);
    assert_eq!(d.same, policies - 2);
}

/// A catalog file changes policy by policy and stays valid: a put that would break it changes
/// nothing, a removal takes the Disabled rule and the conflicts with it.
#[test]
fn policies_and_groups_are_copied_into_another_catalog() {
    let source = sample();
    let mut mine = to_catalog_file(&Templates::default(), "custom-1", "Mine", "", "now");
    let root = mine.groups[0].id.clone();
    let n = mine
        .copy_from(&source, CopyWhat::Policy("admx.sample.turnoff.off"), &root)
        .unwrap();
    assert_eq!(n, 1, "the policy with its Disabled rule");
    let ids: Vec<&str> = mine.rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"admx.sample.turnoff") && ids.contains(&"admx.sample.turnoff.off"));
    let uk = mine
        .texts
        .get("uk")
        .expect("the texts in Ukrainian come along");
    assert!(uk.rules.contains_key("admx.sample.turnoff"));
    // The group of the sample with the group inside it, made again under the root.
    let sample_group = source
        .groups
        .iter()
        .find(|g| g.title.starts_with("Sample"))
        .unwrap()
        .id
        .clone();
    let n = mine
        .copy_from(&source, CopyWhat::Group(&sample_group), &root)
        .unwrap();
    assert!(n >= 2, "every policy of the group and of the group in it");
    assert!(mine.groups.iter().any(|g| g.title == "Inner"));
    let twice = mine.rules.len();
    mine.copy_from(&source, CopyWhat::Group(&sample_group), &root)
        .unwrap();
    assert_eq!(mine.rules.len(), twice, "a copy again replaces the copies");
    mine.check().unwrap();
    assert!(
        mine.copy_from(&source, CopyWhat::Policy("admx.none"), &root)
            .is_err()
    );
    assert!(
        mine.copy_from(&source, CopyWhat::Group(&sample_group), "admx.none")
            .is_err()
    );
    // The maxLength of a text becomes the limit of its parameter.
    let level = source
        .rules
        .iter()
        .find(|r| r.id == "admx.sample.level")
        .unwrap();
    assert!(level.params.values().any(|p| matches!(
        p,
        Param::Text {
            max_length: Some(40),
            ..
        }
    )));
}

#[test]
fn a_catalog_file_is_edited_policy_by_policy_and_stays_valid() {
    let mut f = sample();
    let group = f.groups[1].id.clone();
    let mut rule = f.new_policy(&group, "Custom thing").unwrap();
    assert_eq!(rule.id, "admx.test.custom-thing");
    rule.actions.push(Action::Reg {
        path: catalog_path("HKLM\\Software\\Policies\\Test").unwrap(),
        name: "Custom".to_owned(),
        kind: RegKind::DWord,
        value: Value::from(1),
        default: None,
        why: String::new(),
    });
    f.put_rule(rule.clone()).unwrap();
    assert!(f.rules.iter().any(|r| r.id == rule.id));
    assert!(
        f.new_policy(&group, "Custom thing").is_err(),
        "the id is taken"
    );
    let mut bad = rule.clone();
    if let Action::Reg { path, .. } = &mut bad.actions[0] {
        *path = "HKCR:\\X".to_owned();
    }
    assert!(
        f.put_rule(bad).is_err(),
        "a key of a form a catalog never writes"
    );
    let kept = f.rules.iter().find(|r| r.id == rule.id).unwrap();
    assert!(
        matches!(&kept.actions[0], Action::Reg { path, .. } if path.starts_with("HKLM:")),
        "a put that fails changes nothing"
    );
    let top = f.groups[0].id.clone();
    assert!(f.remove_group(&top).is_err(), "the top group stays");
    let sub = f.new_group(&top, "Removed later").unwrap();
    let inner = f.new_group(&sub, "Inner").unwrap();
    let doomed = f.new_policy(&inner, "Doomed").unwrap();
    let doomed_id = doomed.id.clone();
    f.put_rule(doomed).unwrap();
    assert_eq!(f.remove_group(&sub).unwrap(), 1);
    assert!(f.groups.iter().all(|g| g.id != sub && g.id != inner));
    assert!(f.rules.iter().all(|r| r.id != doomed_id));
    f.remove_rule("admx.sample.turnoff.off").unwrap();
    assert!(
        !f.rules
            .iter()
            .any(|r| r.id.starts_with("admx.sample.turnoff"))
    );
    assert!(f.rules.iter().all(|r| {
        !r.conflicts
            .iter()
            .any(|c| c.starts_with("admx.sample.turnoff"))
    }));
    let g = f.new_group(&group, "Mine").unwrap();
    assert!(f.groups.iter().any(|x| x.id == g));
    f.check().unwrap();
    assert_eq!(
        catalog_path("HKCU\\Software\\X").as_deref(),
        Some("DU:\\Software\\X")
    );
    assert_eq!(catalog_path("HKU\\X"), None);
}

#[test]
fn templates_become_rules_and_groups() {
    let f = sample();
    assert_eq!(f.groups.len(), 3, "{:?}", f.groups);
    assert_eq!(f.groups[0].parent.as_deref(), Some(ROOT_POLICIES));
    assert_eq!(f.groups[1].title, "Sample & tests");
    let ids: Vec<&str> = f.rules.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "admx.sample.turnoff",
            "admx.sample.turnoff.off",
            "admx.sample.level",
            "admx.sample.level.off",
            "admx.sample.listed",
            "admx.sample.listed.off",
            "admx.sample.lines",
            "admx.sample.lines.off",
        ]
    );
    let off = &f.rules[0];
    assert_eq!(off.summary, "Explains A lot.");
    assert!(
        matches!(&off.actions[0], Action::Reg { path, name, value, .. } if path == "HKLM:\\Software\\Policies\\Test" && name == "Off" && value == &Value::from(1))
    );
    assert_eq!(off.conflicts, vec!["admx.sample.turnoff.off".to_owned()]);
    let level = &f.rules[2];
    assert_eq!(level.phase, Phase::DefaultUser);
    assert!(
        matches!(&level.actions[0], Action::Reg { path, value, .. } if path.starts_with("DU:\\") && value == &Value::from("{e1}"))
    );
    assert!(
        matches!(&level.params["e1"], Param::Enum { default, values, .. } if default == &Value::from(2) && values.len() == 2)
    );
    assert!(matches!(
        &level.params["e2"],
        Param::Int {
            default: 10,
            min: 1,
            max: 50,
            ..
        }
    ));
    let listed = f
        .rules
        .iter()
        .find(|r| r.id == "admx.sample.listed")
        .unwrap();
    assert!(matches!(&listed.actions[0],
        Action::RegList { param, names: ListNames::Items, additive: false, kind: RegKind::String, .. } if param == "e1"));
    assert!(matches!(&listed.params["e1"], Param::List { .. }));
    let off = f
        .rules
        .iter()
        .find(|r| r.id == "admx.sample.listed.off")
        .unwrap();
    assert!(
        matches!(&off.actions[0], Action::RegList { param, .. } if param.is_empty()),
        "Disabled clears the list"
    );
    assert!(
        f.skipped
            .iter()
            .any(|(p, why)| p == "sample:Unsafe" && why.contains("unsafe"))
    );
    assert_eq!(
        f.texts["uk"].rules["admx.sample.turnoff"]
            .title
            .as_deref()
            .unwrap(),
        "Вимкнути зразок"
    );
    f.check().unwrap();
}

#[test]
fn an_archive_gives_its_templates_or_its_catalog_file() {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("winweaver-zip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |path: &Path, entries: &[(&str, &str)]| {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, text) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    };
    let templates = dir.join("templates.zip");
    write(
        &templates,
        &[
            ("windows/admx/sample.admx", ADMX),
            ("windows/admx/en-US/sample.adml", ADML),
            ("Windows/ADMX/uk-UA/Sample.adml", ADML_UK),
            ("readme.txt", "not a template"),
        ],
    );
    let Archive::Templates(t) = read_archive(&templates).unwrap() else {
        panic!("templates");
    };
    let zipped = to_catalog_file(&t, "test", "Sample templates", "26300", "now");
    let plain = sample();
    let ids = |f: &CatalogFile| f.rules.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&zipped), ids(&plain));
    assert_eq!(
        zipped.texts.keys().collect::<Vec<_>>(),
        plain.texts.keys().collect::<Vec<_>>(),
        "the ADML of the cultures are found whatever their case"
    );
    let catalog = dir.join("catalog.zip");
    let json = serde_json::to_string(&plain).unwrap();
    write(&catalog, &[("test.wwcatalog", &json)]);
    let Archive::Catalog(c) = read_archive(&catalog).unwrap() else {
        panic!("a catalog file");
    };
    assert_eq!(c.id, "test");
    let neither = dir.join("neither.zip");
    write(&neither, &[("readme.txt", "x")]);
    assert!(read_archive(&neither).is_err());
    let two = dir.join("two.zip");
    write(&two, &[("a.wwcatalog", &json), ("b.wwcatalog", &json)]);
    assert!(read_archive(&two).is_err(), "one catalog file at a time");
    std::fs::write(dir.join("bad.zip"), "not a zip").unwrap();
    assert!(read_archive(&dir.join("bad.zip")).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_header_and_the_rule_ids_are_read_without_the_whole_file() {
    let f = sample();
    let dir = std::env::temp_dir().join(format!("winweaver-header-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.wwcatalog");
    f.save(&path).unwrap();
    let head = CatalogHeader::read(&path).unwrap();
    assert_eq!(
        (head.id.as_str(), head.name.as_str()),
        ("test", "Sample templates")
    );
    assert_eq!(head.windows, "26300");
    let ids = rule_ids(&path).unwrap();
    assert_eq!(ids.len(), f.rules.len());
    assert!(ids.contains("admx.sample.turnoff.off"));
    std::fs::write(&path, "{\"format\":\"x\",\"rules\":[]}").unwrap();
    assert!(CatalogHeader::read(&path).is_none());
    assert!(rule_ids(&path).is_err(), "not a catalog file");
    std::fs::write(
        &path,
        "{\"format\":\"winweaver-catalog\",\"rules\":[{\"id\":",
    )
    .unwrap();
    assert!(rule_ids(&path).is_err(), "a cut file is an error");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_catalog_file_merges_and_crafted_files_are_refused() {
    let f = sample();
    let base = Catalog::embedded().unwrap();
    let (merged, notes) = merge(&base, &[f.clone(), f.clone()]).unwrap();
    assert_eq!(merged.rules().len(), base.rules().len() + 8);
    assert_eq!(notes.len(), 1, "the second copy is left out");
    assert_eq!(merged.root_of("admx.test"), Some(ROOT_POLICIES));
    let mut texts = CatalogTexts::embedded().unwrap();
    merge_texts(&mut texts, std::slice::from_ref(&f));
    let uk = texts.iter().find(|t| t.code == "uk").unwrap();
    assert!(uk.rules.contains_key("admx.sample.turnoff"));
    let off = uk.rules["admx.sample.turnoff.off"]
        .title
        .as_deref()
        .unwrap_or_default();
    assert!(off.ends_with(&format!(": {}", uk.disabled)), "{off}");
    // A script is a rule of the root Other or of the installation, as in the built-in catalog.
    let mut bad = f.clone();
    bad.rules[0].actions.push(Action::Ps {
        script: crate::catalog::Lines::One("x".into()),
    });
    assert!(
        bad.check().unwrap_err().contains("mechanism"),
        "a script under the root of the policies"
    );
    let mut good = bad.clone();
    let id = good.rules[0].id.clone();
    let other = good.groups[0].id.clone() + ".other";
    good.groups.push(crate::catalog::Group {
        id: other.clone(),
        title: "Other".into(),
        order: 2000,
        summary: String::new(),
        parent: Some(crate::catalog::ROOT_OTHER.into()),
    });
    good.rules[0].group = other;
    good.check().unwrap();
    let code: Vec<&str> = good.code_rules().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(code, [id.as_str()]);
    let mut bad = f.clone();
    if let Action::Reg { path, .. } = &mut bad.rules[0].actions[0] {
        *path = "HKCR:\\X".into();
    }
    assert!(
        bad.check().is_err(),
        "only the machine, the default user, the current user and the users"
    );
    let mut bad = f.clone();
    bad.rules[0].conflicts.push("no.such.rule".into());
    assert!(bad.check().is_err(), "a conflict with a rule nowhere");
    let mut fine = f.clone();
    let builtin = base.rules()[0].id.clone();
    fine.rules[0].conflicts.push(builtin.clone());
    fine.check().unwrap();
    let (merged, _) = merge(&base, std::slice::from_ref(&fine)).unwrap();
    let kept = merged.rule(&fine.rules[0].id).unwrap();
    assert!(
        kept.conflicts.contains(&builtin),
        "a conflict with a built-in rule stays"
    );
    let mut bad = f;
    if let Action::Reg { value, .. } = &mut bad.rules[0].actions[0] {
        *value = Value::from("x]]>y");
    }
    assert!(bad.check().is_err(), "nothing that closes a CDATA section");
}

/// A catalog file of version 1 held a group as its title and a rule as its title and summary; it
/// is still read, and written again as version 2 with the texts of the translation files.
#[test]
fn a_catalog_file_of_version_1_is_read() {
    let f = sample();
    let mut old: Value = serde_json::from_str(&f.to_json()).unwrap();
    old["version"] = Value::from(1);
    old["texts"] = serde_json::json!({
        "uk": {
            "groups": { "admx.test": "Зразки" },
            "rules": { "admx.sample.turnoff": ["Вимкнути", "Опис"] }
        }
    });
    let read = CatalogFile::parse(&old.to_string(), "old.wwcatalog").unwrap();
    assert_eq!(read.version, CATALOG_VERSION);
    let uk = &read.texts["uk"];
    assert_eq!(uk.groups["admx.test"].title.as_deref(), Some("Зразки"));
    let rule = &uk.rules["admx.sample.turnoff"];
    assert_eq!(
        (rule.title.as_deref(), rule.summary.as_deref()),
        (Some("Вимкнути"), Some("Опис"))
    );
    let again = CatalogFile::parse(&read.to_json(), "new.wwcatalog").unwrap();
    assert_eq!(again.texts, read.texts, "the new form reads back the same");
}

/// A policy, a group and the file get new ids across the file: the Disabled rule, the
/// conflicts, the groups of the rules and the translations follow; a taken id or one outside the
/// imports changes nothing.
#[test]
fn ids_are_renamed_across_the_catalog_file() {
    let mut f = sample();
    let pairs = f
        .rename_rule("admx.sample.turnoff", "admx.sample.switch")
        .unwrap();
    assert_eq!(
        pairs,
        [
            (
                "admx.sample.turnoff".to_owned(),
                "admx.sample.switch".to_owned()
            ),
            (
                "admx.sample.turnoff.off".to_owned(),
                "admx.sample.switch.off".to_owned()
            ),
        ]
    );
    let off = f
        .rules
        .iter()
        .find(|r| r.id == "admx.sample.switch.off")
        .unwrap();
    assert_eq!(off.conflicts, ["admx.sample.switch"]);
    assert!(f.texts["uk"].rules.contains_key("admx.sample.switch"));
    assert!(!f.texts["uk"].rules.contains_key("admx.sample.turnoff"));
    let other = f
        .rules
        .iter()
        .find(|r| !r.id.starts_with("admx.sample.switch"))
        .unwrap()
        .id
        .clone();
    let before = f.clone();
    assert!(
        f.rename_rule("admx.sample.switch", &other).is_err(),
        "a taken id"
    );
    assert!(
        f.rename_rule("admx.sample.switch", "sample.switch")
            .is_err(),
        "outside the imports"
    );
    assert_eq!(f, before, "a failed rename changes nothing");
    // A group: its rules and the groups in it follow.
    let group = f.groups[1].id.clone();
    let inside: Vec<String> = f
        .rules
        .iter()
        .filter(|r| r.group == group)
        .map(|r| r.id.clone())
        .collect();
    f.rename_group(&group, "admx.test.renamed").unwrap();
    for id in &inside {
        let r = f.rules.iter().find(|r| r.id == *id).unwrap();
        assert_eq!(r.group, "admx.test.renamed");
    }
    // The file: the ids named after its old id take the new one, the others stay.
    let pairs = f.rename_id("mine").unwrap();
    assert_eq!(f.id, "mine");
    assert!(f.groups.iter().any(|g| g.id == "admx.mine"));
    assert!(f.groups.iter().all(|g| !g.id.starts_with("admx.test")));
    assert!(
        f.rules.iter().any(|r| r.id == "admx.sample.switch"),
        "not named after the file"
    );
    assert!(pairs.iter().all(|(o, _)| o.starts_with("admx.test")));
    f.check().unwrap();
    assert!(f.rename_id("not.an.id").is_err());
}

#[test]
fn a_policy_a_built_in_rule_writes_is_linked_to_it() {
    let f = sample();
    let base = Catalog::embedded().unwrap();
    let mut builtin = base.rules()[0].clone();
    builtin.id = "test.turn-off".to_owned();
    builtin.requires.clear();
    builtin.conflicts.clear();
    builtin.params.clear();
    builtin.actions = vec![Action::Reg {
        path: "HKLM:\\SOFTWARE\\Policies\\Test".to_owned(),
        name: "off".to_owned(),
        kind: RegKind::DWord,
        value: Value::from(1),
        default: None,
        why: String::new(),
    }];
    let mut rules = base.rules().to_vec();
    rules.push(builtin.clone());
    let with_rule = Catalog::from_parts(base.version(), base.groups().to_vec(), rules).unwrap();
    let (catalog, _) = merge(&with_rule, &[f]).unwrap();
    let mut profile = crate::profile::Profile::from_defaults("test", &catalog);
    profile.set_on("test.turn-off", true);
    let r = crate::resolver::resolve(&catalog, &profile);
    assert_eq!(r.linked_by("admx.sample.turnoff"), Some("test.turn-off"));
    assert!(r.shown_on("admx.sample.turnoff") && !r.in_effect("admx.sample.turnoff"));
    assert_eq!(r.linked_by("admx.sample.turnoff.off"), None, "it writes 0");
    assert_eq!(
        r.linked_by("admx.sample.level"),
        None,
        "a policy with parameters"
    );
    profile.set_on("test.turn-off", false);
    let r = crate::resolver::resolve(&catalog, &profile);
    assert_eq!(
        r.linked_by("admx.sample.turnoff"),
        None,
        "the built-in rule is off"
    );
}

#[test]
fn lists_of_templates_are_built_applied_and_checked() {
    let base = Catalog::embedded().unwrap();
    let (catalog, _) = merge(&base, &[sample()]).unwrap();
    let mut profile = crate::profile::Profile::from_defaults("test", &catalog);
    let set = |profile: &mut crate::profile::Profile, id: &str, name: &str, items: &[&str]| {
        profile.set_on(id, true);
        let choice = profile.rules.entry(id.to_owned()).or_default();
        choice
            .params
            .insert(name.to_owned(), Value::from(items.to_vec()));
    };
    set(
        &mut profile,
        "admx.sample.listed",
        "e1",
        &["a.example", " ", "b.example"],
    );
    set(&mut profile, "admx.sample.lines", "e1", &["one", "two"]);
    set(&mut profile, "admx.sample.lines", "e2", &["x", "y"]);
    let listed = catalog.rule("admx.sample.listed").unwrap();
    let params = crate::render::params_of(&catalog, &profile, listed);
    let line =
        crate::render::script_lines(listed, &listed.actions[0], &params, listed.phase).unwrap();
    assert_eq!(
        line,
        vec![
            "Set-RegList -Path 'HKLM:\\Software\\Policies\\Test\\L' -Type String -Names @('a.example','b.example') -Values @('a.example','b.example')"
                .to_owned()
        ]
    );
    let lines = catalog.rule("admx.sample.lines").unwrap();
    let params = crate::render::params_of(&catalog, &profile, lines);
    let list = crate::render::script_lines(lines, &lines.actions[0], &params, lines.phase).unwrap();
    assert_eq!(
        list[0],
        "Set-RegList -Path 'HKLM:\\Software\\Policies\\Test\\N' -Type String -Names @('1','2') -Values @('x','y')"
    );
    let multi =
        crate::render::script_lines(lines, &lines.actions[1], &params, lines.phase).unwrap();
    assert!(
        multi[0].contains("-Name 'Lines' -Type MultiString") && multi[0].contains("'one'"),
        "{multi:?}"
    );
    let ids = vec!["admx.sample.lines".to_owned()];
    let plan = crate::apply::plan_apply(&catalog, &profile, &ids, "n");
    assert!(plan.steps.iter().any(|s| matches!(&s.op,
        crate::apply::Op::SetList { names, values, additive: false, .. } if names == &["1", "2"] && values == &["x", "y"])));
    let checks = crate::thispc::plan(&catalog, &profile, &ids);
    let key = "HKLM:\\Software\\Policies\\Test\\N";
    let mut m = crate::thispc::MemoryMachine::default();
    m.set(key, "1", crate::registry::Data::Sz("x".into()));
    m.set(key, "2", crate::registry::Data::Sz("y".into()));
    let status = |m: &crate::thispc::MemoryMachine| {
        let report = crate::thispc::run(m, &checks, false, "now");
        report
            .results
            .iter()
            .find(|r| r.check.ends_with("(list)"))
            .map(|r| r.status)
    };
    assert_eq!(status(&m), Some(crate::thispc::Status::Ok));
    m.set(key, "3", crate::registry::Data::Sz("z".into()));
    assert_eq!(
        status(&m),
        Some(crate::thispc::Status::Differs),
        "a value outside the list"
    );
    let word = format!("caf{}", char::from_u32(0xe9).unwrap());
    set(&mut profile, "admx.sample.lines", "e2", &[word.as_str()]);
    let resolution = crate::resolver::resolve(&catalog, &profile);
    let issues = crate::validate::check(&catalog, &profile, &resolution);
    assert!(
        issues.iter().any(|i| i.text.contains("ASCII")),
        "{issues:?}"
    );
    let items = |list: &[&str]| list.iter().map(|t| (*t).to_owned()).collect::<Vec<_>>();
    assert!(values::list_entries(ListNames::Pairs, "", &items(&["a=1", "b"])).is_err());
    assert!(values::list_entries(ListNames::Items, "", &items(&["a", "A"])).is_err());
    assert_eq!(
        values::list_entries(ListNames::Numbered, "item", &items(&["x"])).unwrap(),
        (vec!["item1".to_owned()], vec!["x".to_owned()])
    );
}

#[test]
fn xml_without_dtd_or_unknown_entities() {
    assert!(parse_xml("<!DOCTYPE x [<!ENTITY a \"b\">]><x>&a;</x>").is_err());
    assert!(parse_xml("<x>&unknown;</x>").is_err());
    assert_eq!(parse_xml("<x>a &lt; b</x>").unwrap().text, "a < b");
    let deep = format!("{}{}", "<a>".repeat(80), "</a>".repeat(80));
    assert!(parse_xml(&deep).is_err());
    assert!(
        safe_key("Software\\Policies\\X") && !safe_key("Software\\..\\X") && !safe_key("A\\\\B")
    );
}

#[cfg(windows)]
mod windows_tests {
    use crate::admx::{merge, read_file, read_folder, to_catalog_file};
    use crate::catalog::Catalog;

    #[test]
    fn the_templates_of_this_windows_import_check_and_merge() {
        let dir = std::path::Path::new(r"C:\Windows\PolicyDefinitions");
        if !dir.is_dir() {
            return;
        }
        let t = read_folder(dir);
        assert!(t.policies() > 1000, "{} policies", t.policies());
        let f = to_catalog_file(&t, "system", "Policy templates of this Windows", "", "now");
        f.check().unwrap();
        assert!(f.rules.len() > 1000);
        let base = Catalog::embedded().unwrap();
        let (merged, _) = merge(&base, &[f]).unwrap();
        assert!(merged.rules().len() > base.rules().len() + 1000);
        let one = read_file(&dir.join("WindowsUpdate.admx"));
        assert!(one.policies() > 10, "{} policies", one.policies());
        let f = to_catalog_file(&one, "file-test", "WindowsUpdate", "", "now");
        f.check().unwrap();
        assert!(!f.rules.is_empty());
    }
}

/// A read of the whole PC covers the policies of the loaded imports too (they were left out
/// until 0.28.1, so a new profile of this PC never had them).
#[test]
fn a_read_of_the_whole_pc_covers_the_loaded_imports() {
    let base = Catalog::embedded().unwrap();
    let f = sample();
    let (merged, _) = merge(&base, std::slice::from_ref(&f)).unwrap();
    let ids = crate::thispc::whole_pc_rules(&merged);
    assert!(f.rules.iter().all(|r| ids.contains(&r.id)));
    assert_eq!(ids.len(), merged.rules().len());
}
