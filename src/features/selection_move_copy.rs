//! Feature selection.move_copy: dragging inside a selection (with a Select
//! or Free-Form Select tool active) moves it. The first move lifts a fresh
//! selection (source filled with the background colour, or left alone when
//! Ctrl was held at the press). Ctrl at the press stamps a copy at the
//! current position before moving; Shift stamps the content at every move
//! event, smearing a trail. Press-to-release is one undo step; a press and
//! release without movement changes nothing.

use crate::registry::Registry;
use crate::selection::SelGesture;
use crate::state::State;
use crate::tools::PointerEvent;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.pointer_hook(hook);
}

fn hook(st: &mut State, ev: &PointerEvent) -> HResult<bool> {
    match st.ptr_kind {
        Some(PointerKind::Press(_)) => Ok(press(st, ev)),
        Some(PointerKind::Move) => Ok(drag(st, ev)),
        Some(PointerKind::Release(_)) => Ok(release(st, ev)),
        None => Ok(false),
    }
}

fn press(st: &mut State, ev: &PointerEvent) -> bool {
    if !matches!(st.tool, Tool::Select | Tool::FreeFormSelect) {
        return false;
    }
    // Right-click with a selection tool does nothing.
    if ev.button != Some(Button::Left) {
        return false;
    }
    let (Some(sel), Some((x, y))) = (st.sel.as_ref(), ev.image) else { return false };
    let b = sel.bounds;
    if x < b.x0 || x > b.x1 || y < b.y0 || y > b.y1 {
        return false;
    }
    st.sel_drag = None;
    st.sel_gesture = Some(SelGesture::Move { last: (x, y), ctrl: ev.mods.ctrl, started: false });
    true
}

fn drag(st: &mut State, ev: &PointerEvent) -> bool {
    let Some(SelGesture::Move { last, ctrl, started }) = st.sel_gesture.clone() else { return false };
    let Some(p) = ev.image else { return true };
    if !started {
        // One undo step per drag, recorded before the source is touched.
        st.push_undo_sel();
        let fresh = st.sel.as_ref().is_some_and(|s| !s.is_floating());
        if fresh {
            st.lift_selection(!ctrl);
        }
        if ctrl {
            st.stamp_selection();
        }
    }
    let (dx, dy) = (p.0 - last.0, p.1 - last.1);
    if let Some(sel) = st.sel.as_mut() {
        let b = sel.bounds;
        sel.bounds = Rect::new(b.x0 + dx, b.y0 + dy, b.x1 + dx, b.y1 + dy);
    }
    if ev.mods.shift {
        st.stamp_selection();
    }
    st.sel_gesture = Some(SelGesture::Move { last: p, ctrl, started: true });
    true
}

fn release(st: &mut State, ev: &PointerEvent) -> bool {
    let Some(SelGesture::Move { last, .. }) = st.sel_gesture else { return false };
    // A release away from the last position counts as a final move.
    if ev.image.is_some_and(|p| p != last) {
        drag(st, ev);
    }
    st.sel_gesture = None;
    true
}
