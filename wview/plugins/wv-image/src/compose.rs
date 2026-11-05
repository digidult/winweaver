//! The frames of an animation (GIF) drawn on its canvas: each frame is a rectangle of the canvas
//! with its own pixels, drawn over what the frame before left, and its disposal says what the
//! next one finds: everything kept (0 and 1), its rectangle cleared (2) or the canvas as it was
//! before it (3).

use crate::picture::Picture;

/// A frame as the file stores it.
pub struct Frame {
    /// Its place on the canvas.
    pub left: u32,
    pub top: u32,
    pub picture: Picture,
    pub disposal: u8,
}

/// The canvas of an animation.
pub struct Canvas {
    size: [u32; 2],
    pixels: Vec<u8>,
    /// What the frame drawn last leaves the next one: its rectangle with the disposal, and for 3
    /// the pixels it covered.
    after: Option<(u8, [u32; 4], Vec<u8>)>,
}

impl Canvas {
    pub fn new(size: [u32; 2]) -> Canvas {
        Canvas {
            size,
            pixels: vec![0; (size[0] * size[1] * 4) as usize],
            after: None,
        }
    }

    /// The rectangle of a frame on the canvas: left, top, right and bottom, cut at its edges.
    fn rect(&self, frame: &Frame) -> [u32; 4] {
        let right = (frame.left + frame.picture.size[0]).min(self.size[0]);
        let bottom = (frame.top + frame.picture.size[1]).min(self.size[1]);
        [frame.left.min(right), frame.top.min(bottom), right, bottom]
    }

    fn row(&self, x: u32, y: u32) -> usize {
        ((y * self.size[0] + x) * 4) as usize
    }

    /// The pixels of a rectangle of the canvas, row after row.
    fn cut(&self, [l, t, r, b]: [u32; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        for y in t..b {
            out.extend_from_slice(&self.pixels[self.row(l, y)..self.row(r, y)]);
        }
        out
    }

    /// Draws the next frame and gives the canvas as it shows then.
    pub fn add(&mut self, frame: &Frame) -> Picture {
        if let Some((disposal, [l, t, r, b], saved)) = self.after.take() {
            match disposal {
                2 => {
                    for y in t..b {
                        let (from, to) = (self.row(l, y), self.row(r, y));
                        self.pixels[from..to].fill(0);
                    }
                }
                3 => {
                    let width = ((r - l) * 4) as usize;
                    for (i, y) in (t..b).enumerate() {
                        let from = self.row(l, y);
                        self.pixels[from..from + width]
                            .copy_from_slice(&saved[i * width..(i + 1) * width]);
                    }
                }
                _ => {}
            }
        }
        let rect = self.rect(frame);
        let saved = if frame.disposal == 3 {
            self.cut(rect)
        } else {
            Vec::new()
        };
        let [l, t, r, b] = rect;
        let width = frame.picture.size[0];
        for y in t..b {
            for x in l..r {
                let from = (((y - frame.top) * width + (x - frame.left)) * 4) as usize;
                let pixel = &frame.picture.pixels[from..from + 4];
                if pixel[3] != 0 {
                    let to = self.row(x, y);
                    self.pixels[to..to + 4].copy_from_slice(pixel);
                }
            }
        }
        self.after = Some((frame.disposal, rect, saved));
        Picture {
            pixels: self.pixels.clone(),
            size: self.size,
        }
    }
}

/// The time a frame shows, in milliseconds, of its delay in hundredths of a second: a delay of 0
/// or 1 shows for 100 ms, as browsers show it.
pub fn delay_ms(hundredths: u32) -> u32 {
    if hundredths <= 1 {
        100
    } else {
        hundredths * 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of one colour (its red channel), with a transparent pixel where `hole` says.
    fn frame(
        left: u32,
        top: u32,
        size: [u32; 2],
        red: u8,
        disposal: u8,
        hole: Option<(u32, u32)>,
    ) -> Frame {
        let mut pixels = Vec::new();
        for y in 0..size[1] {
            for x in 0..size[0] {
                let alpha = if hole == Some((x, y)) { 0 } else { 255 };
                pixels.extend_from_slice(&[red, 0, 0, alpha]);
            }
        }
        Frame {
            left,
            top,
            picture: Picture { pixels, size },
            disposal,
        }
    }

    /// The red channel of every pixel, with 0 where it is transparent.
    fn reds(p: &Picture) -> Vec<u8> {
        p.pixels
            .chunks(4)
            .map(|c| if c[3] == 0 { 0 } else { c[0] })
            .collect()
    }

    #[test]
    fn a_frame_draws_over_what_the_one_before_left() {
        let mut canvas = Canvas::new([3, 1]);
        let first = canvas.add(&frame(0, 0, [3, 1], 10, 1, None));
        assert_eq!(reds(&first), vec![10, 10, 10]);
        let second = canvas.add(&frame(1, 0, [2, 1], 20, 1, Some((1, 0))));
        assert_eq!(
            reds(&second),
            vec![10, 20, 10],
            "a transparent pixel shows the frame before"
        );
    }

    #[test]
    fn the_disposals_clear_or_restore_the_rectangle() {
        let mut canvas = Canvas::new([3, 1]);
        canvas.add(&frame(0, 0, [3, 1], 10, 1, None));
        canvas.add(&frame(1, 0, [1, 1], 20, 2, None));
        let after_clear = canvas.add(&frame(0, 0, [1, 1], 30, 3, None));
        assert_eq!(
            reds(&after_clear),
            vec![30, 0, 10],
            "2 cleared the rectangle of its frame"
        );
        let after_restore = canvas.add(&frame(2, 0, [1, 1], 40, 1, None));
        assert_eq!(
            reds(&after_restore),
            vec![10, 0, 40],
            "3 brought back what its frame covered"
        );
    }

    #[test]
    fn a_frame_past_the_edge_is_cut() {
        let mut canvas = Canvas::new([2, 2]);
        let p = canvas.add(&frame(1, 1, [4, 4], 50, 2, None));
        assert_eq!(reds(&p), vec![0, 0, 0, 50]);
        let p = canvas.add(&frame(5, 5, [1, 1], 60, 1, None));
        assert_eq!(
            reds(&p),
            vec![0, 0, 0, 0],
            "outside the canvas it draws nothing"
        );
    }

    #[test]
    fn a_short_delay_shows_as_browsers_show_it() {
        assert_eq!(delay_ms(0), 100);
        assert_eq!(delay_ms(1), 100);
        assert_eq!(delay_ms(4), 40);
    }
}
