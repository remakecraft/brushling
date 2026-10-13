//! Feature image.clear_image: Image > Clear Image (Ctrl+Shift+N) fills the
//! whole image with the background colour. An active selection is dropped
//! first, without committing floating content.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("Image > Clear Image", clear_image);
    r.shortcut(Key::Char('n'), Mods::CTRL_SHIFT, "Image > Clear Image");
}

fn clear_image(st: &mut State) -> HResult<()> {
    st.discard_selection();
    st.doc.push_undo();
    let bg = st.bg;
    for y in 0..st.doc.height as i32 {
        for x in 0..st.doc.width as i32 {
            // `put` applies the colour-mode and palette snapping rules.
            st.doc.put(x, y, bg);
        }
    }
    Ok(())
}
