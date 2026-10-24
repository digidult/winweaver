use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;
use crate::catalog::{Action, Catalog};

#[test]
fn paths_take_the_form_of_regedit() {
    let long = "HKEY_LOCAL_MACHINE\\Software\\Policies";
    for p in [
        "HKLM:\\Software\\Policies",
        "hklm\\Software\\Policies\\",
        "Computer\\HKEY_LOCAL_MACHINE\\Software\\Policies",
        "HKLM:/Software/Policies",
    ] {
        assert_eq!(normalize(p).as_deref(), Some(long), "{p}");
    }
    assert_eq!(
        normalize("DU:\\Software\\X").as_deref(),
        Some("HKEY_CURRENT_USER\\Software\\X")
    );
    assert_eq!(normalize("HKU").as_deref(), Some("HKEY_USERS"));
    assert_eq!(normalize("C:\\Windows"), None);
    assert_eq!(normalize("HKLM\\a\\..\\b"), None);
    assert_eq!(parent(long), Some("HKEY_LOCAL_MACHINE\\Software"));
}

#[test]
fn every_type_goes_to_bytes_and_back_and_parses_from_its_editor() {
    for data in [
        Data::Sz("text \u{439}".to_owned()),
        Data::ExpandSz("%SystemRoot%\\x".to_owned()),
        Data::MultiSz(vec!["a".to_owned(), "b c".to_owned()]),
        Data::Dword(0xdead_beef),
        Data::Qword(u64::MAX),
        Data::Binary(vec![0, 1, 0xab]),
        Data::None(Vec::new()),
    ] {
        let (kind, bytes) = data.to_raw();
        assert_eq!(Data::from_raw(kind, &bytes), data);
        assert_eq!(
            Data::parse(&data.type_name(), &data.edit_text()).unwrap(),
            data
        );
    }
    assert_eq!(Data::parse("REG_DWORD", "0x10").unwrap(), Data::Dword(16));
    assert!(Data::parse("REG_DWORD", "4294967296").is_err());
    assert!(Data::parse("REG_BINARY", "abc").is_err());
    assert_eq!(Data::Dword(1).display(), "0x00000001 (1)");
    let link = Data::from_raw(6, &[1, 2]);
    assert_eq!(link.type_name(), "REG_LINK");
    assert!(!link.editable());
}

#[test]
fn the_registry_in_memory_keeps_keys_and_values() {
    let reg = MemoryRegistry::default();
    reg.create_key("HKCU\\Software\\WinWeaver\\Test").unwrap();
    reg.set("HKCU\\Software\\WinWeaver\\Test", "N", &Data::Dword(1))
        .unwrap();
    let key = reg.read("HKEY_CURRENT_USER\\Software\\WinWeaver").unwrap();
    assert_eq!(key.subkeys, vec!["Test"]);
    let test = reg.read("hkcu\\software\\winweaver\\test").unwrap();
    assert_eq!(test.values[0].data, Data::Dword(1));
    reg.delete_value("HKCU\\Software\\WinWeaver\\Test", "n")
        .unwrap();
    assert!(
        reg.read("HKCU\\Software\\WinWeaver\\Test")
            .unwrap()
            .values
            .is_empty()
    );
    reg.delete_key("HKCU\\Software\\WinWeaver").unwrap();
    assert!(reg.read("HKCU\\Software\\WinWeaver\\Test").is_err());
    assert!(reg.read("HKCU").is_ok(), "a hive is always there");
}

#[test]
fn a_search_finds_keys_values_and_data_under_a_key() {
    let reg = MemoryRegistry::default();
    let root = "HKEY_CURRENT_USER\\Software\\WinWeaverFind";
    let alpha = format!("{root}\\Alpha");
    let beta = format!("{root}\\Beta");
    reg.create_key(&format!("{alpha}\\Inner")).unwrap();
    reg.create_key(&beta).unwrap();
    let lines = Data::MultiSz(vec!["one".to_owned(), "Blue".to_owned()]);
    reg.set(&alpha, "Colour", &Data::Sz("dark blue".to_owned()))
        .unwrap();
    reg.set(&beta, "Lines", &lines).unwrap();
    reg.set(&beta, "Size", &Data::Dword(255)).unwrap();
    let (stop, read) = (AtomicBool::new(false), AtomicUsize::new(0));
    let find = |text: &str, keys: bool, data: bool, whole: bool, limit: usize| {
        let query = Query {
            text: text.to_owned(),
            keys,
            values: true,
            data,
            whole,
        };
        search(&reg, root, &query, limit, 1000, &stop, &read)
    };
    let places = |found: &Found| -> Vec<String> {
        found
            .hits
            .iter()
            .map(|h| format!("{}|{}", h.path, h.value.as_deref().unwrap_or("-")))
            .collect()
    };
    let blue = find("blue", true, true, false, 100);
    assert!(blue.complete);
    assert_eq!(blue.keys, 4);
    assert_eq!(
        places(&blue),
        [format!("{alpha}|Colour"), format!("{beta}|Lines")]
    );
    let whole = find("blue", true, true, true, 100);
    assert_eq!(places(&whole), [format!("{beta}|Lines")]);
    let inner = find("INN", true, false, false, 100);
    assert_eq!(places(&inner), [format!("{alpha}\\Inner|-")]);
    let number = find("0xff", false, true, true, 100);
    assert_eq!(places(&number), [format!("{beta}|Size")]);
    let first = find("e", true, true, false, 1);
    assert_eq!(places(&first), [format!("{root}|-")]);
    assert!(!first.complete);
    assert!(find("", true, true, false, 100).hits.is_empty());
    stop.store(true, Ordering::Relaxed);
    let stopped = find("blue", true, true, false, 100);
    assert!(stopped.hits.is_empty() && !stopped.complete);
}

#[test]
fn the_rules_that_write_a_key_are_found() {
    let catalog = Catalog::embedded().unwrap();
    let (path, name) = catalog
        .rules()
        .iter()
        .find_map(|r| {
            r.actions.iter().find_map(|a| match a {
                Action::Reg { path, name, .. } => Some((path.clone(), name.clone())),
                _ => None,
            })
        })
        .unwrap();
    let found = writers(&catalog, &path, Some(&name));
    assert!(!found.is_empty(), "{path} {name}");
    assert!(writers(&catalog, "HKLM\\Software\\NobodyWritesThis", None).is_empty());
}

#[test]
fn the_rules_that_write_the_same_values_link_both_ways() {
    let catalog = Catalog::embedded().unwrap();
    for rule in catalog.rules() {
        for (other, values) in same_values(&catalog, rule) {
            assert_ne!(other.id, rule.id, "a rule does not name itself");
            assert!(!values.is_empty());
            assert!(
                same_values(&catalog, other)
                    .iter()
                    .any(|(r, _)| r.id == rule.id),
                "{} names {} but not back",
                rule.id,
                other.id
            );
        }
    }
}

#[test]
fn keys_and_values_are_renamed() {
    let reg = MemoryRegistry::default();
    let key = "HKEY_CURRENT_USER\\Software\\WinWeaverRename\\Old";
    reg.create_key(&format!("{key}\\Inner")).unwrap();
    reg.set(key, "Size", &Data::Dword(7)).unwrap();
    reg.set(&format!("{key}\\Inner"), "Text", &Data::Sz("x".to_owned()))
        .unwrap();
    rename_value(&reg, key, "Size", "Width").unwrap();
    rename_value(&reg, key, "Width", "WIDTH").unwrap();
    let read = reg.read(key).unwrap();
    assert_eq!(read.values.len(), 1);
    assert_eq!(read.values[0].name, "WIDTH");
    assert_eq!(read.values[0].data, Data::Dword(7));
    reg.set(key, "Other", &Data::Dword(1)).unwrap();
    assert!(
        rename_value(&reg, key, "Other", "width").is_err(),
        "a name taken"
    );
    let new = reg.rename_key(key, "New").unwrap();
    assert_eq!(new, "HKEY_CURRENT_USER\\Software\\WinWeaverRename\\New");
    assert!(reg.read(key).is_err(), "the old key is gone");
    let inner = reg.read(&format!("{new}\\Inner")).unwrap();
    assert_eq!(inner.values[0].data, Data::Sz("x".to_owned()));
    assert!(reg.rename_key(&new, "a\\b").is_err());
    assert!(
        reg.rename_key("HKEY_CURRENT_USER", "X").is_err(),
        "a hive stays"
    );
}
