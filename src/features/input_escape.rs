//! Feature input.escape: the Esc key cancels the operation in progress.
//!
//! Dialogs (Esc = Cancel) and open menus (Esc closes) are handled before the
//! key gets here. What remains, in order:
//! 1. a pointer gesture with the button held is cancelled: canvas handle
//!    drag, selection move/resize, Select / Free-Form drag, pencil, brush,
//!    eraser and airbrush strokes, Line and shape drags. The picture and the
//!    selection go back to their state at the press and no undo step is left;
//!    the rest of the gesture, up to the release, is ignored;
//! 2. the active tool's own Esc (unfinished Curve or Polygon is discarded, a
//!    text frame is committed);
//! 3. a selection that is not being dragged ends, committing floating pixels.

use crate::doc::Snapshot;
use crate::registry::{registry, Registry};
use crate::selection::Sel;
use crate::state::State;
use crate::tools::PointerEvent;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.pointer_hook(remember_press);
}

/// What a cancelled selection gesture puts back.
#[derive(Debug, Clone)]
pub struct EscBase {
    picture: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    dirty: bool,
    sel: Option<Sel>,
}

/// Pointer hook that never consumes: at a press it records the document and
/// selection for the selection gestures that follow.
fn remember_press(st: &mut State, _ev: &PointerEvent) -> HResult<bool> {
    if matches!(st.ptr_kind, Some(PointerKind::Press(_))) {
        let relevant = st.sel.is_some() || matches!(st.tool, Tool::Select | Tool::FreeFormSelect);
        st.esc_base = relevant.then(|| EscBase {
            picture: st.doc.snapshot(),
            undo: st.doc.undo.clone(),
            redo: st.doc.redo.clone(),
            dirty: st.doc.dirty,
            sel: st.sel.clone(),
        });
    }
    Ok(false)
}

/// Cancel the selection gesture in progress; true when there was one.
fn cancel_selection_gesture(st: &mut State) -> bool {
    if st.sel_gesture.is_none() && st.sel_drag.is_none() {
        return false;
    }
    st.sel_gesture = None;
    st.sel_drag = None;
    if let Some(base) = st.esc_base.take() {
        st.doc.restore(base.picture);
        st.doc.undo = base.undo;
        st.doc.redo = base.redo;
        st.doc.dirty = base.dirty;
        st.sel = base.sel;
    }
    true
}

/// Cancel the button-held gesture, if any. Returns true when something was
/// cancelled and the tool's own Esc must not run on top of it.
pub(crate) fn cancel_gesture(st: &mut State) -> bool {
    let Some(h) = st.held else { return false };
    if h.cancelled {
        return false;
    }
    let mut done = super::canvas_resize_handle::cancel_drag(st);
    done |= cancel_selection_gesture(st);
    let had_stroke = st.stroke_base.is_some();
    super::mouse_colors::cancel_stroke(st);
    done |= had_stroke;
    // Gestures that only show a preview until the release.
    done |= matches!(st.tool, Tool::Line | Tool::Rectangle | Tool::Ellipse | Tool::RoundedRectangle);
    if matches!(st.tool, Tool::Text) && matches!(st.text_drag, Some(crate::features::tool_text::TextDrag::Create { .. })) {
        st.text_drag = None;
    }
    // Curve, Polygon and Text finish their own Esc below; the release of
    // this press must not act on what they discarded.
    let own = matches!(st.tool, Tool::Curve | Tool::Polygon | Tool::Text);
    if done || own {
        if let Some(h) = st.held.as_mut() {
            h.cancelled = true;
        }
    }
    done && !own
}

/// Handle an Esc key press that no dialog or menu took. Returns true when it
/// was consumed.
pub fn escape(st: &mut State, mods: Mods) -> HResult<bool> {
    let cancelled = cancel_gesture(st);
    if cancelled {
        return Ok(true);
    }
    if let Some(h) = registry().tool_handler(st.tool) {
        if h.key(st, Key::Escape, mods)? {
            return Ok(true);
        }
    }
    if st.sel.is_some() {
        st.sel_drag = None;
        st.end_selection();
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::{App, Button, Key, Mods, PointerKind, Pos, Rgb, StartOptions, Tool};

    fn app(tag: &str) -> App {
        let mut a = App::start(StartOptions::new(std::env::temp_dir().join(format!("brushling-esc-{}-{tag}", std::process::id())))).unwrap();
        a.new_document(32, 32, Rgb::W).unwrap();
        a
    }
    fn ptr(a: &mut App, k: PointerKind, x: i32, y: i32) {
        a.pointer(k, Pos::Image(x, y), Mods::NONE).unwrap();
    }
    fn esc(a: &mut App) {
        a.key(Key::Escape, Mods::NONE).unwrap();
    }
    fn blank(a: &App) -> bool {
        a.pixels().unwrap().pixels.iter().all(|p| *p == Rgb::W)
    }

    #[test]
    fn shape_and_brush_drags_cancel() {
        for tool in [Tool::Rectangle, Tool::Ellipse, Tool::RoundedRectangle, Tool::Brush, Tool::Eraser, Tool::Airbrush] {
            let mut a = app("shape");
            a.select_tool(tool).unwrap();
            ptr(&mut a, PointerKind::Press(Button::Left), 3, 3);
            ptr(&mut a, PointerKind::Move, 20, 20);
            esc(&mut a);
            ptr(&mut a, PointerKind::Release(Button::Left), 20, 20);
            assert!(blank(&a), "{tool:?}");
            assert!(a.st.doc.undo.is_empty(), "{tool:?}");
            assert!(a.render(crate::Surface::Composite).unwrap().pixels.iter().all(|p| *p == Rgb::W), "{tool:?}");
        }
    }

    #[test]
    fn select_drag_cancel_leaves_no_selection() {
        let mut a = app("seldrag");
        a.select_tool(Tool::Select).unwrap();
        ptr(&mut a, PointerKind::Press(Button::Left), 2, 2);
        ptr(&mut a, PointerKind::Move, 9, 9);
        esc(&mut a);
        ptr(&mut a, PointerKind::Release(Button::Left), 9, 9);
        assert!(a.selection().unwrap().is_none());
    }

    #[test]
    fn selection_move_cancel_restores_picture_and_selection() {
        let mut a = app("selmove");
        a.set_pixel(2, 2, Rgb::K).unwrap();
        a.select_tool(Tool::Select).unwrap();
        ptr(&mut a, PointerKind::Press(Button::Left), 1, 1);
        ptr(&mut a, PointerKind::Move, 4, 4);
        ptr(&mut a, PointerKind::Release(Button::Left), 4, 4);
        let before = a.selection().unwrap().unwrap().bounds;
        ptr(&mut a, PointerKind::Press(Button::Left), 2, 2);
        ptr(&mut a, PointerKind::Move, 12, 2);
        esc(&mut a);
        ptr(&mut a, PointerKind::Release(Button::Left), 12, 2);
        assert_eq!(a.selection().unwrap().unwrap().bounds, before);
        assert_eq!(a.pixel(2, 2).unwrap(), Rgb::K);
        assert_eq!(a.pixel(12, 2).unwrap(), Rgb::W);
        assert!(a.st.doc.undo.is_empty());
    }

    #[test]
    fn curve_and_polygon_discarded() {
        let mut a = app("curve");
        a.select_tool(Tool::Curve).unwrap();
        ptr(&mut a, PointerKind::Press(Button::Left), 2, 2);
        ptr(&mut a, PointerKind::Move, 20, 2);
        esc(&mut a);
        ptr(&mut a, PointerKind::Release(Button::Left), 20, 2);
        ptr(&mut a, PointerKind::Press(Button::Left), 10, 10);
        ptr(&mut a, PointerKind::Release(Button::Left), 10, 10);
        assert!(blank(&a));
        let mut a = app("poly");
        a.select_tool(Tool::Polygon).unwrap();
        ptr(&mut a, PointerKind::Press(Button::Left), 2, 2);
        ptr(&mut a, PointerKind::Move, 20, 2);
        ptr(&mut a, PointerKind::Release(Button::Left), 20, 2);
        esc(&mut a);
        assert!(blank(&a));
        assert!(a.render(crate::Surface::Composite).unwrap().pixels.iter().all(|p| *p == Rgb::W));
    }
}
