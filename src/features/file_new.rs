//! Feature file.new: File > New (Ctrl+N) and the colour reset.

use crate::doc::Document;
use crate::features::file_save_changes_prompt::{guard, Pending};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("File > New", new_command);
    r.shortcut(Key::Char('n'), Mods::CTRL, "File > New");
}

/// File > New: the save-changes check, then [`new_document`].
fn new_command(st: &mut State) -> HResult<()> {
    guard(st, Pending::New)
}

/// Replace the document with a clean untitled image of the default size,
/// filled with the current background colour, then reset the colour
/// indicator to black on white. Tool, options and redefined swatches stay;
/// a floating selection is dropped without being committed.
pub(crate) fn new_document(st: &mut State) -> HResult<()> {
    let (w, h) = st.default_new_size();
    let doc = Document::new(w, h, st.bg)?;
    st.replace_document(doc);
    st.fg = Rgb::K;
    st.bg = Rgb::W;
    Ok(())
}
