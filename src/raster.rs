//! Small pixel helpers shared by every drawing feature. They are pure: they
//! compute coordinates and never touch the document, so a tool decides how
//! (and whether) each point is placed.

use crate::types::Rect;

/// Thick strokes, ellipses and rounded rectangles (shapes, curve, polygon).
pub(crate) mod shapes;

/// The inclusive rectangle spanned by two corner points, corners in any order.
pub fn rect_from_corners(a: (i32, i32), b: (i32, i32)) -> Rect {
    Rect::new(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// Re-order a rectangle so that `x0 <= x1` and `y0 <= y1`.
pub fn normalize(r: Rect) -> Rect {
    rect_from_corners((r.x0, r.y0), (r.x1, r.y1))
}

/// Whether (x,y) lies inside a `w` x `h` raster.
pub fn in_bounds(x: i32, y: i32, w: u32, h: u32) -> bool {
    x >= 0 && y >= 0 && (x as i64) < w as i64 && (y as i64) < h as i64
}

/// Intersection of an inclusive rectangle with a `w` x `h` raster, if any.
pub fn clip_rect(r: Rect, w: u32, h: u32) -> Option<Rect> {
    let r = normalize(r);
    let c = Rect::new(r.x0.max(0), r.y0.max(0), r.x1.min(w as i32 - 1), r.y1.min(h as i32 - 1));
    (c.x0 <= c.x1 && c.y0 <= c.y1).then_some(c)
}

/// Every pixel of the 1-pixel Bresenham line from `a` to `b`, both ends
/// included, in drawing order. Points may lie outside the canvas; callers
/// clip when placing (see `Document::put`). The line is produced from `a`
/// towards `b`, so a feature that needs a symmetric result orders its end
/// points itself.
pub fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = (a.0 as i64, a.1 as i64);
    let (x1, y1) = (b.0 as i64, b.1 as i64);
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        out.push((x as i32, y as i32));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_includes_both_ends_and_is_connected() {
        let p = line_points((0, 0), (5, 2));
        assert_eq!(p.first(), Some(&(0, 0)));
        assert_eq!(p.last(), Some(&(5, 2)));
        assert_eq!(p.len(), 6);
        for w in p.windows(2) {
            assert!((w[1].0 - w[0].0).abs() <= 1 && (w[1].1 - w[0].1).abs() <= 1);
        }
    }

    #[test]
    fn single_point_line() {
        assert_eq!(line_points((3, 3), (3, 3)), vec![(3, 3)]);
    }

    #[test]
    fn clip_discards_outside() {
        assert_eq!(clip_rect(Rect::new(-5, 3, 40, 3), 32, 32), Some(Rect::new(0, 3, 31, 3)));
        assert_eq!(clip_rect(Rect::new(40, 0, 50, 5), 32, 32), None);
    }
}
