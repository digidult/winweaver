//! Image files made in memory for the tests of the plugin and of a program that takes it as a
//! library (the feature `library`): a BMP of any size and a GIF of several frames.

/// A BMP of 24 bits, the colour of each pixel by `pixel`.
pub fn bmp(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let stride = (w * 3 + 3) & !3;
    let size = stride * h;
    let mut out = Vec::with_capacity(54 + size as usize);
    out.extend_from_slice(b"BM");
    for v in [54 + size, 0, 54, 40] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    for v in [0u32, size, 2835, 2835, 0, 0] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for y in (0..h).rev() {
        let start = out.len();
        for x in 0..w {
            let [r, g, b] = pixel(x, y);
            out.extend_from_slice(&[b, g, r]);
        }
        out.resize(start + stride as usize, 0);
    }
    out
}

/// A frame of a GIF: its place on the canvas, its pixels as indexes of the palette, its delay in
/// hundredths of a second, its disposal and its transparent index.
pub struct GifFrame {
    pub left: u16,
    pub top: u16,
    pub size: [u16; 2],
    pub indexes: Vec<u8>,
    pub delay: u16,
    pub disposal: u8,
    pub transparent: Option<u8>,
}

impl GifFrame {
    pub fn new(
        left: u16,
        top: u16,
        size: [u16; 2],
        indexes: Vec<u8>,
        delay: u16,
        disposal: u8,
        transparent: Option<u8>,
    ) -> GifFrame {
        GifFrame {
            left,
            top,
            size,
            indexes,
            delay,
            disposal,
            transparent,
        }
    }
}

/// The codes of LZW, each of nine bits, packed from the lowest bit.
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, code: u32) {
        self.acc |= code << self.n;
        self.n += 9;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
}

/// The indexes as LZW of a code size of 8 without compression: every index a code of nine bits,
/// and a clear code before the table of the decoder grows to ten bits.
fn lzw(indexes: &[u8]) -> Vec<u8> {
    const CLEAR: u32 = 256;
    const END: u32 = 257;
    let mut bits = Bits {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    bits.put(CLEAR);
    for (i, &index) in indexes.iter().enumerate() {
        if i > 0 && i % 200 == 0 {
            bits.put(CLEAR);
        }
        bits.put(u32::from(index));
    }
    bits.put(END);
    if bits.n > 0 {
        bits.out.push(bits.acc as u8);
    }
    bits.out
}

/// A GIF of a canvas of `screen`, a palette of up to 256 colours and the frames, played forever.
pub fn gif(screen: [u16; 2], palette: &[[u8; 3]], frames: &[GifFrame]) -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    for v in screen {
        out.extend_from_slice(&v.to_le_bytes());
    }
    // A global palette of 256 colours, the background its first.
    out.extend_from_slice(&[0xF7, 0, 0]);
    for i in 0..256 {
        out.extend_from_slice(palette.get(i).unwrap_or(&[0, 0, 0]));
    }
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);
    for frame in frames {
        let flags = ((frame.disposal & 7) << 2) | u8::from(frame.transparent.is_some());
        out.extend_from_slice(&[0x21, 0xF9, 0x04, flags]);
        out.extend_from_slice(&frame.delay.to_le_bytes());
        out.extend_from_slice(&[frame.transparent.unwrap_or(0), 0x00, 0x2C]);
        for v in [frame.left, frame.top, frame.size[0], frame.size[1]] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&[0x00, 0x08]);
        for block in lzw(&frame.indexes).chunks(255) {
            out.push(block.len() as u8);
            out.extend_from_slice(block);
        }
        out.push(0x00);
    }
    out.push(0x3B);
    out
}
