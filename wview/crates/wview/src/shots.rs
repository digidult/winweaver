//! Pictures of the window of wview for the review of its design. Only when
//! `WINWEAVER_SCREENSHOTS` names a folder (CI sets it on the Windows runner): the page of the
//! plugins, a Markdown page, an image fitted, then zoomed and moved, Settings, Plugins, an
//! animation, a large image fitted and at 100% in its tiles, a DLL of this Windows in its tree and
//! its bytes, as text and with its resources open; drawn with wgpu without a screen and saved there as PNG files; a
//! renderer that cannot start writes `wview-error.txt`.

use std::path::{Path, PathBuf};

use wwrt::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use wwrt::egui_kittest::Harness;
use wwrt::ww::{App, ShellAct, Window};

use crate::app::{Act, Wview};
use crate::tests::{bmp, click_tree, decoded, english, folder, system_dll, three_frames, window};

fn shot(h: &mut Harness<'_, Window<Wview>>, dir: &Path, name: &str) {
    for _ in 0..6 {
        h.step();
    }
    let result = h.render().map_err(|e| e.to_string()).and_then(|image| {
        image
            .save(dir.join(format!("{name}.png")))
            .map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        let _ = std::fs::write(dir.join("wview-error.txt"), format!("{name}: {e}"));
    }
}

fn button(h: &Harness<'_, Window<Wview>>, at: Pos2, pressed: bool) {
    h.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

#[test]
fn screenshots_of_wview() {
    let Some(dir) = std::env::var_os("WINWEAVER_SCREENSHOTS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let files = folder("shots");
    let page = files.join("sample.md");
    std::fs::write(&page, wwrt::ww::sample::PAGE).unwrap();
    let image = files.join("gradient.bmp");
    let gradient = bmp(640, 400, |x, y| {
        [(x * 255 / 639) as u8, (y * 255 / 399) as u8, 160]
    });
    std::fs::write(&image, gradient).unwrap();
    let animation = files.join("frames.gif");
    std::fs::write(&animation, three_frames()).unwrap();
    let large = files.join("large.bmp");
    let picture = bmp(4000, 2500, |x, y| {
        let ring = ((x as i64 - 2000).pow(2) + (y as i64 - 1250).pow(2)).isqrt() as u32;
        [
            (ring % 64 * 4) as u8,
            (x * 255 / 3999) as u8,
            (y * 255 / 2499) as u8,
        ]
    });
    std::fs::write(&large, picture).unwrap();
    let mut window = window(english(wwrt::serde_json::json!({ "animate": false })));
    // What `ww::run` does after the session: a window without a tab shows the plugins.
    window.app.restored(&mut window.shell);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(|cc| {
            wwrt::ww::style::install_fonts(&cc.egui_ctx);
            window
        });
    shot(&mut h, &dir, "v1-plugins");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(page))]);
    shot(&mut h, &dir, "v2-markdown");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(image))]);
    shot(&mut h, &dir, "v3-image-fit");
    let at = Pos2::new(520.0, 380.0);
    h.hover_at(at);
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 400.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(12);
    let to = at + egui::vec2(-160.0, -90.0);
    button(&h, at, true);
    h.run_steps(2);
    h.hover_at(to);
    h.run_steps(2);
    button(&h, to, false);
    shot(&mut h, &dir, "v4-image-zoomed-moved");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenSection(1))]);
    shot(&mut h, &dir, "v5-settings-plugins");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(animation))]);
    shot(&mut h, &dir, "v6-animation");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(large))]);
    decoded(&mut h);
    shot(&mut h, &dir, "v7-large-image-fit");
    h.state_mut().apply(vec![Act::Tool("actual")]);
    shot(&mut h, &dir, "v8-large-image-tiles");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(system_dll()))]);
    h.run_steps(3);
    // The sections opened and the first one selected: its header marked in the bytes.
    click_tree(&h);
    h.run_steps(2);
    for key in [
        Key::ArrowDown,
        Key::ArrowDown,
        Key::ArrowDown,
        Key::ArrowRight,
        Key::ArrowDown,
    ] {
        h.key_press(key);
        h.run_steps(2);
    }
    shot(&mut h, &dir, "v9-binary-tree-bytes");
    h.state_mut().apply(vec![Act::Tool("text")]);
    shot(&mut h, &dir, "v10-binary-text");
    // The resources: their types, one of them open with the names it holds read when they open.
    h.state_mut().apply(vec![Act::Tool("bytes")]);
    h.run_steps(2);
    click_tree(&h);
    h.run_steps(2);
    let mut keys = vec![Key::ArrowDown; 14];
    keys.push(Key::ArrowRight);
    keys.extend([Key::ArrowDown; 7]);
    keys.extend([Key::ArrowRight, Key::ArrowDown, Key::ArrowRight]);
    for key in keys {
        h.key_press(key);
        h.run_steps(1);
    }
    shot(&mut h, &dir, "v11-binary-resources");
}
