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
//! - `image/x-group-icon; members=ICON`: the directory of a group of icons (`GRPICONDIR`), whose
//!   entries name their images by an id; with the images (`members`, found by the view) it becomes
//!   one ICO file of every size.
//! - `image/x-group-cursor; members=CURSOR`: the directory of a group of cursors, the same way: one
//!   ICO file of the images of the cursors, their hot spots left out, since WIC decodes no cursor.

use wwrt::mime;

pub const DIB: &str = "image/x-dib";
pub const DIB_ICON: &str = "image/x-dib-icon";
pub const DIB_CURSOR: &str = "image/x-dib-cursor";
pub const GROUP_ICON: &str = "image/x-group-icon";
pub const GROUP_CURSOR: &str = "image/x-group-cursor";

/// The images of a group of icons, by the ids its entries name.
pub type Members<'a> = dyn FnMut(u16) -> Option<Vec<u8>> + 'a;

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
        Some(GROUP_ICON) => group_count(first, 1).map(|_| "image/x-icon".to_owned()),
        Some(GROUP_CURSOR) => group_count(first, 2).map(|_| "image/x-icon".to_owned()),
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

/// The count of the entries of the directory of a group, by its header: of icons (`kind` 1) or
/// of cursors (2).
fn group_count(bytes: &[u8], kind: u16) -> Option<usize> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
    let count = u16_at(4)?;
    (u16_at(0)? == 0 && u16_at(2)? == kind && count > 0).then_some(usize::from(count))
}

/// One ICO file of the images of a group of icons: the directory gives the size, the colours, the
/// planes and the bits of each and the id of its image; an image `members` does not give is left
/// out.
pub fn group_icon(dir: &[u8], members: &mut Members) -> Result<Vec<u8>, String> {
    group(dir, 1, members)
}

/// One ICO file of the images of a group of cursors: an entry of the directory gives the width,
/// the height (of the bitmap and its mask), the planes and the bits of an image and its id; the
/// image of a cursor has its hot spot (four bytes) before it, left out.
pub fn group_cursor(dir: &[u8], members: &mut Members) -> Result<Vec<u8>, String> {
    group(dir, 2, members)
}

/// One ICO file of a group of icons (`kind` 1) or of cursors (2).
fn group(dir: &[u8], kind: u16, members: &mut Members) -> Result<Vec<u8>, String> {
    let count = group_count(dir, kind).ok_or("not the directory of a group of images")?;
    if dir.len() < 6 + count * 14 {
        return Err("the directory of the images is cut short".to_owned());
    }
    let side = |v: u16| match v {
        n @ 1..=255 => n as u8,
        _ => 0,
    };
    let mut images = Vec::new();
    for i in 0..count {
        let entry = &dir[6 + i * 14..6 + (i + 1) * 14];
        let id = u16::from_le_bytes([entry[12], entry[13]]);
        let Some(bytes) = members(id) else {
            continue;
        };
        if kind == 1 {
            images.push((entry[..8].to_vec(), bytes));
            continue;
        }
        let Some(image) = bytes.get(4..).filter(|b| !b.is_empty()) else {
            continue;
        };
        let (width, height) = (
            u16::from_le_bytes([entry[0], entry[1]]),
            u16::from_le_bytes([entry[2], entry[3]]),
        );
        // The height of a bitmap counts its mask too; a PNG is its own size.
        let height = if image.starts_with(PNG) {
            height
        } else {
            height / 2
        };
        let mut head = vec![side(width), side(height), 0, 0];
        head.extend_from_slice(&entry[4..8]);
        images.push((head, image.to_vec()));
    }
    if images.is_empty() {
        return Err("no image of the icon".to_owned());
    }
    let n = u16::try_from(images.len()).map_err(|_| "too many images")?;
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&n.to_le_bytes());
    let mut offset = 6 + images.len() * 16;
    for (head, bytes) in &images {
        let size = u32::try_from(bytes.len()).map_err(|_| "an image too large")?;
        let at = u32::try_from(offset).map_err(|_| "an icon too large")?;
        out.extend_from_slice(head);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&at.to_le_bytes());
        offset += bytes.len();
    }
    for (_, bytes) in images {
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// The bytes of a node made a file of the type they show as, with that type; `members` gives the
/// images of a group of icons.
pub fn prepared(
    declared: Option<&str>,
    bytes: Vec<u8>,
    members: &mut Members,
) -> Result<(String, Vec<u8>), String> {
    let Some(shown) = shown_type(declared, &bytes) else {
        return Err("no type of a file".to_owned());
    };
    let form = declared.map(mime::canonical);
    let bytes = match form.as_deref() {
        Some(DIB) => bmp_file(&bytes)?,
        Some(DIB_ICON) if shown != "image/png" => ico_file(&bytes)?,
        Some(DIB_CURSOR) if shown != "image/png" => ico_file(&bytes[4..])?,
        Some(DIB_CURSOR) => bytes[4..].to_vec(),
        Some(GROUP_ICON) => group_icon(&bytes, members)?,
        Some(GROUP_CURSOR) => group_cursor(&bytes, members)?,
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

    /// No images of a group: the forms that need none.
    fn none(_: u16) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn a_group_of_icons_becomes_one_ico_file() {
        let png = {
            let mut p = PNG.to_vec();
            p.extend([0; 8]);
            p
        };
        // Three entries: a bitmap of 2 x 2 (id 1), a PNG of 256 (id 7), an id of no image (9).
        let mut dir = vec![0, 0, 1, 0, 3, 0];
        for (side, bits, id) in [(2u8, 8u16, 1u16), (0, 32, 7), (16, 32, 9)] {
            dir.extend([side, side, 0, 0]);
            dir.extend(1u16.to_le_bytes());
            dir.extend(bits.to_le_bytes());
            dir.extend(0u32.to_le_bytes());
            dir.extend(id.to_le_bytes());
        }
        let declared = Some("image/x-group-icon; members=ICON");
        assert_eq!(shown_type(declared, &dir).as_deref(), Some("image/x-icon"));
        let mut members = |id: u16| match id {
            1 => Some(dib(4)),
            7 => Some(png.clone()),
            _ => None,
        };
        let (mime, file) = prepared(declared, dir.clone(), &mut members).unwrap();
        assert_eq!(mime, "image/x-icon");
        assert_eq!(
            &file[..6],
            &[0, 0, 1, 0, 2, 0],
            "the image of no member is left out"
        );
        let size = |at: usize| u32::from_le_bytes(file[at..at + 4].try_into().unwrap()) as usize;
        let (first, second) = (6usize, 22usize);
        assert_eq!(
            (size(first + 8), size(first + 12)),
            (dib(4).len(), 6 + 2 * 16)
        );
        assert_eq!(size(second + 12), 6 + 2 * 16 + dib(4).len());
        assert_eq!(&file[size(second + 12)..], &png[..]);
        assert_eq!(wwrt::mime::sniff(&file), Some("image/x-icon"));
        assert!(
            prepared(declared, dir, &mut none).is_err(),
            "no image at all"
        );
        assert!(
            group_icon(&[0, 0, 1, 0, 5, 0], &mut none).is_err(),
            "cut short"
        );
    }

    #[test]
    fn a_group_of_cursors_becomes_one_ico_file_without_its_hot_spots() {
        // One entry: a bitmap of 2 x 2 with its mask (a height of 4), 8 bits, the image id 3.
        let mut dir = vec![0, 0, 2, 0, 1, 0];
        dir.extend(2u16.to_le_bytes());
        dir.extend(4u16.to_le_bytes());
        dir.extend(1u16.to_le_bytes());
        dir.extend(8u16.to_le_bytes());
        dir.extend(0u32.to_le_bytes());
        dir.extend(3u16.to_le_bytes());
        let declared = Some("image/x-group-cursor; members=CURSOR");
        assert_eq!(shown_type(declared, &dir).as_deref(), Some("image/x-icon"));
        assert_eq!(
            shown_type(Some("image/x-group-icon"), &dir),
            None,
            "no icons"
        );
        let mut image = vec![7, 0, 9, 0];
        image.extend(dib(4));
        let mut members = |id: u16| (id == 3).then(|| image.clone());
        let (mime, file) = prepared(declared, dir, &mut members).unwrap();
        assert_eq!(mime, "image/x-icon");
        assert_eq!(&file[..10], &[0, 0, 1, 0, 1, 0, 2, 2, 0, 0]);
        assert_eq!(&file[22..], &dib(4)[..], "the hot spot is left out");
    }

    #[test]
    fn a_bitmap_of_a_resource_becomes_a_bmp_file() {
        let bytes = dib(2);
        assert_eq!(shown_type(Some(DIB), &bytes).as_deref(), Some("image/bmp"));
        let (mime, file) = prepared(Some(DIB), bytes.clone(), &mut none).unwrap();
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
        assert!(prepared(Some(DIB), b"not a bitmap".to_vec(), &mut none).is_err());
    }

    #[test]
    fn an_image_of_an_icon_becomes_an_ico_file_or_stays_a_png() {
        let (mime, file) = prepared(Some(DIB_ICON), dib(4), &mut none).unwrap();
        assert_eq!(mime, "image/x-icon");
        assert_eq!(&file[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!((file[6], file[7]), (2, 2), "the mask is half of the height");
        assert_eq!(u32::from_le_bytes(file[18..22].try_into().unwrap()), 22);
        assert_eq!(wwrt::mime::sniff(&file), Some("image/x-icon"));
        let mut png = PNG.to_vec();
        png.extend([0; 8]);
        let (mime, same) = prepared(Some(DIB_ICON), png.clone(), &mut none).unwrap();
        assert_eq!((mime.as_str(), same), ("image/png", png.clone()));
        let mut cursor = vec![1, 0, 2, 0];
        cursor.extend(&png);
        assert_eq!(
            prepared(Some(DIB_CURSOR), cursor, &mut none).unwrap().1,
            png
        );
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
