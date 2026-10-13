//! Feature tool.fill: implement a `ToolHandler` here and register it with
//! `r.tool(Tool::Fill, &FILL)`. See `src/README.md`.
//!
//! Flood fill over the 4-connected region of exactly the clicked colour.

use crate::features::mouse_colors::paint_colour;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Fill;

static FILL: Fill = Fill;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Fill, &FILL);
}

/// Nearest palette entry for an indexed image; other images keep the colour.
fn snap(palette: Option<&[Rgb]>, c: Rgb) -> Rgb {
    let dist = |p: &Rgb| {
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        d(p.r, c.r) + d(p.g, c.g) + d(p.b, c.b)
    };
    palette
        .and_then(|p| p.iter().min_by_key(|p| dist(p)))
        .copied()
        .unwrap_or(c)
}

impl ToolHandler for Fill {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some((x, y)) = ev.image else { return Ok(()) };
        let Some(target) = st.doc.get(x, y) else { return Ok(()) };
        let chosen = paint_colour(st, ev.button);
        let paint = snap(st.doc.palette.as_deref(), chosen);
        if paint == target {
            return Ok(());
        }
        st.doc.push_undo();
        let (w, h) = (st.doc.width as i32, st.doc.height as i32);
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            if cx < 0 || cy < 0 || cx >= w || cy >= h || st.doc.get(cx, cy) != Some(target) {
                continue;
            }
            st.doc.put(cx, cy, paint);
            stack.extend([(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
        }
        Ok(())
    }
}
