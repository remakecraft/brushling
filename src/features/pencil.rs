//! Feature tool.pencil: 1-pixel freehand drawing. Without Shift every pointer
//! move paints the line from the previous to the new position; with Shift
//! held at the press the stroke is one live straight line from the press
//! point, snapped to 0/45/90 degrees, redrawn over the pre-stroke pixels.

use crate::raster::line_points;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Pencil;
static PENCIL: Pencil = Pencil;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Pencil, &PENCIL);
}

/// Snap rule shared with the line tool: end point of a Shift-constrained
/// line from `start` towards `end`.
fn snap(start: (i32, i32), end: (i32, i32)) -> (i32, i32) {
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

fn colour(st: &State, button: Option<Button>) -> Rgb {
    if button == Some(Button::Right) {
        st.bg
    } else {
        st.fg
    }
}

fn paint_line(st: &mut State, a: (i32, i32), b: (i32, i32), c: Rgb) {
    for (x, y) in line_points(a, b) {
        st.doc.put(x, y, c);
    }
}

/// Put the pre-stroke pixels back (Shift preview redraw).
fn restore_base(st: &mut State) {
    if let Some(base) = st.pencil_base.clone() {
        let w = st.doc.info().width as usize;
        for (i, c) in base.into_iter().enumerate() {
            st.doc.put((i % w) as i32, (i / w) as i32, c);
        }
    }
}

impl ToolHandler for Pencil {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        st.pencil_last = None;
        st.pencil_base = None;
        let Some(p) = ev.image else { return Ok(()) };
        st.doc.push_undo();
        if ev.mods.shift {
            st.pencil_base = Some(st.doc.image().pixels);
        }
        let c = colour(st, ev.button);
        st.doc.put(p.0, p.1, c);
        st.pencil_last = Some(p);
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(last) = st.pencil_last else { return Ok(()) };
        let Some(p) = ev.image else { return Ok(()) };
        let c = colour(st, ev.button);
        if st.pencil_base.is_some() {
            let start = st.held.and_then(|h| h.start_image).unwrap_or(last);
            restore_base(st);
            paint_line(st, start, snap(start, p), c);
        } else {
            paint_line(st, last, p, c);
        }
        st.pencil_last = Some(p);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if st.pencil_base.is_some() {
            self.drag(st, ev)?;
        }
        st.pencil_last = None;
        st.pencil_base = None;
        Ok(())
    }
}
