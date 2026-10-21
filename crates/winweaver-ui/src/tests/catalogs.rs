//! The policy templates: imports, the unknown options, the editor of a catalog, the links between catalogs.

use super::*;

/// Only the imports the settings name are read; Load catalogs finds the one an unknown option
/// of the profile belongs to, marks it and loads it; unchecking it forgets it again.
#[test]
fn only_the_checked_imports_load_and_unknown_options_find_their_catalog() {
    let dir = std::env::temp_dir().join(format!("winweaver-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    sample_import(&dir);
    let catalog = Catalog::embedded().unwrap();
    let earlier = Settings {
        loaded_imports: None,
        hidden_imports: vec!["test".to_owned()],
        ..Settings::default()
    };
    let mut app = App::new(catalog.clone(), Vec::new(), earlier);
    app.imports_dir = dir.join("imports");
    app.load_imports();
    assert!(app.imports.is_empty(), "a hidden import stays unread");
    assert_eq!(app.settings.loaded_imports, Some(Vec::new()));
    assert_eq!(app.import_list.len(), 1);
    assert_eq!(
        app.import_list[0].name, "Test templates",
        "the name comes from the header of the file"
    );
    let id = "admx.sample.turnoff";
    let mut profile = Profile::new("P", catalog.version());
    profile.set_on(id, true);
    profile.split_unknown(&app.catalog);
    app.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
        profile,
        None,
        &app.catalog,
    )))];
    app.active = 0;
    let state = |app: &App| match &app.docs[0] {
        Doc::Profile(d) => (
            d.profile.unknown.len(),
            d.profile.is_on(id),
            d.issues.iter().any(|i| i.severity == Severity::Warning),
            d.sel.clone(),
        ),
        _ => panic!("a profile"),
    };
    assert_eq!(state(&app).0, 1);
    assert!(state(&app).2, "the open warns of the option");
    app.apply(vec![Act::Profile(ProfileAct::Select(Sel::Unknown(
        id.to_owned(),
    )))]);
    app.apply(vec![Act::Import(ImportAct::LoadCatalogs)]);
    assert_eq!(app.imports.len(), 1, "{}", app.status);
    assert_eq!(app.settings.loaded_imports, Some(vec!["test".to_owned()]));
    let (unknown, on, _, sel) = state(&app);
    assert_eq!((unknown, on), (0, true), "the option is known and on again");
    assert_eq!(sel, Sel::Rule(id.to_owned()), "the selection follows it");
    app.apply(vec![Act::Import(ImportAct::Toggle("test".to_owned()))]);
    assert!(
        app.imports.is_empty(),
        "unchecked, the import leaves memory"
    );
    assert_eq!(app.settings.loaded_imports, Some(Vec::new()));
    let (unknown, _, _, sel) = state(&app);
    assert_eq!((unknown, sel), (1, Sel::Unknown(id.to_owned())));
    let mut next = App::new(
        catalog,
        Vec::new(),
        Settings {
            loaded_imports: None,
            ..Settings::default()
        },
    );
    next.imports_dir = dir.join("imports");
    next.load_imports();
    assert_eq!(
        next.imports.len(),
        1,
        "settings of an earlier version load what was not hidden"
    );
    assert_eq!(next.settings.loaded_imports, Some(vec!["test".to_owned()]));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The options of catalogs that are not loaded have their group at the end of the tree,
/// unless the settings hide them.
#[test]
fn the_unknown_options_have_their_group_in_the_tree() {
    use egui_kittest::kittest::Queryable;
    let catalog = Catalog::embedded().unwrap();
    let mut profile = Profile::new("P", catalog.version());
    profile.set_on("admx.other.policy", true);
    profile.split_unknown(&catalog);
    let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
    app.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
        profile, None, &catalog,
    )))];
    app.active = 0;
    app.apply(vec![Act::Profile(ProfileAct::Expand(
        UNKNOWN_GROUP.to_owned(),
        true,
    ))]);
    let mut harness = egui_kittest::Harness::new_eframe(|_| app);
    harness.step();
    let has = |h: &egui_kittest::Harness<'_, App>, label: &str| {
        h.query_all_by_label(label).next().is_some()
    };
    assert!(has(&harness, "Unknown options"));
    assert!(has(&harness, "admx.other.policy"));
    harness
        .state_mut()
        .apply(vec![Act::Profile(ProfileAct::Select(Sel::Unknown(
            "admx.other.policy".to_owned(),
        )))]);
    harness.step();
    assert!(has(&harness, "Load catalogs"));
    harness
        .state_mut()
        .apply(vec![Act::Profile(ProfileAct::Select(Sel::None))]);
    harness.state_mut().settings.hide_unknown = true;
    harness.step();
    harness.step();
    assert!(!has(&harness, "Unknown options"), "hidden by the settings");
}

/// A new catalog gets a policy in its editor; a key of another root is refused and changes
/// nothing; the save loads the policy into the catalog; a removal asks first.
#[test]
fn a_new_catalog_is_edited_checked_and_saved() {
    let dir = std::env::temp_dir().join(format!("winweaver-editor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let catalog = Catalog::embedded().unwrap();
    let settings = Settings {
        loaded_imports: Some(Vec::new()),
        ..Settings::default()
    };
    let mut app = App::new(catalog, Vec::new(), settings);
    app.imports_dir = dir.join("imports");
    app.apply(vec![Act::Catalog(CatalogAct::New)]);
    let Some(Doc::CatalogEdit(d)) = app.docs.get_mut(app.active) else {
        panic!("the editor: {}", app.status);
    };
    d.new_name = "Mute".to_owned();
    d.new_policy();
    let id = d.form.as_ref().map(|f| f.id.clone()).unwrap();
    assert_eq!(id, "admx.custom-1.mute");
    if let Some(form) = d.form.as_mut() {
        form.actions.push(cataledit::ActionRow {
            remove: false,
            key: "HKU\\X".to_owned(),
            name: "Mute".to_owned(),
            kind: winweaver_core::catalog::RegKind::DWord,
            value: "1".to_owned(),
            keep: None,
        });
    }
    d.apply();
    assert!(d.error.is_some(), "a key of another root is refused");
    let rule = d.file.rules.iter().find(|r| r.id == id).unwrap();
    assert!(rule.actions.is_empty(), "and changes nothing");
    if let Some(form) = d.form.as_mut() {
        form.actions[0].key = "HKEY_LOCAL_MACHINE\\Software\\Policies\\Zoom".to_owned();
    }
    d.apply();
    assert!(d.error.is_none(), "{:?}", d.error);
    app.apply(vec![Act::File(FileAct::Save)]);
    assert!(app.catalog.rule(&id).is_some(), "{}", app.status);
    app.apply(vec![Question::remove_policy(id.clone()).into()]);
    assert!(app.question.is_some(), "a removal asks");
    app.apply(vec![Act::Answer(true), Act::File(FileAct::Save)]);
    assert!(app.catalog.rule(&id).is_none());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A link opens the catalog tab of an import at the policy, and the same policy in the tree
/// of another import is selected there, not in the first tree.
#[test]
fn links_open_the_catalog_at_a_policy_and_select_it_in_another_tree() {
    let dir = std::env::temp_dir().join(format!("winweaver-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    sample_import(&dir);
    let again = admx::to_catalog_file(
        &admx::read_folder(&dir.join("admx")),
        "again",
        "Again",
        "",
        "now",
    );
    again
        .save(&dir.join("imports").join("again.wwcatalog"))
        .unwrap();
    let catalog = Catalog::embedded().unwrap();
    let settings = Settings {
        loaded_imports: Some(vec!["test".to_owned(), "again".to_owned()]),
        ..Settings::default()
    };
    let mut app = App::new(catalog, Vec::new(), settings);
    app.imports_dir = dir.join("imports");
    app.load_imports();
    let id = "admx.sample.turnoff";
    let alias: Vec<String> = app.catalog.alias_groups(id).map(str::to_owned).collect();
    assert_eq!(alias.len(), 1, "the second import shows the policy too");
    app.apply(vec![Act::Profile(ProfileAct::ShowIn(
        id.to_owned(),
        alias[0].clone(),
    ))]);
    let Doc::Profile(doc) = &app.docs[0] else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Rule(id.to_owned()));
    assert_eq!(doc.focus.as_ref(), Some(&alias[0]));
    assert!(doc.expanded.contains(&alias[0]), "its tree opens to it");
    app.apply(vec![Act::Catalog(CatalogAct::OpenAt(
        "again".to_owned(),
        id.to_owned(),
    ))]);
    let Some(Doc::Catalog(c)) = app.docs.get(app.active) else {
        panic!("the catalog tab");
    };
    assert_eq!((c.id.as_str(), c.sel.as_deref()), ("again", Some(id)));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Removing an import, discarding changes, forgetting the recent files and renewing the
/// token ask first; two catalogs compare in a page.
#[test]
fn removals_ask_first_and_two_catalogs_compare() {
    let dir = std::env::temp_dir().join(format!("winweaver-ask-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    sample_import(&dir);
    let mut copy = admx::to_catalog_file(
        &admx::read_folder(&dir.join("admx")),
        "copy",
        "Copy",
        "",
        "now",
    );
    copy.rules.clear();
    copy.save(&dir.join("imports").join("copy.wwcatalog"))
        .unwrap();
    let catalog = Catalog::embedded().unwrap();
    let settings = Settings {
        loaded_imports: Some(vec!["test".to_owned()]),
        recent: vec![PathBuf::from("a.wwprofile")],
        ..Settings::default()
    };
    let mut app = App::new(catalog, Vec::new(), settings);
    app.imports_dir = dir.join("imports");
    app.load_imports();
    app.apply(vec![
        Question::remove_import("test", "test".to_owned()).into(),
    ]);
    assert!(app.question.is_some());
    app.apply(vec![Act::Answer(false)]);
    assert!(dir.join("imports").join("test.wwcatalog").exists());
    assert_eq!(app.imports.len(), 1);
    app.apply(vec![Act::Profile(ProfileAct::Toggle(
        "install.bypass-tpm".to_owned(),
        false,
    ))]);
    let act = app.discard_act(0);
    app.apply(vec![act]);
    assert!(app.question.is_some(), "a tab with changes asks");
    app.apply(vec![Act::Answer(false)]);
    assert_eq!(app.docs.len(), 1);
    app.apply(vec![Question::forget_recent().into(), Act::Answer(true)]);
    assert!(app.settings.recent.is_empty());
    app.apply(vec![Question::renew_token().into()]);
    assert!(app.question.is_some());
    app.apply(vec![Act::Answer(false)]);
    app.apply(vec![
        Act::Import(ImportAct::Toggle("copy".to_owned())),
        Act::Catalog(CatalogAct::Open("test".to_owned())),
        Act::Catalog(CatalogAct::Compare(CatalogRef::Import("copy".to_owned()))),
    ]);
    let Some(Doc::Markdown(page)) = app.docs.get(app.active) else {
        panic!("a page of the comparison: {}", app.status);
    };
    assert!(
        page.text.contains("Only in Test templates"),
        "{}",
        page.text
    );
    assert!(page.text.contains("`admx.sample.turnoff`"), "{}", page.text);
    std::fs::remove_dir_all(&dir).unwrap();
}
