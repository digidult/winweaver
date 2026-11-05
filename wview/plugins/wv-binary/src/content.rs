//! What a node of bytes holds, for the plugin of its type (`wwrt::plugin::Host::open_inside`): the
//! MIME type its description names (`content` of a field, `format::Content`) or its first bytes
//! show (`wwrt::mime::sniff`), and the bytes made a file of that type where a description names
//! one of the forms of Windows that lack the header of their file:
//!
//! - `image/x-dib`: a bitmap of the resources of a program, its header of information, colours and
//!   pixels; the header of a BMP file comes before it.
//! - `image/x-dib-icon`: an image of an icon, a bitmap of twice its height with its mask, or a
//!   PNG; the bitmap becomes an ICO file of one image, the PNG stays as it is.
//! - `image/x-dib-cursor`: the image of a cursor, its hot spot (four bytes) before an icon.

use wwrt::mime;

pub const DIB: &str = "image/x-dib";
pub const DIB_ICON: &str = "image/x-dib-icon";
pub const DIB_CURSOR: &str = "image/x-dib-cursor";

/// The most bytes of a node shown by the plugin of their type, in MB.
pub const LARGEST_MB: u64 = 64;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The MIME type the bytes of a node show as: the form of a description made a file of its own
/// type, the type a description names, or the one of its first bytes.
pub fn shown_type(declared: Option<&str>, first: &[u8]) -> Option<String> {
    match declared.map(mime::canonical).as_deref() {
        Some(DIB) => dib_header(first).map(|_| "image/bmp".to_owned()),
        Some(DIB_ICON) => icon_type(first),
        Some(DIB_CURSOR) => icon_type(first.get(4..)?),
        Some(_) => declared.map(str::to_owned),
        None => mime::sniff(first).map(str::to_owned),
    }
}

fn icon_type(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(PNG) {
        Some("image/png".to_owned())
    } else {
        dib_header(bytes).map(|_| "image/x-icon".to_owned())
    }
}

/// The bytes of a node made a file of the type they show as, with that type.
pub fn prepared(declared: Option<&str>, bytes: Vec<u8>) -> Result<(String, Vec<u8>), String> {
    let Some(shown) = shown_type(declared, &bytes) else {
        return Err("no type of a file".to_owned());
    };
    let form = declared.map(mime::canonical);
    let bytes = match form.as_deref() {
        Some(DIB) => bmp_file(&bytes)?,
        Some(DIB_ICON) if shown != "image/png" => ico_file(&bytes)?,
        Some(DIB_CURSOR) if shown != "image/png" => ico_file(&bytes[4..])?,
        Some(DIB_CURSOR) => bytes[4..].to_vec(),
        _ => bytes,
    };
    Ok((shown, bytes))
}

/// The header of a bitmap without its file: its size, its bits a pixel, and where its pixels
/// start after the header, its masks and its colours.
struct DibHeader {
    width: i32,
    height: i32,
    bits: u16,
    pixels: usize,
}

fn dib_header(bytes: &[u8]) -> Option<DibHeader> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let size = u32_at(0)? as usize;
    if size == 12 {
        // BITMAPCOREHEADER: sizes of 16 bits, colours of three bytes.
        let bits = u16_at(10)?;
        let colours = if bits <= 8 { 1usize << bits } else { 0 };
        return (u16_at(8)? == 1 && bits > 0).then_some(DibHeader {
            width: i32::from(u16_at(4)?),
            height: i32::from(u16_at(6)?),
            bits,
            pixels: 12 + colours * 3,
        });
    }
    if !matches!(size, 40 | 52 | 56 | 64 | 108 | 124) || bytes.len() < size {
        return None;
    }
    let (planes, bits) = (u16_at(12)?, u16_at(14)?);
    if planes != 1 || !matches!(bits, 1 | 2 | 4 | 8 | 16 | 24 | 32) {
        return None;
    }
    let compression = u32_at(16)?;
    let used = u32_at(32)? as usize;
    // BI_BITFIELDS and BI_ALPHABITFIELDS put their masks after a header of 40 bytes.
    let masks = match (size, compression) {
        (40, 3) => 12,
        (40, 6) => 16,
        _ => 0,
    };
    let colours = match used {
        0 if bits <= 8 => 1usize << bits,
        n => n.min(256),
    };
    Some(DibHeader {
        width: u32_at(4)? as i32,
        height: u32_at(8)? as i32,
        bits,
        pixels: size + masks + colours * 4,
    })
}

/// A BMP file of a bitmap without its file: its header of 14 bytes before it.
pub fn bmp_file(dib: &[u8]) -> Result<Vec<u8>, String> {
    let header = dib_header(dib).ok_or("not a bitmap of Windows")?;
    let total = u32::try_from(14 + dib.len()).map_err(|_| "a bitmap too large")?;
    let mut out = Vec::with_capacity(14 + dib.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&total.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(14 + header.pixels as u32).to_le_bytes());
    out.extend_from_slice(dib);
    Ok(out)
}

/// An ICO file of one image, of the bitmap of an icon (twice its height, with its mask).
pub fn ico_file(dib: &[u8]) -> Result<Vec<u8>, String> {
    let header = dib_header(dib).ok_or("not an image of an icon")?;
    let side = |v: i32| match v.unsigned_abs() {
        n @ 1..=255 => n as u8,
        _ => 0,
    };
    let colours = if header.bits < 8 {
        1u8 << header.bits
    } else {
        0
    };
    let size = u32::try_from(dib.len()).map_err(|_| "an icon too large")?;
    let mut out = Vec::with_capacity(22 + dib.len());
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    out.extend_from_slice(&[side(header.width), side(header.height / 2), colours, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&header.bits.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(dib);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bitmap of 2 x 2 at 8 bits with its four colours, the form of a BITMAP resource.
    fn dib(height: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(40u32.to_le_bytes());
        out.extend(2i32.to_le_bytes());
        out.extend(height.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(8u16.to_le_bytes());
        out.extend([0u8; 16]);
        out.extend(4u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend([0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0]);
        out.extend([0, 1, 0, 0, 2, 3, 0, 0]);
        out
    }

    #[test]
    fn a_bitmap_of_a_resource_becomes_a_bmp_file() {
        let bytes = dib(2);
        assert_eq!(shown_type(Some(DIB), &bytes).as_deref(), Some("image/bmp"));
        let (mime, file) = prepared(Some(DIB), bytes.clone()).unwrap();
        assert_eq!(mime, "image/bmp");
        assert_eq!(&file[..2], b"BM");
        assert_eq!(
            u32::from_le_bytes(file[2..6].try_into().unwrap()) as usize,
            file.len()
        );
        // The pixels start after the file header, the header of 40 bytes and four colours.
        assert_eq!(
            u32::from_le_bytes(file[10..14].try_into().unwrap()),
            14 + 40 + 16
        );
        assert_eq!(wwrt::mime::sniff(&file), Some("image/bmp"));
        assert!(prepared(Some(DIB), b"not a bitmap".to_vec()).is_err());
    }

    #[test]
    fn an_image_of_an_icon_becomes_an_ico_file_or_stays_a_png() {
        let (mime, file) = prepared(Some(DIB_ICON), dib(4)).unwrap();
        assert_eq!(mime, "image/x-icon");
        assert_eq!(&file[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!((file[6], file[7]), (2, 2), "the mask is half of the height");
        assert_eq!(u32::from_le_bytes(file[18..22].try_into().unwrap()), 22);
        assert_eq!(wwrt::mime::sniff(&file), Some("image/x-icon"));
        let mut png = PNG.to_vec();
        png.extend([0; 8]);
        let (mime, same) = prepared(Some(DIB_ICON), png.clone()).unwrap();
        assert_eq!((mime.as_str(), same), ("image/png", png.clone()));
        let mut cursor = vec![1, 0, 2, 0];
        cursor.extend(&png);
        assert_eq!(prepared(Some(DIB_CURSOR), cursor).unwrap().1, png);
    }

    #[test]
    fn a_type_named_or_found_by_the_first_bytes() {
        assert_eq!(
            shown_type(Some("application/xml"), b"<?xml").as_deref(),
            Some("application/xml")
        );
        assert_eq!(shown_type(None, PNG).as_deref(), Some("image/png"));
        assert_eq!(shown_type(None, &[0, 1, 2, 3]), None);
    }
}
