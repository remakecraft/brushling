//! Feature format.bmp_read: Windows bitmap decoder.
//!
//! Recognised by the leading `BM`. Supports info headers of 12, 40, 108 and
//! 124 bytes; 1/4/8/16/24/32 bits per pixel; compression 0, RLE8, RLE4 and
//! bit fields; bottom-up and top-down rows. [`decode_bmp`] is the pure entry
//! point; the registered decoder wraps it for `State::open_path`.

use crate::doc::MAX_DIM;
use crate::formats::{DecodeError, Decoded, CANNOT_OPEN};
use crate::registry::Registry;
use crate::types::*;

/// Shown for an unknown or unsupported compression value.
pub const NOT_ENOUGH_MEMORY: &str = "There is not enough memory or resources to open this image.";

pub fn register(r: &mut Registry) {
    r.decoder(decode_registered);
}

fn decode_registered(bytes: &[u8]) -> Option<Result<Decoded, DecodeError>> {
    if !bytes.starts_with(b"BM") {
        return None;
    }
    Some(decode_bmp(bytes))
}

fn fail() -> DecodeError {
    DecodeError { message: CANNOT_OPEN.to_owned() }
}

fn le16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Expand an `n`-bit channel value to 8 bits by bit replication.
fn expand(v: u32, n: u32) -> u8 {
    if n == 0 {
        return 0;
    }
    if n >= 8 {
        return (v >> (n - 8)) as u8;
    }
    let mut x = v << (8 - n);
    let mut s = n;
    while s < 8 {
        x |= x >> s;
        s *= 2;
    }
    x as u8
}

/// One channel of a bit-field pixel: lowest set bit position and width.
#[derive(Clone, Copy)]
struct Mask {
    mask: u32,
    shift: u32,
    bits: u32,
}

impl Mask {
    fn new(mask: u32) -> Mask {
        if mask == 0 {
            return Mask { mask, shift: 0, bits: 0 };
        }
        let shift = mask.trailing_zeros();
        let bits = (mask >> shift).trailing_ones();
        Mask { mask, shift, bits }
    }

    fn get(&self, px: u32) -> u8 {
        expand((px & self.mask) >> self.shift, self.bits)
    }
}

/// Decode a bitmap file. The error message is the text shown after the path.
pub(crate) fn decode_bmp(b: &[u8]) -> Result<Decoded, DecodeError> {
    if !b.starts_with(b"BM") {
        return Err(fail());
    }
    let off = le32(b, 10).ok_or_else(fail)? as usize;
    let hsize = le32(b, 14).ok_or_else(fail)? as usize;
    if !matches!(hsize, 12 | 40 | 108 | 124) {
        return Err(fail());
    }
    let (w, h_raw, bpp, comp, colors_used) = if hsize == 12 {
        (
            le16(b, 18).ok_or_else(fail)? as i64,
            le16(b, 20).ok_or_else(fail)? as i64,
            le16(b, 24).ok_or_else(fail)? as u32,
            0u32,
            0u32,
        )
    } else {
        (
            le32(b, 18).ok_or_else(fail)? as i32 as i64,
            le32(b, 22).ok_or_else(fail)? as i32 as i64,
            le16(b, 28).ok_or_else(fail)? as u32,
            le32(b, 30).ok_or_else(fail)?,
            le32(b, 46).ok_or_else(fail)?,
        )
    };
    if comp > 3 {
        return Err(DecodeError { message: NOT_ENOUGH_MEMORY.to_owned() });
    }
    if !matches!(bpp, 1 | 4 | 8 | 16 | 24 | 32) {
        return Err(fail());
    }
    // Compression must fit the depth: RLE8 only 8-bit, RLE4 only 4-bit,
    // bit fields only 16/32-bit.
    let comp_ok = match comp {
        0 => true,
        1 => bpp == 8,
        2 => bpp == 4,
        _ => bpp == 16 || bpp == 32,
    };
    let top_down = h_raw < 0;
    if !comp_ok || w <= 0 || h_raw == 0 || (top_down && (comp == 1 || comp == 2)) {
        return Err(fail());
    }
    let h = h_raw.abs();
    if w > MAX_DIM as i64 || h > MAX_DIM as i64 {
        return Err(fail());
    }
    let (w, h) = (w as usize, h as usize);

    // Bit masks for 16/32-bit images.
    let mut masks_at = 14 + hsize;
    let (rm, gm, bm) = if comp == 3 {
        let at = if hsize >= 108 { 14 + 40 } else { 14 + hsize };
        let m = |i: usize| le32(b, at + 4 * i).ok_or_else(fail);
        let t = (Mask::new(m(0)?), Mask::new(m(1)?), Mask::new(m(2)?));
        if hsize == 40 {
            masks_at += 12;
        }
        t
    } else if bpp == 16 {
        (Mask::new(0x7C00), Mask::new(0x03E0), Mask::new(0x001F))
    } else {
        (Mask::new(0x00FF_0000), Mask::new(0x0000_FF00), Mask::new(0x0000_00FF))
    };

    // Palette (indexed depths only).
    let palette = if bpp <= 8 {
        let entry = if hsize == 12 { 3 } else { 4 };
        let max = 1usize << bpp;
        let n = if hsize == 12 || colors_used == 0 { max } else { (colors_used as usize).min(max) };
        let end = masks_at.checked_add(n * entry).ok_or_else(fail)?;
        let raw = b.get(masks_at..end).ok_or_else(fail)?;
        Some(raw.chunks(entry).map(|c| Rgb::new(c[2], c[1], c[0])).collect::<Vec<_>>())
    } else {
        None
    };

    let data = b.get(off..).ok_or_else(fail)?;
    let lookup = |i: usize| -> Rgb {
        palette.as_ref().and_then(|p| p.get(i)).copied().unwrap_or(Rgb::K)
    };

    // Rows as stored, first stored row first.
    let mut stored: Vec<Rgb> = vec![Rgb::K; w * h];
    match comp {
        1 | 2 => {
            let idx = decode_rle(data, w, h, bpp == 4)?;
            for (d, i) in stored.iter_mut().zip(idx) {
                *d = lookup(i as usize);
            }
        }
        _ => {
            let rb = (w * bpp as usize).div_ceil(32) * 4;
            let need = rb.checked_mul(h).ok_or_else(fail)?;
            if data.len() < need {
                return Err(fail());
            }
            for y in 0..h {
                let row = &data[y * rb..(y + 1) * rb];
                for x in 0..w {
                    stored[y * w + x] = match bpp {
                        1 => lookup(((row[x / 8] >> (7 - x % 8)) & 1) as usize),
                        4 => lookup(((row[x / 2] >> (if x % 2 == 0 { 4 } else { 0 })) & 15) as usize),
                        8 => lookup(row[x] as usize),
                        16 => {
                            let v = u16::from_le_bytes([row[2 * x], row[2 * x + 1]]) as u32;
                            Rgb::new(rm.get(v), gm.get(v), bm.get(v))
                        }
                        24 => Rgb::new(row[3 * x + 2], row[3 * x + 1], row[3 * x]),
                        _ => {
                            let v = u32::from_le_bytes([row[4 * x], row[4 * x + 1], row[4 * x + 2], row[4 * x + 3]]);
                            Rgb::new(rm.get(v), gm.get(v), bm.get(v))
                        }
                    };
                }
            }
        }
    }

    let pixels = if top_down {
        stored
    } else {
        stored.chunks(w).rev().flatten().copied().collect()
    };
    Ok(Decoded {
        image: Image { width: w as u32, height: h as u32, pixels },
        depth: if bpp <= 8 { bpp } else { 24 },
        palette,
    })
}

/// Expand RLE8 / RLE4 data to one palette index per pixel, rows stored
/// bottom-up. Pixels the stream skips stay index 0. A token cut short by the
/// end of the data is a truncated file.
fn decode_rle(data: &[u8], w: usize, h: usize, four: bool) -> Result<Vec<u8>, DecodeError> {
    let mut out = vec![0u8; w * h];
    let (mut x, mut y) = (0usize, 0usize);
    let mut p = 0usize;
    let put = |x: usize, y: usize, v: u8, out: &mut Vec<u8>| {
        if x < w && y < h {
            out[y * w + x] = v;
        }
    };
    while p < data.len() {
        let n = data[p] as usize;
        let c = *data.get(p + 1).ok_or_else(fail)?;
        p += 2;
        if n > 0 {
            for i in 0..n {
                let v = if four { if i % 2 == 0 { c >> 4 } else { c & 15 } } else { c };
                put(x + i, y, v, &mut out);
            }
            x += n;
            continue;
        }
        match c {
            0 => {
                x = 0;
                y += 1;
            }
            1 => return Ok(out),
            2 => {
                let dx = *data.get(p).ok_or_else(fail)? as usize;
                let dy = *data.get(p + 1).ok_or_else(fail)? as usize;
                p += 2;
                x += dx;
                y += dy;
            }
            count => {
                let count = count as usize;
                let bytes = if four { count.div_ceil(2) } else { count };
                let run = data.get(p..p + bytes).ok_or_else(fail)?;
                for i in 0..count {
                    let v = if four {
                        if i % 2 == 0 { run[i / 2] >> 4 } else { run[i / 2] & 15 }
                    } else {
                        run[i]
                    };
                    put(x + i, y, v, &mut out);
                }
                x += count;
                p += bytes + (bytes & 1);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(w: i32, h: i32, bpp: u16, comp: u32, pal: &[[u8; 4]], data: &[u8]) -> Vec<u8> {
        let off = 54 + pal.len() * 4;
        let mut v = b"BM".to_vec();
        v.extend_from_slice(&((off + data.len()) as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&(off as u32).to_le_bytes());
        v.extend_from_slice(&40u32.to_le_bytes());
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&h.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&bpp.to_le_bytes());
        v.extend_from_slice(&comp.to_le_bytes());
        v.extend_from_slice(&[0; 12]);
        v.extend_from_slice(&(pal.len() as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        pal.iter().for_each(|p| v.extend_from_slice(p));
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn rgbw_bottom_up_and_top_down() {
        let (r, g, b, w) = ([0, 0, 255], [0, 255, 0], [255, 0, 0], [255, 255, 255]);
        let row = |a: [u8; 3], c: [u8; 3]| [a.as_slice(), c.as_slice(), &[0, 0]].concat();
        let up = hdr(2, 2, 24, 0, &[], &[row(b, w), row(r, g)].concat());
        let down = hdr(2, -2, 24, 0, &[], &[row(r, g), row(b, w)].concat());
        let want = vec![Rgb::R, Rgb::G, Rgb::B, Rgb::W];
        assert_eq!(decode_bmp(&up).unwrap().image.pixels, want);
        assert_eq!(decode_bmp(&down).unwrap().image.pixels, want);
    }

    #[test]
    fn rle8_and_errors() {
        let pal = [[0, 0, 0, 0], [0, 0, 255, 0], [0, 255, 0, 0], [255, 0, 0, 0]];
        let data = [2, 3, 2, 0, 0, 0, 3, 1, 1, 2, 0, 0, 0, 1];
        let d = decode_bmp(&hdr(4, 2, 8, 1, &pal, &data)).unwrap();
        assert_eq!(d.depth, 8);
        assert_eq!(d.image.pixels, [Rgb::R, Rgb::R, Rgb::R, Rgb::G, Rgb::B, Rgb::B, Rgb::K, Rgb::K]);
        assert_eq!(decode_bmp(&hdr(2, 2, 24, 4, &[], &[0; 4])).unwrap_err().message, NOT_ENOUGH_MEMORY);
        assert_eq!(decode_bmp(&hdr(2, 2, 24, 0, &[], &[0; 15])).unwrap_err().message, CANNOT_OPEN);
        assert!(decode_registered(b"\x89PNG").is_none());
    }

    #[test]
    fn sixteen_bit_555() {
        let d = decode_bmp(&hdr(3, 1, 16, 0, &[], &[0x00, 0x7C, 0x1F, 0x00, 0x00, 0x02, 0, 0])).unwrap();
        assert_eq!(d.image.pixels, [Rgb::R, Rgb::B, Rgb::new(0, 132, 0)]);
        assert_eq!((d.depth, d.palette), (24, None));
    }
}
