//! Feature edit.clear_select_all: Edit > Clear Selection (Del) fills the
//! selected pixels with the background colour and removes the selection;
//! Edit > Select All (Ctrl+A) commits any selection, switches to the Select
//! tool and selects the whole image.

use crate::registry::Registry;
use crate::selection::Sel;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command_if("Edit > Clear Selection", clear_selection, Some(State::has_selection));
    r.shortcut(Key::Delete, Mods::NONE, "Edit > Clear Selection");
    r.command("Edit > Select All", select_all);
    r.shortcut(Key::Char('a'), Mods::CTRL, "Edit > Select All");
}

fn clear_selection(st: &mut State) -> HResult<()> {
    let Some(sel) = st.sel.as_ref() else { return Ok(()) };
    // A floating selection's content is discarded; its source area was
    // already vacated when it was lifted, so only unlifted ones edit pixels.
    if !sel.is_floating() {
        st.doc.push_undo();
    }
    st.erase_selection();
    Ok(())
}

fn select_all(st: &mut State) -> HResult<()> {
    st.end_selection();
    st.select_tool(Tool::Select)?;
    let bounds = Rect::new(0, 0, st.doc.width as i32 - 1, st.doc.height as i32 - 1);
    st.set_selection(Sel::rect(bounds));
    Ok(())
}
