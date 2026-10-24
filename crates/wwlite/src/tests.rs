//! Tests of the light edition, in a process of their own (the edition is set for the process):
//! the parts left out are not offered and do nothing, the profile is edited, checked and built.
//! With `WINWEAVER_SCREENSHOTS` naming a folder, the window is drawn there as PNG files (`l1-` to
//! `l4-`), with wgpu and without a screen.

use std::path::PathBuf;

use egui_kittest::kittest::Queryable;
use winweaver_core::catalog::Catalog;
use winweaver_core::i18n::CatalogTexts;
use winweaver_ui::{
    Act, Doc, FileAct, MarksAct, PcAct, ProfileAct, RegistryAct, Sel, Settings, ShellAct,
    WinWeaver, Window, style,
};
use ww::eframe::egui;

type Harness<'a> = egui_kittest::Harness<'a, Window<WinWeaver>>;

fn harness(size: egui::Vec2) -> Harness<'static> {
    winweaver_ui::enter_lite();
    let catalog = Catalog::embedded().unwrap();
    let mut settings = Settings::default();
    settings.extra.show_guide_at_start = false;
    let app = Window::new(
        WinWeaver::new(catalog, CatalogTexts::embedded().unwrap()),
        settings,
    );
    egui_kittest::Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(|cc| {
            style::install_fonts(&cc.egui_ctx);
            app
        })
}

fn steps(h: &mut Harness<'_>) {
    for _ in 0..4 {
        h.step();
    }
}

#[test]
fn the_light_edition_offers_the_profile_alone() {
    let mut h = harness(egui::vec2(1280.0, 800.0));
    steps(&mut h);
    let has = |h: &Harness<'_>, label: &str| h.query_all_by_label(label).next().is_some();
    assert!(has(&h, "Check") && has(&h, "Build"));
    assert!(!has(&h, "This PC"), "no This PC in the command bar");
    assert!(!has(&h, "Catalogs"), "no imports in the command bar");
    let tabs = h.state().shell.docs.len();
    h.state_mut().apply(vec![
        Act::Pc(PcAct::Read),
        Act::Marks(MarksAct::Open),
        Act::Registry(RegistryAct::Open("HKEY_CURRENT_USER".to_owned(), None)),
    ]);
    assert_eq!(
        h.state().shell.docs.len(),
        tabs,
        "the parts left out do nothing"
    );
    h.state_mut().apply(vec![Act::File(FileAct::OpenGuide)]);
    let guide = h.state().shell.docs.last().map(|d| match d {
        Doc::Markdown(m) => m.plain,
        _ => false,
    });
    assert_eq!(guide, Some(true), "the guide is plain text");
}

#[test]
fn screenshots_of_the_light_edition() {
    let Some(dir) = std::env::var_os("WINWEAVER_SCREENSHOTS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = harness(egui::vec2(1440.0, 900.0));
    let shot = |h: &mut Harness<'_>, name: &str| {
        steps(h);
        match h.render() {
            Ok(image) => {
                let _ = image.save(dir.join(format!("{name}.png")));
            }
            Err(e) => {
                let _ = std::fs::write(dir.join("wwlite-error.txt"), format!("{name}: {e}"));
            }
        }
    };
    shot(&mut h, "l1-profile");
    h.state_mut().apply(vec![
        Act::Profile(ProfileAct::Expand("policies".to_owned(), true)),
        Act::Profile(ProfileAct::Expand("defender".to_owned(), true)),
        Act::Profile(ProfileAct::Select(Sel::Rule("defender.pua".to_owned()))),
    ]);
    shot(&mut h, "l2-rule");
    h.state_mut().apply(vec![Act::File(FileAct::OpenGuide)]);
    shot(&mut h, "l3-guide");
    h.state_mut()
        .apply(vec![Act::Shell(ShellAct::OpenSettings)]);
    shot(&mut h, "l4-settings");
}
