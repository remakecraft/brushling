//! Reusable shape rasterisers (thick strokes, ellipses, rounded rectangles)
//! shared by the shape, line, curve and polygon tools. All helpers are pure:
//! they return coordinates (possibly outside the canvas) and never touch the
//! document.

use crate::types::Rect;

/// The spec's 1-pixel path between two points, both ends included, produced
/// from `a` towards `b`: the major axis advances one pixel per step and the
/// minor coordinate is rounded, with exact ties rounded toward `a`.
pub(crate) fn path_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let dx = b.0 as i64 - a.0 as i64;
    let dy = b.1 as i64 - a.1 as i64;
    let n = dx.abs().max(dy.abs());
    if n == 0 {
        return vec![a];
    }
    // round(num / n) with ties toward zero (towards the start point)
    let round = |num: i64| -> i64 {
        let (q, r) = (num.abs() / n, num.abs() % n);
        let m = if 2 * r > n { q + 1 } else { q };
        if num < 0 {
            -m
        } else {
            m
        }
    };
    (0..=n)
        .map(|t| {
            if dx.abs() >= dy.abs() {
                ((a.0 as i64 + dx.signum() * t) as i32, (a.1 as i64 + round(dy * t)) as i32)
            } else {
                ((a.0 as i64 + round(dx * t)) as i32, (a.1 as i64 + dy.signum() * t) as i32)
            }
        })
        .collect()
}

/// Offsets of the `w` x `w` square stroke stamp: `[-floor(w/2), ceil(w/2)-1]`
/// on both axes, returned as `(lo, hi)`.
pub(crate) fn stamp_range(w: i32) -> (i32, i32) {
    let w = w.max(1);
    (-(w / 2), (w + 1) / 2 - 1)
}

/// Stamp a `w` x `w` square on every point of `path` (duplicates possible).
pub(crate) fn stamp_square(path: &[(i32, i32)], w: i32) -> Vec<(i32, i32)> {
    let w = w.max(1);
    let (lo, hi) = stamp_range(w);
    let mut out = Vec::with_capacity(path.len() * (w * w) as usize);
    for &(x, y) in path {
        for oy in lo..=hi {
            for ox in lo..=hi {
                out.push((x + ox, y + oy));
            }
        }
    }
    out
}

/// A width-`w` straight stroke from `a` to `b`: the 1-pixel path with the
/// square stamp at every point.
pub(crate) fn thick_line(a: (i32, i32), b: (i32, i32), w: i32) -> Vec<(i32, i32)> {
    stamp_square(&path_points(a, b), w)
}

/// Which closed shape to rasterise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeKind {
    Rectangle,
    Ellipse,
    RoundedRectangle,
}

/// The pixels of a shape, split so a caller can colour them separately. The
/// two sets never overlap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShapePixels {
    pub outline: Vec<(i32, i32)>,
    pub interior: Vec<(i32, i32)>,
}

impl ShapePixels {
    /// Outline followed by interior: the whole area of the shape.
    pub fn all(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.outline.iter().chain(self.interior.iter()).copied()
    }
}

/// End point of a Shift-constrained square drag: side extent
/// `min(|dx|,|dy|)` from `start` in the drag's direction.
pub(crate) fn square_end(start: (i32, i32), end: (i32, i32)) -> (i32, i32) {
    let dx = end.0 as i64 - start.0 as i64;
    let dy = end.1 as i64 - start.1 as i64;
    let m = dx.abs().min(dy.abs());
    ((start.0 as i64 + dx.signum() * m) as i32, (start.1 as i64 + dy.signum() * m) as i32)
}

/// Rasterise `kind` inside the inclusive rectangle `r` (corners in any
/// order) with border width `w`. Nothing falls outside `r`.
pub(crate) fn shape_pixels(kind: ShapeKind, r: Rect, w: i32) -> ShapePixels {
    let r = super::normalize(r);
    let w = w.max(1);
    match kind {
        ShapeKind::Rectangle => rect_pixels(r, w),
        ShapeKind::Ellipse => curved_pixels(r, w, |iw, ih, _, _| (iw as f64 / 2.0, ih as f64 / 2.0)),
        // corner radii come from the drag rectangle, not from the border width
        ShapeKind::RoundedRectangle => curved_pixels(r, w, |iw, ih, rw, rh| {
            ((rw / 2).min(8).min(iw / 2) as f64, (rh / 2).min(8).min(ih / 2) as f64)
        }),
    }
}

/// Outer `w` rows/columns are the outline, the rest the interior; a shape
/// that is no wider than `2w` in some direction is all outline.
fn rect_pixels(r: Rect, w: i32) -> ShapePixels {
    let mut out = ShapePixels::default();
    let (tw, th) = (r.x1 as i64 - r.x0 as i64 + 1, r.y1 as i64 - r.y0 as i64 + 1);
    let solid = tw <= 2 * w as i64 || th <= 2 * w as i64;
    for y in r.y0..=r.y1 {
        for x in r.x0..=r.x1 {
            let inner = !solid && x >= r.x0 + w && x <= r.x1 - w && y >= r.y0 + w && y <= r.y1 - w;
            if inner {
                out.interior.push((x, y));
            } else {
                out.outline.push((x, y));
            }
        }
    }
    out
}

/// Shared by the ellipse and the rounded rectangle: the 1-pixel outline of
/// the area inside the rectangle inset by the half stroke, stamped with the
/// w x w square, plus the interior pixels of that area. `radii` gets the
/// inset and the full box sizes `(iw, ih, rw, rh)`.
fn curved_pixels(r: Rect, w: i32, radii: impl Fn(i32, i32, i32, i32) -> (f64, f64)) -> ShapePixels {
    let (lo, hi) = stamp_range(w);
    // inset of the centre path; collapses to the middle when the stroke
    // is wider than the shape
    let inset = |a: i32, b: i32| -> (i32, i32) {
        let (na, nb) = (a - lo, b - hi);
        if na <= nb {
            (na, nb)
        } else {
            let mid = (a as i64 + b as i64).div_euclid(2) as i32;
            (mid, mid)
        }
    };
    let (ix0, ix1) = inset(r.x0, r.x1);
    let (iy0, iy1) = inset(r.y0, r.y1);
    let (iw, ih) = (ix1 - ix0 + 1, iy1 - iy0 + 1);
    let (rx, ry) = radii(iw, ih, r.x1 - r.x0 + 1, r.y1 - r.y0 + 1);
    let area = |x: i32, y: i32| -> bool { inside(x - ix0, y - iy0, iw, ih, rx, ry) };

    // 1-pixel outline: area pixels with a 4-neighbour outside the area
    let mut centre = Vec::new();
    for y in iy0..=iy1 {
        for x in ix0..=ix1 {
            if area(x, y) && !(area(x - 1, y) && area(x + 1, y) && area(x, y - 1) && area(x, y + 1)) {
                centre.push((x, y));
            }
        }
    }
    let (tw, th) = ((r.x1 - r.x0 + 1) as usize, (r.y1 - r.y0 + 1) as usize);
    let idx = |x: i32, y: i32| (y - r.y0) as usize * tw + (x - r.x0) as usize;
    let mut stroke = vec![false; tw * th];
    for (x, y) in stamp_square(&centre, w) {
        if x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1 {
            stroke[idx(x, y)] = true;
        }
    }
    let mut out = ShapePixels::default();
    for y in r.y0..=r.y1 {
        for x in r.x0..=r.x1 {
            if stroke[idx(x, y)] {
                out.outline.push((x, y));
            } else if area(x, y) {
                out.interior.push((x, y));
            }
        }
    }
    out
}

/// Whether the centre of pixel (`px`,`py`) (relative to the top-left of a
/// `w` x `h` box) lies inside the box with quarter-ellipse corners of radii
/// `rx`, `ry`. Radii equal to half the box give the inscribed ellipse.
fn inside(px: i32, py: i32, w: i32, h: i32, rx: f64, ry: f64) -> bool {
    if px < 0 || py < 0 || px >= w || py >= h {
        return false;
    }
    if rx <= 0.0 || ry <= 0.0 {
        return true;
    }
    let (cx, cy) = (px as f64 + 0.5, py as f64 + 0.5);
    // distance beyond the straight part, towards the nearest corner arc
    let ex = (rx - cx).max(cx - (w as f64 - rx)).max(0.0);
    let ey = (ry - cy).max(cy - (h as f64 - ry)).max(0.0);
    (ex / rx).powi(2) + (ey / ry).powi(2) <= 1.0 + 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn path_rounds_ties_toward_start() {
        assert_eq!(path_points((0, 0), (4, 1)), vec![(0, 0), (1, 0), (2, 0), (3, 1), (4, 1)]);
        assert_eq!(path_points((3, 3), (3, 3)), vec![(3, 3)]);
        assert_eq!(path_points((4, 1), (0, 0)).first(), Some(&(4, 1)));
    }

    #[test]
    fn square_stamp_offsets() {
        assert_eq!(stamp_range(1), (0, 0));
        assert_eq!(stamp_range(2), (-1, 0));
        assert_eq!(stamp_range(3), (-1, 1));
        assert_eq!(stamp_range(5), (-2, 2));
    }

    #[test]
    fn rect_border_and_solid() {
        let p = shape_pixels(ShapeKind::Rectangle, Rect::new(2, 2, 7, 5), 1);
        assert_eq!(p.outline.len(), 16);
        assert_eq!(p.interior.len(), 8);
        let p = shape_pixels(ShapeKind::Rectangle, Rect::new(0, 0, 3, 3), 2);
        assert_eq!(p.outline.len(), 16);
        assert!(p.interior.is_empty());
    }

    #[test]
    fn ellipse_is_symmetric_and_disjoint() {
        for w in 1..=3 {
            let p = shape_pixels(ShapeKind::Ellipse, Rect::new(0, 0, 20, 10), w);
            let o: HashSet<_> = p.outline.iter().copied().collect();
            assert!(o.iter().all(|&(x, y)| o.contains(&(20 - x, y)) && o.contains(&(x, 10 - y))));
            assert!(p.interior.iter().all(|q| !o.contains(q)));
        }
    }

    #[test]
    fn rounded_rect_cuts_corners() {
        let p = shape_pixels(ShapeKind::RoundedRectangle, Rect::new(0, 0, 39, 29), 1);
        let o: HashSet<_> = p.outline.iter().copied().collect();
        assert!(!o.contains(&(0, 0)) && o.contains(&(20, 0)) && o.contains(&(0, 15)));
    }
}
