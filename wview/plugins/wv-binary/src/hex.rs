//! The hexadecimal view: rows of the offset, the bytes (a space more after every eight) and their
//! ASCII, read from the file for the rows in view alone; the range of the node selected is marked,
//! and a click on a byte gives its offset.

use std::ops::Range;

use wwrt::egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Sense, Vec2};
use wwrt::ww::style;

use crate::engine::Source;

/// The column (in characters from the start of the bytes) of byte `i` of a row.
pub fn column(i: u64) -> u64 {
    i * 3 + i / 8
}

/// The characters of the bytes of a row, with the space after the last one.
pub fn width(per_row: u64) -> u64 {
    column(per_row.max(1) - 1) + 3
}

/// The byte of a row under a column of characters of its bytes, or of its ASCII after them.
pub fn byte_at(col: f32, per_row: u64) -> Option<u64> {
    if col < 0.0 {
        return None;
    }
    let col = col as u64;
    let ascii = width(per_row) + 1;
    if col >= ascii {
        let i = col - ascii;
        return (i < per_row).then_some(i);
    }
    (0..per_row).find(|&i| col >= column(i) && col < column(i) + 2)
}

/// The text of the bytes of a row and of their ASCII (a dot for a byte that is no character).
pub fn row_text(bytes: &[u8], per_row: u64) -> (String, String) {
    let mut hex = String::with_capacity(width(per_row) as usize);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            hex.push(' ');
            if i % 8 == 0 {
                hex.push(' ');
            }
        }
        hex.push_str(&format!("{b:02X}"));
    }
    let ascii = bytes
        .iter()
        .map(|b| match b {
            0x20..=0x7e => *b as char,
            _ => '.',
        })
        .collect();
    (hex, ascii)
}

/// The digits of the offsets of a file of `len` bytes: 8, or more for a file above 4 GB.
pub fn digits(len: u64) -> usize {
    let needed = (64 - len.max(1).leading_zeros()).div_ceil(4) as usize;
    needed.max(8)
}

/// What the view did in a frame.
#[derive(Default)]
pub struct Output {
    /// A byte clicked.
    pub clicked: Option<u64>,
}

/// The view of a file, with `marked` drawn behind its bytes; `reveal` scrolls to an offset.
pub fn show(
    ui: &mut egui::Ui,
    source: &mut dyn Source,
    per_row: u64,
    marked: Option<Range<u64>>,
    reveal: Option<u64>,
) -> Output {
    let mut out = Output::default();
    let p = style::palette(ui);
    let font = style::mono(ui, 13.0);
    let char_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    let row_height = (font.size * 1.45).round();
    let len = source.len();
    let rows = len.div_ceil(per_row).max(1);
    let digits = digits(len) as f32;
    let bytes_x = (digits + 2.0) * char_width;
    let ascii_x = bytes_x + (width(per_row) + 1) as f32 * char_width;
    let full = Vec2::new(ascii_x + (per_row as f32 + 1.0) * char_width, row_height);
    let mut area = egui::ScrollArea::both()
        .id_salt("wv-binary-hex")
        .auto_shrink(false);
    if let Some(at) = reveal {
        let row = (at / per_row) as f32;
        let view = ui.available_height();
        area = area.vertical_scroll_offset((row * row_height - view / 3.0).max(0.0));
    }
    let shown = rows.min(usize::MAX as u64 / 2) as usize;
    area.show_rows(ui, row_height, shown, |ui, range| {
        let mut buffer = vec![0u8; per_row as usize];
        for row in range {
            let start = row as u64 * per_row;
            let n = source.read(start, &mut buffer);
            let bytes = &buffer[..n];
            let (rect, response) = ui.allocate_exact_size(full, Sense::click());
            let painter = ui.painter_at(rect);
            let left = rect.left();
            if let Some(m) = &marked {
                let from = m.start.max(start);
                let to = m.end.min(start + n as u64);
                if from < to {
                    let (a, b) = (from - start, to - start - 1);
                    let fill = p.sel.gamma_multiply(0.55);
                    let hex = Rect::from_min_max(
                        Pos2::new(
                            left + bytes_x + column(a) as f32 * char_width - 2.0,
                            rect.top(),
                        ),
                        Pos2::new(
                            left + bytes_x + (column(b) + 2) as f32 * char_width + 2.0,
                            rect.bottom(),
                        ),
                    );
                    painter.rect_filled(hex, CornerRadius::same(2), fill);
                    let ascii = Rect::from_min_max(
                        Pos2::new(left + ascii_x + a as f32 * char_width, rect.top()),
                        Pos2::new(left + ascii_x + (b + 1) as f32 * char_width, rect.bottom()),
                    );
                    painter.rect_filled(ascii, CornerRadius::same(2), fill);
                }
            }
            let y = rect.center().y;
            let (hex, ascii) = row_text(bytes, per_row);
            let offset = format!("{start:0width$X}", width = digits as usize);
            let text = |x: f32, s: String, color: Color32| {
                painter.text(
                    Pos2::new(left + x, y),
                    Align2::LEFT_CENTER,
                    s,
                    font.clone(),
                    color,
                );
            };
            text(0.0, offset, p.muted);
            text(bytes_x, hex, p.text);
            text(ascii_x, ascii, p.muted);
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let x = pos.x - left;
                let col = if x >= ascii_x {
                    (width(per_row) + 1) as f32 + (x - ascii_x) / char_width
                } else {
                    (x - bytes_x) / char_width
                };
                if let Some(i) = byte_at(col, per_row)
                    && i < n as u64
                {
                    out.clicked = Some(start + i);
                }
            }
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_has_its_bytes_and_their_ascii() {
        let bytes: Vec<u8> = (0x41..0x51).collect();
        let (hex, ascii) = row_text(&bytes, 16);
        assert_eq!(hex, "41 42 43 44 45 46 47 48  49 4A 4B 4C 4D 4E 4F 50");
        assert_eq!(ascii, "ABCDEFGHIJKLMNOP");
        assert_eq!(row_text(&[0, 0x7f, b'a'], 16).1, "..a");
        assert_eq!(
            hex.len() as u64 + 1,
            width(16),
            "a space after the last byte"
        );
    }

    #[test]
    fn a_column_names_its_byte() {
        assert_eq!(byte_at(0.0, 16), Some(0));
        assert_eq!(byte_at(1.5, 16), Some(0));
        assert_eq!(byte_at(2.5, 16), None, "the space between two bytes");
        assert_eq!(byte_at(25.0, 16), Some(8), "after the space of the middle");
        assert_eq!(byte_at(47.0, 16), Some(15));
        assert_eq!(byte_at(48.5, 16), None, "the space after the last byte");
        let ascii = (width(16) + 1) as f32;
        assert_eq!(byte_at(ascii, 16), Some(0));
        assert_eq!(byte_at(ascii + 15.0, 16), Some(15));
        assert_eq!(byte_at(ascii + 16.0, 16), None);
    }

    #[test]
    fn the_offsets_grow_with_the_file() {
        assert_eq!(digits(0), 8);
        assert_eq!(digits(0xFFFF_FFFF), 8);
        assert_eq!(digits(0x1_0000_0000), 9);
    }
}
