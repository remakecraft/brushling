//! Feature tools.mouse_colors: which colour the left and right buttons draw
//! with, and what the second button does mid-stroke.
//!
//! The left button paints the foreground colour, the right button the
//! background colour; Ctrl changes nothing (there is no third colour). The
//! button that starts a stroke decides its colour for the whole stroke.
//! Pressing the other button during a stroke cancels it: the document goes
//! back to its state at the press and no undo step is left behind.

use crate::doc::Snapshot;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// Colour a tool paints with for a gesture driven by `button` (`None`, a
/// plain hover, counts as the left button).
pub fn paint_colour(st: &State, button: Option<Button>) -> Rgb {
    if button == Some(Button::Right) {
        st.bg
    } else {
        st.fg
    }
}

/// Everything a cancelled stroke has to put back: the picture and the undo
/// bookkeeping (the capped history may have dropped its oldest step).
#[derive(Debug, Clone)]
pub struct StrokeBase {
    picture: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    dirty: bool,
}

/// Tools whose press-to-release gesture is a stroke that can be cancelled.
fn is_stroke_tool(t: Tool) -> bool {
    matches!(
        t,
        Tool::Pencil
            | Tool::Brush
            | Tool::Airbrush
            | Tool::Line
            | Tool::Eraser
            | Tool::Fill
            | Tool::Rectangle
            | Tool::Ellipse
            | Tool::RoundedRectangle
    )
}

/// Called on every canvas press, before the tool sees it: remember the
/// document so the gesture can be cancelled.
pub fn begin_stroke(st: &mut State) {
    st.stroke_base = is_stroke_tool(st.tool).then(|| StrokeBase {
        picture: st.doc.snapshot(),
        undo: st.doc.undo.clone(),
        redo: st.doc.redo.clone(),
        dirty: st.doc.dirty,
    });
}

/// Cancel the stroke in progress, if any: pixels restored, no undo step. The
/// held button stays down but is marked cancelled, so nothing it does until
/// its release reaches the tool.
pub fn cancel_stroke(st: &mut State) {
    let Some(base) = st.stroke_base.take() else { return };
    st.doc.restore(base.picture);
    st.doc.undo = base.undo;
    st.doc.redo = base.redo;
    st.doc.dirty = base.dirty;
    st.pencil_last = None;
    st.eraser_last = None;
    st.brush_stroke = None;
    st.pencil_base = None;
    st.pencil_active = false;
    st.pencil_pushed = false;
    if let Some(h) = st.held.as_mut() {
        h.cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state(tool: Tool) -> State {
        let dir = std::env::temp_dir().join(format!("brushling-mouse-colors-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.replace_document(crate::doc::Document::new(32, 32, Rgb::W).unwrap());
        st.select_tool(tool).unwrap();
        st
    }

    #[test]
    fn other_button_cancels_shapes_and_fill() {
        for tool in [Tool::Rectangle, Tool::Ellipse, Tool::RoundedRectangle, Tool::Fill] {
            let mut st = state(tool);
            st.pointer(PointerKind::Press(Button::Left), Pos::Image(2, 2), Mods::NONE).unwrap();
            st.pointer(PointerKind::Move, Pos::Image(20, 15), Mods::NONE).unwrap();
            st.pointer(PointerKind::Press(Button::Right), Pos::Image(20, 15), Mods::NONE).unwrap();
            assert!(st.held.is_some_and(|h| h.cancelled), "{tool:?} not cancelled");
            st.pointer(PointerKind::Release(Button::Left), Pos::Image(20, 15), Mods::NONE).unwrap();
            assert!(st.doc.pixels.iter().all(|p| *p == Rgb::W), "{tool:?} left pixels");
            assert!(st.doc.undo.is_empty(), "{tool:?} left an undo step");
            assert!(!st.doc.dirty);
        }
    }

    #[test]
    fn shape_preview_stops_once_cancelled() {
        let mut st = state(Tool::Rectangle);
        st.pointer(PointerKind::Press(Button::Left), Pos::Image(2, 2), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Image(20, 15), Mods::NONE).unwrap();
        let img = st.render(Surface::Composite).unwrap();
        assert!(img.pixels.iter().any(|p| *p == Rgb::K));
        st.pointer(PointerKind::Press(Button::Right), Pos::Image(20, 15), Mods::NONE).unwrap();
        let img = st.render(Surface::Composite).unwrap();
        assert!(img.pixels.iter().all(|p| *p == Rgb::W));
    }
}
