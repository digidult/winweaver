//! The calls of WIC: the decoders of this Windows, and an image decoded into pictures in memory.
//! Each runs in the thread that calls it, with COM joined there.

use std::path::Path;

use wwrt::plugin::Format;
use wwrt::windows::Win32::Foundation::GENERIC_READ;
use wwrt::windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatGif, GUID_ContainerFormatIco,
    GUID_WICPixelFormat32bppRGBA, IWICBitmapCodecInfo, IWICBitmapDecoder, IWICBitmapSource,
    IWICImagingFactory, IWICMetadataQueryReader, IWICPalette, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom, WICComponentEnumerateDefault,
    WICDecodeMetadataCacheOnDemand, WICDecoder,
};
use wwrt::windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use wwrt::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use wwrt::windows::Win32::System::Variant::{VT_UI1, VT_UI2, VT_UI4};
use wwrt::windows::core::{HSTRING, Interface};
use wwrt::ww::i18n::tr;

use crate::compose::{Canvas, Frame, delay_ms};
use crate::picture::{Picture, TOP, within};

/// The memory the frames of one animation take at most; the frames past it are left out.
const ANIMATION_BYTES: usize = 512 << 20;

/// How an image is decoded.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The pixels an image is decoded with at most; a larger one is decoded smaller.
    pub budget: u64,
    /// Whether the orientation of EXIF turns the image.
    pub orient: bool,
}

/// An image decoded.
pub struct Decoded {
    /// The name of the decoder of WIC (PNG Decoder).
    pub format: String,
    /// The size of the image in its file, as it is seen (turned by its orientation).
    pub size: [u32; 2],
    pub kind: Kind,
    /// The frames of an animation or the pages of a file of several.
    pub count: u32,
    /// Whether its orientation turned it.
    pub turned: bool,
}

pub enum Kind {
    /// A still image, or one page of a file of several: its levels, the first as decoded, each
    /// next one half of it (`Picture::levels`).
    Still { levels: Vec<Picture>, page: u32 },
    /// An animation (GIF): every frame drawn on its canvas, with the milliseconds it shows; fewer
    /// than `count` when they did not fit in memory.
    Animation { frames: Vec<(Picture, u32)> },
}

impl Decoded {
    /// The size the image was decoded at.
    pub fn shown(&self) -> [u32; 2] {
        match &self.kind {
            Kind::Still { levels, .. } => levels[0].size,
            Kind::Animation { frames } => frames[0].0.size,
        }
    }
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

fn error(e: wwrt::windows::core::Error) -> String {
    format!("WIC: {}", e.message())
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

/// A number of the metadata of a decoder or a frame (`/grctlext/Delay`), when it has one.
fn number(reader: &IWICMetadataQueryReader, name: &str) -> Option<u32> {
    let mut value = PROPVARIANT::default();
    // SAFETY: the value is a PROPVARIANT WIC fills; its union is read by the type it names and
    // cleared after.
    unsafe {
        reader
            .GetMetadataByName(&HSTRING::from(name), &raw mut value)
            .ok()?;
        let inner = &value.Anonymous.Anonymous;
        let n = match inner.vt {
            VT_UI1 => Some(u32::from(inner.Anonymous.bVal)),
            VT_UI2 => Some(u32::from(inner.Anonymous.uiVal)),
            VT_UI4 => Some(inner.Anonymous.ulVal),
            _ => None,
        };
        let _ = PropVariantClear(&raw mut value);
        n
    }
}

/// The decoders of this Windows with their extensions (".bmp,.dib,.rle").
pub fn decoders() -> Vec<Format> {
    let Ok(factory) = factory() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // SAFETY: the enumerator and the infos are the COM objects of WIC, used as documented.
    unsafe {
        let Ok(list) = factory
            .CreateComponentEnumerator(WICDecoder.0 as u32, WICComponentEnumerateDefault.0 as u32)
        else {
            return out;
        };
        loop {
            let mut items = [None];
            let mut fetched = 0u32;
            if list.Next(&mut items, Some(&raw mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(Ok(info)) = items[0].take().map(|u| u.cast::<IWICBitmapCodecInfo>()) else {
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

/// The pixels of a source in RGBA, scaled to `to` when it is smaller than the source.
///
/// # Safety
/// `source` is a source of WIC of the size `from`.
unsafe fn rgba(
    factory: &IWICImagingFactory,
    source: &IWICBitmapSource,
    from: [u32; 2],
    to: [u32; 2],
) -> Result<Vec<u8>, String> {
    // SAFETY: the scaler and the converter are COM objects of WIC used as documented; the buffer
    // is as large as the stride times the height.
    unsafe {
        let mut source = source.clone();
        if to != from {
            let scaler = factory.CreateBitmapScaler().map_err(error)?;
            scaler
                .Initialize(&source, to[0], to[1], WICBitmapInterpolationModeFant)
                .map_err(error)?;
            source = scaler.cast().map_err(error)?;
        }
        let converter = factory.CreateFormatConverter().map_err(error)?;
        converter
            .Initialize(
                &source,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None::<&IWICPalette>,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .map_err(error)?;
        let mut pixels = vec![0u8; to[0] as usize * to[1] as usize * 4];
        converter
            .CopyPixels(std::ptr::null(), to[0] * 4, &mut pixels)
            .map_err(error)?;
        Ok(pixels)
    }
}

/// The image of a file: a page of it (`None`: the first, or for an icon its largest), or every
/// frame of an animation.
pub fn decode(path: &Path, page: Option<u32>, options: Options) -> Result<Decoded, String> {
    let factory = factory()?;
    // SAFETY: the decoder and its frames are COM objects of WIC used as documented.
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
        let count = decoder.GetFrameCount().map_err(error)?.max(1);
        let container = decoder.GetContainerFormat().unwrap_or_default();
        if container == GUID_ContainerFormatGif
            && count > 1
            && let Some(decoded) = animation(&factory, &decoder, &format, count)?
        {
            return Ok(decoded);
        }
        let page = match page {
            Some(p) => p.min(count - 1),
            None if container == GUID_ContainerFormatIco => largest(&decoder, count),
            None => 0,
        };
        still(&factory, &decoder, format, count, page, options)
    }
}

/// The frame of an icon with the most pixels.
///
/// # Safety
/// `decoder` is a decoder of WIC with `count` frames.
unsafe fn largest(decoder: &IWICBitmapDecoder, count: u32) -> u32 {
    let mut best = (0u64, 0u32);
    for i in 0..count {
        let (mut w, mut h) = (0u32, 0u32);
        // SAFETY: the frame is a COM object of WIC used as documented.
        let ok = unsafe {
            decoder
                .GetFrame(i)
                .and_then(|f| f.GetSize(&raw mut w, &raw mut h))
                .is_ok()
        };
        let area = u64::from(w) * u64::from(h);
        if ok && area > best.0 {
            best = (area, i);
        }
    }
    best.1
}

/// One frame as a still image with its levels.
///
/// # Safety
/// `decoder` is a decoder of WIC with more than `page` frames.
unsafe fn still(
    factory: &IWICImagingFactory,
    decoder: &IWICBitmapDecoder,
    format: String,
    count: u32,
    page: u32,
    options: Options,
) -> Result<Decoded, String> {
    // SAFETY: the frame and its metadata are COM objects of WIC used as documented.
    unsafe {
        let frame = decoder.GetFrame(page).map_err(error)?;
        let (mut w, mut h) = (0u32, 0u32);
        frame.GetSize(&raw mut w, &raw mut h).map_err(error)?;
        if w == 0 || h == 0 {
            return Err(tr("The image is empty").to_owned());
        }
        let orientation = if options.orient {
            frame
                .GetMetadataQueryReader()
                .ok()
                .and_then(|r| number(&r, "System.Photo.Orientation"))
                .unwrap_or(1)
        } else {
            1
        };
        let source: IWICBitmapSource = frame.cast().map_err(error)?;
        let to = within([w, h], options.budget);
        let pixels = rgba(factory, &source, [w, h], to)?;
        let turned = (2..=8).contains(&orientation);
        let picture = Picture { pixels, size: to }.oriented(orientation as u16);
        let size = if turned && orientation >= 5 {
            [h, w]
        } else {
            [w, h]
        };
        Ok(Decoded {
            format,
            size,
            kind: Kind::Still {
                levels: picture.levels(),
                page,
            },
            count,
            turned,
        })
    }
}

/// Every frame of a GIF drawn on its canvas, or `None` for one whose canvas is larger than one
/// texture, which shows as its first frame.
///
/// # Safety
/// `decoder` is a decoder of WIC with `count` frames.
unsafe fn animation(
    factory: &IWICImagingFactory,
    decoder: &IWICBitmapDecoder,
    format: &str,
    count: u32,
) -> Result<Option<Decoded>, String> {
    // SAFETY: the frames and their metadata are COM objects of WIC used as documented.
    unsafe {
        let screen = decoder.GetMetadataQueryReader().ok().map(|r| {
            [
                number(&r, "/logscrdesc/Width").unwrap_or(0),
                number(&r, "/logscrdesc/Height").unwrap_or(0),
            ]
        });
        let mut frames = Vec::new();
        let mut canvas: Option<Canvas> = None;
        let mut bytes = 0usize;
        for i in 0..count {
            let frame = decoder.GetFrame(i).map_err(error)?;
            let (mut w, mut h) = (0u32, 0u32);
            frame.GetSize(&raw mut w, &raw mut h).map_err(error)?;
            let reader = frame.GetMetadataQueryReader().ok();
            let field = |name: &str| reader.as_ref().and_then(|r| number(r, name));
            let left = field("/imgdesc/Left").unwrap_or(0);
            let top = field("/imgdesc/Top").unwrap_or(0);
            if canvas.is_none() {
                let size = match screen {
                    Some([sw, sh]) if sw > 0 && sh > 0 => [sw, sh],
                    _ => [left + w, top + h],
                };
                if size[0].max(size[1]) > TOP || size[0] == 0 || size[1] == 0 {
                    return Ok(None);
                }
                canvas = Some(Canvas::new(size));
            }
            let Some(canvas) = canvas.as_mut() else {
                break;
            };
            let source: IWICBitmapSource = frame.cast().map_err(error)?;
            let pixels = rgba(factory, &source, [w, h], [w, h])?;
            let drawn = canvas.add(&Frame {
                left,
                top,
                picture: Picture {
                    pixels,
                    size: [w, h],
                },
                disposal: field("/grctlext/Disposal").unwrap_or(0) as u8,
            });
            bytes += drawn.pixels.len();
            frames.push((drawn, delay_ms(field("/grctlext/Delay").unwrap_or(0))));
            if bytes >= ANIMATION_BYTES {
                break;
            }
        }
        let Some(first) = frames.first() else {
            return Ok(None);
        };
        Ok(Some(Decoded {
            format: format.to_owned(),
            size: first.0.size,
            kind: Kind::Animation { frames },
            count,
            turned: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{GifFrame, bmp, gif};
    use wwrt::windows::Win32::Foundation::GENERIC_WRITE;
    use wwrt::windows::Win32::Graphics::Imaging::{
        GUID_ContainerFormatTiff, GUID_WICPixelFormat32bppBGRA, IWICBitmapFrameEncode,
        WICBitmapEncoderNoCache,
    };
    use wwrt::windows::Win32::System::Com::StructuredStorage::{
        IPropertyBag2, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
    };

    const OPTIONS: Options = Options {
        budget: 256_000_000,
        orient: true,
    };

    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wv-image-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A TIFF of the pages given (BGRA pixels with their size), the second turned by the
    /// orientation of EXIF `orientation`.
    fn tiff(path: &Path, pages: &[([u32; 2], Vec<u8>)], orientation: u16) {
        let factory = factory().unwrap();
        // SAFETY: the encoder, the stream, the frames and the metadata writer are COM objects of
        // WIC used as documented.
        unsafe {
            let stream = factory.CreateStream().unwrap();
            stream
                .InitializeFromFilename(&HSTRING::from(path), GENERIC_WRITE.0)
                .unwrap();
            let encoder = factory
                .CreateEncoder(&GUID_ContainerFormatTiff, std::ptr::null())
                .unwrap();
            encoder
                .Initialize(&stream, WICBitmapEncoderNoCache)
                .unwrap();
            for (i, (size, pixels)) in pages.iter().enumerate() {
                let mut frame: Option<IWICBitmapFrameEncode> = None;
                let mut bag: Option<IPropertyBag2> = None;
                encoder
                    .CreateNewFrame(&raw mut frame, &raw mut bag)
                    .unwrap();
                let frame = frame.unwrap();
                frame.Initialize(bag.as_ref()).unwrap();
                frame.SetSize(size[0], size[1]).unwrap();
                let mut pixel_format = GUID_WICPixelFormat32bppBGRA;
                frame.SetPixelFormat(&raw mut pixel_format).unwrap();
                assert_eq!(pixel_format, GUID_WICPixelFormat32bppBGRA);
                if i == 1 {
                    let writer = frame.GetMetadataQueryWriter().unwrap();
                    let value = PROPVARIANT {
                        Anonymous: PROPVARIANT_0 {
                            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                                vt: VT_UI2,
                                Anonymous: PROPVARIANT_0_0_0 { uiVal: orientation },
                                ..PROPVARIANT_0_0::default()
                            }),
                        },
                    };
                    writer
                        .SetMetadataByName(&HSTRING::from("/ifd/{ushort=274}"), &raw const value)
                        .unwrap();
                }
                frame.WritePixels(size[1], size[0] * 4, pixels).unwrap();
                frame.Commit().unwrap();
            }
            encoder.Commit().unwrap();
        }
    }

    fn red_of(picture: &Picture) -> Vec<u8> {
        picture.pixels.chunks(4).map(|c| c[0]).collect()
    }

    #[test]
    fn a_page_of_a_tiff_and_the_orientation_of_its_exif() {
        let dir = folder("tiff");
        let path = dir.join("pages.tiff");
        // BGRA: the first page 3 x 2 grey, the second 2 x 1 of a red and a green pixel.
        let first = ([3, 2], vec![90u8; 3 * 2 * 4]);
        let second = ([2, 1], vec![0, 0, 255, 255, 0, 255, 0, 255]);
        tiff(&path, &[first, second], 6);
        let page = decode(&path, None, OPTIONS).unwrap();
        assert_eq!((page.count, page.size, page.turned), (2, [3, 2], false));
        let turned = decode(&path, Some(1), OPTIONS).unwrap();
        assert_eq!(turned.size, [1, 2], "6 turns it a quarter clockwise");
        assert!(turned.turned);
        let Kind::Still { levels, page } = &turned.kind else {
            panic!("a page is a still image");
        };
        assert_eq!(*page, 1);
        assert_eq!(red_of(&levels[0]), vec![255, 0], "the red pixel on top");
        let off = Options {
            orient: false,
            ..OPTIONS
        };
        assert_eq!(decode(&path, Some(1), off).unwrap().size, [2, 1]);
    }

    #[test]
    fn the_frames_of_a_gif_on_its_canvas() {
        let dir = folder("gif");
        let path = dir.join("frames.gif");
        let palette = [[0, 0, 0], [255, 0, 0], [0, 255, 0], [0, 0, 255]];
        let frames = [
            GifFrame::new(0, 0, [3, 1], vec![1, 1, 1], 5, 1, None),
            GifFrame::new(1, 0, [2, 1], vec![2, 3], 0, 2, Some(3)),
            GifFrame::new(2, 0, [1, 1], vec![3], 10, 1, None),
        ];
        std::fs::write(&path, gif([3, 1], &palette, &frames)).unwrap();
        let decoded = decode(&path, None, OPTIONS).unwrap();
        assert_eq!((decoded.count, decoded.size), (3, [3, 1]));
        let Kind::Animation { frames } = &decoded.kind else {
            panic!("a GIF of three frames is an animation");
        };
        let shown: Vec<(Vec<[u8; 4]>, u32)> = frames
            .iter()
            .map(|(p, ms)| {
                let px = p
                    .pixels
                    .chunks(4)
                    .map(|c| [c[0], c[1], c[2], c[3]])
                    .collect();
                (px, *ms)
            })
            .collect();
        let (red, green, blue) = ([255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]);
        assert_eq!(shown[0], (vec![red, red, red], 50));
        assert_eq!(
            shown[1],
            (vec![red, green, red], 100),
            "the transparent index shows the frame before; a delay of 0 is 100 ms"
        );
        assert_eq!(
            shown[2].0[1][3], 0,
            "the disposal 2 cleared the rectangle of the second frame"
        );
        assert_eq!((shown[2].0[0], shown[2].0[2]), (red, blue));
    }

    #[test]
    fn a_large_image_is_decoded_smaller() {
        let dir = folder("budget");
        let path = dir.join("large.bmp");
        std::fs::write(&path, bmp(100, 100, |x, _| [x as u8, 0, 0])).unwrap();
        let options = Options {
            budget: 2500,
            ..OPTIONS
        };
        let decoded = decode(&path, None, options).unwrap();
        assert_eq!((decoded.size, decoded.shown()), ([100, 100], [50, 50]));
        let none = dir.join("none.xyz");
        std::fs::write(&none, b"not an image").unwrap();
        assert!(decode(&none, None, OPTIONS).is_err());
    }
}
