//! Feature tools.toolbox: the 16 tool buttons (2 x 8, row-major), which one is
//! pressed, whether it can be clicked, the hover tooltip and the one-line
//! description the status bar shows while a button is highlighted.
//!
//! Option values live in `State::options` and are kept per option set by
//! `State::set_tool_option`, so they survive switching tools.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// Hover time before a tooltip appears.
pub const TOOLTIP_DELAY_MS: u64 = 500;

/// Tooltip name of a tool.
pub fn name(tool: Tool) -> &'static str {
    match tool {
        Tool::FreeFormSelect => "Free-Form Select",
        Tool::Select => "Select",
        Tool::Eraser => "Eraser/Color Eraser",
        Tool::Fill => "Fill With Color",
        Tool::PickColor => "Pick Color",
        Tool::Magnifier => "Magnifier",
        Tool::Pencil => "Pencil",
        Tool::Brush => "Brush",
        Tool::Airbrush => "Airbrush",
        Tool::Text => "Text",
        Tool::Line => "Line",
        Tool::Curve => "Curve",
        Tool::Rectangle => "Rectangle",
        Tool::Polygon => "Polygon",
        Tool::Ellipse => "Ellipse",
        Tool::RoundedRectangle => "Rounded Rectangle",
    }
}

/// Status bar description of a tool button.
pub fn hint(tool: Tool) -> &'static str {
    match tool {
        Tool::FreeFormSelect => "Selects an irregularly shaped part of the picture to move, copy or edit.",
        Tool::Select => "Selects a rectangular part of the picture to move, copy or edit.",
        Tool::Eraser => "Erases a square of the picture; with the right button it replaces only the foreground color.",
        Tool::Fill => "Floods a closed area with the current color.",
        Tool::PickColor => "Takes a color from the picture and makes it the drawing color.",
        Tool::Magnifier => "Changes the magnification of the picture.",
        Tool::Pencil => "Draws a free-hand line one pixel wide.",
        Tool::Brush => "Paints with a brush of the chosen shape and size.",
        Tool::Airbrush => "Sprays a fine mist of color.",
        Tool::Text => "Types text onto the picture.",
        Tool::Line => "Draws a straight line in the chosen width.",
        Tool::Curve => "Draws a curved line from a stroke and up to two bends.",
        Tool::Rectangle => "Draws a rectangle, outlined or filled.",
        Tool::Polygon => "Draws a closed shape made of straight sides.",
        Tool::Ellipse => "Draws an ellipse, outlined or filled.",
        Tool::RoundedRectangle => "Draws a rectangle with rounded corners, outlined or filled.",
    }
}

/// A button can be clicked. Text needs a zoom of 100% or less.
pub fn tool_enabled(st: &State, tool: Tool) -> bool {
    tool != Tool::Text || st.zoom <= 100
}

pub fn buttons(st: &State) -> Vec<ToolButton> {
    Tool::ALL
        .iter()
        .map(|&tool| ToolButton { tool, name: name(tool).to_owned(), enabled: tool_enabled(st, tool), active: st.tool == tool })
        .collect()
}

/// A click on a button: a disabled button does nothing.
pub fn click(st: &mut State, tool: Tool) -> HResult<()> {
    if tool_enabled(st, tool) {
        st.select_tool(tool)
    } else {
        Ok(())
    }
}

/// Rest the pointer on `target`. The tooltip timer restarts when the target
/// changes. A menu item must exist.
pub fn hover(st: &mut State, target: Hover) -> HResult<()> {
    if let Hover::MenuItem(path) = &target {
        st.menu_entry(path)?;
    }
    if target != st.hover {
        st.hover_since = st.now_ms;
        st.hover = target;
    }
    Ok(())
}

/// The tooltip shown now: only for a tool button, after the delay.
pub fn tooltip(st: &State) -> Option<String> {
    match st.hover {
        Hover::ToolButton(t) if st.now_ms.saturating_sub(st.hover_since) >= TOOLTIP_DELAY_MS => Some(name(t).to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn hints_are_unique() {
        let set: HashSet<_> = Tool::ALL.iter().map(|&t| hint(t)).collect();
        assert_eq!(set.len(), Tool::ALL.len());
    }
}
