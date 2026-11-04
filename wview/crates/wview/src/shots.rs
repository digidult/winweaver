//! Pictures of the window of wview for the review of its design. Only when
//! `WINWEAVER_SCREENSHOTS` names a folder (CI sets it on the Windows runner): the page of the
//! plugins, a Markdown page, an image fitted, then zoomed and moved, Settings, Plugins; drawn with
//! wgpu without a screen and saved there as PNG files; a renderer that cannot start writes
//! `wview-error.txt`.

use std::path::{Path, PathBuf};

use wwrt::egui::{self, Event, Modifiers, PointerButton, Pos2};
use wwrt::egui_kittest::Harness;
use wwrt::ww::{App, Settings, ShellAct, Window};

use crate::app::{Act, Wview};
use crate::tests::{bmp, folder, window};

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
    let settings = Settings {
        language: "en".to_owned(),
        ..Settings::default()
    };
    let mut window = window(settings);
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
}
