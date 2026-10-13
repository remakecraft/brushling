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
    if st.sel.is_none() {
        return Ok(());
    }
    // Clearing is one undo step whether it erases pixels or, for a floating
    // selection (source area already vacated at the lift), only discards the
    // floating content.
    st.doc.push_undo();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn clearing_a_floating_selection_is_one_undo_step() {
        let dir = std::env::temp_dir().join(format!("brushling-clearsel-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.replace_document(crate::doc::Document::new(32, 32, Rgb::W).unwrap());
        st.set_selection(Sel::rect(Rect::new(2, 2, 5, 5)));
        let mut sel = st.sel.take().unwrap();
        sel.floating = Some(vec![Rgb::K; 16]);
        st.sel = Some(sel);
        assert!(st.sel.as_ref().unwrap().is_floating());
        clear_selection(&mut st).unwrap();
        assert!(st.sel.is_none());
        assert_eq!(st.doc.undo.len(), 1);
    }
}
