//! This PC read in the background.

use super::*;

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
    app.apply(vec![Act::Pc(PcAct::Check)]);
    assert!(app.pc_read.is_some());
    app.apply(vec![Act::Pc(PcAct::Read)]);
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

    app.apply(vec![Act::Pc(PcAct::Read)]);
    wait_pc_read(&mut app);
    assert_eq!(app.docs.len(), 3);
    assert_eq!(app.active, 2);
    let Some(Doc::Profile(read)) = app.docs.get(2) else {
        panic!("the profile read from this PC");
    };
    assert_eq!(read.profile.name, "MEMORY");

    // A tab closed during the read is missed without harm.
    app.active = 0;
    app.apply(vec![Act::Pc(PcAct::Check)]);
    app.docs.remove(0);
    app.active = 0;
    wait_pc_read(&mut app);
    assert_eq!(
        app.status,
        tr("Check on this PC: the tab of the profile was closed")
    );
    assert_eq!(app.docs.len(), 2);
}
