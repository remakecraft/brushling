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

/// End point of a Shift-constrained line from `start` towards `end`
/// (0/45/90 degree snap rule of tool.line, shared by Pencil and Line): if
/// |dy| < 0.4142 |dx| the line is horizontal, else if |dx| < 0.4142 |dy|
/// vertical, else diagonal with m = min(|dx|, |dy|) in each direction.
pub fn snap_line_end(start: (i32, i32), end: (i32, i32)) -> (i32, i32) {
    const T: f64 = 0.4142;
    let dx = end.0 as i64 - start.0 as i64;
    let dy = end.1 as i64 - start.1 as i64;
    let (ax, ay) = (dx.abs(), dy.abs());
    if (ay as f64) < T * ax as f64 {
        (end.0, start.1)
    } else if (ax as f64) < T * ay as f64 {
        (start.0, end.1)
    } else {
        let m = ax.min(ay);
        ((start.0 as i64 + dx.signum() * m) as i32, (start.1 as i64 + dy.signum() * m) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_discards_outside() {
        assert_eq!(clip_rect(Rect::new(-5, 3, 40, 3), 32, 32), Some(Rect::new(0, 3, 31, 3)));
        assert_eq!(clip_rect(Rect::new(40, 0, 50, 5), 32, 32), None);
    }

    #[test]
    fn snap_all_eight_directions() {
        let o = (10, 10);
        for (end, want) in [
            ((30, 13), (30, 10)),
            ((-10, 13), (-10, 10)),
            ((12, 30), (10, 30)),
            ((12, -10), (10, -10)),
            ((30, 25), (25, 25)),
            ((-10, 25), (-5, 25)),
            ((30, -5), (25, -5)),
            ((-10, -30), (-10, -10)),
            ((10, 10), (10, 10)),
        ] {
            assert_eq!(snap_line_end(o, end), want, "{end:?}");
        }
        // boundary: |dy| = 0.4142 |dx| is already diagonal-ish, not horizontal
        assert_eq!(snap_line_end((0, 0), (100, 41)), (100, 0));
        assert_eq!(snap_line_end((0, 0), (100, 42)), (42, 42));
    }
}
