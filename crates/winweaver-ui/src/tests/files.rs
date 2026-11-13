//! The files: profiles, drafts, bookmarks, diffs, registry files.

use super::*;
use crate::ShellAct;

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
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    let dir = std::env::temp_dir().join(format!("winweaver-marks-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    app.app.full.marks_path = dir.join(bookmarks::DEFAULT_FILE);
    app.apply(vec![
        Act::Profile(ProfileAct::Select(Sel::Rule("install.bypass-tpm".into()))),
        Act::Marks(MarksAct::Toggle),
    ]);
    assert_eq!(app.app.full.marks.items.len(), 1, "{}", app.shell.status);
    assert_eq!(app.app.full.marks.items[0].item, "install.bypass-tpm");
    assert_eq!(
        app.app.full.marks.items[0].path, "",
        "an unsaved profile has no file to link"
    );
    assert_eq!(
        app.app.full.marks.items[0].title, "",
        "the title comes from the catalog"
    );
    assert!(
        app.app.full.marks_path.exists(),
        "the default file is created at the first bookmark"
    );
    app.apply(vec![
        Act::Profile(ProfileAct::Select(Sel::None)),
        Act::Marks(MarksAct::Open),
        Act::Marks(MarksAct::GoTo(0)),
    ]);
    assert_eq!(app.shell.active, 0, "{}", app.shell.status);
    assert_eq!(
        app.app.active_profile(&mut app.shell).unwrap().sel,
        Sel::Rule("install.bypass-tpm".into())
    );
    app.apply(vec![Act::Marks(MarksAct::Toggle)]);
    assert!(app.app.full.marks.items.is_empty());
    app.apply(vec![
        Act::Marks(MarksAct::Toggle),
        Act::Marks(MarksAct::SetComment(
            0,
            "Ask the customer\nabout the TPM".into(),
        )),
        Act::Marks(MarksAct::Toggle),
    ]);
    assert_eq!(
        app.app.full.marks.items.len(),
        1,
        "a bookmark with a comment asks first"
    );
    assert!(app.shell.question.is_some());
    app.apply(vec![Act::Shell(ShellAct::Answer(false))]);
    assert_eq!(app.app.full.marks.items.len(), 1);
    app.apply(vec![
        Act::Marks(MarksAct::Open),
        Act::Marks(MarksAct::Select(0)),
    ]);
    let question = marksview::remove_question(
        &app.app.catalog,
        None,
        &app.app.imports,
        &app.app.full.marks,
        0,
    )
    .expect("the bookmark is there");
    app.apply(vec![question.into()]);
    assert!(app.shell.question.is_some());
    app.apply(vec![Act::Shell(ShellAct::Answer(true))]);
    assert!(app.app.full.marks.items.is_empty());
    assert_eq!(app.app.marks_doc(&mut app.shell).and_then(|m| m.sel), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_registry_file_and_a_diff_come_back_from_their_drafts() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-drafts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = Window::new(
        WinWeaver::new(catalog.clone(), Vec::new()),
        Settings::default(),
    );
    app.shell.drafts_dir = dir.join("drafts");
    let reg = dir.join("x.reg");
    let text = "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Software\\X]\r\n\"N\"=dword:00000001\r\n";
    std::fs::write(&reg, text).unwrap();
    app.apply(vec![Act::Shell(ShellAct::OpenPath(reg.clone()))]);
    if let Some(Doc::RegFile(d)) = app.shell.docs.last_mut() {
        d.file.lines.push(Line::Comment("; a change".to_owned()));
        d.dirty = true;
    }
    let base = Profile::new("Base", catalog.version());
    let mut other = Profile::new("Other", catalog.version());
    other.rules.insert(
        "install.bypass-tpm".to_owned(),
        winweaver_core::profile::Choice {
            on: true,
            ..Default::default()
        },
    );
    let diff = Diff::between(
        &base,
        &other,
        diff::Side::of(&base, None),
        diff::Side::of(&other, None),
    );
    let mut d = DiffDoc::new(diff, None);
    d.dirty = true;
    app.shell.open_tab(Doc::Diff(Box::new(d)));
    app.shell.write_drafts(true);
    let (drafts, _) = session::read_drafts(&app.shell.drafts_dir);
    assert_eq!(drafts.len(), 2, "a draft of each");
    let mut next = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    next.shell.drafts_dir = app.shell.drafts_dir.clone();
    next.restore_session();
    let file = next.shell.docs.iter().find_map(|d| match d {
        Doc::RegFile(f) => Some(f),
        _ => None,
    });
    let file = file.expect("the registry file is back");
    assert!(file.dirty);
    assert!(
        file.file
            .lines
            .iter()
            .any(|l| l == &Line::Comment("; a change".to_owned()))
    );
    assert!(
        next.shell
            .docs
            .iter()
            .any(|d| matches!(d, Doc::Diff(x) if x.dirty && x.diff.rules.len() == 1)),
        "the diff is back"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn drafts_come_back_after_a_restart() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = Window::new(
        WinWeaver::new(catalog.clone(), Vec::new()),
        Settings::default(),
    );
    app.shell.drafts_dir = dir.join("drafts");
    app.shell.logs_dir = dir.join("logs");
    app.apply(vec![Act::Profile(ProfileAct::Toggle(
        "install.bypass-tpm".into(),
        false,
    ))]);
    app.shell.write_drafts(true);
    let (drafts, _) = session::read_drafts(&app.shell.drafts_dir);
    assert_eq!(drafts.len(), 1);
    let mut next = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    next.shell.drafts_dir = app.shell.drafts_dir.clone();
    next.shell.logs_dir = app.shell.logs_dir.clone();
    next.restore_session();
    assert_eq!(
        next.shell.docs.len(),
        1,
        "the untouched start tab gives way to the draft"
    );
    let Doc::Profile(doc) = &next.shell.docs[0] else {
        panic!("a profile");
    };
    assert!(doc.dirty && !doc.profile.is_on("install.bypass-tpm"));
    next.apply(vec![Act::Shell(ShellAct::CloseTab(0, true))]);
    assert!(
        session::read_drafts(&next.shell.drafts_dir).0.is_empty(),
        "discarding deletes the draft"
    );
    next.shell.log_status();
    assert!(dir.join("logs").is_dir());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn comparing_with_a_preset_opens_a_diff_tab_that_applies() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    assert_eq!(app.shell.docs.len(), 2);
    assert!(matches!(&app.shell.docs[1], Doc::Diff(d) if !d.diff.is_empty()));
    assert_eq!(app.shell.docs[1].title(), "Office vs Strict.wwdiff");
    app.apply(vec![Act::Diff(DiffAct::Apply)]);
    assert_eq!(app.shell.active, 0);
    let Doc::Profile(office) = &app.shell.docs[0] else {
        panic!("the first tab is the profile");
    };
    assert!(office.dirty);
    let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
    assert_eq!(office.profile.rules, strict.rules);
}

#[test]
fn an_item_of_a_diff_takes_a_comment_applies_alone_and_leads_to_both_profiles() {
    let catalog = Catalog::embedded().unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![Act::Diff(DiffAct::CompareWith("Strict"))]);
    let Doc::Diff(d) = &app.shell.docs[1] else {
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
    let Doc::Diff(d) = &app.shell.docs[1] else {
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
    assert!(matches!(&app.shell.docs[1], Doc::Diff(d) if d.diff.comments.is_empty()));
    app.apply(vec![Act::Diff(DiffAct::ApplyItem(key.clone()))]);
    assert_eq!(app.shell.active, 1, "the diff stays in front");
    let strict = Profile::parse(PRESETS[1].1, "Strict").unwrap();
    let Doc::Profile(office) = &app.shell.docs[0] else {
        panic!("the first tab is the profile");
    };
    assert_eq!(office.profile.rules.get(&id), strict.rules.get(&id));
    app.apply(vec![Act::Diff(DiffAct::GoTo(key.clone(), true))]);
    assert_eq!(
        app.shell.docs.len(),
        3,
        "the preset Strict opens in a new tab"
    );
    assert_eq!(app.shell.active, 2);
    assert_eq!(
        app.app.active_profile(&mut app.shell).unwrap().sel,
        Sel::Rule(id.clone())
    );
    app.shell.active = 1;
    app.apply(vec![Act::Diff(DiffAct::GoTo(key, false))]);
    assert_eq!(app.shell.active, 0);
    assert_eq!(
        app.app.active_profile(&mut app.shell).unwrap().sel,
        Sel::Rule(id)
    );
}

#[test]
fn opening_a_file_adds_a_tab_and_remembers_it() {
    let catalog = Catalog::embedded().unwrap();
    let dir = std::env::temp_dir().join(format!("winweaver-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("strict.wwprofile");
    std::fs::write(&path, PRESETS[1].1).unwrap();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.app.open_any(&mut app.shell, path.clone());
    assert_eq!(app.shell.docs.len(), 2);
    assert_eq!(app.shell.active, 1);
    assert_eq!(app.shell.settings.recent, vec![path.clone()]);
    app.app.open_any(&mut app.shell, path.clone());
    assert_eq!(app.shell.docs.len(), 2, "the same file activates its tab");
    app.app
        .open_any(&mut app.shell, dir.join("missing.wwprofile"));
    assert!(
        app.shell.status.starts_with("Open failed"),
        "{}",
        app.shell.status
    );
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
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![Act::Shell(ShellAct::OpenPath(path.clone()))]);
    let Some(Doc::RegFile(d)) = app.shell.docs.get_mut(app.shell.active) else {
        panic!("the file has no tab: {}", app.shell.status);
    };
    // A new value goes after the last value of the selected key.
    d.at = Some(2);
    d.start_new(regfileview::EditKind::Value);
    let edit = d.edit.as_mut().unwrap();
    edit.name = "B".to_owned();
    edit.text = "text".to_owned();
    assert!(d.apply_edit());
    assert!(app.shell.docs[app.shell.active].dirty());
    app.apply(vec![Act::Shell(ShellAct::Save)]);
    assert!(
        !app.shell.docs[app.shell.active].dirty(),
        "{}",
        app.shell.status
    );
    let saved = RegFile::load(&path).unwrap().to_text();
    assert!(saved.contains("; kept"), "{saved}");
    assert!(saved.contains("\"B\"=\"text\""), "{saved}");
    // Removing the key asks first; its comment stays.
    if let Some(Doc::RegFile(d)) = app.shell.docs.get_mut(app.shell.active) {
        d.line = None;
    }
    let Some(Doc::RegFile(d)) = app.shell.docs.get(app.shell.active) else {
        panic!("the tab of the file is gone");
    };
    let question = d.remove_question().expect("the key is removable");
    app.apply(vec![question.into()]);
    assert!(app.shell.question.is_some());
    app.apply(vec![Act::Shell(ShellAct::Answer(true))]);
    let Some(Doc::RegFile(d)) = app.shell.docs.get(app.shell.active) else {
        panic!("the tab of the file is gone");
    };
    let text = d.file.to_text();
    assert!(text.contains("; kept"), "{text}");
    assert!(!text.contains("WinWeaver]"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The selected option of a profile goes into a new registry file and into one open in a tab,
/// with its id, its title and its description in comments, after the lines the file had.
#[test]
fn an_option_of_a_profile_goes_into_a_registry_file() {
    use winweaver_core::catalog::Action;
    use winweaver_core::regfile::{Line, RegFile};
    let catalog = Catalog::embedded().unwrap();
    let rule = catalog
        .rules()
        .iter()
        .find(|r| {
            r.params.is_empty()
                && !r.actions.is_empty()
                && r.actions
                    .iter()
                    .all(|a| matches!(a, Action::Reg { path, .. } if path.starts_with("HKLM:")))
        })
        .unwrap()
        .clone();
    let mut app = Window::new(WinWeaver::new(catalog, Vec::new()), Settings::default());
    app.apply(vec![Act::File(FileAct::NewProfile(None))]);
    let profile_tab = app.shell.active;
    if let Some(Doc::Profile(doc)) = app.shell.docs.get_mut(profile_tab) {
        doc.sel = Sel::Rule(rule.id.clone());
    }
    app.apply(vec![Act::RegFile(RegFileAct::AddRules(None))]);
    let Some(Doc::RegFile(new)) = app.shell.docs.get(app.shell.active) else {
        panic!("a new registry file: {}", app.shell.status);
    };
    assert!(new.dirty);
    let comment = |file: &RegFile, text: &str| {
        file.lines
            .iter()
            .any(|l| matches!(l, Line::Comment(c) if c == &format!("; {text}")))
    };
    assert!(comment(&new.file, &format!("Rule {}", rule.id)));
    assert!(comment(&new.file, &rule.title), "the title of {}", rule.id);
    // A registry file open in a tab gets the lines after its own.
    let open =
        RegFile::parse("Windows Registry Editor Version 5.00\r\n\r\n; mine\r\n\r\n").unwrap();
    let before = open.lines.len();
    let doc = regfileview::RegFileDoc::new(open, std::path::PathBuf::from("mine.reg"));
    app.shell.open_tab(Doc::RegFile(Box::new(doc)));
    let target = app.shell.active;
    app.shell.active = profile_tab;
    app.apply(vec![Act::RegFile(RegFileAct::AddRules(Some(target)))]);
    assert_eq!(app.shell.active, target, "the file comes to the front");
    let Some(Doc::RegFile(d)) = app.shell.docs.get(target) else {
        panic!("the registry file");
    };
    assert!(d.dirty);
    assert!(d.file.lines.len() > before);
    assert!(comment(&d.file, "mine"), "its own lines stay");
    assert!(comment(&d.file, &rule.title));
    let values = d
        .file
        .lines
        .iter()
        .filter(|l| matches!(l, Line::Value { .. }))
        .count();
    assert!(values > 0);
    assert!(matches!(d.file.lines.last(), Some(Line::Blank)));
}
