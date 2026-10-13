//! Feature tool.shapes: Rectangle, Ellipse and Rounded Rectangle dragged out
//! inside the inclusive press/release rectangle, in the current border width
//! and fill style. The drag is a live preview in the composite layer; the
//! release commits it as one undo step. Shift makes the shape a square.
//!
//! Colours: the left button outlines in the foreground and fills the interior
//! (style 2) with the background; the right button swaps the two. Style 3
//! paints the whole area in the outline colour.

use std::cell::Cell;

use crate::raster::rect_from_corners;
use crate::raster::shapes::{shape_pixels, square_end, ShapeKind};
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct ShapeTool(ShapeKind);
static RECTANGLE: ShapeTool = ShapeTool(ShapeKind::Rectangle);
static ELLIPSE: ShapeTool = ShapeTool(ShapeKind::Ellipse);
static ROUNDED: ShapeTool = ShapeTool(ShapeKind::RoundedRectangle);

pub fn register(r: &mut Registry) {
    r.tool(Tool::Rectangle, &RECTANGLE);
    r.tool(Tool::Ellipse, &ELLIPSE);
    r.tool(Tool::RoundedRectangle, &ROUNDED);
    r.composite_layer(preview);
}

thread_local! {
    /// Shift state at the latest pointer event of the gesture, so the preview
    /// layer (which only sees `&State`) constrains like the commit does.
    static SHIFT: Cell<bool> = const { Cell::new(false) };
}

fn kind_of(tool: Tool) -> Option<ShapeKind> {
    match tool {
        Tool::Rectangle => Some(ShapeKind::Rectangle),
        Tool::Ellipse => Some(ShapeKind::Ellipse),
        Tool::RoundedRectangle => Some(ShapeKind::RoundedRectangle),
        _ => None,
    }
}

/// Outline and interior colours for the button used.
fn colours(st: &State, button: Button) -> (Rgb, Rgb) {
    if button == Button::Right {
        (st.bg, st.fg)
    } else {
        (st.fg, st.bg)
    }
}

/// Every pixel of the shape with the colour it gets, drawing order.
fn paint_list(st: &State, kind: ShapeKind, button: Button, a: (i32, i32), b: (i32, i32), shift: bool) -> Vec<((i32, i32), Rgb)> {
    let b = if shift { square_end(a, b) } else { b };
    let px = shape_pixels(kind, rect_from_corners(a, b), st.options.line_width as i32);
    let (edge, fill) = colours(st, button);
    let mut out = Vec::with_capacity(px.outline.len() + px.interior.len());
    match st.options.fill_style {
        2 => {
            out.extend(px.interior.iter().map(|&p| (p, fill)));
            out.extend(px.outline.iter().map(|&p| (p, edge)));
        }
        3 => out.extend(px.all().map(|p| (p, edge))),
        _ => out.extend(px.outline.iter().map(|&p| (p, edge))),
    }
    out
}

fn preview(st: &State, img: &mut Image) {
    let Some(kind) = kind_of(st.tool) else { return };
    let (Some(h), Some(cur)) = (st.held, st.pointer_image) else { return };
    if h.cancelled {
        return;
    }
    let Some(start) = h.start_image else { return };
    for ((x, y), c) in paint_list(st, kind, h.button, start, cur, SHIFT.with(Cell::get)) {
        if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
            img.pixels[y as usize * img.width as usize + x as usize] = c;
        }
    }
}

impl ToolHandler for ShapeTool {
    fn press(&self, _st: &mut State, ev: &PointerEvent) -> HResult<()> {
        SHIFT.with(|s| s.set(ev.mods.shift));
        Ok(())
    }

    fn drag(&self, _st: &mut State, ev: &PointerEvent) -> HResult<()> {
        SHIFT.with(|s| s.set(ev.mods.shift));
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(h) = st.held else { return Ok(()) };
        let Some(start) = h.start_image else { return Ok(()) };
        let Some(cur) = ev.image.or(st.pointer_image) else { return Ok(()) };
        let list = paint_list(st, self.0, h.button, start, cur, ev.mods.shift);
        st.doc.push_undo();
        for ((x, y), c) in list {
            st.doc.put(x, y, c);
        }
        Ok(())
    }
}
