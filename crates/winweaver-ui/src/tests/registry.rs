//! The registry tab: edits, questions, bookmarks, the rights, the search.

use super::*;

#[test]
fn the_rights_of_a_key_change_after_a_question() {
    use winweaver_core::registry::{MemoryRegistry, Registry};
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let reg = std::sync::Arc::new(MemoryRegistry::default());
    let key = "HKEY_CURRENT_USER\\Software\\WinWeaverRights";
    reg.create_key(key).unwrap();
    app.registry = reg.clone();
    app.apply(vec![Act::Registry(RegistryAct::Open(key.to_owned(), None))]);
    app.apply(vec![Act::Registry(RegistryAct::Rights)]);
    let open = |app: &App| match app.docs.get(app.active) {
        Some(Doc::Registry(d)) => d.rights.is_some(),
        _ => false,
    };
    let Some(Doc::Registry(d)) = app.docs.get_mut(app.active) else {
        panic!("the registry tab");
    };
    let rights = d.rights.as_mut().expect("the window of the rights");
    assert_eq!(rights.rights.entries.len(), 2, "what the key inherits");
    let owner = rights.names.get("BA").map(String::as_str);
    assert_eq!(owner, Some("Administrators"));
    rights.add = "Users".to_owned();
    app.apply(vec![
        Act::Registry(RegistryAct::RightsAdd),
        crate::question::change_rights(key).into(),
    ]);
    assert!(app.question.is_some(), "a change of the rights asks first");
    assert!(open(&app));
    app.apply(vec![Act::Answer(true)]);
    let text = reg.security(key).unwrap();
    assert!(text.contains("(A;CI;KR;;;BU)"), "{text}");
    assert!(!open(&app), "the window closes");
}

/// The registry tab writes a value of a type through its editor, asks before it deletes and
/// bookmarks a value; here on a registry in memory.
#[test]
fn the_registry_tab_edits_asks_and_bookmarks() {
    use winweaver_core::registry::{Data, MemoryRegistry, Registry};
    let dir = std::env::temp_dir().join(format!("winweaver-reg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.marks_path = dir.join("marks.wwmarks");
    let reg = std::sync::Arc::new(MemoryRegistry::default());
    let key = "HKEY_CURRENT_USER\\Software\\WinWeaverTest";
    reg.create_key(key).unwrap();
    app.registry = reg.clone();
    app.apply(vec![Act::Registry(RegistryAct::Open(
        "HKCU:\\Software\\WinWeaverTest".to_owned(),
        None,
    ))]);
    let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
        panic!("the registry tab");
    };
    assert_eq!(d.path, key);
    app.apply(vec![Act::Registry(RegistryAct::New(
        regview::EditKind::NewValue,
        "REG_DWORD",
    ))]);
    if let Some(Doc::Registry(d)) = app.docs.get_mut(app.active)
        && let Some(edit) = &mut d.edit
    {
        edit.name = "N".to_owned();
        edit.text = "0x10".to_owned();
    }
    app.apply(vec![Act::Registry(RegistryAct::Save)]);
    assert_eq!(reg.read(key).unwrap().values[0].data, Data::Dword(16));
    app.apply(vec![Act::Registry(RegistryAct::AskDelete)]);
    assert!(app.question.is_some(), "a deletion asks");
    app.apply(vec![
        Act::Answer(false),
        Act::Registry(RegistryAct::Bookmark),
    ]);
    assert_eq!(reg.read(key).unwrap().values.len(), 1);
    assert!(
        app.marks
            .items
            .iter()
            .any(|b| b.is_registry() && b.path == key && b.item == "N")
    );
    app.apply(vec![
        Act::Registry(RegistryAct::AskDelete),
        Act::Answer(true),
    ]);
    assert!(reg.read(key).unwrap().values.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The search of the registry tab runs in a thread of its own under the selected key and
/// lists what it finds; a click on a place shows it.
#[test]
fn the_registry_tab_searches_under_its_key() {
    use winweaver_core::registry::{Data, MemoryRegistry, Registry};
    fn search(app: &mut App) -> &mut regview::Search {
        match app.docs.get_mut(app.active) {
            Some(Doc::Registry(d)) => &mut d.search,
            _ => panic!("the registry tab"),
        }
    }
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let reg = std::sync::Arc::new(MemoryRegistry::default());
    let key = "HKEY_CURRENT_USER\\Software\\WinWeaverSearch";
    let inner = format!("{key}\\Inner");
    let blue = Data::Sz("dark blue".to_owned());
    reg.create_key(&inner).unwrap();
    reg.set(&inner, "Colour", &blue).unwrap();
    app.registry = reg;
    app.apply(vec![Act::Registry(RegistryAct::Open(key.to_owned(), None))]);
    app.apply(vec![Act::Registry(RegistryAct::Search)]);
    assert!(search(&mut app).open);
    // An empty text starts nothing.
    app.apply(vec![Act::Registry(RegistryAct::Find)]);
    assert!(search(&mut app).running.is_none());
    search(&mut app).text = "BLUE".to_owned();
    app.apply(vec![Act::Registry(RegistryAct::Find)]);
    assert!(search(&mut app).running.is_some());
    let ctx = egui::Context::default();
    let started = std::time::Instant::now();
    while search(&mut app).running.is_some() {
        assert!(started.elapsed() < std::time::Duration::from_secs(30));
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.poll_registry_search(&ctx);
    }
    let found = search(&mut app).found.clone().expect("a result");
    assert!(found.complete);
    assert_eq!(found.hits.len(), 1);
    assert_eq!(found.hits[0].path, inner);
    assert_eq!(found.hits[0].value.as_deref(), Some("Colour"));
    let hit = found.hits[0].clone();
    app.apply(vec![Act::Registry(RegistryAct::Go(hit.path, hit.value))]);
    let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
        panic!("the registry tab");
    };
    assert_eq!(d.value.as_deref(), Some("Colour"));
}
