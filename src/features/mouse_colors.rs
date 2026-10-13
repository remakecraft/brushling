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
    matches!(t, Tool::Pencil | Tool::Brush | Tool::Airbrush | Tool::Line | Tool::Eraser)
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
    st.pencil_base = None;
    st.pencil_active = false;
    st.pencil_pushed = false;
    if let Some(h) = st.held.as_mut() {
        h.cancelled = true;
    }
}
