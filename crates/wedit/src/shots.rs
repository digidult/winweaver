//! Pictures of the window of wedit for the review of its design. Only when
//! `WINWEAVER_SCREENSHOTS` names a folder (CI sets it on the Windows runner): a text with long
//! lines with and without wrap, a page, its text, a narrow window, drawn with wgpu without a
//! screen and saved there as PNG files; a renderer that cannot start writes `wedit-error.txt`.

use std::path::{Path, PathBuf};

use ww::eframe::egui;
use ww::{Settings, ShellAct, Window};

use crate::app::{Act, Doc, Wedit};

type Harness<'a> = egui_kittest::Harness<'a, Window<Wedit>>;

fn shot(h: &mut Harness<'_>, dir: &Path, name: &str) {
    for _ in 0..4 {
        h.step();
    }
    let result = h.render().map_err(|e| e.to_string()).and_then(|image| {
        image
            .save(dir.join(format!("{name}.png")))
            .map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        let _ = std::fs::write(dir.join("wedit-error.txt"), format!("{name}: {e}"));
    }
}

#[test]
fn screenshots_of_wedit() {
    let Some(dir) = std::env::var_os("WINWEAVER_SCREENSHOTS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let text = dir.join("wedit-long-lines.txt");
    let long = "A line of the text that is much longer than the window, so that the wrap of the \
                lines shows: with Wrap lines on it folds at the right edge of the view, and with \
                it off the view scrolls sideways and the line stays one line.";
    std::fs::write(&text, format!("Short line\n{long}\n\nThird line\n{long}\n")).unwrap();
    let page = dir.join("wedit-sample.md");
    std::fs::write(&page, ww::sample::PAGE).unwrap();
    let window = Window::new(Wedit::new(Vec::new()), Settings::default());
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(|cc| {
            ww::style::install_fonts(&cc.egui_ctx);
            window
        });
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(text))]);
    shot(&mut harness, &dir, "w1-text-wrap");
    harness.state_mut().shell.settings.wrap_lines = false;
    shot(&mut harness, &dir, "w2-text-nowrap");
    harness.state_mut().shell.settings.wrap_lines = true;
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(page))]);
    shot(&mut harness, &dir, "w3-page");
    harness.set_size(egui::vec2(820.0, 800.0));
    shot(&mut harness, &dir, "w4-page-narrow");
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.state_mut().apply(vec![Act::EditAsText]);
    shot(&mut harness, &dir, "w5-page-as-text");
    let agents = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../AGENTS.md");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenPath(agents))]);
    if let Some(Doc::Page(m)) = harness.state_mut().shell.docs.last_mut() {
        m.anchor = Some("7-work-status".to_owned());
        m.jump = true;
    }
    shot(&mut harness, &dir, "w6-agents-status");
    harness
        .state_mut()
        .apply(vec![Act::Shell(ShellAct::UseTheme("dark".to_owned()))]);
    shot(&mut harness, &dir, "w7-agents-dark");
    harness.state_mut().apply(vec![Act::Shell(ShellAct::OpenSettings)]);
    shot(&mut harness, &dir, "w8-settings");
}
