//! The policy templates: imports, the unknown options, the editor of a catalog, the links between catalogs.

use super::*;
use crate::ShellAct;

/// Only the imports the settings name are read; Load catalogs finds the one an unknown option
/// of the profile belongs to, marks it and loads it; unchecking it forgets it again.
#[test]
fn only_the_checked_imports_load_and_unknown_options_find_their_catalog() {
    let dir = std::env::temp_dir().join(format!("winweaver-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    sample_import(&dir);
    let catalog = Catalog::embedded().unwrap();
    let earlier = Settings {
        extra: Extra {
            loaded_imports: None,
            hidden_imports: vec!["test".to_owned()],
            ..Extra::default()
        },
        ..Settings::default()
    };
    let mut app = Window::new(WinWeaver::new(catalog.clone(), Vec::new()), earlier);
    app.app.imports_dir = dir.join("imports");
    app.app.load_imports(&mut app.shell);
    assert!(app.app.imports.is_empty(), "a hidden import stays unread");
    assert_eq!(app.shell.settings.extra.loaded_imports, Some(Vec::new()));
    assert_eq!(app.app.import_list.len(), 1);
    assert_eq!(
        app.app.import_list[0].name, "Test templates",
        "the name comes from the header of the file"
    );
    let id = "admx.sample.turnoff";
    let mut profile = Profile::new("P", catalog.version());
    profile.set_on(id, true);
    profile.split_unknown(&app.app.catalog);
    app.shell.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
        profile,
        None,
        &app.app.catalog,
    )))];
    app.shell.active = 0;
    let state = |app: &Window<WinWeaver>| match &app.shell.docs[0] {
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
    assert_eq!(app.app.imports.len(), 1, "{}", app.shell.status);
    assert_eq!(
        app.shell.settings.extra.loaded_imports,
        Some(vec!["test".to_owned()])
    );
    let (unknown, on, _, sel) = state(&app);
    assert_eq!((unknown, on), (0, true), "the option is known and on again");
    assert_eq!(sel, Sel::Rule(id.to_owned()), "the selection follows it");
    app.apply(vec![Act::Import(ImportAct::Toggle("test".to_owned()))]);
    assert!(
        app.app.imports.is_empty(),
        "unchecked, the import leaves memory"
    );
    assert_eq!(app.shell.settings.extra.loaded_imports, Some(Vec::new()));
    let (unknown, _, _, sel) = state(&app);
    assert_eq!((unknown, sel), (1, Sel::Unknown(id.to_owned())));
    let mut next = Window::new(
        WinWeaver::new(catalog, Vec::new()),
        Settings {
            extra: Extra {
                loaded_imports: None,
                ..Extra::default()
            },
            ..Settings::default()
        },
    );
    next.app.imports_dir = dir.join("imports");
    next.app.load_imports(&mut next.shell);
    assert_eq!(
        next.app.imports.len(),
        1,
        "settings of an earlier version load what was not hidden"
    );
    assert_eq!(
        next.shell.settings.extra.loaded_imports,
        Some(vec!["test".to_owned()])
    );
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
    let mut app = Window::new(
        WinWeaver::new(catalog.clone(), Vec::new()),
        Settings::default(),
    );
    app.shell.docs = vec![Doc::Profile(Box::new(ProfileDoc::new(
        profile, None, &catalog,
    )))];
    app.shell.active = 0;
    app.apply(vec![Act::Profile(ProfileAct::Expand(
        UNKNOWN_GROUP.to_owned(),
        true,
    ))]);
    let mut harness = egui_kittest::Harness::new_eframe(|_| app);
    harness.step();
    let has = |h: &egui_kittest::Harness<'_, Window<WinWeaver>>, label: &str| {
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
    harness.state_mut().shell.settings.extra.hide_unknown = true;
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
        extra: Extra {
            loaded_imports: Some(Vec::new()),
            ..Extra::default()
        },
        ..Settings::default()
    };
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), settings);
    app.app.imports_dir = dir.join("imports");
    app.apply(vec![Act::Catalog(CatalogAct::New)]);
    let Some(Doc::CatalogEdit(d)) = app.shell.docs.get_mut(app.shell.active) else {
        panic!("the editor: {}", app.shell.status);
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
    app.apply(vec![Act::Shell(ShellAct::Save)]);
    assert!(app.app.catalog.rule(&id).is_some(), "{}", app.shell.status);
    app.apply(vec![crate::question::remove_policy(id.clone()).into()]);
    assert!(app.shell.question.is_some(), "a removal asks");
    app.apply(vec![
        Act::Shell(ShellAct::Answer(true)),
        Act::Shell(ShellAct::Save),
    ]);
    assert!(app.app.catalog.rule(&id).is_none());
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
        extra: Extra {
            loaded_imports: Some(vec!["test".to_owned(), "again".to_owned()]),
            ..Extra::default()
        },
        ..Settings::default()
    };
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), settings);
    app.app.imports_dir = dir.join("imports");
    app.app.load_imports(&mut app.shell);
    let id = "admx.sample.turnoff";
    let alias: Vec<String> = app
        .app
        .catalog
        .alias_groups(id)
        .map(str::to_owned)
        .collect();
    assert_eq!(alias.len(), 1, "the second import shows the policy too");
    app.apply(vec![Act::Profile(ProfileAct::ShowIn(
        id.to_owned(),
        alias[0].clone(),
    ))]);
    let Doc::Profile(doc) = &app.shell.docs[0] else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Rule(id.to_owned()));
    assert_eq!(doc.focus.as_ref(), Some(&alias[0]));
    assert!(doc.expanded.contains(&alias[0]), "its tree opens to it");
    app.apply(vec![Act::Catalog(CatalogAct::OpenAt(
        "again".to_owned(),
        id.to_owned(),
    ))]);
    let Some(Doc::Catalog(c)) = app.shell.docs.get(app.shell.active) else {
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
        extra: Extra {
            loaded_imports: Some(vec!["test".to_owned()]),
            ..Extra::default()
        },
        recent: vec![PathBuf::from("a.wwprofile")],
        ..Settings::default()
    };
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), settings);
    app.app.imports_dir = dir.join("imports");
    app.app.load_imports(&mut app.shell);
    app.apply(vec![
        crate::question::remove_import("test", "test".to_owned()).into(),
    ]);
    assert!(app.shell.question.is_some());
    app.apply(vec![Act::Shell(ShellAct::Answer(false))]);
    assert!(dir.join("imports").join("test.wwcatalog").exists());
    assert_eq!(app.app.imports.len(), 1);
    app.apply(vec![Act::Profile(ProfileAct::Toggle(
        "install.bypass-tpm".to_owned(),
        false,
    ))]);
    app.apply(vec![Act::Shell(ShellAct::CloseTab(0, false))]);
    let question = app
        .shell
        .question
        .as_ref()
        .expect("a tab with changes asks");
    assert!(question.alt.is_some(), "Don't save beside Save");
    app.apply(vec![Act::Shell(ShellAct::Answer(false))]);
    assert_eq!(app.shell.docs.len(), 1);
    assert!(
        app.shell.docs[0].dirty(),
        "Cancel keeps the tab and its changes"
    );
    app.apply(vec![
        app.shell.forget_recent_act(),
        Act::Shell(ShellAct::Answer(true)),
    ]);
    assert!(app.shell.settings.recent.is_empty());
    app.apply(vec![crate::question::renew_token().into()]);
    assert!(app.shell.question.is_some());
    app.apply(vec![Act::Shell(ShellAct::Answer(false))]);
    app.apply(vec![
        Act::Import(ImportAct::Toggle("copy".to_owned())),
        Act::Catalog(CatalogAct::Open("test".to_owned())),
        Act::Catalog(CatalogAct::Compare(CatalogRef::Import("copy".to_owned()))),
    ]);
    let Some(Doc::Markdown(page)) = app.shell.docs.get(app.shell.active) else {
        panic!("a page of the comparison: {}", app.shell.status);
    };
    assert!(
        page.text.contains("Only in Test templates"),
        "{}",
        page.text
    );
    assert!(page.text.contains("`admx.sample.turnoff`"), "{}", page.text);
    std::fs::remove_dir_all(&dir).unwrap();
}
