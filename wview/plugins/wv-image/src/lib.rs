//! Images through the Windows Imaging Component: the plugin opens every format a decoder of this
//! Windows names (BMP, PNG, JPEG, GIF, TIFF, ICO, JPEG XR, DDS, and WebP, HEIF or AVIF where their
//! extensions are installed), read once at the load.
//!
//! The view fits the image to the tab or shows it at a zoom of its own: the wheel zooms at the
//! pointer, a drag moves the image, a double click goes between the fit and 100%. With the focus
//! the arrows, the page keys, Home and End go to the other images of the folder, + and - zoom,
//! 0 fits and 1 shows 100%. Its settings: smooth pixels above 100% and the largest side of a
//! decoded image.

use std::path::Path;
use std::sync::OnceLock;

use wwrt::egui::{self, Key, Pos2, Rect, Vec2};
use wwrt::input::{self, Cursor};
use wwrt::plugin::{Format, Host, Plugin, Request, Step, Tool, View, set_setting, setting};
use wwrt::serde_json::Value;
use wwrt::ww::i18n::{tr, trf};

/// The longest side of a decoded image by default: a larger image is scaled down by WIC as it is
/// decoded (a texture of 8192 pixels is what every renderer of the window takes).
const MAX_SIDE: u32 = 8192;
/// The sides a setting offers.
const SIDES: [u32; 3] = [4096, 8192, 16384];
/// The keys of the view while it has the focus.
const KEYS: [Key; 11] = [
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::PageUp,
    Key::PageDown,
    Key::Home,
    Key::End,
    Key::Plus,
    Key::Equals,
    Key::Minus,
    Key::Num0,
    Key::Num1,
];
/// The zooms a view takes, of the size of the image.
const LEAST: f32 = 0.02;
const MOST: f32 = 32.0;

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
        let max_side: u32 = setting(settings, "max_side", MAX_SIDE);
        let image = wic::decode(path, max_side.clamp(SIDES[0], SIDES[2]))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Box::new(ImageView {
            name,
            format: image.format,
            pixels: Some(image.pixels),
            size: image.size,
            shown: image.shown,
            smooth: setting(settings, "smooth", true),
            texture: None,
            zoom: Zoom::Fit,
            offset: Vec2::ZERO,
            drawn: 1.0,
            rect: Rect::NOTHING,
        }))
    }

    fn translations(&self) -> &'static [(&'static str, &'static str)] {
        &TRANSLATIONS
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&self, ui: &mut egui::Ui, settings: &mut Value) -> bool {
        let mut smooth: bool = setting(settings, "smooth", true);
        let mut side: u32 = setting(settings, "max_side", MAX_SIDE);
        let mut changed = false;
        if ui
            .checkbox(&mut smooth, tr("Smooth the pixels of a zoom above 100%"))
            .changed()
        {
            set_setting(settings, "smooth", smooth);
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label(tr("Largest side of a decoded image"));
            for s in SIDES {
                if ui.radio(side == s, format!("{s}")).clicked() && side != s {
                    side = s;
                    set_setting(settings, "max_side", side);
                    changed = true;
                }
            }
        });
        ui.weak(tr(
            "The settings apply to the images opened after a change.",
        ));
        changed
    }
}

/// How large the image is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Zoom {
    /// The whole image in the tab, never above its own size.
    Fit,
    /// So many points of the window a pixel of the image.
    Scale(f32),
}

struct ImageView {
    name: String,
    /// The name of the decoder of WIC (PNG Decoder).
    format: String,
    /// The pixels until the first frame makes the texture of them.
    pixels: Option<Vec<u8>>,
    /// The size of the image in its file and the size it was decoded at.
    size: [u32; 2],
    shown: [u32; 2],
    smooth: bool,
    texture: Option<egui::TextureHandle>,
    zoom: Zoom,
    /// Where the centre of the image is from the centre of the view, in points.
    offset: Vec2,
    /// The scale of the last frame, for the status bar and a zoom from the fitted size.
    drawn: f32,
    /// The area of the last frame, for the commands of the bar.
    rect: Rect,
}

impl ImageView {
    /// The scale of the fit in an area.
    fn fit(&self, rect: Rect) -> f32 {
        let (w, h) = (self.size[0] as f32, self.size[1] as f32);
        (rect.width() / w).min(rect.height() / h).clamp(LEAST, 1.0)
    }

    fn scale(&self, rect: Rect) -> f32 {
        match self.zoom {
            Zoom::Fit => self.fit(rect),
            Zoom::Scale(s) => s,
        }
    }

    /// A new scale with the point `at` of the window where it was on the image.
    fn zoom_at(&mut self, rect: Rect, at: Pos2, new: f32) {
        let old = self.scale(rect);
        let new = new.clamp(LEAST, MOST);
        let centre = rect.center() + self.offset;
        let point = (at - centre) / old;
        self.offset = at - rect.center() - point * new;
        self.zoom = Zoom::Scale(new);
    }

    fn fit_now(&mut self) {
        self.zoom = Zoom::Fit;
        self.offset = Vec2::ZERO;
    }

    /// The image stays at least partly in the view.
    fn keep_in(&mut self, rect: Rect, scale: f32) {
        let size = Vec2::new(self.size[0] as f32, self.size[1] as f32) * scale;
        let room = ((size + rect.size()) / 2.0 - Vec2::splat(32.0)).max(Vec2::ZERO);
        self.offset = self.offset.clamp(-room, room);
    }

    fn texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        if self.texture.is_none() {
            let pixels = self.pixels.take()?;
            let size = [self.shown[0] as usize, self.shown[1] as usize];
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
            let options = if self.smooth {
                egui::TextureOptions::LINEAR
            } else {
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    ..egui::TextureOptions::LINEAR
                }
            };
            self.texture = Some(ctx.load_texture(&self.name, image, options));
        }
        self.texture.clone()
    }
}

impl View for ImageView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let Some(texture) = self.texture(ui.ctx()) else {
            return;
        };
        egui::CentralPanel::default().show(ui, |ui| {
            let rect = ui.max_rect();
            self.rect = rect;
            let (_, input) = input::area(ui, ui.id().with("image"), rect, &KEYS);
            let at = input
                .pointer
                .map(|p| rect.min + p.to_vec2())
                .unwrap_or(rect.center());
            for (key, _) in &input.keys {
                let scale = self.scale(rect);
                match key {
                    Key::ArrowLeft | Key::PageUp => {
                        host.requests.push(Request::Browse(Step::Previous))
                    }
                    Key::ArrowRight | Key::PageDown => {
                        host.requests.push(Request::Browse(Step::Next))
                    }
                    Key::Home => host.requests.push(Request::Browse(Step::First)),
                    Key::End => host.requests.push(Request::Browse(Step::Last)),
                    Key::Plus | Key::Equals => self.zoom_at(rect, rect.center(), scale * 1.25),
                    Key::Minus => self.zoom_at(rect, rect.center(), scale / 1.25),
                    Key::Num0 => self.fit_now(),
                    Key::Num1 => self.zoom_at(rect, rect.center(), 1.0),
                    _ => {}
                }
            }
            let mut factor = input.zoom;
            if input.wheel.y != 0.0 {
                factor *= (-input.wheel.y * 0.002).exp();
            }
            if (factor - 1.0).abs() > 1e-4 {
                let scale = self.scale(rect);
                self.zoom_at(rect, at, scale * factor);
            }
            if input.double_clicked {
                if self.zoom == Zoom::Fit {
                    self.zoom_at(rect, at, 1.0);
                } else {
                    self.fit_now();
                }
            }
            if input.dragging && input.drag != Vec2::ZERO {
                if self.zoom == Zoom::Fit {
                    self.zoom = Zoom::Scale(self.fit(rect));
                }
                self.offset += input.drag;
            }
            let scale = self.scale(rect);
            self.keep_in(rect, scale);
            self.drawn = scale;
            let size = Vec2::new(self.size[0] as f32, self.size[1] as f32) * scale;
            if input.dragging {
                input::cursor(ui, Cursor::Grabbing);
            } else if input.hovered && (size.x > rect.width() || size.y > rect.height()) {
                input::cursor(ui, Cursor::Grab);
            }
            let image = Rect::from_center_size(rect.center() + self.offset, size);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter_at(rect)
                .image(texture.id(), image, uv, egui::Color32::WHITE);
        });
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                id: "fit",
                icon: "\u{1F5BC}",
                label: "Fit",
            },
            Tool {
                id: "actual",
                icon: "\u{1F4AF}",
                label: "100%",
            },
            Tool {
                id: "in",
                icon: "\u{2795}",
                label: "Zoom in",
            },
            Tool {
                id: "out",
                icon: "\u{2796}",
                label: "Zoom out",
            },
        ]
    }

    fn act(&mut self, id: &str) {
        let rect = self.rect;
        let scale = self.scale(rect);
        match id {
            "fit" => self.fit_now(),
            "actual" => self.zoom_at(rect, rect.center(), 1.0),
            "in" => self.zoom_at(rect, rect.center(), scale * 1.25),
            "out" => self.zoom_at(rect, rect.center(), scale / 1.25),
            _ => {}
        }
    }

    fn browses(&self) -> bool {
        true
    }

    fn status(&self) -> String {
        let mut line = trf(
            "{w} x {h}, {format}, {zoom}%",
            &[
                ("w", &self.size[0]),
                ("h", &self.size[1]),
                ("format", &self.format),
                ("zoom", &format!("{:.0}", self.drawn * 100.0)),
            ],
        );
        if self.shown != self.size {
            line.push_str(", ");
            line.push_str(&trf(
                "decoded at {w} x {h}",
                &[("w", &self.shown[0]), ("h", &self.shown[1])],
            ));
        }
        line
    }
}

/// The calls of WIC.
mod wic {
    use std::path::Path;

    use wwrt::plugin::Format;
    use wwrt::windows::Win32::Foundation::GENERIC_READ;
    use wwrt::windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppRGBA, IWICBitmapCodecInfo,
        IWICBitmapSource, IWICImagingFactory, IWICPalette, WICBitmapDitherTypeNone,
        WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom, WICComponentEnumerateDefault,
        WICDecodeMetadataCacheOnDemand, WICDecoder,
    };
    use wwrt::windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    use wwrt::windows::core::{HSTRING, Interface};
    use wwrt::ww::i18n::tr;

    /// An image decoded into RGBA.
    pub struct Image {
        pub size: [u32; 2],
        pub shown: [u32; 2],
        pub pixels: Vec<u8>,
        pub format: String,
    }

    fn factory() -> Result<IWICImagingFactory, String> {
        // SAFETY: COM is joined (or was, in another apartment: the call then fails harmlessly)
        // before the factory of WIC, an in-process server, is made.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("WIC: {e}"))
        }
    }

    /// A text of WIC read into a buffer of UTF-16.
    fn text(read: impl Fn(&mut [u16], *mut u32) -> wwrt::windows::core::Result<()>) -> String {
        let mut buffer = vec![0u16; 1024];
        let mut len = 0u32;
        if read(&mut buffer, &raw mut len).is_err() {
            return String::new();
        }
        let len = (len as usize).min(buffer.len());
        String::from_utf16_lossy(&buffer[..len])
            .trim_end_matches('\0')
            .to_owned()
    }

    /// The decoders of this Windows with their extensions (".bmp,.dib,.rle").
    pub fn decoders() -> Vec<Format> {
        let Ok(factory) = factory() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        // SAFETY: the enumerator and the infos are the COM objects of WIC, used as documented.
        unsafe {
            let Ok(list) = factory.CreateComponentEnumerator(
                WICDecoder.0 as u32,
                WICComponentEnumerateDefault.0 as u32,
            ) else {
                return out;
            };
            loop {
                let mut items = [None];
                let mut fetched = 0u32;
                if list.Next(&mut items, Some(&raw mut fetched)).is_err() || fetched == 0 {
                    break;
                }
                let Some(Ok(info)) = items[0].take().map(|u| u.cast::<IWICBitmapCodecInfo>())
                else {
                    continue;
                };
                let name = text(|b, l| info.GetFriendlyName(b, l));
                let extensions: Vec<&'static str> = text(|b, l| info.GetFileExtensions(b, l))
                    .split(',')
                    .map(|e| e.trim().trim_start_matches('.').to_lowercase())
                    .filter(|e| !e.is_empty())
                    .map(|e| &*Box::leak(e.into_boxed_str()))
                    .collect();
                if !extensions.is_empty() {
                    out.push(Format {
                        name: Box::leak(name.into_boxed_str()),
                        extensions: Box::leak(extensions.into_boxed_slice()),
                    });
                }
            }
        }
        out
    }

    /// The first frame of the file in RGBA, scaled down to `max` pixels a side at most.
    pub fn decode(path: &Path, max: u32) -> Result<Image, String> {
        let factory = factory()?;
        let err = |e: wwrt::windows::core::Error| format!("WIC: {}", e.message());
        // SAFETY: the decoder, the frame, the scaler and the converter are COM objects of WIC used
        // as documented; the buffer is as large as the stride times the height.
        unsafe {
            let decoder = factory
                .CreateDecoderFromFilename(
                    &HSTRING::from(path),
                    None,
                    GENERIC_READ,
                    WICDecodeMetadataCacheOnDemand,
                )
                .map_err(|_| tr("Windows has no decoder for this file").to_owned())?;
            let format = decoder
                .GetDecoderInfo()
                .map(|i| text(|b, l| i.GetFriendlyName(b, l)))
                .unwrap_or_default();
            let frame = decoder.GetFrame(0).map_err(err)?;
            let (mut w, mut h) = (0u32, 0u32);
            frame.GetSize(&raw mut w, &raw mut h).map_err(err)?;
            if w == 0 || h == 0 {
                return Err(tr("The image is empty").to_owned());
            }
            let mut source: IWICBitmapSource = frame.cast().map_err(err)?;
            let (mut sw, mut sh) = (w, h);
            if w.max(h) > max {
                let k = f64::from(max) / f64::from(w.max(h));
                sw = ((f64::from(w) * k) as u32).max(1);
                sh = ((f64::from(h) * k) as u32).max(1);
                let scaler = factory.CreateBitmapScaler().map_err(err)?;
                scaler
                    .Initialize(&source, sw, sh, WICBitmapInterpolationModeFant)
                    .map_err(err)?;
                source = scaler.cast().map_err(err)?;
            }
            let converter = factory.CreateFormatConverter().map_err(err)?;
            converter
                .Initialize(
                    &source,
                    &GUID_WICPixelFormat32bppRGBA,
                    WICBitmapDitherTypeNone,
                    None::<&IWICPalette>,
                    0.0,
                    WICBitmapPaletteTypeCustom,
                )
                .map_err(err)?;
            let mut pixels = vec![0u8; sw as usize * sh as usize * 4];
            converter
                .CopyPixels(std::ptr::null(), sw * 4, &mut pixels)
                .map_err(err)?;
            Ok(Image {
                size: [w, h],
                shown: [sw, sh],
                pixels,
                format,
            })
        }
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
