//! Feature selection.resize: dragging one of the selection's eight handles
//! stretches the content non-proportionally (nearest neighbour, from the
//! content as it was when the drag began). A corner handle moves that corner,
//! an edge handle only that edge; the opposite side stays fixed and the size
//! never drops below 1 x 1. A fresh selection floats first (source cleared to
//! the background colour). Press-to-release is one undo step.

use super::selection_image_ops_scope::Raster;
use crate::registry::Registry;
use crate::selection::SelGesture;
use crate::state::State;
use crate::tools::PointerEvent;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.pointer_hook(hook);
}

fn hook(st: &mut State, ev: &PointerEvent) -> HResult<bool> {
    match (st.ptr_kind, ev.pos) {
        (Some(PointerKind::Press(_)), Pos::SelectionHandle(handle)) => {
            let Some(sel) = st.sel.as_ref() else { return Ok(false) };
            st.sel_gesture = Some(SelGesture::Resize { handle, start: sel.bounds, base: None });
            st.sel_drag = None;
            Ok(true)
        }
        (Some(PointerKind::Move), _) => Ok(drag(st, ev)),
        (Some(PointerKind::Release(_)), _) => Ok(release(st, ev)),
        _ => Ok(false),
    }
}

/// Bounds after dragging `handle` to image point `p`, from the start bounds.
/// The dragged side never crosses the fixed one (minimum size 1 x 1).
pub fn dragged_bounds(handle: Handle, b: Rect, p: (i32, i32)) -> Rect {
    let (left, right, top, bottom) = match handle {
        Handle::TopLeft => (true, false, true, false),
        Handle::Top => (false, false, true, false),
        Handle::TopRight => (false, true, true, false),
        Handle::Left => (true, false, false, false),
        Handle::Right => (false, true, false, false),
        Handle::BottomLeft => (true, false, false, true),
        Handle::Bottom => (false, false, false, true),
        Handle::BottomRight => (false, true, false, true),
    };
    Rect::new(
        if left { p.0.min(b.x1) } else { b.x0 },
        if top { p.1.min(b.y1) } else { b.y0 },
        if right { p.0.max(b.x0) } else { b.x1 },
        if bottom { p.1.max(b.y0) } else { b.y1 },
    )
}

fn drag(st: &mut State, ev: &PointerEvent) -> bool {
    let Some(SelGesture::Resize { handle, start, base }) = st.sel_gesture.clone() else { return false };
    let Some(p) = ev.image else { return true };
    let base = match base {
        Some(b) => b,
        None => {
            // First move: the undo step and the lift happen now, and the
            // content at this moment is what every later move resamples.
            st.doc.push_undo();
            st.lift_selection(true);
            let Some(sel) = st.sel.as_ref() else { return true };
            let Some(pixels) = sel.floating.clone() else { return true };
            Raster {
                width: sel.bounds.width() as u32,
                height: sel.bounds.height() as u32,
                pixels,
                mask: sel.mask.clone(),
            }
        }
    };
    let nb = dragged_bounds(handle, start, p);
    let out = base.stretched(nb.width() as u32, nb.height() as u32);
    if let Some(sel) = st.sel.as_mut() {
        sel.bounds = nb;
        sel.mask = out.mask;
        sel.floating = Some(out.pixels);
    }
    st.sel_gesture = Some(SelGesture::Resize { handle, start, base: Some(base) });
    true
}

fn release(st: &mut State, ev: &PointerEvent) -> bool {
    let Some(SelGesture::Resize { .. }) = st.sel_gesture else { return false };
    if ev.image.is_some() {
        drag(st, ev);
    }
    st.sel_gesture = None;
    true
}
