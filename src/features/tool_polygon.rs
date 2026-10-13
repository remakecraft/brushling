//! Feature tool.polygon: a closed shape built edge by edge. The first edge is
//! dragged, every further click adds a vertex at its release. A double-click
//! or a click released on the first vertex closes it: the fill style is
//! applied (interior = pixel centres inside the vertex path, even-odd rule),
//! the outline is stamped on top in the current width, one undo step.
//! Shift snaps each edge from the previous vertex like tool.line. Esc
//! discards; selecting another tool closes and commits.

use crate::features::mouse_colors::paint_colour;
use crate::raster::shapes::{path_points, stamp_square};
use crate::raster::snap_line_end;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

type P = (i32, i32);

/// Per-gesture polygon data (a field of [`State`]).
#[derive(Debug, Clone, Default)]
pub struct PolyState {
    /// Fixed vertices; empty when no polygon is under construction.
    verts: Vec<P>,
    button: Option<Button>,
    /// Pointer position for the live edge; cleared when a vertex is fixed so
    /// the next edge only appears once the pointer moves again.
    cursor: Option<P>,
    shift: bool,
    /// The press closed the polygon: its release must do nothing.
    skip_release: bool,
}

struct Polygon;
static POLYGON: Polygon = Polygon;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Polygon, &POLYGON);
    r.composite_layer(preview);
}

fn snap(prev: P, cur: P, shift: bool) -> P {
    if shift {
        snap_line_end(prev, cur)
    } else {
        cur
    }
}

/// Outline pixels (edges between consecutive vertices, plus the closing edge
/// when `closed`), stamped with width `w`.
fn outline(verts: &[P], closed: bool, w: i32) -> Vec<P> {
    let mut path = Vec::new();
    for pair in verts.windows(2) {
        path.extend(path_points(pair[0], pair[1]));
    }
    if closed && verts.len() >= 2 {
        path.extend(path_points(verts[verts.len() - 1], verts[0]));
    }
    if path.is_empty() {
        path.extend(verts.iter().copied());
    }
    stamp_square(&path, w)
}

/// Pixels whose centre lies inside the closed vertex path (even-odd rule),
/// computed in exact integer arithmetic on doubled coordinates.
fn interior(verts: &[P]) -> Vec<P> {
    if verts.len() < 3 {
        return Vec::new();
    }
    let x0 = verts.iter().map(|v| v.0).min().unwrap_or(0);
    let x1 = verts.iter().map(|v| v.0).max().unwrap_or(0);
    let y0 = verts.iter().map(|v| v.1).min().unwrap_or(0);
    let y1 = verts.iter().map(|v| v.1).max().unwrap_or(0);
    let mut out = Vec::new();
    for y in y0..=y1 {
        let py = 2 * y as i64 + 1;
        for x in x0..=x1 {
            let px = 2 * x as i64 + 1;
            let mut inside = false;
            for i in 0..verts.len() {
                let a = verts[i];
                let b = verts[(i + 1) % verts.len()];
                let (ax, ay) = (2 * a.0 as i64, 2 * a.1 as i64);
                let (bx, by) = (2 * b.0 as i64, 2 * b.1 as i64);
                if (ay > py) != (by > py) {
                    // does the edge cross the scan line to the right of px?
                    let den = by - ay;
                    let lhs = (px - ax) * den;
                    let rhs = (py - ay) * (bx - ax);
                    if (den > 0 && lhs < rhs) || (den < 0 && lhs > rhs) {
                        inside = !inside;
                    }
                }
            }
            if inside {
                out.push((x, y));
            }
        }
    }
    out
}

/// Outline and interior colours for the button used (as in tool.shapes).
fn colours(st: &State, button: Option<Button>) -> (Rgb, Rgb) {
    if button == Some(Button::Right) {
        (st.bg, st.fg)
    } else {
        (st.fg, st.bg)
    }
}

/// The finished polygon as ordered paint operations.
fn closed_paint(st: &State, verts: &[P], button: Option<Button>) -> Vec<(P, Rgb)> {
    let (edge, fill) = colours(st, button);
    let mut out = Vec::new();
    match st.options.fill_style {
        2 => out.extend(interior(verts).into_iter().map(|p| (p, fill))),
        3 => out.extend(interior(verts).into_iter().map(|p| (p, edge))),
        _ => {}
    }
    out.extend(outline(verts, true, st.options.line_width as i32).into_iter().map(|p| (p, edge)));
    out
}

fn close(st: &mut State) {
    let verts = std::mem::take(&mut st.poly.verts);
    let button = st.poly.button;
    st.poly.cursor = None;
    if verts.len() < 2 {
        return;
    }
    let list = closed_paint(st, &verts, button);
    st.doc.push_undo();
    for ((x, y), c) in list {
        st.doc.paint(x, y, c);
    }
}

fn preview(st: &State, img: &mut Image) {
    if st.tool != Tool::Polygon || st.poly.verts.is_empty() {
        return;
    }
    let mut verts = st.poly.verts.clone();
    if let (Some(cur), Some(&last)) = (st.poly.cursor, verts.last()) {
        verts.push(snap(last, cur, st.poly.shift));
    }
    let c = paint_colour(st, st.poly.button);
    for (x, y) in outline(&verts, false, st.options.line_width as i32) {
        if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
            img.pixels[y as usize * img.width as usize + x as usize] = c;
        }
    }
}

fn track(st: &mut State, ev: &PointerEvent) {
    st.poly.shift = ev.mods.shift;
    if let Some(p) = ev.image.or(st.pointer_image) {
        st.poly.cursor = Some(p);
    }
}

impl ToolHandler for Polygon {
    fn deactivate(&self, st: &mut State) -> HResult<()> {
        close(st);
        Ok(())
    }

    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        st.poly.skip_release = false;
        track(st, ev);
        if st.poly.verts.is_empty() {
            let Some(p) = ev.image else { return Ok(()) };
            st.poly.verts.push(p);
            st.poly.button = ev.button;
        } else if ev.double_click {
            st.poly.skip_release = true;
            close(st);
        }
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        track(st, ev);
        Ok(())
    }

    fn hover(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        track(st, ev);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if st.poly.skip_release {
            st.poly.skip_release = false;
            return Ok(());
        }
        let (Some(&first), Some(&last)) = (st.poly.verts.first(), st.poly.verts.last()) else { return Ok(()) };
        let Some(cur) = ev.image.or(st.pointer_image) else { return Ok(()) };
        let end = snap(last, cur, ev.mods.shift);
        st.poly.cursor = None;
        if st.poly.verts.len() >= 2 && end == first {
            close(st);
        } else {
            st.poly.verts.push(end);
        }
        Ok(())
    }

    fn key(&self, st: &mut State, key: Key, _mods: Mods) -> HResult<bool> {
        if key == Key::Escape && !st.poly.verts.is_empty() {
            st.poly.verts.clear();
            st.poly.cursor = None;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_interior_is_pixels_inside() {
        let v = [(2, 2), (12, 2), (12, 12), (2, 12)];
        let i = interior(&v);
        assert_eq!(i.len(), 100);
        assert!(i.contains(&(2, 2)) && i.contains(&(11, 11)) && !i.contains(&(12, 5)));
    }
}
