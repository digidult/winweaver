//! Tests of the answer file build on the presets and on profiles with every option.

use std::path::Path;

use winweaver_core::catalog::{Catalog, Phase};
use winweaver_core::profile::{Account, Profile};
use winweaver_core::render::{self, ASK_KEY, Build, MAX_COMMAND, TEMP_COMPUTER_NAME};

fn preset(name: &str) -> Profile {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/presets/{name}.wwprofile"));
    Profile::load(&path).unwrap()
}

fn build(profile: &Profile) -> Build {
    let catalog = Catalog::embedded().unwrap();
    render::build(&catalog, profile, "test").unwrap_or_else(|e| panic!("{e}"))
}

fn well_formed(xml: &str) {
    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(e) => panic!("not well formed at {}: {e}", reader.error_position()),
        }
    }
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn office_builds_like_the_reference() {
    let catalog = Catalog::embedded().unwrap();
    let office = preset("office");
    let b = build(&office);
    well_formed(&b.xml);
    assert!(b.xml.is_ascii());
    assert!(b.xml.contains("\r\n") && !b.xml.contains("\n\n\n"));
    assert!(b.xml.contains("<Key>VK7JG-NPHTM-C97JM-9MPGT-3V66T</Key>"));
    assert!(b.xml.contains("<WillShowUI>OnError</WillShowUI>"));
    assert!(b.xml.contains("<InputLocale>0409:00000409;0422:00020422;0419:00000419</InputLocale>"));
    assert!(b.xml.contains("<UILanguage>uk-UA</UILanguage>"));
    assert!(b.xml.contains("<TimeZone>FLE Standard Time</TimeZone>"));
    assert!(b.xml.contains("<HideEULAPage>true</HideEULAPage>"));
    assert!(b.xml.contains("<ProtectYourPC>3</ProtectYourPC>"));
    assert_eq!(count(&b.xml, "<LocalAccount wcm:action=\"add\">"), 4, "two accounts, two architectures");
    assert_eq!(count(&b.xml, "<Name>Admin</Name>"), 2);
    assert!(b.xml.contains("<Description>Extract embedded scripts from this answer file</Description>"));
    assert!(b.xml.contains("BypassNRO"));
    assert!(!b.xml.contains("<ComputerName>"));
    assert!(!b.xml.contains("JoinWorkgroup") && !b.xml.contains("ImageInstall"));
    let names: Vec<&str> = b.scripts.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["Setup-System.ps1", "Setup-User.ps1", "Post-OOBE.ps1"]);
    for (name, _) in &b.scripts {
        assert!(b.xml.contains(&format!("<File path=\"C:\\ProgramData\\Unattend\\Scripts\\{name}\"><![CDATA[")));
    }
    let system = &b.scripts[0].1;
    assert!(system.contains("Set-ServiceStart -Name 'Spooler' -Start 2"));
    assert!(system.contains("Set-Reg -Path ($du + '\\Software\\"));
    assert!(system.contains("# [install.netfx3]"));
    assert!(system.contains("Active Setup\\Installed Components"));
    assert!(system.contains("Unattend-PostOOBE"));
    assert!(system.ends_with("exit 0\n"));
    assert!(!system.contains("{{"));
    let with_blocks = |phase: Phase| -> Vec<&str> {
        catalog
            .rules()
            .iter()
            .filter(|r| b.rule_ids.contains(&r.id) && r.phase == phase)
            .filter(|r| r.actions.iter().any(|a| !a.is_xml()))
            .map(|r| r.id.as_str())
            .collect()
    };
    let mut expected = with_blocks(Phase::Specialize);
    expected.extend(with_blocks(Phase::DefaultUser));
    let markers: Vec<&str> = system
        .lines()
        .filter_map(|l| l.trim().strip_prefix("# [").and_then(|s| s.strip_suffix(']')))
        .collect();
    assert_eq!(markers, expected, "every rule in effect with script actions has one block, in catalog order");
    let user = &b.scripts[1].1;
    assert!(user.contains("$InputLanguages = @('en-US','uk-UA','ru-UA')"));
    assert!(user.contains("$InputFallback = @('en-US','uk-UA','ru')"));
    assert!(user.contains("$TransientLanguages = @('ru-UA')"));
    let post = &b.scripts[2].1;
    assert!(post.contains("$accounts = @('Admin','User')"));
    assert!(!post.contains("Set-AccountText -Name"));
    assert!(b.xml.contains("\"format\": \"winweaver-profile\""));
    assert_eq!(b.rule_ids.len(), 214);
}

#[test]
fn home_asks_for_the_key_and_needs_one_script() {
    let b = build(&preset("home"));
    well_formed(&b.xml);
    assert!(b.xml.contains(&format!("<Key>{ASK_KEY}</Key>")));
    assert!(b.xml.contains("<WillShowUI>Always</WillShowUI>"));
    assert_eq!(b.scripts.len(), 1);
    assert!(b.xml.contains("<UserAccounts>"));
    assert_eq!(b.rule_ids.len(), 70);
}

#[test]
fn the_forms_reach_the_passes() {
    let mut p = preset("office");
    p.install.account_mode = "ask".into();
    p.install.computer_name_mode = "template".into();
    p.install.computer_name = "PC-{random}".into();
    p.install.workgroup = "LAB".into();
    p.install.disk_mode = "partition".into();
    p.install.disk_id = "1".into();
    p.install.partition_id = "4".into();
    p.install.product_key_mode = "key".into();
    p.install.product_key = "vk7jg-nphtm-c97jm-9mpgt-3v66t".into();
    let b = build(&p);
    well_formed(&b.xml);
    assert!(!b.xml.contains("<UserAccounts>"));
    assert!(b.xml.contains(&format!("<ComputerName>{TEMP_COMPUTER_NAME}</ComputerName>")));
    assert!(b.xml.contains("<JoinWorkgroup>LAB</JoinWorkgroup>"));
    assert!(b.xml.contains("<DiskID>1</DiskID>") && b.xml.contains("<PartitionID>4</PartitionID>"));
    assert!(b.xml.contains("<Key>VK7JG-NPHTM-C97JM-9MPGT-3V66T</Key>"));
    let system = &b.scripts[0].1;
    assert!(system.contains("COMPUTER NAME FROM THE TEMPLATE"));
    assert!(system.contains("@('text:PC-','random:6')"));
    let post = b.scripts.iter().find(|(n, _)| n == "Post-OOBE.ps1").unwrap();
    assert!(post.1.contains("$accounts = @()"));
}

#[test]
fn account_texts_outside_ascii_go_to_post_oobe() {
    let mut p = preset("home");
    p.accounts = vec![Account {
        name: "Admin".into(),
        display_name: "Адміністратор".into(),
        group: "Administrators".into(),
        description: "Starter".into(),
        password: String::new(),
    }];
    let b = build(&p);
    well_formed(&b.xml);
    assert!(b.xml.is_ascii());
    assert!(b.xml.contains("<DisplayName>Admin</DisplayName>"));
    assert!(b.xml.contains("<Description>Starter</Description>"));
    let post = b.scripts.iter().find(|(n, _)| n == "Post-OOBE.ps1").expect("Post-OOBE for the texts");
    assert!(post.1.contains("Set-AccountText -Name 'Admin' -FullName (ConvertFrom-Base64Text '"));
    assert!(b.scripts[0].1.contains("Unattend-PostOOBE"));
}

#[test]
fn commands_stay_within_the_limit() {
    for name in ["office", "strict", "laptop", "home"] {
        let b = build(&preset(name));
        for line in b.xml.lines() {
            if let Some(path) = line.trim().strip_prefix("<Path>").and_then(|s| s.strip_suffix("</Path>")) {
                let text = path.replace("&amp;", "&").replace("&gt;", ">").replace("&lt;", "<");
                assert!(text.chars().count() <= MAX_COMMAND, "{name}: {text}");
            }
        }
    }
}

#[test]
fn an_unknown_edition_or_language_is_an_error() {
    let catalog = Catalog::embedded().unwrap();
    let mut p = preset("office");
    p.install.edition = "Home".into();
    assert!(render::build(&catalog, &p, "t").is_err());
    let mut p = preset("office");
    p.languages.input.push("xx-XX".into());
    assert!(render::build(&catalog, &p, "t").is_err());
    let mut p = preset("office");
    p.name = "Назва".into();
    let err = render::build(&catalog, &p, "t").unwrap_err().to_string();
    assert!(err.contains("ASCII"), "{err}");
}
