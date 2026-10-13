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
    !st.doc.undo.is_empty()
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

/// Floating content is not part of a snapshot; once the picture steps back
/// across the lift that created it, the floating selection is gone.
fn drop_floating(st: &mut State) {
    if st.sel.as_ref().is_some_and(|s| s.is_floating()) {
        st.discard_selection();
    }
}

fn undo(st: &mut State) -> HResult<()> {
    discard_unfinished(st);
    if let Some(snap) = st.doc.undo.pop() {
        let current = st.doc.restore(snap);
        st.doc.redo.push(current);
        drop_floating(st);
    }
    Ok(())
}

fn repeat(st: &mut State) -> HResult<()> {
    discard_unfinished(st);
    if let Some(snap) = st.doc.redo.pop() {
        let current = st.doc.restore(snap);
        st.doc.undo.push(current);
        drop_floating(st);
    }
    Ok(())
}
