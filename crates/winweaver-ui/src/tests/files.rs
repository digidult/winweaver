//! The files: profiles, drafts, bookmarks, diffs, registry files.

use super::*;

#[test]
fn file_stems_are_safe() {
    assert_eq!(file_stem("Office 2026"), "Office 2026");
    assert_eq!(file_stem("a/b:c"), "a_b_c");
    assert_eq!(file_stem("   "), "profile");
    assert_eq!(
        with_extension(PathBuf::from("x")),
        PathBuf::from("x.wwprofile")
    );
    assert_eq!(
        with_extension(PathBuf::from("x.json")),
        PathBuf::from("x.json")
    );
}

#[test]
fn bookmarks_toggle_and_lead_back() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let dir = std::env::temp_dir().join(format!("winweaver-marks-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    app.marks_path = dir.join(bookmarks::DEFAULT_FILE);
    app.apply(vec![
        Act::Profile(ProfileAct::Select(Sel::Rule("install.bypass-tpm".into()))),
        Act::Marks(MarksAct::Toggle),
    ]);
    assert_eq!(app.marks.items.len(), 1, "{}", app.status);
    assert_eq!(app.marks.items[0].item, "install.bypass-tpm");
    assert_eq!(
        app.marks.items[0].path, "",
        "an unsaved profile has no file to link"
    );
    assert_eq!(
        app.marks.items[0].title, "",
        "the title comes from the catalog"
    );
    assert!(
        app.marks_path.exists(),
        "the default file is created at the first bookmark"
    );
    app.apply(vec![
        Act::Profile(ProfileAct::Select(Sel::None)),
        Act::Marks(MarksAct::Open),
        Act::Marks(MarksAct::GoTo(0)),
    ]);
    assert_eq!(app.active, 0, "{}", app.status);
    assert_eq!(
        app.active_profile().unwrap().sel,
        Sel::Rule("install.bypass-tpm".into())
    );
    app.apply(vec![Act::Marks(MarksAct::Toggle)]);
    assert!(app.marks.items.is_empty());
    app.apply(vec![
        Act::Marks(MarksAct::Toggle),
        Act::Marks(MarksAct::SetComment(
            0,
            "Ask the customer\nabout the TPM".into(),
        )),
        Act::Marks(MarksAct::Toggle),
    ]);
    assert_eq!(
        app.marks.items.len(),
        1,
        "a bookmark with a comment asks first"
    );
    assert!(app.question.is_some());
    app.apply(vec![Act::Answer(false)]);
    assert_eq!(app.marks.items.len(), 1);
    app.apply(vec![
        Act::Marks(MarksAct::Open),
        Act::Marks(MarksAct::Select(0)),
    ]);
    let question = marksview::remove_question(&app.catalog, None, &app.imports, &app.marks, 0)
        .expect("the bookmark is there");
    app.apply(vec![question.into()]);
    assert!(app.question.is_some());
    app.apply(vec![Act::Answer(true)]);
    assert!(app.marks.items.is_empty());
    assert_eq!(app.marks_doc().and_then(|m| m.sel), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn drafts_come_back_after_a_restart() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
    app.drafts_dir = dir.join("drafts");
    app.logs_dir = dir.join("logs");
    app.apply(vec![Act::Profile(ProfileAct::Toggle(
        "install.bypass-tpm".into(),
        false,
    ))]);
    app.write_drafts(true);
    let (drafts, _) = session::read_drafts(&app.drafts_dir);
    assert_eq!(drafts.len(), 1);
    let mut next = App::new(catalog, Vec::new(), Settings::default());
    next.drafts_dir = app.drafts_dir.clone();
    next.logs_dir = app.logs_dir.clone();
    next.restore_session();
    assert_eq!(
        next.docs.len(),
        1,
        "the untouched start tab gives way to the draft"
    );
    let Doc::Profile(doc) = &next.docs[0] else {
        panic!("a profile");
    };
    assert!(doc.dirty && !doc.profile.is_on("install.bypass-tpm"));
    next.apply(vec![Act::Nav(NavAct::CloseTab(0, true))]);
    assert!(
        session::read_drafts(&next.drafts_dir).0.is_empty(),
        "discarding deletes the draft"
    );
    next.log_status();
    assert!(dir.join("logs").is_dir());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn comparing_with_a_preset_opens_a_diff_tab_that_applies() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    assert_eq!(app.docs.len(), 2);
    assert!(matches!(&app.docs[1], Doc::Diff(d) if !d.diff.is_empty()));
    assert_eq!(app.docs[1].title(), "Office vs Strict.wwdiff");
    app.apply(vec![Act::Diff(DiffAct::Apply)]);
    assert_eq!(app.active, 0);
    let Doc::Profile(office) = &app.docs[0] else {
        panic!("the first tab is the profile");
    };
    assert!(office.dirty);
    let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
    assert_eq!(office.profile.rules, strict.rules);
}

#[test]
fn an_item_of_a_diff_takes_a_comment_applies_alone_and_leads_to_both_profiles() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    let Doc::Diff(d) = &app.docs[1] else {
        panic!("the second tab is the diff");
    };
    let key = d
        .diff
        .keys()
        .into_iter()
        .find(|k| ItemKey::rule(k).is_some())
        .expect("Strict differs in a rule");
    let id = ItemKey::rule(&key).expect("the key of a rule").to_owned();
    app.apply(vec![
        Act::Diff(DiffAct::Select(key.clone())),
        Act::Diff(DiffAct::Comment(
            key.clone(),
            "Checked with the lab
and kept"
                .into(),
        )),
    ]);
    let Doc::Diff(d) = &app.docs[1] else {
        panic!("the second tab is the diff");
    };
    assert_eq!(d.sel.as_deref(), Some(key.as_str()));
    assert_eq!(
        d.diff.comments[&key],
        "Checked with the lab
and kept"
    );
    assert!(d.dirty);
    app.apply(vec![Act::Diff(DiffAct::Comment(key.clone(), "  ".into()))]);
    assert!(matches!(&app.docs[1], Doc::Diff(d) if d.diff.comments.is_empty()));
    app.apply(vec![Act::Diff(DiffAct::ApplyItem(key.clone()))]);
    assert_eq!(app.active, 1, "the diff stays in front");
    let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
    let Doc::Profile(office) = &app.docs[0] else {
        panic!("the first tab is the profile");
    };
    assert_eq!(office.profile.rules.get(&id), strict.rules.get(&id));
    app.apply(vec![Act::Diff(DiffAct::GoTo(key.clone(), true))]);
    assert_eq!(app.docs.len(), 3, "the preset Strict opens in a new tab");
    assert_eq!(app.active, 2);
    assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id.clone()));
    app.active = 1;
    app.apply(vec![Act::Diff(DiffAct::GoTo(key, false))]);
    assert_eq!(app.active, 0);
    assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id));
}

#[test]
fn opening_a_file_adds_a_tab_and_remembers_it() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("strict.wwprofile");
    std::fs::write(&path, PRESETS[1].1).unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.open_path(path.clone());
    assert_eq!(app.docs.len(), 2);
    assert_eq!(app.active, 1);
    assert_eq!(app.settings.recent, vec![path.clone()]);
    app.open_path(path.clone());
    assert_eq!(app.docs.len(), 2, "the same file activates its tab");
    app.open_path(dir.join("missing.wwprofile"));
    assert!(app.status.starts_with("Open failed"), "{}", app.status);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_registry_file_opens_in_its_tab_and_keeps_its_comments() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-reg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.reg");
    let text = [
        regfile::HEADER,
        "",
        "; kept",
        "[HKEY_CURRENT_USER\\Software\\WinWeaver]",
        "\"A\"=dword:00000001",
    ]
    .join("\r\n");
    std::fs::write(&path, text).unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::File(FileAct::OpenPath(path.clone()))]);
    let Some(Doc::RegFile(d)) = app.docs.get_mut(app.active) else {
        panic!("the file has no tab: {}", app.status);
    };
    // A new value goes after the last value of the selected key.
    d.at = Some(2);
    d.start_new(regfileview::EditKind::Value);
    let edit = d.edit.as_mut().unwrap();
    edit.name = "B".to_owned();
    edit.text = "text".to_owned();
    assert!(d.apply_edit());
    assert!(app.docs[app.active].dirty());
    app.apply(vec![Act::File(FileAct::Save)]);
    assert!(!app.docs[app.active].dirty(), "{}", app.status);
    let saved = RegFile::load(&path).unwrap().to_text();
    assert!(saved.contains("; kept"), "{saved}");
    assert!(saved.contains("\"B\"=\"text\""), "{saved}");
    // Removing the key asks first; its comment stays.
    if let Some(Doc::RegFile(d)) = app.docs.get_mut(app.active) {
        d.line = None;
    }
    let Some(Doc::RegFile(d)) = app.docs.get(app.active) else {
        panic!("the tab of the file is gone");
    };
    let question = d.remove_question().expect("the key is removable");
    app.apply(vec![question.into()]);
    assert!(app.question.is_some());
    app.apply(vec![Act::Answer(true)]);
    let Some(Doc::RegFile(d)) = app.docs.get(app.active) else {
        panic!("the tab of the file is gone");
    };
    let text = d.file.to_text();
    assert!(text.contains("; kept"), "{text}");
    assert!(!text.contains("WinWeaver]"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}
