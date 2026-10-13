//! Feature edit.undo_repeat: Edit > Undo (Ctrl+Z, Alt+Backspace) and
//! Edit > Repeat (F4) over the bounded snapshot history in `doc.rs`.
//! `Document::push_undo` records steps (capped, and clears the repeat list).

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command_if("Edit > Undo", undo, Some(can_undo));
    r.command_if("Edit > Repeat", repeat, Some(can_repeat));
    r.shortcut(Key::Char('z'), Mods::CTRL, "Edit > Undo");
    r.shortcut(Key::Backspace, Mods::ALT, "Edit > Undo");
    r.shortcut(Key::F4, Mods::NONE, "Edit > Repeat");
}

fn can_undo(st: &State) -> bool {
    crate::features::tool_text::frame_active(st) || !st.doc.undo.is_empty()
}

fn can_repeat(st: &State) -> bool {
    !st.doc.redo.is_empty()
}

/// Undo and Repeat throw away an unfinished Curve or Polygon (it is never
/// restored) before stepping through the history.
fn discard_unfinished(st: &mut State) {
    st.curve = Default::default();
    st.poly = Default::default();
}

/// Undo steps the history, restoring the selection with the picture for
/// steps that recorded one (a floating selection is part of such a step).
/// While a text frame is active it undoes the last text edit instead.
fn undo(st: &mut State) -> HResult<()> {
    if crate::features::tool_text::undo_text_edit(st) {
        return Ok(());
    }
    discard_unfinished(st);
    st.step_history(true);
    Ok(())
}

fn repeat(st: &mut State) -> HResult<()> {
    discard_unfinished(st);
    st.step_history(false);
    Ok(())
}
