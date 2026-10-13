//! Feature view.grid: View > Zoom > Show Grid (Ctrl+G). A workspace-only
//! overlay of 1 px grey lines on the leftmost column and topmost row of every
//! image pixel's block, drawn only when the box is checked and zoom > 200%.
//! The check mark is `State::show_grid`; the overlay never reaches the
//! document, saved files or the clipboard.

use crate::features::view_zoom::image_to_screen;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

/// Grid line colour.
pub const GRID_GREY: Rgb = Rgb::new(128, 128, 128);

pub fn register(r: &mut Registry) {
    r.command("View > Zoom > Show Grid", toggle);
    r.shortcut(Key::Char('g'), Mods::CTRL, "View > Zoom > Show Grid");
    r.workspace_layer(layer);
}

fn toggle(st: &mut State) -> HResult<()> {
    st.show_grid = !st.show_grid;
    Ok(())
}

/// Whether grid lines are visible at this zoom.
pub fn visible(st: &State) -> bool {
    st.show_grid && st.zoom > 200
}

fn layer(st: &State, img: &mut Image) {
    if !visible(st) {
        return;
    }
    let z = st.scale() as i64;
    let (ox, oy) = image_to_screen(st, 0, 0);
    let (w, h) = (img.width as i64, img.height as i64);
    // Canvas extent on screen, clipped to the workspace.
    let x_lo = ox.max(0);
    let y_lo = oy.max(0);
    let x_hi = (ox + st.doc.width as i64 * z).min(w);
    let y_hi = (oy + st.doc.height as i64 * z).min(h);
    for sy in y_lo..y_hi {
        let row_line = (sy - oy) % z == 0;
        for sx in x_lo..x_hi {
            if row_line || (sx - ox) % z == 0 {
                img.pixels[(sy * w + sx) as usize] = GRID_GREY;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn rendered(zoom: u32, grid: bool) -> Image {
        let dir = std::env::temp_dir().join(format!("brushling-grid-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.zoom = zoom;
        st.show_grid = grid;
        st.render(Surface::Workspace).unwrap()
    }

    #[test]
    fn lines_on_block_edges_above_200() {
        let img = rendered(400, true);
        assert_eq!(img.get(16, 16), Some(GRID_GREY));
        assert_eq!(img.get(18, 16), Some(GRID_GREY));
        assert_eq!(img.get(16, 18), Some(GRID_GREY));
        assert_eq!(img.get(17, 17), Some(Rgb::W));
    }

    #[test]
    fn no_lines_when_off_or_at_200() {
        assert_eq!(rendered(400, false).get(16, 16), Some(Rgb::W));
        let img = rendered(200, true);
        assert!((4..68).all(|y| (4..68).all(|x| img.get(x, y) == Some(Rgb::W))));
    }
}
