//! The decoders of the crate `image` for the formats the decoders of WIC on this Windows lack:
//! WebP (where its extension of Windows is not installed), TGA, Netpbm, QOI, Radiance HDR and
//! farbfeld, all of Rust. An image is read whole into memory, decoded to RGBA within the limits
//! of the crate (512 MB of pixels), made smaller to the budget of the settings and kept with its
//! levels as an image of WIC is; an animation of WebP shows its first frame.

use std::io::Cursor;

use wwrt::image::imageops::FilterType;
use wwrt::image::{ImageFormat, ImageReader};
use wwrt::plugin::Format;

use crate::picture::{Picture, within};
use crate::wic::{Decoded, Kind, Options, Source};

/// The largest file these decoders read.
const LARGEST: u64 = 1 << 30;

/// The formats of the crate: its format, the short name the status bar shows, and the format of
/// the plugin.
pub static FORMATS: [(ImageFormat, &str, Format); 6] = [
    (
        ImageFormat::WebP,
        "WebP",
        Format {
            name: "WebP images",
            extensions: &["webp"],
            mimes: &["image/webp"],
        },
    ),
    (
        ImageFormat::Tga,
        "TGA",
        Format {
            name: "TGA images",
            extensions: &["tga", "icb", "vda", "vst"],
            mimes: &["image/x-tga", "image/x-targa"],
        },
    ),
    (
        ImageFormat::Pnm,
        "Netpbm",
        Format {
            name: "Netpbm images",
            extensions: &["pbm", "pgm", "ppm", "pnm", "pam"],
            mimes: &[
                "image/x-portable-anymap",
                "image/x-portable-bitmap",
                "image/x-portable-graymap",
                "image/x-portable-pixmap",
            ],
        },
    ),
    (
        ImageFormat::Qoi,
        "QOI",
        Format {
            name: "QOI images",
            extensions: &["qoi"],
            mimes: &["image/qoi"],
        },
    ),
    (
        ImageFormat::Hdr,
        "Radiance HDR",
        Format {
            name: "Radiance HDR images",
            extensions: &["hdr"],
            mimes: &["image/vnd.radiance"],
        },
    ),
    (
        ImageFormat::Farbfeld,
        "farbfeld",
        Format {
            name: "Farbfeld images",
            extensions: &["ff"],
            mimes: &["image/x-farbfeld"],
        },
    ),
];

/// The formats of the crate whose extensions no decoder of WIC names, for the formats of the
/// plugin.
pub fn missing(wic: &[Format]) -> Vec<&'static Format> {
    FORMATS
        .iter()
        .map(|(_, _, f)| f)
        .filter(|f| {
            !wic.iter()
                .any(|w| w.extensions.iter().any(|e| f.extensions.contains(e)))
        })
        .collect()
}

/// The image of a file or of bytes by the decoder of its format among the ones of the crate: by
/// the extension of a file, else by its first bytes.
pub fn decode(source: &Source, options: Options) -> Result<Decoded, String> {
    let bytes = match source {
        Source::File(path) => {
            let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
            if size > LARGEST {
                return Err(format!("larger than {} MB", LARGEST >> 20));
            }
            std::fs::read(path).map_err(|e| e.to_string())?
        }
        Source::Bytes(bytes) => bytes.to_vec(),
    };
    let by_name = match source {
        Source::File(path) => ImageFormat::from_path(path).ok(),
        Source::Bytes(_) => None,
    };
    let reader = ImageReader::new(Cursor::new(bytes.as_slice()));
    let reader = match by_name.filter(|f| FORMATS.iter().any(|(g, _, _)| g == f)) {
        Some(format) => {
            let mut r = reader;
            r.set_format(format);
            r
        }
        None => reader.with_guessed_format().map_err(|e| e.to_string())?,
    };
    let Some(short) = reader
        .format()
        .and_then(|f| FORMATS.iter().find(|(g, _, _)| *g == f).map(|(_, s, _)| *s))
    else {
        return Err("no decoder of the crate image for this file".to_owned());
    };
    let image = reader.decode().map_err(|e| e.to_string())?;
    let size = [image.width(), image.height()];
    if size[0] == 0 || size[1] == 0 {
        return Err("the image is empty".to_owned());
    }
    let to = within(size, options.budget);
    let rgba = if to == size {
        image.into_rgba8()
    } else {
        image
            .resize_exact(to[0], to[1], FilterType::Triangle)
            .into_rgba8()
    };
    let picture = Picture {
        pixels: rgba.into_raw(),
        size: to,
    };
    Ok(Decoded {
        format: format!("{short} (image)"),
        size,
        kind: Kind::Still {
            levels: picture.levels(),
            page: 0,
        },
        count: 1,
        turned: false,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wwrt::image::{DynamicImage, RgbaImage};

    use super::*;

    const OPTIONS: Options = Options {
        budget: 256_000_000,
        orient: true,
    };

    /// An image of 5 x 3 of the crate in a format.
    fn encoded(format: ImageFormat) -> Vec<u8> {
        let picture = RgbaImage::from_fn(5, 3, |x, y| {
            wwrt::image::Rgba([(x * 50) as u8, (y * 80) as u8, 120, 255])
        });
        let image = match format {
            ImageFormat::Hdr => {
                DynamicImage::ImageRgb32F(DynamicImage::ImageRgba8(picture).to_rgb32f())
            }
            ImageFormat::Pnm => {
                DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(picture).to_rgb8())
            }
            ImageFormat::Farbfeld => {
                DynamicImage::ImageRgba16(DynamicImage::ImageRgba8(picture).to_rgba16())
            }
            _ => DynamicImage::ImageRgba8(picture),
        };
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    #[test]
    fn every_format_of_the_crate_decodes_from_bytes_and_files() {
        let dir = std::env::temp_dir().join(format!("wv-image-other-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (format, short, f) in &FORMATS {
            let bytes = encoded(*format);
            let path = dir.join(format!("a.{}", f.extensions[0]));
            std::fs::write(&path, &bytes).unwrap();
            let decoded = decode(&Source::File(path), OPTIONS).unwrap();
            assert_eq!(decoded.size, [5, 3], "{short}");
            assert_eq!(decoded.format, format!("{short} (image)"));
            if *format != ImageFormat::Tga {
                // A TGA has no magic: bytes alone do not name it.
                let from_bytes = decode(&Source::Bytes(Arc::from(bytes)), OPTIONS).unwrap();
                assert_eq!(from_bytes.size, [5, 3], "{short} from its bytes");
            }
        }
        let png = dir.join("a.qoi");
        std::fs::write(&png, b"not an image").unwrap();
        assert!(decode(&Source::File(png), OPTIONS).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_formats_windows_decodes_are_left_to_it() {
        // The name of a decoder of WIC is its own, no text of the plugin.
        let wic = [Format {
            name: concat!("WebP ", "Decoder"),
            extensions: &["webp"],
            mimes: &["image/webp"],
        }];
        let left: Vec<&str> = missing(&wic).iter().map(|f| f.name).collect();
        assert!(
            !left.contains(&"WebP images") && left.contains(&"TGA images"),
            "{left:?}"
        );
    }
}
