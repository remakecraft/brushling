//! Feature format.color_reduction: saving at 1, 4 or 8 bits maps every pixel
//! to the nearest colour of a fixed palette (no dithering, no optimisation).
//! file.save calls [`reduce`] and asks [`loses_colors`] whether the
//! colour-loss warning is due.

use crate::features::format_bmp_write::nearest_index;
use crate::registry::Registry;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// Text of the warning shown before a lossy save (buttons OK / Cancel).
pub const WARNING: &str = "Saving at this colour depth will permanently change some colours. Continue?";

/// The classic 16 colours, indices 0 to 15.
const BASE16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// The fixed target palette of a depth (1: black/white, 4: 16 colours, 8: 256
/// colours = 16 + 6x6x6 cube + 24 greys); empty for any other depth.
pub(crate) fn fixed_palette(depth: u32) -> Vec<Rgb> {
    let base = || BASE16.iter().map(|&(r, g, b)| Rgb::new(r, g, b));
    match depth {
        1 => vec![Rgb::K, Rgb::W],
        4 => base().collect(),
        8 => {
            let mut v: Vec<Rgb> = base().collect();
            for r in 0..6u8 {
                for g in 0..6u8 {
                    for b in 0..6u8 {
                        v.push(Rgb::new(51 * r, 51 * g, 51 * b));
                    }
                }
            }
            v.extend((0..24u8).map(|k| Rgb::new(8 + 10 * k, 8 + 10 * k, 8 + 10 * k)));
            v
        }
        _ => Vec::new(),
    }
}

/// Palette indices for `pixels` against the palette `target`. With `mono`
/// (the fixed 1-bit palette) a pixel is white when 299R+587G+114B >= 128000;
/// otherwise it takes the nearest entry by squared RGB distance, ties to the
/// lower index.
pub(crate) fn reduce(pixels: &[Rgb], target: &[Rgb], mono: bool) -> Vec<u8> {
    pixels
        .iter()
        .map(|c| {
            if mono {
                let lum = 299 * c.r as u32 + 587 * c.g as u32 + 114 * c.b as u32;
                u8::from(lum >= 128_000)
            } else {
                nearest_index(target, *c).unwrap_or(0)
            }
        })
        .collect()
}

/// Whether at least one pixel's colour is missing from `target`.
pub(crate) fn loses_colors(pixels: &[Rgb], target: &[Rgb]) -> bool {
    let mut seen: Vec<Rgb> = Vec::new();
    pixels.iter().any(|c| {
        if seen.contains(c) {
            return false;
        }
        let known = target.contains(c);
        if known {
            seen.push(*c);
        }
        !known
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_have_expected_sizes() {
        assert_eq!(fixed_palette(1).len(), 2);
        assert_eq!(fixed_palette(4).len(), 16);
        let p = fixed_palette(8);
        assert_eq!(p.len(), 256);
        assert_eq!(p[16 + 36 * 5 + 6 * 2 + 1], Rgb::new(255, 102, 51));
        assert_eq!(p[255], Rgb::new(238, 238, 238));
    }

    #[test]
    fn mapping_rules() {
        let mono = fixed_palette(1);
        assert_eq!(reduce(&[Rgb::new(200, 100, 50), Rgb::new(128, 128, 128)], &mono, true), [0, 1]);
        let p8 = fixed_palette(8);
        assert_eq!(reduce(&[Rgb::new(100, 100, 100), Rgb::K], &p8, false), [102, 0]);
        assert!(loses_colors(&[Rgb::new(1, 2, 3)], &fixed_palette(4)));
        assert!(!loses_colors(&[Rgb::W, Rgb::K], &fixed_palette(4)));
    }
}
