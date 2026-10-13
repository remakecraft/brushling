//! Feature canvas.resize_handle: three handles outside the canvas (bottom-right
//! corner, right-edge midpoint, bottom-edge midpoint). Dragging one sets the
//! size on the axes it changes to the pointer's image coordinate (clamped to
//! 1..=9999), cropping at the right/bottom and filling new area with the
//! background colour. A pointer hook cannot tell a move from the release, so
//! the document is resized live from the picture captured at the press; the
//! drag ends with the pointer button (`State::held`). It is one undo step, and
//! a net change of zero leaves no step and no dirty flag. While dragging, a
//! dashed outline shows the new size and the status bar size pane reads `W,H`
//! (see [`drag_size_text`]).

use crate::doc::{Snapshot, MAX_DIM};
use crate::features::image_attributes::resized_pixels;
use crate::registry::Registry;
use crate::render::CANVAS_MARGIN;
use crate::state::State;
use crate::tools::PointerEvent;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.pointer_hook(hook);
    r.workspace_layer(layer);
}

/// A handle drag in progress, with what it needs to redo the resize from
/// scratch on every move.
#[derive(Debug, Clone)]
pub struct CanvasDrag {
    handle: CanvasHandle,
    base: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    dirty: bool,
    /// The size the document has right now.
    size: (u32, u32),
}

/// `W,H` for the status bar size pane while a handle is being dragged.
pub fn drag_size_text(st: &State) -> Option<String> {
    let d = st.canvas_drag.as_ref().filter(|_| on_handle_gesture(st))?;
    Some(format!("{},{}", d.size.0, d.size.1))
}

/// A button is held that went down on a canvas resize handle.
fn on_handle_gesture(st: &State) -> bool {
    st.held.is_some_and(|h| matches!(h.start, Pos::CanvasHandle(_)))
}

fn changes_width(h: CanvasHandle) -> bool {
    matches!(h, CanvasHandle::Corner | CanvasHandle::Right)
}

fn changes_height(h: CanvasHandle) -> bool {
    matches!(h, CanvasHandle::Corner | CanvasHandle::Bottom)
}

fn clamp_dim(v: i32) -> u32 {
    v.clamp(1, MAX_DIM as i32) as u32
}

fn hook(st: &mut State, ev: &PointerEvent) -> HResult<bool> {
    if let Pos::CanvasHandle(handle) = ev.pos {
        // A press on a handle. Only the left button drags; others are ignored.
        if ev.button == Some(Button::Left) {
            // Floating content is committed before the canvas changes.
            st.end_selection();
            st.canvas_drag = Some(CanvasDrag {
                handle,
                base: st.doc.snapshot(),
                undo: st.doc.undo.clone(),
                redo: st.doc.redo.clone(),
                dirty: st.doc.dirty,
                size: (st.doc.width, st.doc.height),
            });
        }
        return Ok(true);
    }
    // Only a gesture that started on a handle belongs to this feature.
    if !on_handle_gesture(st) {
        st.canvas_drag = None;
        return Ok(false);
    }
    let Some(drag) = st.canvas_drag.take() else { return Ok(true) };
    let drag = match ev.image {
        Some(at) => update(st, drag, at),
        None => drag,
    };
    st.canvas_drag = Some(drag);
    Ok(true)
}

/// Re-derive the document from the press-time picture for pointer `at`.
fn update(st: &mut State, mut drag: CanvasDrag, at: (i32, i32)) -> CanvasDrag {
    let b = &drag.base;
    let w = if changes_width(drag.handle) { clamp_dim(at.0) } else { b.width };
    let h = if changes_height(drag.handle) { clamp_dim(at.1) } else { b.height };
    // Back to the picture and history as they were at the press.
    st.doc.undo.clone_from(&drag.undo);
    st.doc.redo.clone_from(&drag.redo);
    st.doc.dirty = drag.dirty;
    st.doc.width = b.width;
    st.doc.height = b.height;
    st.doc.pixels.clone_from(&b.pixels);
    if (w, h) != (b.width, b.height) {
        st.doc.push_undo();
        st.doc.pixels = resized_pixels(&b.pixels, b.width, b.height, w, h, st.bg);
        st.doc.width = w;
        st.doc.height = h;
    }
    drag.size = (w, h);
    drag
}

// ----- workspace drawing

/// Screen pixels per image pixel for handle geometry: the one place the
/// handle layout depends on zoom.
fn handle_scale(st: &State) -> i64 {
    st.scale() as i64
}

/// Workspace position of an image coordinate.
fn to_screen(st: &State, x: i64, y: i64) -> (i64, i64) {
    let z = handle_scale(st);
    (CANVAS_MARGIN as i64 - st.scroll.0 as i64 + x * z, CANVAS_MARGIN as i64 - st.scroll.1 as i64 + y * z)
}

fn put(img: &mut Image, x: i64, y: i64, c: Rgb) {
    if x >= 0 && y >= 0 && x < img.width as i64 && y < img.height as i64 {
        img.pixels[y as usize * img.width as usize + x as usize] = c;
    }
}

fn layer(st: &State, img: &mut Image) {
    let (w, h) = (st.doc.width as i64, st.doc.height as i64);
    // 3x3 handles just outside the right and bottom edges.
    let blue = Rgb::new(0, 0, 128);
    // Handle centres in screen space: 2 px outside the edge, or halfway along
    // the zoomed edge.
    let (x0, y0) = to_screen(st, 0, 0);
    let (sw, sh) = (w * handle_scale(st), h * handle_scale(st));
    for (sx, sy) in [(x0 + sw + 2, y0 + sh + 2), (x0 + sw + 2, y0 + sh / 2), (x0 + sw / 2, y0 + sh + 2)] {
        for dy in -1..=1 {
            for dx in -1..=1 {
                put(img, sx + dx, sy + dy, blue);
            }
        }
    }
    // Dashed outline of the size being dragged.
    if st.canvas_drag.is_some() && on_handle_gesture(st) {
        let (x0, y0) = to_screen(st, 0, 0);
        let (x1, y1) = to_screen(st, w, h);
        let dash = |i: i64| if i / 2 % 2 == 0 { Rgb::K } else { Rgb::W };
        for i in 0..=(x1 - x0) {
            put(img, x0 + i, y1, dash(i));
        }
        for i in 0..=(y1 - y0) {
            put(img, x1, y0 + i, dash(i));
        }
    }
}

/// Esc during a handle drag: the picture, size and history go back to what
/// they were at the press. True when a drag was in progress.
pub fn cancel_drag(st: &mut State) -> bool {
    if !on_handle_gesture(st) {
        return false;
    }
    let Some(drag) = st.canvas_drag.take() else { return false };
    st.doc.restore(drag.base);
    st.doc.undo = drag.undo;
    st.doc.redo = drag.redo;
    st.doc.dirty = drag.dirty;
    true
}
