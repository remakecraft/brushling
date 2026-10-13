//! Feature tool.eraser: left-drag stamps a square of background colour along
//! the pointer path; right-drag (colour eraser) replaces only pixels equal to
//! the foreground colour. One press-to-release is one undo step.

use crate::raster::line_points;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Eraser;
static ERASER: Eraser = Eraser;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Eraser, &ERASER);
}

/// Stamp the square of side `size` centred (per the spec offsets) on `p`.
fn stamp(st: &mut State, p: (i32, i32), colour_erase: bool) {
    let n = st.options.eraser_size as i32;
    let (lo, hi) = (-(n / 2), (n + 1) / 2 - 1);
    let (fg, bg) = (st.fg, st.bg);
    for dy in lo..=hi {
        for dx in lo..=hi {
            let (x, y) = (p.0 + dx, p.1 + dy);
            if colour_erase && st.doc.get(x, y) != Some(fg) {
                continue;
            }
            st.doc.put(x, y, bg);
        }
    }
}

fn stamp_path(st: &mut State, a: (i32, i32), b: (i32, i32), colour_erase: bool) {
    for p in line_points(a, b) {
        stamp(st, p, colour_erase);
    }
}

impl ToolHandler for Eraser {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        st.pencil_last = None;
        let Some(p) = ev.image else { return Ok(()) };
        st.doc.push_undo();
        stamp(st, p, ev.button == Some(Button::Right));
        st.pencil_last = Some(p);
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(last) = st.pencil_last else { return Ok(()) };
        let Some(p) = ev.image else { return Ok(()) };
        stamp_path(st, last, p, ev.button == Some(Button::Right));
        st.pencil_last = Some(p);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        self.drag(st, ev)?;
        st.pencil_last = None;
        Ok(())
    }
}
