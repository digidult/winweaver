//! The tests of wview: its translations and its guides, a file in the tab of its plugin, the keys
//! and the mouse of an image, an animation and its frames, a large image in tiles.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wwrt::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use wwrt::egui_kittest::Harness;
use wwrt::plugin::Plugins;
use wwrt::testing;
use wwrt::ww::{Settings, ShellAct, Window};

use crate::app::{Act, Doc, Extra, GUIDE, GUIDES, LANGUAGES, Wview};
pub(crate) use wv_image::sample::{GifFrame, bmp, gif};

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
pub(crate) fn front(w: &Window<Wview>) -> (PathBuf, String) {
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

/// Settings in English, with the plugin of images as `image` says.
pub(crate) fn english(image: wwrt::serde_json::Value) -> Settings<Extra> {
    let mut settings: Settings<Extra> = Settings {
        language: "en".to_owned(),
        ..Settings::default()
    };
    settings.extra.plugins.insert("wv_image".to_owned(), image);
    settings
}

/// A GIF of three frames of 320 x 200, stripes of red, green and blue a step further in each,
/// every frame for a tenth of a second.
pub(crate) fn three_frames() -> Vec<u8> {
    let palette = [[0, 0, 0], [220, 60, 60], [60, 180, 90], [70, 110, 230]];
    let frames: Vec<GifFrame> = (0..3u32)
        .map(|i| {
            let stripes = (0..200u32)
                .flat_map(|y| (0..320u32).map(move |x| ((x + y + i * 20) / 20 % 3 + 1) as u8))
                .collect();
            GifFrame::new(0, 0, [320, 200], stripes, 10, 1, None)
        })
        .collect();
    gif([320, 200], &palette, &frames)
}

/// Frames of the window until the image in front is decoded (a thread of its own decodes it).
pub(crate) fn decoded(h: &mut Harness<'_, Window<Wview>>) {
    for _ in 0..500 {
        h.step();
        if !front(h.state()).1.contains("Decoding") {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn tool_labels(w: &Window<Wview>) -> Vec<&'static str> {
    match w.shell.active_doc() {
        Some(Doc::File(d)) => d.view.tools().iter().map(|t| t.label).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn an_animation_and_its_frames() {
    let dir = folder("animation");
    std::fs::write(dir.join("a.gif"), three_frames()).unwrap();
    let mut window = window(english(wwrt::serde_json::json!({ "animate": false })));
    open(&mut window, dir.join("a.gif"));
    assert!(
        front(&window).1.contains("frame 1 of 3"),
        "{}",
        front(&window).1
    );
    assert!(tool_labels(&window).contains(&"Play"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_eframe(|_| window);
    h.run_steps(3);
    click(&h, Pos2::new(400.0, 320.0));
    h.run_steps(2);
    h.key_press(Key::Period);
    h.run_steps(2);
    assert!(
        front(h.state()).1.contains("frame 2 of 3"),
        "{}",
        front(h.state()).1
    );
    h.key_press(Key::Comma);
    h.key_press(Key::Comma);
    h.run_steps(2);
    assert!(
        front(h.state()).1.contains("frame 3 of 3"),
        "the frames go round: {}",
        front(h.state()).1
    );
    h.key_press(Key::Space);
    h.run_steps(2);
    assert!(tool_labels(h.state()).contains(&"Pause"), "Space plays it");
}

#[test]
fn a_large_image_shows_in_tiles_and_the_arrows_move_it() {
    let dir = folder("large");
    let wide = bmp(5000, 40, |x, _| [(x % 256) as u8, 0, 90]);
    std::fs::write(dir.join("a-wide.bmp"), wide).unwrap();
    std::fs::write(dir.join("b.bmp"), bmp(8, 8, |_, _| [0, 200, 0])).unwrap();
    let mut window = window(english(wwrt::serde_json::Value::Null));
    open(&mut window, dir.join("a-wide.bmp"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_eframe(|_| window);
    decoded(&mut h);
    click(&h, Pos2::new(400.0, 320.0));
    h.run_steps(2);
    h.key_press(Key::Num1);
    h.run_steps(4);
    let status = front(h.state()).1;
    assert!(
        status.contains("5000 x 40") && status.contains("100%"),
        "{status}"
    );
    assert!(!status.contains("decoded at"), "decoded whole: {status}");
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    assert_eq!(
        front(h.state()).0,
        dir.join("a-wide.bmp"),
        "an image wider than the view moves with the arrows"
    );
    h.key_press(Key::PageDown);
    h.run_steps(2);
    assert_eq!(
        front(h.state()).0,
        dir.join("b.bmp"),
        "Page Down takes the next image"
    );
}
