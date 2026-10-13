//! Feature tool.pick_color: a click on the canvas takes the pixel's colour as
//! the foreground (left button) or background (right button) colour, then
//! the previously active tool comes back (Pencil if there was none).

use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct PickColor;
static PICK_COLOR: PickColor = PickColor;

pub fn register(r: &mut Registry) {
    r.tool(Tool::PickColor, &PICK_COLOR);
}

impl ToolHandler for PickColor {
    /// The pick happens on release; no pixel or undo state is touched. A
    /// release outside the canvas does nothing and keeps the tool active.
    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some((x, y)) = ev.image else { return Ok(()) };
        let Some(c) = st.doc.get(x, y) else { return Ok(()) };
        match ev.button {
            Some(Button::Right) => st.bg = c,
            _ => st.fg = c,
        }
        let back = if st.prev_tool == Tool::PickColor { Tool::Pencil } else { st.prev_tool };
        st.select_tool(back)
    }
}
