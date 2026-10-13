//! Feature tool.line_width: the width option shared by Line, Curve, the shape
//! tools and Polygon (stored in `st.options.line_width`, set through
//! `ToolOption::LineWidth`), plus Ctrl+NumPad+ / Ctrl+NumPad- to step it.
//! The keys are bound as shortcuts to hidden commands that are enabled only
//! while one of those tools is active, so other tools ignore them. A running
//! drag redraws because every previewing tool reads the width live.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const THICKER: &str = "Options > Line Width > Thicker";
const THINNER: &str = "Options > Line Width > Thinner";
const MIN: u32 = 1;
const MAX: u32 = 5;

pub fn register(r: &mut Registry) {
    r.command_if(THICKER, thicker, Some(uses_width));
    r.command_if(THINNER, thinner, Some(uses_width));
    r.shortcut(Key::NumPlus, Mods::CTRL, THICKER);
    r.shortcut(Key::NumMinus, Mods::CTRL, THINNER);
}

/// Whether a tool draws with the line width.
pub(crate) fn tool_uses_width(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Line | Tool::Curve | Tool::Rectangle | Tool::Polygon | Tool::Ellipse | Tool::RoundedRectangle
    )
}

fn uses_width(st: &State) -> bool {
    tool_uses_width(st.tool)
}

fn thicker(st: &mut State) -> HResult<()> {
    st.options.line_width = (st.options.line_width + 1).clamp(MIN, MAX);
    Ok(())
}

fn thinner(st: &mut State) -> HResult<()> {
    st.options.line_width = st.options.line_width.saturating_sub(1).clamp(MIN, MAX);
    Ok(())
}
