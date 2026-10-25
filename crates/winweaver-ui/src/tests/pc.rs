//! This PC read in the background.

use super::*;

/// A read of this PC runs in a thread of its own, one at a time, and its result lands in the
/// tab it started from even when another tab came to the front meanwhile.
#[test]
fn this_pc_is_read_in_the_background() {
    let catalog = Catalog::embedded().unwrap();
    let profile = Profile::from_defaults("Office", &catalog);
    let mut app = Window::new(
        WinWeaver::new(catalog.clone(), Vec::new()),
        Settings::default(),
    );
    app.shell.docs = vec![
        Doc::Profile(Box::new(ProfileDoc::new(profile.clone(), None, &catalog))),
        Doc::Profile(Box::new(ProfileDoc::new(profile, None, &catalog))),
    ];
    app.shell.active = 0;
    app.app.full.pc = Some(std::sync::Arc::new(thispc::MemoryMachine::default()));
    app.apply(vec![Act::Pc(PcAct::Check)]);
    assert!(app.app.full.pc_read.is_some());
    app.apply(vec![Act::Pc(PcAct::Read)]);
    assert_eq!(
        app.shell.status,
        tr("This PC is being read; wait until the read ends")
    );
    app.shell.active = 1;
    wait_pc_read(&mut app);
    let issues = |app: &Window<WinWeaver>, i: usize| match app.shell.docs.get(i) {
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

    app.apply(vec![Act::Pc(PcAct::Read)]);
    wait_pc_read(&mut app);
    assert_eq!(app.shell.docs.len(), 3);
    assert_eq!(app.shell.active, 2);
    let Some(Doc::Profile(read)) = app.shell.docs.get(2) else {
        panic!("the profile read from this PC");
    };
    assert_eq!(read.profile.name, "MEMORY");

    // A tab closed during the read is missed without harm.
    app.shell.active = 0;
    app.apply(vec![Act::Pc(PcAct::Check)]);
    app.shell.docs.remove(0);
    app.shell.active = 0;
    wait_pc_read(&mut app);
    assert_eq!(
        app.shell.status,
        tr("Check on this PC: the tab of the profile was closed")
    );
    assert_eq!(app.shell.docs.len(), 2);
}
