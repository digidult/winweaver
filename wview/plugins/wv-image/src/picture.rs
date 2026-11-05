//! Pictures in memory: RGBA pixels with their size, the orientation of EXIF, the levels of a large
//! picture (each half of the one before) and the size an image is decoded at within a budget.

/// The pixels of a picture, RGBA with straight alpha, row after row.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub pixels: Vec<u8>,
    pub size: [u32; 2],
}

/// The side the last level of a picture fits in: one texture of every renderer, and a tile
/// grid of at most two by two.
pub const TOP: u32 = 2048;

impl Picture {
    /// The picture as the orientation of EXIF (the tag 274) says it was seen: 2 mirrored, 3 turned
    /// half round, 4 upside down mirrored, 5 transposed, 6 turned a quarter clockwise, 7
    /// transversed, 8 a quarter anticlockwise; 1 and an unknown value leave it as it is.
    pub fn oriented(self, orientation: u16) -> Picture {
        if !(2..=8).contains(&orientation) {
            return self;
        }
        let [w, h] = self.size;
        let size = if orientation >= 5 { [h, w] } else { [w, h] };
        let mut pixels = vec![0u8; self.pixels.len()];
        for y in 0..h {
            for x in 0..w {
                let (nx, ny) = match orientation {
                    2 => (w - 1 - x, y),
                    3 => (w - 1 - x, h - 1 - y),
                    4 => (x, h - 1 - y),
                    5 => (y, x),
                    6 => (h - 1 - y, x),
                    7 => (h - 1 - y, w - 1 - x),
                    _ => (y, w - 1 - x),
                };
                let from = ((y * w + x) * 4) as usize;
                let to = ((ny * size[0] + nx) * 4) as usize;
                pixels[to..to + 4].copy_from_slice(&self.pixels[from..from + 4]);
            }
        }
        Picture { pixels, size }
    }

    /// The picture at half its size (a side of an odd length rounds up), every pixel the mean of
    /// the two by two it covers, weighted by their alpha.
    pub fn half(&self) -> Picture {
        let [w, h] = self.size;
        let size = [w.div_ceil(2), h.div_ceil(2)];
        let mut pixels = Vec::with_capacity((size[0] * size[1] * 4) as usize);
        for y in 0..size[1] {
            for x in 0..size[0] {
                let mut sum = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
                    let at = ((sy * w + sx) * 4) as usize;
                    let a = u32::from(self.pixels[at + 3]);
                    for (c, s) in sum.iter_mut().take(3).enumerate() {
                        *s += u32::from(self.pixels[at + c]) * a;
                    }
                    sum[3] += a;
                }
                // A pixel of no alpha has no colour.
                let alpha = sum[3];
                for s in &sum[..3] {
                    pixels.push((s + alpha / 2).checked_div(alpha).unwrap_or(0) as u8);
                }
                pixels.push(alpha.div_ceil(4).min(255) as u8);
            }
        }
        Picture { pixels, size }
    }

    /// The levels of the picture: itself, then each half of the one before, until one fits in
    /// `TOP` pixels a side.
    pub fn levels(self) -> Vec<Picture> {
        let mut out = vec![self];
        while let Some(last) = out.last()
            && last.size[0].max(last.size[1]) > TOP
        {
            let next = last.half();
            out.push(next);
        }
        out
    }
}

/// The size an image of `size` is decoded at with at most `budget` pixels: its own, or smaller in
/// both sides by one factor.
pub fn within(size: [u32; 2], budget: u64) -> [u32; 2] {
    let [w, h] = size;
    let area = u64::from(w) * u64::from(h);
    if area <= budget || area == 0 {
        return size;
    }
    let k = (budget as f64 / area as f64).sqrt();
    let mut w = ((f64::from(w) * k) as u32).max(1);
    let mut h = ((f64::from(h) * k) as u32).max(1);
    // A side that could not shrink below one pixel leaves the other the whole budget.
    if u64::from(w) * u64::from(h) > budget {
        if w > h {
            w = (budget / u64::from(h)).clamp(1, u64::from(u32::MAX)) as u32;
        } else {
            h = (budget / u64::from(w)).clamp(1, u64::from(u32::MAX)) as u32;
        }
    }
    [w, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture whose pixels are numbered: the red channel of each is its index.
    fn numbered(w: u32, h: u32) -> Picture {
        let pixels = (0..w * h).flat_map(|i| [i as u8, 0, 0, 255]).collect();
        Picture {
            pixels,
            size: [w, h],
        }
    }

    fn rows(p: &Picture) -> Vec<Vec<u8>> {
        p.pixels
            .chunks(4)
            .map(|c| c[0])
            .collect::<Vec<_>>()
            .chunks(p.size[0] as usize)
            .map(<[u8]>::to_vec)
            .collect()
    }

    #[test]
    fn the_eight_orientations_of_exif() {
        // 0 1 2
        // 3 4 5
        let p = numbered(3, 2);
        let cases: [(u16, Vec<Vec<u8>>); 8] = [
            (1, vec![vec![0, 1, 2], vec![3, 4, 5]]),
            (2, vec![vec![2, 1, 0], vec![5, 4, 3]]),
            (3, vec![vec![5, 4, 3], vec![2, 1, 0]]),
            (4, vec![vec![3, 4, 5], vec![0, 1, 2]]),
            (5, vec![vec![0, 3], vec![1, 4], vec![2, 5]]),
            (6, vec![vec![3, 0], vec![4, 1], vec![5, 2]]),
            (7, vec![vec![5, 2], vec![4, 1], vec![3, 0]]),
            (8, vec![vec![2, 5], vec![1, 4], vec![0, 3]]),
        ];
        for (o, want) in cases {
            let turned = p.clone().oriented(o);
            assert_eq!(rows(&turned), want, "orientation {o}");
            assert_eq!(turned.size[0] as usize, want[0].len(), "orientation {o}");
        }
        assert_eq!(p.clone().oriented(9), p, "an unknown value changes nothing");
    }

    #[test]
    fn a_half_is_the_mean_of_what_it_covers() {
        let p = Picture {
            pixels: vec![
                10, 20, 30, 255, 30, 40, 50, 255, 100, 0, 0, 255, //
                50, 60, 70, 255, 70, 80, 90, 255, 200, 0, 0, 255,
            ],
            size: [3, 2],
        };
        let half = p.half();
        assert_eq!(half.size, [2, 1]);
        assert_eq!(&half.pixels[..4], &[40, 50, 60, 255]);
        assert_eq!(
            &half.pixels[4..],
            &[150, 0, 0, 255],
            "the odd column is its own"
        );
        let clear = Picture {
            pixels: vec![255, 0, 0, 0, 0, 0, 255, 255],
            size: [2, 1],
        };
        assert_eq!(
            clear.half().pixels,
            vec![0, 0, 255, 128],
            "a transparent pixel gives no colour"
        );
    }

    #[test]
    fn the_levels_end_in_one_texture() {
        let p = Picture {
            pixels: vec![0; 5000 * 3 * 4],
            size: [5000, 3],
        };
        let sizes: Vec<[u32; 2]> = p.levels().iter().map(|l| l.size).collect();
        assert_eq!(sizes, vec![[5000, 3], [2500, 2], [1250, 1]]);
        let small = numbered(4, 4);
        assert_eq!(small.levels().len(), 1);
    }

    #[test]
    fn a_large_image_is_decoded_within_the_budget() {
        assert_eq!(within([4000, 3000], 12_000_000), [4000, 3000]);
        let [w, h] = within([40000, 30000], 12_000_000);
        assert_eq!((w, h), (4000, 3000));
        assert!(u64::from(w) * u64::from(h) <= 12_000_000);
        assert_eq!(within([100000, 1], 10), [10, 1], "never a side of zero");
    }
}
