//! Feature view.view_bitmap: View > View Bitmap (Ctrl+F) shows the image alone
//! on a black screen-sized surface at 100% until any mouse press or key press.
//! That input only leaves the mode. The view is a model: `Surface::FullScreen`
//! renders it, `App::view_mode` reports it.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("View > View Bitmap", enter);
    r.shortcut(Key::Char('f'), Mods::CTRL, "View > View Bitmap");
}

fn enter(st: &mut State) -> HResult<()> {
    st.view_bitmap = true;
    Ok(())
}

/// `App::view_mode`.
pub fn mode(st: &State) -> ViewMode {
    if st.view_bitmap {
        ViewMode::ViewBitmap
    } else {
        ViewMode::Normal
    }
}

/// Input filter: while the full-screen view is up, a key press or a mouse
/// button press ends it and is consumed; every other input is swallowed.
/// Returns true when the event must not reach the normal window.
pub fn swallow(st: &mut State, ends_view: bool) -> bool {
    if !st.view_bitmap {
        return false;
    }
    if ends_view {
        st.view_bitmap = false;
    }
    true
}

/// Offset that centres `image` on `screen` along one axis; an image at least
/// as large as the screen starts at 0 (its top-left part is shown).
fn offset(screen: u32, image: u32) -> usize {
    if image >= screen {
        0
    } else {
        ((screen - image) / 2) as usize
    }
}

/// `Surface::FullScreen`: the displayed document (floating selection and text
/// frame included, as if committed) centred on black.
pub fn render(st: &State) -> HResult<Image> {
    if !st.view_bitmap {
        return Err(HarnessError::Invalid("View Bitmap is not active".to_owned()));
    }
    let comp = st.render(Surface::Composite)?;
    let (sw, sh) = st.screen;
    let mut out = Image { width: sw, height: sh, pixels: vec![Rgb::K; sw as usize * sh as usize] };
    let (ox, oy) = (offset(sw, comp.width), offset(sh, comp.height));
    let w = comp.width.min(sw) as usize;
    let h = comp.height.min(sh) as usize;
    for y in 0..h {
        let src = y * comp.width as usize;
        let dst = (oy + y) * sw as usize + ox;
        out.pixels[dst..dst + w].copy_from_slice(&comp.pixels[src..src + w]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::offset;

    #[test]
    fn centring_floors_and_large_images_start_at_zero() {
        assert_eq!(offset(1024, 100), 462);
        assert_eq!(offset(768, 100), 334);
        assert_eq!(offset(1024, 1024), 0);
        assert_eq!(offset(1024, 2000), 0);
        assert_eq!(offset(5, 2), 1);
    }
}
