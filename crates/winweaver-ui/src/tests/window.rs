//! The window: the tabs, the frame of every kind of tab, the screenshots, the themes, the history, the links.

use super::*;
use crate::ShellAct;
use crate::ShellDoc;

#[test]
fn app_starts_with_the_office_preset() {
    let catalog = Catalog::embedded().unwrap();
    let app = Window::new(
        WinWeaver::new(catalog, CatalogTexts::embedded().unwrap()),
        Settings::default(),
    );
    assert_eq!(app.shell.docs.len(), 1);
    assert_eq!(app.shell.docs[0].title(), "Office.wwprofile");
    assert!(!app.shell.docs[0].dirty());
    assert_eq!(app.shell.window_title(), "Office.wwprofile - WinWeaver");
}

#[test]
fn the_guide_opens_once_and_markdown_files_open() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![
        Act::File(FileAct::OpenGuide),
        Act::File(FileAct::OpenGuide),
    ]);
    assert_eq!(app.shell.docs.len(), 2, "the guide tab exists once");
    assert_eq!(app.shell.docs[1].title(), "Guide.md");
    let dir = std::env::temp_dir().join(format!("winweaver-md-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.md");
    std::fs::write(&path, "# Notes\n\nA line.\n").unwrap();
    app.app.open_any(&mut app.shell, path.clone());
    assert_eq!(app.shell.docs.len(), 3);
    assert!(matches!(&app.shell.docs[2], Doc::Markdown(m) if m.text.starts_with("# Notes")));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Draws frames of the whole window without a screen (egui_kittest): every kind of tab is
/// drawn and shows its texts, and a click on a rule of the tree selects it.
#[test]
fn every_kind_of_tab_draws_without_a_screen() {
    use egui_kittest::kittest::Queryable;
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(
        WinWeaver::new(catalog, CatalogTexts::embedded().unwrap()),
        Settings::default(),
    );
    let empty = admx::to_catalog_file(
        &admx::Templates::default(),
        "empty",
        "Empty templates",
        "",
        "now",
    );
    app.app.imports.push(empty);
    app.app.rebuild_catalog(&mut app.shell);
    let mut harness = egui_kittest::Harness::new_eframe(|_| app);
    harness.step();
    let has = |h: &egui_kittest::Harness<'_, Window<WinWeaver>>, label: &str| {
        h.query_all_by_label(label).next().is_some()
    };
    assert!(has(&harness, "Menu"));
    for (act, label) in [
        (Act::Shell(ShellAct::OpenSettings), Some("General")),
        (
            Act::Catalog(CatalogAct::Open("empty".to_owned())),
            Some("Select a policy in the tree."),
        ),
        (Act::Marks(MarksAct::Open), None),
        (Act::Mcp(McpAct::Open), None),
        (Act::File(FileAct::OpenGuide), None),
        (Act::File(FileAct::OpenDocs), Some("README.md")),
        (
            Act::Shell(ShellAct::EditTheme("matrix".to_owned())),
            Some("Use this theme"),
        ),
        (Act::Shell(ShellAct::Activate(0)), None),
    ] {
        harness.state_mut().apply(vec![act]);
        harness.step();
        harness.step();
        if let Some(label) = label {
            assert!(has(&harness, label), "{label} is not drawn");
        }
    }
    assert_eq!(harness.state().shell.docs.len(), 8);
    assert_eq!(
        harness.state().shell.docs[harness.state().shell.active].title(),
        "Office.wwprofile"
    );
    harness.state_mut().apply(vec![
        Act::Profile(ProfileAct::Expand("unattend".to_owned(), true)),
        Act::Profile(ProfileAct::Expand("install".to_owned(), true)),
    ]);
    harness.step();
    harness.step();
    let title = "Bypass the TPM 2.0 check";
    assert!(has(&harness, title), "the rules of an open group are drawn");
    harness.query_all_by_label(title).next().unwrap().click();
    harness.step();
    harness.step();
    assert!(
        matches!(harness.state().shell.docs[0], Doc::Profile(ref d) if d.sel == Sel::Rule("install.bypass-tpm".into())),
        "a click on a rule selects it"
    );
    harness
        .state_mut()
        .apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    harness.step();
    harness.step();
    assert!(has(&harness, "Select a difference to see it here."));
    let key = first_rule_key(harness.state()).expect("Strict differs in a rule");
    harness
        .state_mut()
        .apply(vec![Act::Diff(DiffAct::Select(key))]);
    harness.step();
    harness.step();
    assert!(
        has(&harness, "Apply this difference"),
        "the item is shown on the right"
    );
}

/// The divider between the differences and the item on the right stays where it is whatever
/// item is selected: the item never widens the list or pushes into it.
#[test]
fn the_divider_of_a_diff_stays_put_whatever_item_is_shown() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(
        WinWeaver::new(catalog, CatalogTexts::embedded().unwrap()),
        Settings::default(),
    );
    app.apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    let keys = match app.shell.docs.last() {
        Some(Doc::Diff(d)) => d.diff.keys(),
        _ => Vec::new(),
    };
    assert!(keys.len() > 3, "{keys:?}");
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| app);
    let list = egui::Id::new(("diff-list", "Office", "Strict"));
    let width = |h: &egui_kittest::Harness<'_, Window<WinWeaver>>| {
        h.ctx.read_response(list).map(|r| r.rect.width())
    };
    harness.step();
    harness.step();
    let first = width(&harness).expect("the list is drawn");
    for key in keys {
        harness
            .state_mut()
            .apply(vec![Act::Diff(DiffAct::Select(key.clone()))]);
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
    let app = Window::new(
        WinWeaver::new(catalog, CatalogTexts::embedded().unwrap()),
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
    let shot = |h: &mut egui_kittest::Harness<'_, Window<WinWeaver>>, name: &str| {
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
        Act::Profile(ProfileAct::Expand("policies".to_owned(), true)),
        Act::Profile(ProfileAct::Expand("defender".to_owned(), true)),
        Act::Profile(ProfileAct::Select(Sel::Rule("defender.pua".to_owned()))),
    ]);
    shot(&mut harness, "02-rule");
    harness.state_mut().apply(vec![
        Act::Marks(MarksAct::Toggle),
        Act::Marks(MarksAct::SetComment(
            0,
            "Ask the customer about the Audit mode".into(),
        )),
        Act::Profile(ProfileAct::Check),
    ]);
    shot(&mut harness, "03-checked");
    harness
        .state_mut()
        .apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    if let Some(key) = first_rule_key(harness.state()) {
        harness
            .state_mut()
            .apply(vec![Act::Diff(DiffAct::Select(key))]);
    }
    shot(&mut harness, "04-diff");
    for (act, name) in [
        (Act::Shell(ShellAct::OpenSettings), "05-settings"),
        (Act::Mcp(McpAct::Open), "06-mcp"),
        (Act::Marks(MarksAct::Open), "07-bookmarks"),
        (Act::File(FileAct::OpenGuide), "08-guide"),
    ] {
        harness.state_mut().apply(vec![act]);
        if name == "07-bookmarks" {
            harness
                .state_mut()
                .apply(vec![Act::Marks(MarksAct::Select(0))]);
        }
        shot(&mut harness, name);
    }
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::EditTheme("latte".to_owned()))]);
    shot(&mut harness, "10-theme-editor");
    harness.state_mut().apply(vec![
        Act::Shell(ShellAct::UseTheme("matrix".to_owned())),
        Act::Shell(ShellAct::Activate(0)),
    ]);
    shot(&mut harness, "11-matrix");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::UseTheme("light".to_owned()))]);
    // A registry file of the plan of the first preset, at its first key.
    let catalog = Catalog::embedded().unwrap();
    let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0).unwrap();
    let ids: Vec<String> = catalog.rules().iter().map(|r| r.id.clone()).collect();
    let plan = winweaver_core::apply::plan_apply(&catalog, &profile, &ids, "shot");
    let reg = dir.join("Apply-Office.reg");
    let file = RegFile::from_plan(&plan, "Apply the profile Office");
    file.save(&reg).unwrap();
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(reg))]);
    if let Some(Doc::RegFile(d)) = harness.state_mut().shell.docs.last_mut() {
        let key = |l: &Line| matches!(l, Line::Key { .. });
        d.at = d.file.lines.iter().position(key);
    }
    shot(&mut harness, "12-regfile");
    // Pages: the sample at several widths and settings, and AGENTS.md at its status table.
    let sample = dir.join("sample.md");
    std::fs::write(&sample, ww::sample::PAGE).unwrap();
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(sample))]);
    shot(&mut harness, "13-page");
    harness.state_mut().shell.settings.wrap_lines = false;
    shot(&mut harness, "14-page-nowrap");
    harness.state_mut().shell.settings.wrap_lines = true;
    harness.state_mut().shell.settings.page_width = ww::PageWidth::Characters(80);
    shot(&mut harness, "15-page-80-chars");
    harness.state_mut().shell.settings.page_width = ww::PageWidth::Pixels(700);
    shot(&mut harness, "16-page-700-px");
    // A limit wider than the window: the page follows the window.
    harness.state_mut().shell.settings.page_width = ww::PageWidth::Pixels(1600);
    harness.set_size(egui::vec2(900.0, 900.0));
    shot(&mut harness, "16b-page-1600-px-narrow-window");
    harness.set_size(egui::vec2(1440.0, 900.0));
    harness.state_mut().shell.settings.page_width = ww::PageWidth::View;
    harness.set_size(egui::vec2(900.0, 900.0));
    shot(&mut harness, "17-page-narrow");
    harness.set_size(egui::vec2(1440.0, 900.0));
    let agents = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../AGENTS.md");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(agents))]);
    if let Some(Doc::Markdown(m)) = harness.state_mut().shell.docs.last_mut() {
        m.anchor = Some("7-work-status".to_owned());
        m.jump = true;
    }
    shot(&mut harness, "18-agents-status");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::UseTheme("dark".to_owned()))]);
    shot(&mut harness, "19-agents-dark");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::UseTheme("light".to_owned()))]);
    harness.set_size(egui::vec2(1000.0, 700.0));
    shot(&mut harness, "09-compact");
    harness.set_size(egui::vec2(1440.0, 900.0));
    harness.state_mut().apply(vec![Act::Shell(ShellAct::About)]);
    shot(&mut harness, "20-about");
}

#[test]
fn a_theme_tab_previews_its_theme_and_a_theme_can_be_chosen() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    let (light, _, preference) = app.shell.wanted_theme();
    assert_eq!(light.id, "light");
    assert_eq!(preference, egui::ThemePreference::System);
    app.apply(vec![Act::Shell(ShellAct::EditTheme("matrix".into()))]);
    assert!(matches!(&app.shell.docs[1], Doc::Shell(ShellDoc::Theme(t)) if t.theme.id == "matrix"));
    assert_eq!(app.shell.docs[1].title(), "matrix.wwtheme");
    if let Some(Doc::Shell(ShellDoc::Theme(t))) = app.shell.docs.get_mut(1) {
        t.theme.font_size = 16.0;
    }
    let (_, dark, preference) = app.shell.wanted_theme();
    assert_eq!(
        dark.id, "matrix",
        "the edited theme shows while its tab is in front"
    );
    assert_eq!(dark.font_size, 16.0);
    assert_eq!(preference, egui::ThemePreference::Dark);
    app.apply(vec![Act::Shell(ShellAct::EditTheme("matrix".into()))]);
    assert_eq!(app.shell.docs.len(), 2, "one tab per theme");
    app.apply(vec![Act::Shell(ShellAct::Activate(0))]);
    assert_eq!(
        app.shell.wanted_theme().0.id,
        "light",
        "the settings theme again"
    );
    app.apply(vec![Act::Shell(ShellAct::UseTheme("latte".into()))]);
    assert_eq!(app.shell.settings.custom_theme, "latte");
    assert_eq!(app.shell.settings.theme, ThemeChoice::Light);
    let (light, _, preference) = app.shell.wanted_theme();
    assert_eq!(light.name, "Latte");
    assert_eq!(preference, egui::ThemePreference::Light);
    assert_eq!(app.shell.shown_theme_id(true), "latte");
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
fn tabs_move_and_the_tab_in_front_stays_in_front() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![
        Act::File(FileAct::OpenGuide),
        Act::Marks(MarksAct::Open),
        Act::Shell(ShellAct::OpenSettings),
    ]);
    let titles =
        |app: &Window<WinWeaver>| app.shell.docs.iter().map(Doc::title).collect::<Vec<_>>();
    let start = titles(&app);
    assert_eq!(start.len(), 4);
    assert_eq!(app.shell.active, 3);
    app.apply(vec![Act::Shell(ShellAct::MoveTab(3, 0))]);
    assert_eq!(app.shell.active, 0, "the moved tab stays in front");
    assert_eq!(titles(&app)[0], start[3]);
    assert_eq!(titles(&app)[1..], start[..3]);
    app.apply(vec![
        Act::Shell(ShellAct::Activate(2)),
        Act::Shell(ShellAct::MoveTab(0, 3)),
    ]);
    assert_eq!(
        app.shell.active, 1,
        "the tab in front shifts left when a tab before it moves after it"
    );
    assert_eq!(titles(&app), start);
    app.apply(vec![
        Act::Shell(ShellAct::MoveTab(1, 1)),
        Act::Shell(ShellAct::MoveTab(0, 9)),
    ]);
    assert_eq!(titles(&app), start, "nothing moves out of the row");
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
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    if let Some(Doc::Profile(doc)) = app.shell.docs.get_mut(0) {
        doc.expanded.clear();
    }
    app.apply(vec![Act::Profile(ProfileAct::Reveal(Sel::Group(
        group.clone(),
    )))]);
    let Some(Doc::Profile(doc)) = app.shell.docs.first() else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Group(group));
    assert!(doc.expanded.contains(&parent), "the tree opens to it");
    assert!(doc.reveal);
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
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![
        Act::Marks(MarksAct::Open),
        Act::Profile(ProfileAct::ShowGroupInProfile(group.clone())),
    ]);
    assert_eq!(app.shell.active, 0, "the profile tab comes to the front");
    let Some(Doc::Profile(doc)) = app.shell.docs.first() else {
        panic!("a profile");
    };
    assert_eq!(doc.sel, Sel::Group(group));
    assert!(doc.reveal);
}
