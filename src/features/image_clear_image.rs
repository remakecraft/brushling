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
    // One undo step that remembers the selection, so Undo brings it back.
    st.push_undo_sel();
    st.discard_selection();
    let bg = st.bg;
    for y in 0..st.doc.height as i32 {
        for x in 0..st.doc.width as i32 {
            // `paint` applies the colour-mode and palette snapping rules.
            st.doc.paint(x, y, bg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn undo_brings_the_dropped_selection_back() {
        let dir = std::env::temp_dir().join(format!("brushling-clear-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        let b = Rect::new(1, 1, 2, 2);
        st.sel = Some(crate::selection::Sel { bounds: b, mask: vec![true; 4], floating: None, origin: b });
        let sel = st.sel.clone();
        clear_image(&mut st).unwrap();
        assert!(st.sel.is_none());
        st.step_history(true);
        assert_eq!(st.sel, sel);
    }
}
