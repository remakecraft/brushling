//! Feature tool.select: Select (rectangle) and Free-Form Select. A drag
//! creates the selection on release; creating a new one, clicking outside,
//! switching tool or pressing Esc ends the previous one (floating pixels
//! merge into the image). Making a selection changes no pixels and pushes
//! no undo step. The shared model lives in `selection.rs`.

use crate::raster::shapes::path_points;
use crate::raster::rect_from_corners;
use crate::registry::Registry;
use crate::selection::{Sel, SelDrag};
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct SelectTool {
    freeform: bool,
}
static RECT: SelectTool = SelectTool { freeform: false };
static FREE: SelectTool = SelectTool { freeform: true };

pub fn register(r: &mut Registry) {
    crate::selection::register(r);
    r.tool(Tool::Select, &RECT);
    r.tool(Tool::FreeFormSelect, &FREE);
}

fn clamp(st: &State, p: (i32, i32)) -> (i32, i32) {
    (p.0.clamp(0, st.doc.width as i32 - 1), p.1.clamp(0, st.doc.height as i32 - 1))
}

/// Selection of a closed free-form path: every pixel whose centre lies
/// inside it (even-odd) plus every pixel on the traced path. `None` when
/// the trace touches fewer than two distinct pixels.
pub fn freeform_selection(path: &[(i32, i32)]) -> Option<Sel> {
    let first = *path.first()?;
    if path.iter().all(|p| *p == first) {
        return None;
    }
    let x0 = path.iter().map(|p| p.0).min()?;
    let x1 = path.iter().map(|p| p.0).max()?;
    let y0 = path.iter().map(|p| p.1).min()?;
    let y1 = path.iter().map(|p| p.1).max()?;
    let bounds = Rect::new(x0, y0, x1, y1);
    let (w, h) = (bounds.width() as usize, bounds.height() as usize);
    let mut mask = vec![false; w * h];
    // Vertices sit at pixel centres, so a pixel's centre (x,y) is tested
    // against the polygon in pixel coordinates directly.
    let mut poly: Vec<(i32, i32)> = path.to_vec();
    poly.push(first); // close back to the start
    for y in y0..=y1 {
        for x in x0..=x1 {
            let mut inside = false;
            for e in poly.windows(2) {
                let ((ax, ay), (bx, by)) = (e[0], e[1]);
                if (ay > y) != (by > y) {
                    // Edge crosses this row to the right of x? (no division)
                    let lhs = (x as i64 - ax as i64) * (by as i64 - ay as i64);
                    let rhs = (bx as i64 - ax as i64) * (y as i64 - ay as i64);
                    if (by > ay && lhs < rhs) || (by < ay && lhs > rhs) {
                        inside = !inside;
                    }
                }
            }
            mask[(y - y0) as usize * w + (x - x0) as usize] = inside;
        }
    }
    for &(x, y) in path {
        mask[(y - y0) as usize * w + (x - x0) as usize] = true;
    }
    Some(Sel::new(bounds, mask))
}

impl ToolHandler for SelectTool {
    fn deactivate(&self, st: &mut State) -> HResult<()> {
        st.sel_drag = None;
        st.end_selection();
        Ok(())
    }

    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        // tools.mouse_colors E1: the right button does nothing here.
        if ev.button == Some(Button::Right) {
            return Ok(());
        }
        st.sel_drag = None;
        st.end_selection();
        let Some(p) = ev.image else { return Ok(()) };
        let p = clamp(st, p);
        st.sel_drag = Some(SelDrag { freeform: self.freeform, start: p, cur: p, moved: false, path: vec![p] });
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        // tools.mouse_colors E1: the right button does nothing here.
        if ev.button == Some(Button::Right) {
            return Ok(());
        }
        let Some(p) = ev.image.map(|p| clamp(st, p)) else { return Ok(()) };
        if let Some(d) = st.sel_drag.as_mut() {
            d.moved = true;
            if d.freeform {
                let last = *d.path.last().unwrap_or(&d.start);
                d.path.extend(path_points(last, p).into_iter().skip(1));
            }
            d.cur = p;
        }
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        // tools.mouse_colors E1: the right button does nothing here.
        if ev.button == Some(Button::Right) {
            return Ok(());
        }
        let Some(mut d) = st.sel_drag.take() else { return Ok(()) };
        // The release position extends the gesture but does not by itself
        // count as movement: a press and release in place is a click.
        if let Some(p) = ev.image.map(|p| clamp(st, p)) {
            if d.freeform {
                let last = *d.path.last().unwrap_or(&d.start);
                d.path.extend(path_points(last, p).into_iter().skip(1));
            }
            d.cur = p;
        }
        if !d.moved {
            return Ok(());
        }
        let sel = if d.freeform {
            freeform_selection(&d.path)
        } else {
            Some(Sel::rect(rect_from_corners(d.start, d.cur)))
        };
        if let Some(sel) = sel {
            st.sel = Some(sel);
        }
        Ok(())
    }

    fn key(&self, st: &mut State, key: Key, _mods: Mods) -> HResult<bool> {
        if key == Key::Escape && (st.sel.is_some() || st.sel_drag.is_some()) {
            st.sel_drag = None;
            st.end_selection();
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_path_fills_interior() {
        let corners = [(2, 2), (6, 2), (6, 6), (2, 6), (2, 2)];
        let path: Vec<(i32, i32)> = corners.windows(2).flat_map(|w| path_points(w[0], w[1])).collect();
        let s = freeform_selection(&path).unwrap();
        assert_eq!(s.bounds, Rect::new(2, 2, 6, 6));
        assert!(s.mask.iter().all(|m| *m));
    }

    #[test]
    fn right_button_does_nothing() {
        use crate::settings::Settings;
        let dir = std::env::temp_dir().join(format!("brushling-select-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.replace_document(crate::doc::Document::new(32, 32, Rgb::W).unwrap());
        st.select_tool(Tool::Select).unwrap();
        st.pointer(PointerKind::Press(Button::Left), Pos::Image(2, 2), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Image(8, 8), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), Pos::Image(8, 8), Mods::NONE).unwrap();
        let before = st.sel.as_ref().map(|s| s.bounds);
        assert!(before.is_some());
        st.pointer(PointerKind::Press(Button::Right), Pos::Image(20, 20), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Image(25, 25), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Right), Pos::Image(25, 25), Mods::NONE).unwrap();
        assert_eq!(st.sel.as_ref().map(|s| s.bounds), before);
    }

    #[test]
    fn single_pixel_trace_selects_nothing() {
        assert!(freeform_selection(&[(3, 3), (3, 3)]).is_none());
    }

    #[test]
    fn crossing_path_uses_even_odd() {
        // Bow-tie: the edges cross at the centre; both side lobes are
        // inside, the top and bottom wedges are not.
        let s = freeform_selection(&[(0, 0), (8, 8), (8, 0), (0, 8)]).unwrap();
        assert!(s.contains(1, 4));
        assert!(s.contains(7, 4));
        assert!(!s.contains(4, 1));
        assert!(!s.contains(4, 7));
    }
}
