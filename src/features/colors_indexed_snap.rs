//! Feature colors.indexed_snap: in an image with its own palette every
//! colour written to the document is first replaced by the nearest palette
//! entry (squared RGB distance, ties to the lowest index). The rule is applied
//! in `Document::put`, which every tool and command paints through; the
//! black-and-white threshold rule of image.attributes takes precedence there.

use crate::registry::Registry;
use crate::types::Rgb;

pub fn register(_r: &mut Registry) {}

/// The palette entry nearest to `c`, or `c` itself for an empty palette.
pub fn nearest(palette: &[Rgb], c: Rgb) -> Rgb {
    let dist = |p: &Rgb| {
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        d(p.r, c.r) + d(p.g, c.g) + d(p.b, c.b)
    };
    // `min_by_key` returns the first minimum, which is the lowest index.
    palette.iter().min_by_key(|p| dist(p)).copied().unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_prefers_lowest_index_on_ties() {
        let pal = [Rgb::new(0, 0, 0), Rgb::new(100, 0, 0), Rgb::new(200, 0, 0)];
        assert_eq!(nearest(&pal, Rgb::new(50, 0, 0)), pal[0]);
        assert_eq!(nearest(&pal, Rgb::new(160, 9, 0)), pal[2]);
        assert_eq!(nearest(&[], Rgb::new(1, 2, 3)), Rgb::new(1, 2, 3));
    }
}
