//! The regions of this Windows (`resources::Regions`): every nation `EnumSystemGeoNames` lists,
//! with its GeoID and its name in the language of Windows, for the user region of a profile.

use windows_sys::Win32::Globalization::{
    EnumSystemGeoNames, GEO_FRIENDLYNAME, GEO_ID, GEOCLASS_NATION, GetGeoInfoEx,
};
use winweaver_core::resources::{Region, Regions};

use crate::ffi::{wide, wide_text};

/// The regions, by their names.
pub fn read() -> Regions {
    let mut regions: Vec<Region> = names().into_iter().filter_map(region).collect();
    regions.sort_by_cached_key(|r| r.name.to_lowercase());
    Regions { regions }
}

/// The names of the nations Windows lists: their two-letter codes.
fn names() -> Vec<String> {
    unsafe extern "system" fn each(name: *const u16, data: isize) -> i32 {
        // SAFETY: `data` is the address of the vector `names` passes, alive during the whole
        // enumeration, and `name` a null-terminated string Windows hands to the callback.
        unsafe {
            let names = &mut *(data as *mut Vec<String>);
            names.push(wide_text(name));
        }
        1
    }
    let mut names: Vec<String> = Vec::new();
    let data = &mut names as *mut Vec<String> as isize;
    // SAFETY: the callback only pushes into `names`, which outlives the call.
    unsafe {
        EnumSystemGeoNames(GEOCLASS_NATION as u32, Some(each), data);
    }
    names
}

/// A text about a region, empty when Windows has none.
fn info(name: &[u16], kind: i32) -> String {
    let mut buffer = [0u16; 256];
    let (out, len) = (buffer.as_mut_ptr(), buffer.len() as i32);
    // SAFETY: the name is null-terminated and the buffer valid for the length passed.
    let written = unsafe { GetGeoInfoEx(name.as_ptr(), kind, out, len) };
    if written <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..written as usize - 1])
}

/// The region of a two-letter code with its GeoID and its name; none for a code of another
/// form (the numeric codes of the UN name regions, not nations) or without an id or a name.
fn region(code: String) -> Option<Region> {
    if code.len() != 2 || !code.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let name = wide(&code);
    let id: u32 = info(&name, GEO_ID).trim().parse().ok()?;
    let title = info(&name, GEO_FRIENDLYNAME);
    if title.is_empty() {
        return None;
    }
    Some(Region {
        code: code.to_ascii_uppercase(),
        id,
        name: title,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_regions_of_this_windows_are_read() {
        let r = super::read();
        assert!(r.regions.len() > 100, "{} regions", r.regions.len());
        let ua = r.by_code("UA").unwrap();
        assert_eq!(ua.id, 241);
        assert!(!ua.name.is_empty());
        assert_eq!(r.by_id("244").unwrap().code, "US");
        assert!(r.find("XX").is_none());
    }
}
