//! Images through the Windows Imaging Component: the plugin opens every format a decoder of this
//! Windows names (BMP, PNG, JPEG, GIF, TIFF, ICO, JPEG XR, DDS, and WebP, HEIF or AVIF where their
//! extensions are installed), read once at the load. The view fits the image to the tab or shows
//! it at a zoom of its own; the arrows of the host go to the other images of the folder.

use std::path::Path;
use std::sync::OnceLock;

use wwrt::egui;
use wwrt::plugin::{Format, Host, Plugin, Tool, View};

/// The longest side of the texture of an image: a larger image is scaled down by WIC when it is
/// decoded (a texture of 8192 pixels is what every renderer of the window takes).
const MAX_SIDE: u32 = 8192;

/// The formats of the decoders of this Windows, read once.
static FORMATS: OnceLock<&'static [Format]> = OnceLock::new();

struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn name(&self) -> &'static str {
        "Images (Windows Imaging Component)"
    }

    fn formats(&self) -> &'static [Format] {
        FORMATS.get_or_init(|| {
            // A thread of its own for COM: the main thread of the window keeps the apartment
            // winit chooses for it, which the list must not choose first.
            let list = std::thread::spawn(wic::decoders).join().unwrap_or_default();
            Box::leak(list.into_boxed_slice())
        })
    }

    fn open(&self, path: &Path) -> Result<Box<dyn View>, String> {
        let image = wic::decode(path, MAX_SIDE)?;
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
            texture: None,
            zoom: Zoom::Fit,
            drawn: 1.0,
        }))
    }
}

/// How large the image is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Zoom {
    /// The whole image in the tab, never above its own size.
    Fit,
    /// So many pixels of the tab a pixel of the image.
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
    texture: Option<egui::TextureHandle>,
    zoom: Zoom,
    /// The scale of the last frame, for the status bar and a zoom from the fitted size.
    drawn: f32,
}

impl View for ImageView {
    fn ui(&mut self, ui: &mut egui::Ui, _host: &Host) {
        let texture = match (&self.texture, self.pixels.take()) {
            (Some(t), _) => t.clone(),
            (None, Some(pixels)) => {
                let size = [self.shown[0] as usize, self.shown[1] as usize];
                let image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let t = ui
                    .ctx()
                    .load_texture(&self.name, image, egui::TextureOptions::LINEAR);
                self.texture = Some(t.clone());
                t
            }
            (None, None) => return,
        };
        egui::CentralPanel::default().show(ui, |ui| {
            let room = ui.available_size();
            let (w, h) = (self.size[0] as f32, self.size[1] as f32);
            let scale = match self.zoom {
                Zoom::Fit => (room.x / w).min(room.y / h).min(1.0),
                Zoom::Scale(s) => s,
            };
            let hovered = ui.rect_contains_pointer(ui.max_rect());
            let wheel = ui.input(|i| i.zoom_delta());
            if hovered && wheel != 1.0 {
                self.zoom = Zoom::Scale((scale * wheel).clamp(0.02, 32.0));
            }
            self.drawn = scale;
            let size = egui::vec2(w * scale, h * scale);
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let pad = ((room - size) / 2.0).max(egui::Vec2::ZERO);
                ui.add_space(pad.y);
                ui.horizontal(|ui| {
                    ui.add_space(pad.x);
                    ui.add(egui::Image::new((texture.id(), size)));
                });
            });
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
        self.zoom = match id {
            "fit" => Zoom::Fit,
            "actual" => Zoom::Scale(1.0),
            "in" => Zoom::Scale((self.drawn * 1.25).min(32.0)),
            "out" => Zoom::Scale((self.drawn / 1.25).max(0.02)),
            _ => self.zoom,
        };
    }

    fn browses(&self) -> bool {
        true
    }

    fn status(&self) -> String {
        let mut line = format!(
            "{} x {}, {}, {:.0}%",
            self.size[0],
            self.size[1],
            self.format,
            self.drawn * 100.0
        );
        if self.shown != self.size {
            line.push_str(&format!(
                ", decoded at {} x {}",
                self.shown[0], self.shown[1]
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
                .map_err(|_| "this Windows has no decoder for the file".to_owned())?;
            let format = decoder
                .GetDecoderInfo()
                .map(|i| text(|b, l| i.GetFriendlyName(b, l)))
                .unwrap_or_default();
            let frame = decoder.GetFrame(0).map_err(err)?;
            let (mut w, mut h) = (0u32, 0u32);
            frame.GetSize(&raw mut w, &raw mut h).map_err(err)?;
            if w == 0 || h == 0 {
                return Err("the image is empty".to_owned());
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
