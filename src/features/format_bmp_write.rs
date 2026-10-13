//! Feature format.bmp_write: uncompressed Windows bitmap encoder (1, 4, 8 and
//! 24 bits per pixel, bottom-up, 40-byte info header).
//!
//! Hook for file.save / format.color_reduction: call [`encode_bmp`] with the
//! document pixels, the chosen depth and the palette to write (for indexed
//! depths), or [`encode_bmp_indexed`] when the caller already holds palette
//! indices. [`keeps_source_palette`] states the "same depth keeps the palette"
//! rule from the spec.

use crate::registry::Registry;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// Pixels per metre written for both resolutions (96 dpi).
const PIXELS_PER_METRE: u32 = 3780;

/// Padded byte length of one pixel row.
pub(crate) fn row_bytes(width: u32, depth: u32) -> usize {
    (width as usize * depth as usize).div_ceil(32) * 4
}

/// Whether a document with `source_depth` and `palette` is saved at `depth`
/// with its own palette unchanged.
pub(crate) fn keeps_source_palette(source_depth: u32, palette: &Option<Vec<Rgb>>, depth: u32) -> bool {
    depth <= 8 && source_depth == depth && palette.as_ref().is_some_and(|p| !p.is_empty())
}

/// Index of the palette entry equal to `c`, else the nearest by squared RGB
/// distance; ties go to the lower index. `None` for an empty palette.
pub(crate) fn nearest_index(palette: &[Rgb], c: Rgb) -> Option<u8> {
    let d = |p: &Rgb| {
        let (dr, dg, db) = (p.r as i32 - c.r as i32, p.g as i32 - c.g as i32, p.b as i32 - c.b as i32);
        dr * dr + dg * dg + db * db
    };
    palette.iter().take(256).enumerate().min_by_key(|(i, p)| (d(p), *i)).map(|(i, _)| i as u8)
}

fn check(width: u32, height: u32, pixels: usize) -> HResult<()> {
    if width == 0 || height == 0 || pixels != width as usize * height as usize {
        return Err(HarnessError::Invalid("bitmap pixel buffer does not match its size".into()));
    }
    Ok(())
}

fn header(width: u32, height: u32, depth: u32, palette_len: usize, data_len: usize) -> Vec<u8> {
    let off = 54 + 4 * palette_len;
    let mut v = Vec::with_capacity(off + data_len);
    v.extend_from_slice(b"BM");
    v.extend_from_slice(&((off + data_len) as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(off as u32).to_le_bytes());
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(depth as u16).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(data_len as u32).to_le_bytes());
    v.extend_from_slice(&PIXELS_PER_METRE.to_le_bytes());
    v.extend_from_slice(&PIXELS_PER_METRE.to_le_bytes());
    v.extend_from_slice(&(palette_len as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

/// Encode an image (rows top to bottom) at `depth` 1, 4, 8 or 24. Indexed
/// depths need a non-empty `palette` (at most `2^depth` entries are written);
/// each pixel maps to its exact or nearest palette entry. 24-bit ignores the
/// palette.
pub(crate) fn encode_bmp(pixels: &[Rgb], width: u32, height: u32, depth: u32, palette: &[Rgb]) -> HResult<Vec<u8>> {
    check(width, height, pixels.len())?;
    if depth == 24 {
        let rb = row_bytes(width, 24);
        let mut data = Vec::with_capacity(rb * height as usize);
        for row in pixels.chunks(width as usize).rev() {
            let start = data.len();
            for c in row {
                data.extend_from_slice(&[c.b, c.g, c.r]);
            }
            data.resize(start + rb, 0);
        }
        let mut out = header(width, height, 24, 0, data.len());
        out.extend_from_slice(&data);
        return Ok(out);
    }
    if !matches!(depth, 1 | 4 | 8) {
        return Err(HarnessError::Invalid(format!("bitmap depth {depth} is not supported")));
    }
    let pal = &palette[..palette.len().min(1usize << depth)];
    let idx = pixels
        .iter()
        .map(|c| nearest_index(pal, *c).ok_or_else(|| HarnessError::Invalid("indexed bitmap needs a palette".into())))
        .collect::<HResult<Vec<u8>>>()?;
    encode_bmp_indexed(&idx, width, height, depth, pal)
}

/// Encode palette indices (rows top to bottom) at `depth` 1, 4 or 8 with
/// `palette` written verbatim. Indices that do not fit the depth are an error.
pub(crate) fn encode_bmp_indexed(indices: &[u8], width: u32, height: u32, depth: u32, palette: &[Rgb]) -> HResult<Vec<u8>> {
    check(width, height, indices.len())?;
    if !matches!(depth, 1 | 4 | 8) {
        return Err(HarnessError::Invalid(format!("bitmap depth {depth} is not an indexed depth")));
    }
    if palette.is_empty() || palette.len() > 1usize << depth {
        return Err(HarnessError::Invalid("palette size does not fit the bitmap depth".into()));
    }
    if indices.iter().any(|&i| (i as u32) >= (1u32 << depth)) {
        return Err(HarnessError::Invalid("palette index does not fit the bitmap depth".into()));
    }
    let rb = row_bytes(width, depth);
    let mut data = vec![0u8; rb * height as usize];
    for (y, row) in indices.chunks(width as usize).enumerate() {
        let out = &mut data[(height as usize - 1 - y) * rb..][..rb];
        for (x, &v) in row.iter().enumerate() {
            let bit = x * depth as usize;
            out[bit / 8] |= v << (8 - depth as usize - bit % 8);
        }
    }
    // The table always has 2^depth entries (unused ones black), and colours
    // used says so (format.bmp_write B2, B3).
    let entries = 1usize << depth;
    let mut out = header(width, height, depth, entries, data.len());
    for c in palette.iter().copied().chain(std::iter::repeat(Rgb::K)).take(entries) {
        out.extend_from_slice(&[c.b, c.g, c.r, 0]);
    }
    out.extend_from_slice(&data);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::format_bmp_read::decode_bmp;

    #[test]
    fn short_palette_is_padded_to_full_table() {
        let pal: Vec<Rgb> = (1..=5).map(|i| Rgb::new(i * 10, i, 255 - i)).collect();
        let b = encode_bmp_indexed(&[0, 1, 2, 3, 4, 0, 1, 2], 4, 2, 8, &pal).unwrap();
        let le32 = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        assert_eq!(le32(10), 54 + 4 * 256);
        assert_eq!(le32(46), 256);
        assert_eq!(b.len(), 54 + 4 * 256 + 4 * 2);
        for (i, c) in pal.iter().enumerate() {
            assert_eq!(&b[54 + 4 * i..54 + 4 * i + 4], &[c.b, c.g, c.r, 0]);
        }
        assert_eq!(&b[54 + 20..54 + 24], &[0, 0, 0, 0]);
        // the first five entries survive a read
        let d = decode_bmp(&b).unwrap();
        assert_eq!(d.palette.unwrap()[..5], pal[..]);
    }

    #[test]
    fn roundtrip_all_depths() {
        let pal = [Rgb::K, Rgb::W];
        let px: Vec<Rgb> = (0..21).map(|i| pal[i % 2]).collect();
        for depth in [1u32, 4, 8] {
            let b = encode_bmp(&px, 7, 3, depth, &pal).unwrap();
            let d = decode_bmp(&b).unwrap();
            assert_eq!((d.image.pixels, d.depth), (px.clone(), depth));
        }
        let rgb: Vec<Rgb> = (0..15).map(|i| Rgb::new(i * 10, i * 3, 255 - i)).collect();
        let b = encode_bmp(&rgb, 5, 3, 24, &[]).unwrap();
        assert_eq!(decode_bmp(&b).unwrap().image.pixels, rgb);
    }
}
