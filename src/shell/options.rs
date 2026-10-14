//! The tool options box under the tool grid: each option drawn as a small
//! picture (line widths, brush tips, eraser sizes...) instead of text, the
//! current one on a navy highlight. Labels stay as tooltips.

use brushling::ToolOption;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Ui, Vec2};

use super::theme::{self, Bevel, Pixels};

/// Outer size of the options box in units, edge included.
pub const BOX: (i32, i32) = (42, 66);

/// One option cell `(x, y, w, h)` in units, inside a `w` x `h` interior:
/// `n` options in `columns`, rows of equal height, the grid centred.
pub fn cells(n: usize, columns: usize, inner: (i32, i32)) -> Vec<(i32, i32, i32, i32)> {
    if n == 0 || columns == 0 {
        return Vec::new();
    }
    let rows = n.div_ceil(columns) as i32;
    let cols = columns.min(n) as i32;
    let (cw, ch) = (inner.0 / cols, (inner.1 / rows).min(inner.1));
    let (x0, y0) = ((inner.0 - cw * cols) / 2, (inner.1 - ch * rows) / 2);
    (0..n as i32).map(|i| (x0 + (i % cols) * cw, y0 + (i / cols) * ch, cw, ch)).collect()
}

/// The footprint of brush shape 1..=12 (the brush's own shapes: round,
/// square, rising and falling slash, each large, medium and small).
pub fn brush_footprint(shape: u32) -> Vec<(i32, i32)> {
    let round = |d: i32| {
        let r = (d - 1) / 2;
        let mut v = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    v.push((dx, dy));
                }
            }
        }
        v
    };
    let square = |n: i32| {
        let (lo, hi) = (-(n / 2), (n + 1) / 2 - 1);
        (lo..=hi).flat_map(|dy| (lo..=hi).map(move |dx| (dx, dy))).collect::<Vec<_>>()
    };
    let rising = |n: i32| (0..n).map(|i| (i - n / 2, n / 2 - i)).collect::<Vec<_>>();
    let falling = |n: i32| (0..n).map(|i| (i - n / 2, i - n / 2)).collect::<Vec<_>>();
    match shape {
        1 => round(7),
        2 => round(5),
        3 => round(1),
        4 => square(8),
        5 => square(5),
        6 => square(2),
        7 => rising(8),
        8 => rising(5),
        9 => rising(2),
        10 => falling(8),
        11 => falling(5),
        _ => falling(2),
    }
}

/// A fixed scatter of spray dots within `radius`, densest in the middle.
pub fn spray_dots(radius: i32) -> Vec<(i32, i32)> {
    let mut seed: u32 = 0x2545_f491;
    let mut out = Vec::new();
    let r2 = radius * radius;
    for _ in 0..(radius * 3) {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let dx = (seed % (2 * radius as u32 + 1)) as i32 - radius;
        let dy = ((seed >> 8) % (2 * radius as u32 + 1)) as i32 - radius;
        if dx * dx + dy * dy <= r2 && !out.contains(&(dx, dy)) {
            out.push((dx, dy));
        }
    }
    out
}

/// Draw `option` centred in a `w` x `h` cell at `px`'s origin.
fn picture(px: &Pixels, option: ToolOption, w: i32, h: i32, ink: Color32) {
    let (cx, cy) = (w / 2, h / 2);
    match option {
        ToolOption::Opaque(opaque) => {
            let (x, y) = (cx - 12, cy - 7);
            if opaque {
                px.fill(x, y, 24, 14, Color32::from_rgb(0, 128, 128));
                px.fill(x, y, 24, 1, ink);
                px.fill(x, y + 13, 24, 1, ink);
                px.fill(x, y, 1, 14, ink);
                px.fill(x + 23, y, 1, 14, ink);
            }
            px.fill(x + 4, y + 3, 8, 8, Color32::from_rgb(255, 0, 0));
            px.fill(x + 12, y + 5, 8, 6, Color32::from_rgb(255, 255, 0));
        }
        ToolOption::EraserSize(n) => {
            let n = n as i32;
            px.fill(cx - n / 2, cy - n / 2, n, n, ink);
        }
        ToolOption::BrushShape(s) => {
            for (dx, dy) in brush_footprint(s) {
                px.dot(cx + dx, cy + dy, ink);
            }
        }
        ToolOption::AirbrushRadius(r) => {
            // Drawn at about half size so the large spray fits its cell.
            for (dx, dy) in spray_dots((r as i32 + 2) / 2) {
                px.dot(cx + dx, cy + dy, ink);
            }
        }
        ToolOption::LineWidth(n) => {
            let n = n as i32;
            px.fill(4, cy - n / 2, w - 8, n, ink);
        }
        ToolOption::FillStyle(style) => {
            let (x, y) = (cx - 13, cy - 5);
            if style >= 2 {
                px.fill(x, y, 26, 10, theme::SHADOW);
            }
            if style <= 2 {
                px.fill(x, y, 26, 1, ink);
                px.fill(x, y + 9, 26, 1, ink);
                px.fill(x, y, 1, 10, ink);
                px.fill(x + 25, y, 1, 10, ink);
            }
        }
        // Text, drawn by the caller.
        ToolOption::MagnifierLevel(_) => {}
    }
}

/// The options box at the top of `rect` (centred), for the current tool's
/// option set (label, option, selected). Returns the option clicked.
pub fn options_box(ui: &mut Ui, rect: Rect, set: &[(String, ToolOption, bool)], columns: usize) -> Option<ToolOption> {
    let (u, ppp) = theme::metrics(ui);
    let size = Vec2::new(BOX.0 as f32, BOX.1 as f32) * u;
    let outer = theme::snap(Rect::from_min_size(Pos2::new(rect.center().x - size.x / 2.0, rect.min.y), size), ppp);
    let inner = theme::panel(ui.painter(), outer, Bevel::Sunken, theme::FACE, u);
    let edge = Bevel::Sunken.width() as i32;
    let interior = (BOX.0 - 2 * edge, BOX.1 - 2 * edge);
    let mut picked = None;
    for (i, ((label, option, selected), (x, y, w, h))) in set.iter().zip(cells(set.len(), columns, interior)).enumerate() {
        let min = inner.min + Vec2::new(x as f32, y as f32) * u;
        let cell = Rect::from_min_size(min, Vec2::new(w as f32, h as f32) * u);
        let r = ui.interact(cell, ui.id().with(("brushling-tool-option", i)), Sense::click());
        let p = ui.painter();
        let ink = if *selected { theme::WHITE } else { theme::BLACK };
        if *selected {
            theme::solid(p, cell.shrink(u), theme::NAVY);
        }
        let px = Pixels { painter: p, origin: min, u };
        match option {
            ToolOption::MagnifierLevel(n) => {
                p.text(theme::snap_pos(cell.center(), ppp), Align2::CENTER_CENTER, format!("{n}x"), egui::FontId::proportional(10.0), ink);
            }
            _ => picture(&px, *option, w, h, ink),
        }
        if r.on_hover_text(label.as_str()).clicked() {
            picked = Some(*option);
        }
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_cells_fit_the_box_without_overlap() {
        let inner = (BOX.0 - 4, BOX.1 - 4);
        for (n, cols) in [(2, 1), (3, 1), (4, 1), (5, 1), (12, 3)] {
            let c = cells(n, cols, inner);
            assert_eq!(c.len(), n);
            for (i, a) in c.iter().enumerate() {
                assert!(a.0 >= 0 && a.1 >= 0 && a.0 + a.2 <= inner.0 && a.1 + a.3 <= inner.1, "{n}x{cols}: {a:?}");
                for b in &c[i + 1..] {
                    let apart = a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                    assert!(apart, "{a:?} overlaps {b:?}");
                }
            }
        }
        assert!(cells(0, 1, inner).is_empty());
    }

    #[test]
    fn brush_tips_fit_their_cells() {
        let (_, _, w, h) = cells(12, 3, (BOX.0 - 4, BOX.1 - 4))[0];
        for s in 1..=12 {
            let f = brush_footprint(s);
            assert!(!f.is_empty(), "shape {s}");
            assert!(f.iter().all(|&(dx, dy)| dx.abs() < w / 2 && dy.abs() < h / 2), "shape {s}");
        }
        // Large, medium, small within each family.
        assert!(brush_footprint(1).len() > brush_footprint(2).len());
        assert_eq!(brush_footprint(3).len(), 1);
        assert_eq!(brush_footprint(4).len(), 64);
    }

    #[test]
    fn spray_scatter_grows_with_the_radius_and_stays_inside() {
        let (s, m, l) = (spray_dots(4), spray_dots(8), spray_dots(12));
        assert!(s.len() < m.len() && m.len() < l.len());
        for (dots, r) in [(&s, 4), (&m, 8), (&l, 12)] {
            assert!(dots.iter().all(|&(x, y)| x * x + y * y <= r * r));
        }
    }
}
