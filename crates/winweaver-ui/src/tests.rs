
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
fn app_starts_with_the_office_preset() {
    let catalog = Catalog::embedded().unwrap();
    let app = App::new(
        catalog,
        CatalogTexts::embedded().unwrap(),
        Settings::default(),
    );
    assert_eq!(app.docs.len(), 1);
    assert_eq!(app.docs[0].title(), "Office.wwprofile");
    assert!(!app.docs[0].dirty());
    assert_eq!(app.window_title(), "Office.wwprofile - WinWeaver");
}

#[test]
fn the_guide_opens_once_and_markdown_files_open() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::OpenGuide, Act::OpenGuide]);
    assert_eq!(app.docs.len(), 2, "the guide tab exists once");
    assert_eq!(app.docs[1].title(), "Guide.md");
    let dir = std::env::temp_dir().join(format!("winweaver-md-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.md");
    std::fs::write(&path, "# Notes\n\nA line.\n").unwrap();
    app.open_path(path.clone());
    assert_eq!(app.docs.len(), 3);
    assert!(matches!(&app.docs[2], Doc::Markdown(m) if m.text.starts_with("# Notes")));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Draws frames of the whole window without a screen (egui_kittest): every kind of tab is
/// drawn and shows its texts, and a click on a rule of the tree selects it.
#[test]
fn every_kind_of_tab_draws_without_a_screen() {
    use egui_kittest::kittest::Queryable;
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(
        catalog,
        CatalogTexts::embedded().unwrap(),
        Settings::default(),
    );
    let empty = admx::to_catalog_file(
        &admx::Templates::default(),
        "empty",
        "Empty templates",
        "",
        "now",
    );
    app.imports.push(empty);
    app.rebuild_catalog();
    let mut harness = egui_kittest::Harness::new_eframe(|_| app);
    harness.step();
    let has = |h: &egui_kittest::Harness<'_, App>, label: &str| {
        h.query_all_by_label(label).next().is_some()
    };
    assert!(has(&harness, "Menu"));
    for (act, label) in [
        (Act::OpenSettings, Some("General")),
        (
            Act::OpenCatalog("empty".to_owned()),
            Some("Select a policy in the tree."),
        ),
        (Act::OpenMarks, None),
        (Act::OpenMcp, None),
        (Act::OpenGuide, None),
        (Act::EditTheme("matrix".to_owned()), Some("Use this theme")),
        (Act::Activate(0), None),
    ] {
        harness.state_mut().apply(vec![act]);
        harness.step();
        harness.step();
        if let Some(label) = label {
            assert!(has(&harness, label), "{label} is not drawn");
        }
    }
    assert_eq!(harness.state().docs.len(), 7);
    assert_eq!(
        harness.state().docs[harness.state().active].title(),
        "Office.wwprofile"
    );
    harness.state_mut().apply(vec![
        Act::Expand("unattend".to_owned(), true),
        Act::Expand("install".to_owned(), true),
    ]);
    harness.step();
    harness.step();
    let title = "Bypass the TPM 2.0 check";
    assert!(has(&harness, title), "the rules of an open group are drawn");
    harness.query_all_by_label(title).next().unwrap().click();
    harness.step();
    harness.step();
    assert!(
        matches!(harness.state().docs[0], Doc::Profile(ref d) if d.sel == Sel::Rule("install.bypass-tpm".into())),
        "a click on a rule selects it"
    );
    harness.state_mut().apply(vec![Act::CompareWith("Strict")]);
    harness.step();
    harness.step();
    assert!(has(&harness, "Select a difference to see it here."));
    let key = first_rule_key(harness.state()).expect("Strict differs in a rule");
    harness.state_mut().apply(vec![Act::DiffSelect(key)]);
    harness.step();
    harness.step();
    assert!(
        has(&harness, "Apply this difference"),
        "the item is shown on the right"
    );
}

/// The key of the first rule of the diff in the last tab.
fn first_rule_key(app: &App) -> Option<String> {
    match app.docs.last() {
        Some(Doc::Diff(d)) => d.diff.keys().into_iter().find(|k| k.starts_with("rule:")),
        _ => None,
    }
}

/// The divider between the differences and the item on the right stays where it is whatever
/// item is selected: the item never widens the list or pushes into it.
#[test]
fn the_divider_of_a_diff_stays_put_whatever_item_is_shown() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(
        catalog,
        CatalogTexts::embedded().unwrap(),
        Settings::default(),
    );
    app.apply(vec![Act::CompareWith("Strict")]);
    let keys = match app.docs.last() {
        Some(Doc::Diff(d)) => d.diff.keys(),
        _ => Vec::new(),
    };
    assert!(keys.len() > 3, "{keys:?}");
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| app);
    let list = egui::Id::new(("diff-list", "Office", "Strict"));
    let width =
        |h: &egui_kittest::Harness<'_, App>| h.ctx.read_response(list).map(|r| r.rect.width());
    harness.step();
    harness.step();
    let first = width(&harness).expect("the list is drawn");
    for key in keys {
        harness
            .state_mut()
            .apply(vec![Act::DiffSelect(key.clone())]);
        harness.step();
        harness.step();
        assert_eq!(width(&harness), Some(first), "the divider moved for {key}");
    }
}

/// Pictures of the window for the review of its design. Only when `WINWEAVER_SCREENSHOTS` names
/// a folder (CI sets it on the Windows runner): the main tabs are drawn with wgpu without a
/// screen and saved there as PNG files; a renderer that cannot start writes `error.txt`.
#[test]
fn screenshots_of_the_window() {
    let Some(dir) = std::env::var_os("WINWEAVER_SCREENSHOTS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let catalog = Catalog::embedded().unwrap();
    let app = App::new(
        catalog,
        CatalogTexts::embedded().unwrap(),
        Settings::default(),
    );
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(|cc| {
            style::install_fonts(&cc.egui_ctx);
            app
        });
    let shot = |h: &mut egui_kittest::Harness<'_, App>, name: &str| {
        for _ in 0..4 {
            h.step();
        }
        match h.render() {
            Ok(image) => {
                if let Err(e) = image.save(dir.join(format!("{name}.png"))) {
                    let _ = std::fs::write(dir.join("error.txt"), format!("{name}: {e}"));
                }
            }
            Err(e) => {
                let _ = std::fs::write(dir.join("error.txt"), format!("{name}: {e}"));
            }
        }
    };
    shot(&mut harness, "01-profile");
    harness.state_mut().apply(vec![
        Act::Expand("policies".to_owned(), true),
        Act::Expand("defender".to_owned(), true),
        Act::Select(Sel::Rule("defender.pua".to_owned())),
    ]);
    shot(&mut harness, "02-rule");
    harness.state_mut().apply(vec![
        Act::ToggleBookmark,
        Act::SetMarkComment(0, "Ask the customer about the Audit mode".into()),
        Act::Check,
    ]);
    shot(&mut harness, "03-checked");
    harness.state_mut().apply(vec![Act::CompareWith("Strict")]);
    if let Some(key) = first_rule_key(harness.state()) {
        harness.state_mut().apply(vec![Act::DiffSelect(key)]);
    }
    shot(&mut harness, "04-diff");
    for (act, name) in [
        (Act::OpenSettings, "05-settings"),
        (Act::OpenMcp, "06-mcp"),
        (Act::OpenMarks, "07-bookmarks"),
        (Act::OpenGuide, "08-guide"),
    ] {
        harness.state_mut().apply(vec![act]);
        if name == "07-bookmarks" {
            harness.state_mut().apply(vec![Act::MarkSelect(0)]);
        }
        shot(&mut harness, name);
    }
    harness
        .state_mut()
        .apply(vec![Act::EditTheme("latte".to_owned())]);
    shot(&mut harness, "10-theme-editor");
    harness
        .state_mut()
        .apply(vec![Act::UseTheme("matrix".to_owned()), Act::Activate(0)]);
    shot(&mut harness, "11-matrix");
    harness
        .state_mut()
        .apply(vec![Act::UseTheme("light".to_owned())]);
    // A registry file of the plan of the first preset, at its first key.
    let catalog = Catalog::embedded().unwrap();
    let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0).unwrap();
    let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
    let plan = winweaver_core::apply::plan_apply(&catalog, &profile, &ids, "shot");
    let reg = dir.join("Apply-Office.reg");
    let file = RegFile::from_plan(&plan, "Apply the profile Office");
    file.save(&reg).unwrap();
    harness.state_mut().apply(vec![Act::OpenPath(reg)]);
    if let Some(Doc::RegFile(d)) = harness.state_mut().docs.last_mut() {
        let key = |l: &Line| matches!(l, Line::Key { .. });
        d.at = d.file.lines.iter().position(key);
    }
    shot(&mut harness, "12-regfile");
    harness.set_size(egui::vec2(1000.0, 700.0));
    shot(&mut harness, "09-compact");
}

#[test]
fn a_theme_tab_previews_its_theme_and_a_theme_can_be_chosen() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let (light, _, preference) = app.wanted_theme();
    assert_eq!(light.id, "light");
    assert_eq!(preference, egui::ThemePreference::System);
    app.apply(vec![Act::EditTheme("matrix".into())]);
    assert!(matches!(&app.docs[1], Doc::Theme(t) if t.theme.id == "matrix"));
    assert_eq!(app.docs[1].title(), "matrix.wwtheme");
    if let Some(Doc::Theme(t)) = app.docs.get_mut(1) {
        t.theme.font_size = 16.0;
    }
    let (_, dark, preference) = app.wanted_theme();
    assert_eq!(
        dark.id, "matrix",
        "the edited theme shows while its tab is in front"
    );
    assert_eq!(dark.font_size, 16.0);
    assert_eq!(preference, egui::ThemePreference::Dark);
    app.apply(vec![Act::EditTheme("matrix".into())]);
    assert_eq!(app.docs.len(), 2, "one tab per theme");
    app.apply(vec![Act::Activate(0)]);
    assert_eq!(app.wanted_theme().0.id, "light", "the settings theme again");
    app.apply(vec![Act::UseTheme("latte".into())]);
    assert_eq!(app.settings.custom_theme, "latte");
    assert_eq!(app.settings.theme, ThemeChoice::Light);
    let (light, _, preference) = app.wanted_theme();
    assert_eq!(light.name, "Latte");
    assert_eq!(preference, egui::ThemePreference::Light);
    assert_eq!(app.shown_theme_id(true), "latte");
}

#[test]
fn bookmarks_toggle_and_lead_back() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let dir = std::env::temp_dir().join(format!("winweaver-marks-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    app.marks_path = dir.join(bookmarks::DEFAULT_FILE);
    app.apply(vec![
        Act::Select(Sel::Rule("install.bypass-tpm".into())),
        Act::ToggleBookmark,
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
        Act::Select(Sel::None),
        Act::OpenMarks,
        Act::GoToMark(0),
    ]);
    assert_eq!(app.active, 0, "{}", app.status);
    assert_eq!(
        app.active_profile().unwrap().sel,
        Sel::Rule("install.bypass-tpm".into())
    );
    app.apply(vec![Act::ToggleBookmark]);
    assert!(app.marks.items.is_empty());
    app.apply(vec![
        Act::ToggleBookmark,
        Act::SetMarkComment(0, "Ask the customer\nabout the TPM".into()),
        Act::ToggleBookmark,
    ]);
    assert_eq!(
        app.marks.items.len(),
        1,
        "a bookmark with a comment asks first"
    );
    assert!(app.ask.is_some());
    app.apply(vec![Act::AskNo]);
    assert_eq!(app.marks.items.len(), 1);
    app.apply(vec![
        Act::OpenMarks,
        Act::MarkSelect(0),
        Act::RemoveMarkAsk(0),
    ]);
    assert!(app.ask.is_some());
    app.apply(vec![Act::AskYes]);
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
    app.apply(vec![Act::Toggle("install.bypass-tpm".into(), false)]);
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
    next.apply(vec![Act::CloseTab(0, true)]);
    assert!(
        session::read_drafts(&next.drafts_dir).0.is_empty(),
        "discarding deletes the draft"
    );
    next.log_status();
    assert!(dir.join("logs").is_dir());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn history_moves_back_and_forward() {
    let catalog = Catalog::embedded().unwrap();
    let mut doc = ProfileDoc::new(Profile::from_defaults("x", &catalog), None, &catalog);
    doc.select(Sel::Group("policies".into()));
    doc.select(Sel::Rule("install.bypass-tpm".into()));
    assert!(doc.can_go(-1) && !doc.can_go(1));
    doc.go(-1);
    assert_eq!(doc.sel, Sel::Group("policies".into()));
    doc.go(1);
    assert_eq!(doc.sel, Sel::Rule("install.bypass-tpm".into()));
}

#[test]
fn comparing_with_a_preset_opens_a_diff_tab_that_applies() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::CompareWith("Strict")]);
    assert_eq!(app.docs.len(), 2);
    assert!(matches!(&app.docs[1], Doc::Diff(d) if !d.diff.is_empty()));
    assert_eq!(app.docs[1].title(), "Office vs Strict.wwdiff");
    app.apply(vec![Act::ApplyDiff]);
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
    app.apply(vec![Act::CompareWith("Strict")]);
    let Doc::Diff(d) = &app.docs[1] else {
        panic!("the second tab is the diff");
    };
    let key = d
        .diff
        .keys()
        .into_iter()
        .find(|k| k.starts_with("rule:"))
        .expect("Strict differs in a rule");
    let id = key.trim_start_matches("rule:").to_owned();
    app.apply(vec![
        Act::DiffSelect(key.clone()),
        Act::DiffComment(
            key.clone(),
            "Checked with the lab
and kept"
                .into(),
        ),
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
    app.apply(vec![Act::DiffComment(key.clone(), "  ".into())]);
    assert!(matches!(&app.docs[1], Doc::Diff(d) if d.diff.comments.is_empty()));
    app.apply(vec![Act::DiffApplyItem(key.clone())]);
    assert_eq!(app.active, 1, "the diff stays in front");
    let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
    let Doc::Profile(office) = &app.docs[0] else {
        panic!("the first tab is the profile");
    };
    assert_eq!(office.profile.rules.get(&id), strict.rules.get(&id));
    app.apply(vec![Act::DiffGoTo(key.clone(), true)]);
    assert_eq!(app.docs.len(), 3, "the preset Strict opens in a new tab");
    assert_eq!(app.active, 2);
    assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id.clone()));
    app.active = 1;
    app.apply(vec![Act::DiffGoTo(key, false)]);
    assert_eq!(app.active, 0);
    assert_eq!(app.active_profile().unwrap().sel, Sel::Rule(id));
}

#[test]
fn tabs_move_and_the_tab_in_front_stays_in_front() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::OpenGuide, Act::OpenMarks, Act::OpenSettings]);
    let titles = |app: &App| app.docs.iter().map(Doc::title).collect::<Vec<_>>();
    let start = titles(&app);
    assert_eq!(start.len(), 4);
    assert_eq!(app.active, 3);
    app.apply(vec![Act::MoveTab(3, 0)]);
    assert_eq!(app.active, 0, "the moved tab stays in front");
    assert_eq!(titles(&app)[0], start[3]);
    assert_eq!(titles(&app)[1..], start[..3]);
    app.apply(vec![Act::Activate(2), Act::MoveTab(0, 3)]);
    assert_eq!(
        app.active, 1,
        "the tab in front shifts left when a tab before it moves after it"
    );
    assert_eq!(titles(&app), start);
    app.apply(vec![Act::MoveTab(1, 1), Act::MoveTab(0, 9)]);
    assert_eq!(titles(&app), start, "nothing moves out of the row");
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
    app.apply(vec![Act::OpenPath(path.clone())]);
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
    app.apply(vec![Act::Save]);
    assert!(!app.docs[app.active].dirty(), "{}", app.status);
    let saved = RegFile::load(&path).unwrap().to_text();
    assert!(saved.contains("; kept"), "{saved}");
    assert!(saved.contains("\"B\"=\"text\""), "{saved}");
    // Removing the key asks first; its comment stays.
    if let Some(Doc::RegFile(d)) = app.docs.get_mut(app.active) {
        d.line = None;
    }
    app.apply(vec![Act::AskRegFileRemove]);
    assert!(app.ask.is_some());
    app.apply(vec![Act::AskYes]);
    let Some(Doc::RegFile(d)) = app.docs.get(app.active) else {
        panic!("the tab of the file is gone");
    };
    let text = d.file.to_text();
    assert!(text.contains("; kept"), "{text}");
    assert!(!text.contains("WinWeaver]"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_link_selects_and_opens_the_tree_down_to_it() {
    let catalog = Catalog::embedded().unwrap();
    let group = catalog
        .groups()
        .iter()
        .find(|g| g.parent.is_some())
        .map(|g| g.id.clone())
        .unwrap();
    let parent = catalog
        .group(&group)
        .and_then(|g| g.parent.clone())
        .unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    if let Some(Doc::Profile(doc)) = app.docs.get_mut(0) {
        doc.expanded.clear();
    }
    app.apply(vec![Act::Reveal(Sel::Group(group.clone()))]);
    let Some(Doc::Profile(doc)) = app.docs.first() else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Group(group));
    assert!(doc.expanded.contains(&parent), "the tree opens to it");
    assert!(doc.reveal);
}

#[test]
fn the_rights_of_a_key_change_after_a_question() {
    use winweaver_core::registry::{MemoryRegistry, Registry};
    let catalog = Catalog::embedded().unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    let reg = std::sync::Arc::new(MemoryRegistry::default());
    let key = "HKEY_CURRENT_USER\\Software\\WinWeaverRights";
    reg.create_key(key).unwrap();
    app.registry = reg.clone();
    app.apply(vec![Act::OpenRegistry(key.to_owned(), None)]);
    app.apply(vec![Act::RegistryRights]);
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
    app.apply(vec![Act::RegistryRightsAdd, Act::AskRegistryRights]);
    assert!(app.ask.is_some(), "a change of the rights asks first");
    assert!(open(&app));
    app.apply(vec![Act::AskYes]);
    let text = reg.security(key).unwrap();
    assert!(text.contains("(A;CI;KR;;;BU)"), "{text}");
    assert!(!open(&app), "the window closes");
}

#[test]
fn a_group_shows_in_the_profile_from_another_tab() {
    let catalog = Catalog::embedded().unwrap();
    let group = catalog
        .groups()
        .iter()
        .find(|g| g.parent.is_some())
        .map(|g| g.id.clone())
        .unwrap();
    let mut app = App::new(catalog, Vec::new(), Settings::default());
    app.apply(vec![Act::OpenMarks, Act::ShowGroupInProfile(group.clone())]);
    assert_eq!(app.active, 0, "the profile tab comes to the front");
    let Some(Doc::Profile(doc)) = app.docs.first() else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Group(group));
    assert!(doc.reveal);
}

const SAMPLE_ADMX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
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
  </categories>
  <policies>
    <policy name="TurnOff" class="Machine" displayName="$(string.TurnOff)" key="Software\Policies\Test" valueName="Off">
      <parentCategory ref="Sample" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
  </policies>
</policyDefinitions>"#;

const SAMPLE_ADML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <resources>
    <stringTable>
      <string id="Sample">Sample</string>
      <string id="TurnOff">Turn the sample off</string>
    </stringTable>
  </resources>
</policyDefinitionResources>"#;

/// Writes the sample templates as the catalog file `test` into `<dir>/imports`.
fn sample_import(dir: &Path) {
    let admx = dir.join("admx");
    std::fs::create_dir_all(admx.join("en-US")).unwrap();
    std::fs::write(admx.join("sample.admx"), SAMPLE_ADMX).unwrap();
    std::fs::write(admx.join("en-US").join("sample.adml"), SAMPLE_ADML).unwrap();
    let templates = admx::read_folder(&admx);
    let file = admx::to_catalog_file(&templates, "test", "Test templates", "", "now");
    assert!(
        file.rules.iter().any(|r| r.id == "admx.sample.turnoff"),
        "{:?}",
        file.skipped
    );
    let imports = dir.join("imports");
    std::fs::create_dir_all(&imports).unwrap();
    file.save(&imports.join("test.wwcatalog")).unwrap();
}

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
    app.apply(vec![Act::Select(Sel::Unknown(id.to_owned()))]);
    app.apply(vec![Act::LoadCatalogs]);
    assert_eq!(app.imports.len(), 1, "{}", app.status);
    assert_eq!(app.settings.loaded_imports, Some(vec!["test".to_owned()]));
    let (unknown, on, _, sel) = state(&app);
    assert_eq!((unknown, on), (0, true), "the option is known and on again");
    assert_eq!(sel, Sel::Rule(id.to_owned()), "the selection follows it");
    app.apply(vec![Act::ToggleImport("test".to_owned())]);
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
    app.apply(vec![Act::Expand(UNKNOWN_GROUP.to_owned(), true)]);
    let mut harness = egui_kittest::Harness::new_eframe(|_| app);
    harness.step();
    let has = |h: &egui_kittest::Harness<'_, App>, label: &str| {
        h.query_all_by_label(label).next().is_some()
    };
    assert!(has(&harness, "Unknown options"));
    assert!(has(&harness, "admx.other.policy"));
    harness.state_mut().apply(vec![Act::Select(Sel::Unknown(
        "admx.other.policy".to_owned(),
    ))]);
    harness.step();
    assert!(has(&harness, "Load catalogs"));
    harness.state_mut().apply(vec![Act::Select(Sel::None)]);
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
    app.apply(vec![Act::NewCatalog]);
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
    app.apply(vec![Act::Save]);
    assert!(app.catalog.rule(&id).is_some(), "{}", app.status);
    app.apply(vec![Act::AskCatalogEditRemove(id.clone())]);
    assert!(app.ask.is_some(), "a removal asks");
    app.apply(vec![Act::AskYes, Act::Save]);
    assert!(app.catalog.rule(&id).is_none());
    std::fs::remove_dir_all(&dir).unwrap();
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
    app.apply(vec![Act::OpenRegistry(
        "HKCU:\\Software\\WinWeaverTest".to_owned(),
        None,
    )]);
    let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
        panic!("the registry tab");
    };
    assert_eq!(d.path, key);
    app.apply(vec![Act::RegistryNew(
        regview::EditKind::NewValue,
        "REG_DWORD",
    )]);
    if let Some(Doc::Registry(d)) = app.docs.get_mut(app.active)
        && let Some(edit) = &mut d.edit
    {
        edit.name = "N".to_owned();
        edit.text = "0x10".to_owned();
    }
    app.apply(vec![Act::RegistrySave]);
    assert_eq!(reg.read(key).unwrap().values[0].data, Data::Dword(16));
    app.apply(vec![Act::AskRegistryDelete]);
    assert!(app.ask.is_some(), "a deletion asks");
    app.apply(vec![Act::AskNo, Act::BookmarkRegistry]);
    assert_eq!(reg.read(key).unwrap().values.len(), 1);
    assert!(
        app.marks
            .items
            .iter()
            .any(|b| b.is_registry() && b.path == key && b.item == "N")
    );
    app.apply(vec![Act::AskRegistryDelete, Act::AskYes]);
    assert!(reg.read(key).unwrap().values.is_empty());
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
    app.apply(vec![Act::ShowIn(id.to_owned(), alias[0].clone())]);
    let Doc::Profile(doc) = &app.docs[0] else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Rule(id.to_owned()));
    assert_eq!(doc.focus.as_ref(), Some(&alias[0]));
    assert!(doc.expanded.contains(&alias[0]), "its tree opens to it");
    app.apply(vec![Act::OpenCatalogAt("again".to_owned(), id.to_owned())]);
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
    app.apply(vec![Act::AskRemoveImport("test".to_owned())]);
    assert!(app.ask.is_some());
    app.apply(vec![Act::AskNo]);
    assert!(dir.join("imports").join("test.wwcatalog").exists());
    assert_eq!(app.imports.len(), 1);
    app.apply(vec![Act::Toggle("install.bypass-tpm".to_owned(), false)]);
    app.apply(vec![Act::AskCloseDiscard(0)]);
    assert!(app.ask.is_some(), "a tab with changes asks");
    app.apply(vec![Act::AskNo]);
    assert_eq!(app.docs.len(), 1);
    app.apply(vec![Act::AskForgetRecent, Act::AskYes]);
    assert!(app.settings.recent.is_empty());
    app.apply(vec![Act::AskRenewToken]);
    assert!(app.ask.is_some());
    app.apply(vec![Act::AskNo]);
    app.apply(vec![
        Act::ToggleImport("copy".to_owned()),
        Act::OpenCatalog("test".to_owned()),
        Act::CompareCatalog(CatalogRef::Import("copy".to_owned())),
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

/// Waits for a read of this PC as the frames do.
fn wait_pc_read(app: &mut App) {
    let ctx = egui::Context::default();
    let started = std::time::Instant::now();
    while app.pc_read.is_some() {
        assert!(started.elapsed() < std::time::Duration::from_secs(60));
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.poll_pc_read(&ctx);
    }
}

/// A read of this PC runs in a thread of its own, one at a time, and its result lands in the
/// tab it started from even when another tab came to the front meanwhile.
#[test]
fn this_pc_is_read_in_the_background() {
    let catalog = Catalog::embedded().unwrap();
    let profile = Profile::from_defaults("Office", &catalog);
    let mut app = App::new(catalog.clone(), Vec::new(), Settings::default());
    app.docs = vec![
        Doc::Profile(Box::new(ProfileDoc::new(profile.clone(), None, &catalog))),
        Doc::Profile(Box::new(ProfileDoc::new(profile, None, &catalog))),
    ];
    app.active = 0;
    app.pc = Some(std::sync::Arc::new(thispc::MemoryMachine::default()));
    app.apply(vec![Act::CheckOnPc]);
    assert!(app.pc_read.is_some());
    app.apply(vec![Act::ReadPc]);
    assert_eq!(
        app.status,
        tr("This PC is being read; wait until the read ends")
    );
    app.active = 1;
    wait_pc_read(&mut app);
    let issues = |app: &App, i: usize| match app.docs.get(i) {
        Some(Doc::Profile(d)) => d.issues.clone(),
        _ => panic!("profile tab {i}"),
    };
    // The machine in memory is no administrator: the features and capabilities are not read.
    let first = issues(&app, 0);
    assert!(
        first.iter().any(|i| i.text == admin_issue().text),
        "{first:?}"
    );
    assert!(issues(&app, 1).is_empty());

    app.apply(vec![Act::ReadPc]);
    wait_pc_read(&mut app);
    assert_eq!(app.docs.len(), 3);
    assert_eq!(app.active, 2);
    let Some(Doc::Profile(read)) = app.docs.get(2) else {
        panic!("the profile read from this PC");
    };
    assert_eq!(read.profile.name, "MEMORY");

    // A tab closed during the read is missed without harm.
    app.active = 0;
    app.apply(vec![Act::CheckOnPc]);
    app.docs.remove(0);
    app.active = 0;
    wait_pc_read(&mut app);
    assert_eq!(
        app.status,
        tr("Check on this PC: the tab of the profile was closed")
    );
    assert_eq!(app.docs.len(), 2);
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
    app.apply(vec![Act::OpenRegistry(key.to_owned(), None)]);
    app.apply(vec![Act::RegistrySearch]);
    assert!(search(&mut app).open);
    // An empty text starts nothing.
    app.apply(vec![Act::RegistryFind]);
    assert!(search(&mut app).running.is_none());
    search(&mut app).text = "BLUE".to_owned();
    app.apply(vec![Act::RegistryFind]);
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
    app.apply(vec![Act::RegistryGo(hit.path, hit.value)]);
    let Some(Doc::Registry(d)) = app.docs.get(app.active) else {
        panic!("the registry tab");
    };
    assert_eq!(d.value.as_deref(), Some("Colour"));
}
