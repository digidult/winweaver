//! Images through the Windows Imaging Component: the plugin opens every format a decoder of this
//! Windows names (BMP, PNG, JPEG, GIF, TIFF, ICO, JPEG XR, DDS, and WebP, HEIF or AVIF where their
//! extensions are installed), read once at the load.
//!
//! A format names the MIME types its decoder gives, so that the plugin also opens an image of
//! bytes in memory, a part of another file shown inside its view (an icon inside a program).
//!
//! An image is decoded in a thread of its own (`view::start`), turned by the orientation of its
//! EXIF, and kept with its levels (each half of the one before, `picture`), so that the view draws
//! the tiles of the level its zoom needs (`view`); a GIF plays its frames drawn on its canvas
//! (`compose`), a file of several pages (TIFF, ICO) turns them. The view fits the image to the tab
//! or shows it at a zoom of its own, with the wheel, a drag, a double click and the keys. Its
//! settings: smooth pixels above 100%, the orientation, an animation played when it opens and the
//! largest image decoded whole.

use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use wwrt::egui;
use wwrt::plugin::{Format, Plugin, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::i18n::{tr, trf};

mod compose;
mod picture;
#[cfg(any(test, feature = "library"))]
pub mod sample;
mod view;
mod wic;

use view::ImageView;
use wic::{Options, Source};

/// The largest image decoded whole, in millions of pixels, by default and as the settings offer
/// it; a larger one is decoded smaller.
const MEGAPIXELS: u32 = 256;
const MEGAPIXEL_STEPS: [u32; 3] = [64, 256, 1024];
/// How long the window waits for a decoding before the tab opens without its image.
const WAIT: Duration = Duration::from_millis(250);

/// The formats of the decoders of this Windows, read once.
static FORMATS: OnceLock<&'static [Format]> = OnceLock::new();

/// The name of the plugin (translated by the host).
const NAME: &str = "Images (Windows Imaging Component)";

/// The texts of the plugin in the languages of the window.
static TRANSLATIONS: [(&str, &str); 8] = [
    ("uk", include_str!("../lang/uk.json")),
    ("ru", include_str!("../lang/ru.json")),
    ("de", include_str!("../lang/de.json")),
    ("es", include_str!("../lang/es.json")),
    ("fr", include_str!("../lang/fr.json")),
    ("id", include_str!("../lang/id.json")),
    ("it", include_str!("../lang/it.json")),
    ("tr", include_str!("../lang/tr.json")),
];

/// The view of an image with the settings of the plugin.
fn view_of(source: Source, settings: &Value) -> Result<Box<dyn View>, String> {
    let megapixels: u32 = setting(settings, "max_megapixels", MEGAPIXELS);
    let options = Options {
        budget: u64::from(megapixels.clamp(MEGAPIXEL_STEPS[0], MEGAPIXEL_STEPS[2])) * 1_000_000,
        orient: setting(settings, "orientation", true),
    };
    let job = view::start(source.clone(), None, options);
    // An image decoded at once opens with its tab, and one that does not decode stays closed
    // with the reason; a larger one opens and shows when it is there.
    let first = match job.recv_timeout(WAIT) {
        Ok(Ok(image)) => Ok(image),
        Ok(Err(e)) => return Err(e),
        Err(RecvTimeoutError::Timeout) => Err(job),
        Err(RecvTimeoutError::Disconnected) => {
            return Err(tr("The decoding stopped").to_owned());
        }
    };
    Ok(Box::new(ImageView::new(
        source,
        options,
        setting(settings, "smooth", true),
        setting(settings, "animate", true),
        first,
    )))
}

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn formats(&self) -> &'static [Format] {
        FORMATS.get_or_init(|| {
            // A thread of its own for COM: the main thread of the window keeps the apartment
            // winit chooses for it, which the list must not choose first.
            let list = std::thread::spawn(wic::decoders).join().unwrap_or_default();
            Box::leak(list.into_boxed_slice())
        })
    }

    fn open(&self, path: &Path, settings: &Value) -> Result<Box<dyn View>, String> {
        view_of(Source::File(path.to_path_buf()), settings)
    }

    fn open_bytes(
        &self,
        _name: &str,
        _mime: &str,
        bytes: Vec<u8>,
        settings: &Value,
    ) -> Result<Box<dyn View>, String> {
        view_of(Source::Bytes(Arc::from(bytes)), settings)
    }

    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &TRANSLATIONS
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&self, ui: &mut egui::Ui, settings: &mut Value) -> bool {
        let mut changed = false;
        let mut check = |ui: &mut egui::Ui, settings: &mut Value, name: &str, text: &str| {
            let mut on: bool = setting(settings, name, true);
            if ui.checkbox(&mut on, text).changed() {
                set_setting(settings, name, on);
                changed = true;
            }
        };
        check(
            ui,
            settings,
            "smooth",
            tr("Smooth the pixels of a zoom above 100%"),
        );
        check(
            ui,
            settings,
            "orientation",
            tr("Turn an image by its EXIF orientation"),
        );
        check(
            ui,
            settings,
            "animate",
            tr("Play an animation when it opens"),
        );
        let mut megapixels: u32 = setting(settings, "max_megapixels", MEGAPIXELS);
        ui.horizontal(|ui| {
            ui.label(tr("Largest image decoded whole"));
            for m in MEGAPIXEL_STEPS {
                let text = trf("{n} MP", &[("n", &m)]);
                if ui.radio(megapixels == m, text).clicked() && megapixels != m {
                    megapixels = m;
                    set_setting(settings, "max_megapixels", m);
                    changed = true;
                }
            }
        });
        ui.weak(tr(
            "A larger image is decoded smaller. The settings apply to the images opened after a change.",
        ));
        changed
    }
}

wwrt::plugin!(ImagePlugin);

#[cfg(test)]
mod tests {
    #[test]
    fn every_text_of_the_plugin_is_translated() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let wanted = wwrt::testing::plugin_texts(&src);
        let missing = wwrt::testing::missing(&super::TRANSLATIONS, &wanted);
        assert!(missing.is_empty(), "{missing:#?}");
    }
}
