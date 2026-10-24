//! Tests of the window of wedit: whole frames drawn by egui_kittest without a screen, the keys
//! pressed as events, no window opened.

use egui_kittest::kittest::Queryable;
use ww::eframe::egui::{self, Key, Modifiers};
use ww::mdview::MdDoc;
use ww::{Settings, ShellAct, Window};

use crate::app::{Act, Doc, Wedit};
use crate::textdoc::TextDoc;

type Harness<'a> = egui_kittest::Harness<'a, Window<Wedit>>;

fn harness(docs: Vec<Doc>) -> Harness<'static> {
    let mut window = Window::new(Wedit::new(Vec::new()), Settings::default());
    window.shell.docs.clear();
    for doc in docs {
        window.shell.open_tab(doc);
    }
    window.shell.active = 0;
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(900.0, 600.0))
        .build_eframe(|_| window);
    for _ in 0..3 {
        h.step();
    }
    h
}

fn long_text() -> String {
    (1..=300)
        .map(|n| format!("Line {n} of the text\n"))
        .collect()
}

fn text_line(h: &Harness<'_>) -> usize {
    match h.state().shell.active_doc() {
        Some(Doc::Text(d)) => d.line,
        _ => 0,
    }
}

fn press(h: &mut Harness<'_>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.step();
    h.step();
}

#[test]
fn the_page_keys_move_the_cursor_of_the_text() {
    let mut h = harness(vec![Doc::Text(TextDoc::new(long_text(), None))]);
    assert_eq!(text_line(&h), 1);
    press(&mut h, Modifiers::NONE, Key::PageDown);
    let down = text_line(&h);
    assert!(down > 10, "Page Down went to line {down}");
    press(&mut h, Modifiers::NONE, Key::PageDown);
    assert!(text_line(&h) > down, "a second Page Down goes on");
    press(&mut h, Modifiers::SHIFT, Key::PageUp);
    let Some(Doc::Text(doc)) = h.state().shell.active_doc() else {
        panic!("the text tab");
    };
    let state = egui::text_edit::TextEditState::load(&h.ctx, doc.id()).expect("the editor state");
    let range = state.cursor.char_range().expect("a cursor");
    assert_ne!(range.primary, range.secondary, "Shift+Page Up selects");
    press(&mut h, Modifiers::CTRL, Key::Home);
    assert_eq!(text_line(&h), 1, "Ctrl+Home goes to the start");
    press(&mut h, Modifiers::CTRL, Key::End);
    assert!(text_line(&h) >= 300, "Ctrl+End goes to the end");
}

#[test]
fn the_text_has_the_focus_again_when_its_tab_comes_back() {
    let mut h = harness(vec![
        Doc::Text(TextDoc::new(long_text(), None)),
        Doc::Text(TextDoc::new("Other".to_owned(), None)),
    ]);
    let id = match h.state().shell.docs.first() {
        Some(Doc::Text(d)) => d.id(),
        _ => panic!("the text tab"),
    };
    assert!(
        h.ctx.memory(|m| m.has_focus(id)),
        "the editor has the focus at once"
    );
    press(&mut h, Modifiers::NONE, Key::PageDown);
    let line = text_line(&h);
    h.state_mut().apply(vec![Act::Shell(ShellAct::Activate(1))]);
    h.step();
    h.step();
    assert!(!h.ctx.memory(|m| m.has_focus(id)));
    h.state_mut().apply(vec![Act::Shell(ShellAct::Activate(0))]);
    h.step();
    h.step();
    assert!(h.ctx.memory(|m| m.has_focus(id)), "the focus is back");
    assert_eq!(text_line(&h), line, "the cursor stayed where it was");
}

#[test]
fn the_keys_scroll_a_page() {
    let text: String = (1..=200)
        .map(|n| format!("Paragraph {n} of the page.\n\n"))
        .collect();
    let mut h = harness(vec![Doc::Page(MdDoc::new("page.md", None, text))]);
    let offset = |h: &Harness<'_>| match h.state().shell.active_doc() {
        Some(Doc::Page(m)) => m.offset(),
        _ => -1.0,
    };
    assert_eq!(offset(&h), 0.0);
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    assert!(offset(&h) > 0.0, "the arrow scrolls");
    press(&mut h, Modifiers::NONE, Key::PageDown);
    let page = offset(&h);
    assert!(page > 200.0, "Page Down scrolls by a page: {page}");
    press(&mut h, Modifiers::NONE, Key::End);
    assert!(offset(&h) > page, "End goes to the end");
    press(&mut h, Modifiers::NONE, Key::Home);
    assert_eq!(offset(&h), 0.0, "Home goes to the top");
}

#[test]
fn a_tab_with_changes_asks_before_it_closes() {
    let mut doc = TextDoc::new("Changed".to_owned(), None);
    doc.dirty = true;
    let mut h = harness(vec![
        Doc::Text(doc),
        Doc::Text(TextDoc::new(String::new(), None)),
    ]);
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::CloseTab(0, false))]);
    assert_eq!(
        h.state().shell.docs.len(),
        2,
        "nothing closes before the answer"
    );
    let question = h.state().shell.question.as_ref().expect("a question");
    assert!(question.alt.is_some(), "Don't save beside Save");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::Answer(false))]);
    assert_eq!(h.state().shell.docs.len(), 2, "Cancel keeps the tab");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::CloseTab(0, false))]);
    h.state_mut().apply(vec![Act::Shell(ShellAct::AnswerAlt)]);
    assert_eq!(h.state().shell.docs.len(), 1, "Don't save closes it");
    assert!(h.state().shell.question.is_none());
}

#[test]
fn a_web_link_is_asked_first_and_a_text_file_opens_inside() {
    let dir = std::env::temp_dir().join(format!("wedit-links-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let page = dir.join("page.md");
    std::fs::write(&page, "[notes](notes.txt)").unwrap();
    std::fs::write(dir.join("notes.txt"), "The notes").unwrap();
    let mut h = harness(vec![Doc::Page(MdDoc::new(
        "page.md",
        Some(page),
        String::new(),
    ))]);
    h.state_mut().apply(vec![Act::Shell(ShellAct::OpenLink(
        "https://example.com".to_owned(),
    ))]);
    assert!(h.state().shell.question.is_some(), "a web link asks first");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::Answer(false))]);
    assert!(h.state().shell.open_web.is_none(), "no means no");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenLink("notes.txt".to_owned()))]);
    assert!(
        h.state().shell.question.is_none(),
        "a text file needs no question"
    );
    match h.state().shell.active_doc() {
        Some(Doc::Text(t)) => assert_eq!(t.text, "The notes"),
        _ => panic!("the text file opens in a tab"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn about_is_a_window_over_the_tabs() {
    let mut h = harness(vec![Doc::Text(TextDoc::new(String::new(), None))]);
    h.state_mut().apply(vec![Act::Shell(ShellAct::About)]);
    h.step();
    assert!(h.state().shell.about_open);
    assert!(!h.query_all_by_label("OK").collect::<Vec<_>>().is_empty());
    press(&mut h, Modifiers::NONE, Key::Escape);
    assert!(!h.state().shell.about_open, "Escape closes it");
}
