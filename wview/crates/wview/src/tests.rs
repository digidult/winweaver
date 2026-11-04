//! The tests of wview: its translations and its guides, a file in the tab of its plugin, the keys
//! and the mouse of an image.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wwrt::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use wwrt::egui_kittest::Harness;
use wwrt::plugin::Plugins;
use wwrt::testing;
use wwrt::ww::{Settings, ShellAct, Window};

use crate::app::{Act, Doc, Extra, GUIDE, GUIDES, LANGUAGES, Wview};

#[test]
fn every_text_of_the_window_is_translated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut wanted = testing::texts(&root.join("src"));
    // W# in the repository, or in the tree of a release (wview/sdk/pack.py).
    let framework = ["../../../crates/ww/src", "../../framework/ww/src"]
        .map(|p| root.join(p))
        .into_iter()
        .find(|p| p.is_dir())
        .expect("the sources of W#");
    wanted.extend(testing::texts(&framework));
    // The texts the shell of W# translates without a literal of its own: the hint of New, the
    // hint of the search of a tab, the name of the section of the settings.
    wanted.extend(
        [
            "Open a file (Ctrl+O)",
            "Search the text (Ctrl+F)",
            "Plugins",
        ]
        .map(String::from),
    );
    let missing = testing::missing(&LANGUAGES, &wanted);
    assert!(missing.is_empty(), "{missing:#?}");
}

#[test]
fn every_language_has_its_guide() {
    assert!(GUIDE.starts_with("# wview\n"));
    let codes: Vec<&str> = LANGUAGES.iter().map(|(c, _)| *c).collect();
    let guides: Vec<&str> = GUIDES.iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, guides);
    for (code, text) in GUIDES {
        assert!(text.starts_with("# wview\n"), "{code}");
        assert_eq!(
            text.matches("\n## ").count(),
            GUIDE.matches("\n## ").count(),
            "{code}: the sections of the guide"
        );
    }
}

/// A folder of its own for a test.
pub(crate) fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wview-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A BMP of 24 bits, the colour of each pixel by `pixel`.
pub(crate) fn bmp(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let stride = (w * 3 + 3) & !3;
    let size = stride * h;
    let mut out = Vec::with_capacity(54 + size as usize);
    out.extend_from_slice(b"BM");
    for v in [54 + size, 0, 54, 40] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    for v in [0u32, size, 2835, 2835, 0, 0] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for y in (0..h).rev() {
        let start = out.len();
        for x in 0..w {
            let [r, g, b] = pixel(x, y);
            out.extend_from_slice(&[b, g, r]);
        }
        out.resize(start + stride as usize, 0);
    }
    out
}

/// The plugins of the repository, taken as libraries.
pub(crate) fn plugins() -> Arc<Plugins> {
    let mut plugins = Plugins::default();
    plugins.add(
        PathBuf::from("wv_image.dll"),
        Box::new(wv_image::ImagePlugin),
    );
    plugins.add(PathBuf::from("wv_text.dll"), Box::new(wv_text::TextPlugin));
    Arc::new(plugins)
}

pub(crate) fn window(settings: Settings<Extra>) -> Window<Wview> {
    Window::new(Wview::new(plugins(), Vec::new()), settings)
}

fn open(w: &mut Window<Wview>, path: PathBuf) {
    w.apply(vec![Act::Shell(ShellAct::OpenPath(path))]);
}

/// The file of the tab in front and the status line of its view.
fn front(w: &Window<Wview>) -> (PathBuf, String) {
    match w.shell.active_doc() {
        Some(Doc::File(d)) => (d.path.clone(), d.view.status()),
        _ => (PathBuf::new(), String::new()),
    }
}

fn click(h: &Harness<'_, Window<Wview>>, at: Pos2) {
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
}

#[test]
fn a_file_opens_in_the_tab_of_its_plugin() {
    let dir = folder("open");
    std::fs::write(dir.join("a.md"), "# A page\n\nSome text.\n").unwrap();
    std::fs::write(dir.join("b.cfg"), "x = 1\n").unwrap();
    std::fs::write(dir.join("c.xyz"), "?").unwrap();
    let mut w = window(Settings::default());
    open(&mut w, dir.join("a.md"));
    open(&mut w, dir.join("b.cfg"));
    assert_eq!(w.shell.docs.len(), 2);
    assert!(front(&w).1.contains('1'), "{}", front(&w).1);
    open(&mut w, dir.join("c.xyz"));
    assert_eq!(w.shell.docs.len(), 2, "a file no plugin opens stays closed");
    assert!(w.shell.status.contains("c.xyz"), "{}", w.shell.status);
    open(&mut w, dir.join("a.md"));
    assert_eq!(
        w.shell.docs.len(),
        2,
        "a file open already comes to the front"
    );
    assert_eq!(front(&w).0, dir.join("a.md"));
}

#[test]
fn the_keys_and_the_wheel_of_an_image() {
    let dir = folder("images");
    for (i, name) in ["i1.bmp", "i2.bmp", "i3.bmp"].iter().enumerate() {
        let picture = bmp(64, 48, |x, y| [x as u8 * 4, y as u8 * 5, 80 * i as u8]);
        std::fs::write(dir.join(name), picture).unwrap();
    }
    std::fs::write(dir.join("notes.txt"), "not an image").unwrap();
    let mut window = window(Settings::default());
    open(&mut window, dir.join("i1.bmp"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_eframe(|_| window);
    h.run_steps(3);
    let centre = Pos2::new(400.0, 320.0);
    click(&h, centre);
    h.run_steps(2);
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    assert_eq!(
        front(h.state()).0,
        dir.join("i2.bmp"),
        "the arrow takes the next image"
    );
    h.key_press(Key::End);
    h.run_steps(2);
    assert_eq!(
        front(h.state()).0,
        dir.join("i3.bmp"),
        "End the last; the text is no image"
    );
    h.key_press(Key::Home);
    h.run_steps(2);
    assert_eq!(
        front(h.state()).0,
        dir.join("i1.bmp"),
        "the focus stays after a step"
    );
    h.key_press(Key::Num1);
    h.run_steps(2);
    let actual = front(h.state()).1;
    assert!(actual.contains("100"), "{actual}");
    h.hover_at(centre);
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(12);
    let zoomed = front(h.state()).1;
    assert!(!zoomed.contains("100"), "the wheel zooms in: {zoomed}");
}
