//! Feature tool.line: straight line between press and release in the current
//! width. The drag is a live preview drawn in the composite layer only; the
//! release commits it to the document as one undo step. Shift snaps to 0/45/90 degrees.

use std::cell::Cell;

use crate::raster::shapes::thick_line;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Line;
static LINE: Line = Line;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Line, &LINE);
    r.composite_layer(preview);
}

/// End point of a Shift-constrained line from `start` towards `end`.
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

/// Every pixel of the width-`w` line from `a` to `b` (spec path rule, square stamp).
fn stamps(a: (i32, i32), b: (i32, i32), w: i32) -> Vec<(i32, i32)> {
    thick_line(a, b, w)
}

thread_local! {
    /// Whether Shift was down at the latest pointer event of the gesture, so
    /// the preview layer (which only sees `&State`) snaps like the commit.
    static SHIFT: Cell<bool> = const { Cell::new(false) };
}

fn colour(st: &State, button: Option<Button>) -> Rgb {
    if button == Some(Button::Right) {
        st.bg
    } else {
        st.fg
    }
}

fn end_point(start: (i32, i32), end: (i32, i32), shift: bool) -> (i32, i32) {
    if shift {
        snap(start, end)
    } else {
        end
    }
}

/// Live preview: the line is drawn into the composite only, never the
/// document, until the release commits it.
fn preview(st: &State, img: &mut Image) {
    if st.tool != Tool::Line {
        return;
    }
    let (Some(h), Some(cur)) = (st.held, st.pointer_image) else { return };
    let Some(start) = h.start_image else { return };
    let end = end_point(start, cur, SHIFT.with(Cell::get));
    let c = colour(st, Some(h.button));
    let w = st.options.line_width as i32;
    for (x, y) in stamps(start, end, w) {
        if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
            img.pixels[y as usize * img.width as usize + x as usize] = c;
        }
    }
}

impl ToolHandler for Line {
    fn press(&self, _st: &mut State, ev: &PointerEvent) -> HResult<()> {
        SHIFT.with(|s| s.set(ev.mods.shift));
        Ok(())
    }

    fn drag(&self, _st: &mut State, ev: &PointerEvent) -> HResult<()> {
        SHIFT.with(|s| s.set(ev.mods.shift));
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(start) = st.held.and_then(|h| h.start_image) else { return Ok(()) };
        let Some(cur) = ev.image.or(st.pointer_image) else { return Ok(()) };
        let end = end_point(start, cur, ev.mods.shift);
        let c = colour(st, ev.button);
        let w = st.options.line_width as i32;
        st.doc.push_undo();
        for (x, y) in stamps(start, end, w) {
            st.doc.put(x, y, c);
        }
        Ok(())
    }
}
