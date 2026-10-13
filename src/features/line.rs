//! Feature tool.line: straight line between press and release in the current
//! width. The drag is a live preview redrawn over the pre-gesture pixels (the
//! undo snapshot pushed at the press); release commits the same drawing, so
//! the whole gesture is one undo step. Shift snaps to 0/45/90 degrees.

use crate::raster::line_points;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Line;
static LINE: Line = Line;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Line, &LINE);
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

/// Stamp a `w` x `w` square for every path point, centred as evenly as the
/// width allows.
fn paint(st: &mut State, a: (i32, i32), b: (i32, i32), c: Rgb, w: i32) {
    let lo = -((w - 1) / 2);
    for (x, y) in line_points(a, b) {
        for oy in lo..lo + w {
            for ox in lo..lo + w {
                st.doc.put(x + ox, y + oy, c);
            }
        }
    }
}

/// Redraw the line over the pixels saved at the press.
fn redraw(st: &mut State, ev: &PointerEvent) {
    let Some(start) = st.held.and_then(|h| h.start_image) else { return };
    let Some(end) = ev.image else { return };
    let Some(base) = st.doc.undo.last() else { return };
    if base.width != st.doc.width || base.height != st.doc.height {
        return;
    }
    st.doc.pixels = base.pixels.clone();
    let end = if ev.mods.shift { snap(start, end) } else { end };
    let c = if ev.button == Some(Button::Right) { st.bg } else { st.fg };
    let w = st.options.line_width as i32;
    paint(st, start, end, c, w);
}

impl ToolHandler for Line {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(p) = ev.image else { return Ok(()) };
        st.doc.push_undo();
        let c = if ev.button == Some(Button::Right) { st.bg } else { st.fg };
        let w = st.options.line_width as i32;
        paint(st, p, p, c, w);
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        redraw(st, ev);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        redraw(st, ev);
        Ok(())
    }
}
