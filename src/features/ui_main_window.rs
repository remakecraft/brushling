//! Feature ui.main_window: the headless model of the main window. The title is
//! `<file name> - Brushling`; the layout stacks menu bar, a middle band (tool
//! box docked left, workspace filling the rest), the colour box and the status
//! bar. The window cannot be made smaller than its docked bars, whether or
//! not the tool box / colour box are currently shown.

use crate::registry::Registry;
use crate::render::CANVAS_MARGIN;
use crate::features::view_panels::docked_visible;
use crate::state::State;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// Menu bar height in screen pixels.
pub const MENU_BAR_HEIGHT: u32 = 20;
/// Status bar height in screen pixels.
pub const STATUS_BAR_HEIGHT: u32 = 22;
/// Docked tool box: 2 columns of 25 px buttons, 8 rows, plus the options box.
pub const TOOL_BOX_DOCKED: (u32, u32) = (56, 8 * 25 + 72);
/// Docked colour box: two rows of 14 swatches beside the colour selector.
pub const COLOR_BOX_DOCKED: (u32, u32) = (284, 54);
/// Thickness of a scroll bar.
pub const SCROLL_BAR: u32 = 16;
/// Workspace size a freshly started window has.
const DEFAULT_WORKSPACE: (u32, u32) = (800, 600);

/// Whether a panel is currently shown (view.panels owns the state).
pub fn panel_visible(st: &State, panel: Panel) -> bool {
    crate::features::view_panels::visible(st, panel)
}

/// The smallest client size: width of the docked colour box, height of the
/// menu bar, docked tool box, docked colour box and status bar together.
pub fn min_window() -> (u32, u32) {
    (COLOR_BOX_DOCKED.0, MENU_BAR_HEIGHT + TOOL_BOX_DOCKED.1 + COLOR_BOX_DOCKED.1 + STATUS_BAR_HEIGHT)
}

/// Client size at startup: the workspace comes out 800 x 600.
pub fn default_window() -> (u32, u32) {
    (
        TOOL_BOX_DOCKED.0 + DEFAULT_WORKSPACE.0,
        MENU_BAR_HEIGHT + DEFAULT_WORKSPACE.1 + COLOR_BOX_DOCKED.1 + STATUS_BAR_HEIGHT,
    )
}

fn clamped(window: (u32, u32)) -> (u32, u32) {
    let min = min_window();
    (window.0.max(min.0), window.1.max(min.1))
}

/// Window title.
pub fn title(st: &State) -> String {
    let name = st
        .doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(|| "untitled".to_owned(), |n| n.to_string_lossy().into_owned());
    format!("{name} - Brushling")
}

/// Request a client size; it is raised to the minimum. The render workspace
/// follows the new geometry.
pub fn set_window_size(st: &mut State, w: u32, h: u32) {
    st.window = clamped((w, h));
    refresh_workspace(st);
}

/// Recompute the workspace size from the current layout (window size and the
/// docked bars) and keep the scroll offset inside the canvas.
pub fn refresh_workspace(st: &mut State) {
    let (_, _, ww, wh) = layout(st).workspace;
    st.workspace_size = (ww, wh);
    crate::features::view_zoom::clamp_scroll(st);
}

/// Whether the zoomed canvas needs (horizontal, vertical) scroll bars in an
/// area of `avail`. A bar takes space from the other axis, so the check repeats.
fn scroll_bars(content: (i64, i64), avail: (i64, i64)) -> (bool, bool) {
    let (mut h, mut v) = (false, false);
    loop {
        let aw = avail.0 - if v { SCROLL_BAR as i64 } else { 0 };
        let ah = avail.1 - if h { SCROLL_BAR as i64 } else { 0 };
        let (nh, nv) = (content.0 > aw, content.1 > ah);
        if (nh, nv) == (h, v) {
            return (h, v);
        }
        h |= nh;
        v |= nv;
    }
}

pub fn layout(st: &State) -> Layout {
    let (cw, ch) = clamped(st.window);
    let tool_w = if docked_visible(st, Panel::ToolBox) { TOOL_BOX_DOCKED.0 } else { 0 };
    let color_h = if docked_visible(st, Panel::ColorBox) { COLOR_BOX_DOCKED.1 } else { 0 };
    let status_h = if panel_visible(st, Panel::StatusBar) { STATUS_BAR_HEIGHT } else { 0 };
    let band_w = cw - tool_w;
    let band_h = ch.saturating_sub(MENU_BAR_HEIGHT + color_h + status_h);
    let z = st.scale() as i64;
    let content = (
        st.doc.width as i64 * z + 2 * CANVAS_MARGIN as i64,
        st.doc.height as i64 * z + 2 * CANVAS_MARGIN as i64,
    );
    let (hscroll, vscroll) = scroll_bars(content, (band_w as i64, band_h as i64));
    let ww = band_w.saturating_sub(if vscroll { SCROLL_BAR } else { 0 });
    let wh = band_h.saturating_sub(if hscroll { SCROLL_BAR } else { 0 });
    Layout {
        client_width: cw,
        client_height: ch,
        menu_bar_height: MENU_BAR_HEIGHT,
        status_bar_height: STATUS_BAR_HEIGHT,
        tool_box_docked: TOOL_BOX_DOCKED,
        color_box_docked: COLOR_BOX_DOCKED,
        workspace: (tool_w as i32, MENU_BAR_HEIGHT as i32, ww, wh),
        hscroll,
        vscroll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_bar_takes_space_from_other_axis() {
        // Fits only until the vertical bar narrows the area.
        assert_eq!(scroll_bars((780, 700), (800, 600)), (false, true));
        assert_eq!(scroll_bars((780, 700), (770, 600)), (true, true));
        assert_eq!(scroll_bars((10, 10), (800, 600)), (false, false));
    }

    #[test]
    fn default_workspace_is_800_by_600() {
        let (w, h) = default_window();
        assert_eq!((w - TOOL_BOX_DOCKED.0, h - MENU_BAR_HEIGHT - COLOR_BOX_DOCKED.1 - STATUS_BAR_HEIGHT), (800, 600));
    }
}
