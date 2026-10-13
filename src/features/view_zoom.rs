//! Feature view.zoom: View > Zoom > Normal Size / Large Size / Custom...,
//! their shortcuts, the scroll offset rules and the screen/image mapping used
//! by every feature that has to know where an image pixel is drawn.
//!
//! Zoom is `State::zoom` (percent); the canvas origin on screen is
//! `CANVAS_MARGIN - scroll`. [`set_zoom`] is the only place that changes the
//! zoom: it keeps the top-left visible image pixel in place, clamps the
//! scroll and tells the Text tool.

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::registry::Registry;
use crate::render::CANVAS_MARGIN;
use crate::state::State;
use crate::types::*;

/// Zoom of Large Size and of the magnifier's zoom-in click.
pub const LARGE_ZOOM: u32 = 400;
/// The choices of the Custom Zoom dialog, in display order.
pub const CHOICES: [&str; 5] = ["100%", "200%", "400%", "600%", "800%"];

pub fn register(r: &mut Registry) {
    r.command("View > Zoom > Normal Size", normal_size);
    r.command("View > Zoom > Large Size", large_size);
    r.command("View > Zoom > Custom...", custom);
    r.shortcut(Key::PageDown, Mods::CTRL, "View > Zoom > Normal Size");
    r.shortcut(Key::PageUp, Mods::CTRL, "View > Zoom > Large Size");
}

fn normal_size(st: &mut State) -> HResult<()> {
    st.show_grid = false;
    set_zoom(st, 100)
}

fn large_size(st: &mut State) -> HResult<()> {
    set_zoom(st, LARGE_ZOOM)
}

/// Change the zoom, keeping the image pixel at the top-left of the visible
/// area at the top-left where possible. Choosing the current zoom does
/// nothing. Never touches the document or the undo history.
pub fn set_zoom(st: &mut State, zoom: u32) -> HResult<()> {
    set_zoom_at(st, zoom, None)
}

/// Like [`set_zoom`], but with an explicit new scroll offset (screen pixels at
/// the new zoom) instead of the preserved top-left pixel.
pub fn set_zoom_at(st: &mut State, zoom: u32, scroll: Option<(i32, i32)>) -> HResult<()> {
    if zoom == st.zoom {
        return Ok(());
    }
    let old = st.scale() as i32;
    st.zoom = zoom;
    let new = st.scale() as i32;
    st.scroll = scroll.unwrap_or((
        st.scroll.0.div_euclid(old).saturating_mul(new),
        st.scroll.1.div_euclid(old).saturating_mul(new),
    ));
    clamp_scroll(st);
    st.mag_hover = None;
    crate::features::view_thumbnail::on_zoom_changed(st);
    crate::features::tool_text::on_zoom_changed(st)
}

/// Largest scroll offset per axis: the zoomed canvas plus its margins must
/// not leave empty space beyond the workspace edge.
pub fn max_scroll(st: &State) -> (i32, i32) {
    let z = st.scale() as i64;
    let axis = |len: u32, view: u32| {
        (len as i64 * z + 2 * CANVAS_MARGIN as i64 - view as i64).clamp(0, i32::MAX as i64) as i32
    };
    (axis(st.doc.width, st.workspace_size.0), axis(st.doc.height, st.workspace_size.1))
}

/// Pull the scroll offset back into the canvas.
pub fn clamp_scroll(st: &mut State) {
    let (mx, my) = max_scroll(st);
    st.scroll = (st.scroll.0.clamp(0, mx), st.scroll.1.clamp(0, my));
}

/// Workspace pixel -> image pixel (floor; may lie outside the image).
pub(crate) fn screen_to_image(st: &State, sx: i32, sy: i32) -> (i32, i32) {
    let z = st.scale() as i32;
    ((sx - CANVAS_MARGIN + st.scroll.0).div_euclid(z), (sy - CANVAS_MARGIN + st.scroll.1).div_euclid(z))
}

/// Workspace position of the top-left screen pixel of image pixel (x,y).
pub(crate) fn image_to_screen(st: &State, x: i32, y: i32) -> (i64, i64) {
    let z = st.scale() as i64;
    (CANVAS_MARGIN as i64 - st.scroll.0 as i64 + x as i64 * z, CANVAS_MARGIN as i64 - st.scroll.1 as i64 + y as i64 * z)
}

// ----- Custom Zoom dialog

fn custom(st: &mut State) -> HResult<()> {
    let current = format!("{}%", st.zoom);
    let dialog = Dialog {
        kind: DialogKind::CustomZoom,
        title: "Custom Zoom".to_owned(),
        text: String::new(),
        buttons: vec!["OK".to_owned(), "Cancel".to_owned()],
        fields: vec![OpenDialog::field("Zoom", &current)],
    };
    st.open_dialog(OpenDialog::new(dialog, custom_handler));
    Ok(())
}

/// Parse a dialog choice such as `600%`.
fn parse_choice(value: &str) -> Option<u32> {
    let v = value.trim();
    CHOICES.iter().find(|c| c.eq_ignore_ascii_case(v)).and_then(|c| c.trim_end_matches('%').parse().ok())
}

fn custom_handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            if parse_choice(value).is_none() {
                return Err(HarnessError::Invalid(format!("{value:?} is not one of {CHOICES:?}")));
            }
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("OK") => {
            if let Some(z) = od.dialog.field("Zoom").and_then(parse_choice) {
                set_zoom(st, z)?;
            }
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-zoom-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.workspace_size = (100, 100);
        st
    }

    #[test]
    fn zoom_keeps_top_left_pixel_and_clamps() {
        let mut st = state();
        st.doc = crate::doc::Document::new(100, 100, Rgb::W).unwrap();
        set_zoom(&mut st, 400).unwrap();
        st.scroll = (40, 80);
        set_zoom(&mut st, 800).unwrap();
        assert_eq!(st.scroll, (80, 160));
        set_zoom(&mut st, 100).unwrap();
        assert_eq!(st.scroll, (8, 8));
    }

    #[test]
    fn mapping_roundtrip() {
        let mut st = state();
        st.zoom = 600;
        st.scroll = (12, 0);
        let (sx, sy) = image_to_screen(&st, 5, 2);
        assert_eq!(screen_to_image(&st, sx as i32, sy as i32), (5, 2));
        assert_eq!(screen_to_image(&st, sx as i32 + 5, sy as i32 + 5), (5, 2));
    }
}
