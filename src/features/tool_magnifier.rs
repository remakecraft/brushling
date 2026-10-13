//! Feature tool.magnifier. At 100% the pointer over the canvas shows a
//! rectangle of floor(view / 4) image pixels per axis; a click (either
//! button) zooms to 400% with that rectangle's top-left at the view's
//! top-left and re-selects the previous tool. A click while zoomed returns to
//! 100%. The level options live in `tools::set_tool_option`.
//! Nothing here touches pixels or undo.

use crate::features::view_zoom::{image_to_screen, set_zoom_at, LARGE_ZOOM};
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Magnifier;
static HANDLER: Magnifier = Magnifier;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Magnifier, &HANDLER);
    r.workspace_layer(layer);
}

/// Size in image pixels of the zoom-in rectangle.
fn rect_size(st: &State) -> (i32, i32) {
    let z = (LARGE_ZOOM / 100) as i32;
    ((st.workspace_size.0 as i32 / z).max(1), (st.workspace_size.1 as i32 / z).max(1))
}

/// Top-left of the rectangle centred on image pixel `p`, kept inside the image.
fn rect_origin(st: &State, p: (i32, i32)) -> (i32, i32) {
    let (rw, rh) = rect_size(st);
    let axis = |c: i32, r: i32, len: u32| (c - r / 2).clamp(0, (len as i32 - r).max(0));
    (axis(p.0, rw, st.doc.width), axis(p.1, rh, st.doc.height))
}

impl ToolHandler for Magnifier {
    fn hover(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        st.mag_hover = ev.image.filter(|_| st.zoom == 100);
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        self.hover(st, ev)
    }

    fn deactivate(&self, st: &mut State) -> HResult<()> {
        st.mag_hover = None;
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(p) = ev.image else { return Ok(()) };
        if st.zoom == 100 {
            let (rx, ry) = rect_origin(st, p);
            let z = (LARGE_ZOOM / 100) as i32;
            set_zoom_at(st, LARGE_ZOOM, Some((rx * z, ry * z)))?;
        } else {
            set_zoom_at(st, 100, None)?;
        }
        st.mag_hover = None;
        // The Text tool is unavailable above 100%.
        let back = if st.prev_tool == Tool::Text && st.zoom > 100 { Tool::Pencil } else { st.prev_tool };
        st.select_tool(back)
    }
}

/// The zoom-in rectangle outline under the pointer.
fn layer(st: &State, img: &mut Image) {
    if st.tool != Tool::Magnifier || st.zoom != 100 {
        return;
    }
    let Some(p) = st.mag_hover else { return };
    let (rx, ry) = rect_origin(st, p);
    let (rw, rh) = rect_size(st);
    let (rw, rh) = (rw.min(st.doc.width as i32 - rx), rh.min(st.doc.height as i32 - ry));
    let (x0, y0) = image_to_screen(st, rx, ry);
    let (x1, y1) = (x0 + rw as i64 - 1, y0 + rh as i64 - 1);
    let mut put = |x: i64, y: i64| {
        if x >= 0 && y >= 0 && x < img.width as i64 && y < img.height as i64 {
            img.pixels[(y * img.width as i64 + x) as usize] = Rgb::K;
        }
    };
    for x in x0..=x1 {
        put(x, y0);
        put(x, y1);
    }
    for y in y0..=y1 {
        put(x0, y);
        put(x1, y);
    }
}
