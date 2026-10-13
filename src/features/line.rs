//! Feature tool.line: straight line between press and release in the current
//! width. The drag is a live preview drawn in the composite layer only; the
//! release commits it to the document as one undo step. Shift snaps to 0/45/90 degrees.

use std::cell::Cell;

use crate::raster::{line_points, snap_line_end};
use crate::features::mouse_colors::paint_colour;
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

/// Every pixel of the line from `a` to `b`, each path point widened to a
/// `w` x `w` square centred as evenly as the width allows.
fn stamps(a: (i32, i32), b: (i32, i32), w: i32) -> impl Iterator<Item = (i32, i32)> {
    let lo = -((w - 1) / 2);
    line_points(a, b).into_iter().flat_map(move |(x, y)| {
        (lo..lo + w).flat_map(move |oy| (lo..lo + w).map(move |ox| (x + ox, y + oy)))
    })
}

thread_local! {
    /// Whether Shift was down at the latest pointer event of the gesture, so
    /// the preview layer (which only sees `&State`) snaps like the commit.
    static SHIFT: Cell<bool> = const { Cell::new(false) };
}

fn end_point(start: (i32, i32), end: (i32, i32), shift: bool) -> (i32, i32) {
    if shift {
        snap_line_end(start, end)
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
    if h.cancelled {
        return;
    }
    let Some(start) = h.start_image else { return };
    let end = end_point(start, cur, SHIFT.with(Cell::get));
    let c = paint_colour(st, Some(h.button));
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
        let c = paint_colour(st, ev.button);
        let w = st.options.line_width as i32;
        st.doc.push_undo();
        for (x, y) in stamps(start, end, w) {
            st.doc.put(x, y, c);
        }
        Ok(())
    }
}
