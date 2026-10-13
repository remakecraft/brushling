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
        (Some(PointerKind::Press(button)), Pos::SelectionHandle(handle)) => {
            // Right-click with a selection tool does nothing.
            if button != Button::Left {
                return Ok(false);
            }
            let Some(sel) = st.sel.as_ref() else { return Ok(false) };
            st.sel_gesture = Some(SelGesture::Resize { handle, start: sel.bounds, press: ev.image.or(st.pointer_image), base: None });
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
    let Some(SelGesture::Resize { handle, start, press, base }) = st.sel_gesture.clone() else { return false };
    let Some(p) = ev.image else { return true };
    let base = match base {
        Some(b) => b,
        None => {
            // First move: the undo step and the lift happen now, and the
            // content at this moment is what every later move resamples.
            st.push_undo_sel();
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
    st.sel_gesture = Some(SelGesture::Resize { handle, start, press, base: Some(base) });
    true
}

fn release(st: &mut State, ev: &PointerEvent) -> bool {
    let Some(SelGesture::Resize { press, base, .. }) = st.sel_gesture.clone() else { return false };
    // A click on a handle without movement changes nothing; the release only
    // counts as a final move when the drag began or the pointer left the
    // press point.
    if ev.image.is_some() && (base.is_some() || ev.image != press) {
        drag(st, ev);
    }
    st.sel_gesture = None;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Sel;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-selresize-{}", std::process::id()));
        let mut st = State::new(crate::settings::Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.new_default_document().unwrap();
        st.select_tool(Tool::Select).unwrap();
        st.set_selection(Sel::rect(Rect::new(2, 2, 5, 5)));
        st
    }

    #[test]
    fn click_on_a_handle_without_moving_changes_nothing() {
        let mut st = state();
        let bounds = st.sel.as_ref().unwrap().bounds;
        // The pointer hovers over the handle first, one pixel outside the bounds.
        st.pointer(PointerKind::Move, Pos::Image(6, 6), Mods::NONE).unwrap();
        st.pointer(PointerKind::Press(Button::Left), Pos::SelectionHandle(Handle::BottomRight), Mods::NONE).unwrap();
        // In the GUI a click on the handle sits one pixel outside the bounds.
        st.pointer(PointerKind::Release(Button::Left), Pos::Image(6, 6), Mods::NONE).unwrap();
        let sel = st.sel.as_ref().unwrap();
        assert_eq!(sel.bounds, bounds);
        assert!(!sel.is_floating());
        assert!(st.doc.undo.is_empty());
    }

    #[test]
    fn dragging_a_handle_resizes_in_one_undo_step() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), Pos::SelectionHandle(Handle::BottomRight), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Image(8, 8), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), Pos::Image(8, 8), Mods::NONE).unwrap();
        assert_eq!(st.sel.as_ref().unwrap().bounds, Rect::new(2, 2, 8, 8));
        assert_eq!(st.doc.undo.len(), 1);
    }

    #[test]
    fn right_button_on_a_handle_does_nothing() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Right), Pos::SelectionHandle(Handle::BottomRight), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Image(8, 8), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Right), Pos::Image(8, 8), Mods::NONE).unwrap();
        assert_eq!(st.sel.as_ref().unwrap().bounds, Rect::new(2, 2, 5, 5));
        assert!(st.doc.undo.is_empty());
    }

    #[test]
    fn undo_restores_floating_content_after_a_second_drag() {
        let mut st = state();
        st.doc.put(3, 3, Rgb::K);
        st.set_selection(Sel::rect(Rect::new(2, 2, 5, 5)));
        let drag = |st: &mut State, from: (i32, i32), to: (i32, i32)| {
            st.pointer(PointerKind::Press(Button::Left), Pos::Image(from.0, from.1), Mods::NONE).unwrap();
            st.pointer(PointerKind::Move, Pos::Image(to.0, to.1), Mods::NONE).unwrap();
            st.pointer(PointerKind::Release(Button::Left), Pos::Image(to.0, to.1), Mods::NONE).unwrap();
        };
        drag(&mut st, (3, 3), (13, 3));
        drag(&mut st, (13, 3), (13, 10));
        let moved = st.sel.as_ref().unwrap().bounds;
        st.step_history(true);
        let sel = st.sel.as_ref().expect("selection survives Undo");
        assert!(sel.is_floating(), "floating content survives Undo");
        assert_eq!(sel.bounds, Rect::new(12, 2, 15, 5));
        assert!(sel.floating.as_ref().unwrap().contains(&Rgb::K));
        st.step_history(false);
        assert_eq!(st.sel.as_ref().unwrap().bounds, moved);
    }
}
