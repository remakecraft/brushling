//! Feature view.thumbnail: View > Zoom > Show Thumbnail, a small floating
//! window showing the image at 100% from the top-left image pixel visible in
//! the magnified main view. It is a live view of the displayed document
//! (`Surface::Thumbnail`), closes when the zoom returns to 100% and reports
//! through `App::panel(Panel::Thumbnail)`.

use crate::features::{view_panels, view_zoom};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

/// Client area of the thumbnail window.
pub const SIZE: (u32, u32) = (200, 150);

pub fn register(r: &mut Registry) {
    r.command_if("View > Zoom > Show Thumbnail", toggle, Some(available));
}

/// The item is enabled only while the main view is magnified.
fn available(st: &State) -> bool {
    st.zoom > 100
}

fn toggle(st: &mut State) -> HResult<()> {
    let show = !st.panels.thumbnail;
    view_panels::set_visible(st, Panel::Thumbnail, show);
    Ok(())
}

/// Called by the zoom code after every zoom change: back at 100% there is
/// nothing to magnify, so the window closes and the item is unchecked.
pub fn on_zoom_changed(st: &mut State) {
    if st.zoom <= 100 && st.panels.thumbnail {
        view_panels::set_visible(st, Panel::Thumbnail, false);
    }
}

/// `Surface::Thumbnail`.
pub fn render(st: &State) -> HResult<Image> {
    if !st.panels.thumbnail {
        return Err(HarnessError::Invalid("the thumbnail window is not shown".to_owned()));
    }
    let comp = st.render(Surface::Composite)?;
    // First image pixel visible at the workspace's top-left corner.
    let (x0, y0) = view_zoom::screen_to_image(st, 0, 0);
    let (x0, y0) = (x0.max(0) as i64, y0.max(0) as i64);
    let (w, h) = SIZE;
    let mut out = Image { width: w, height: h, pixels: vec![crate::render::WORKSPACE_GREY; (w * h) as usize] };
    for y in 0..h as i64 {
        let sy = y0 + y;
        if sy >= comp.height as i64 {
            break;
        }
        for x in 0..w as i64 {
            let sx = x0 + x;
            if sx >= comp.width as i64 {
                break;
            }
            out.pixels[(y * w as i64 + x) as usize] = comp.pixels[(sy * comp.width as i64 + sx) as usize];
        }
    }
    Ok(out)
}
