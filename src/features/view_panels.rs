//! Feature view.panels: the Tool Box, Color Box and Status Bar toggles, the
//! floating/docked placement of the tool box, colour box and text toolbar, and
//! the headless model behind `App::panel`, `App::place_panel` and
//! `App::close_panel`.
//!
//! Visibility and placement are independent: hiding a floating panel keeps
//! its floating position, so showing it again restores it. The thumbnail
//! window (view.thumbnail) is also a panel and reports through the same API;
//! the Help window belongs to help.help_about. The text toolbar's visibility
//! stays in `State::text_toolbar` (tool.text owns it).

use crate::features::ui_main_window as mw;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

/// Size of the text toolbar window, for the off-screen test of its position.
pub const TEXT_TOOLBAR_SIZE: (u32, u32) = (300, 32);

/// Visibility and floating position of one movable panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub visible: bool,
    /// `None` while docked.
    pub floating: Option<(i32, i32)>,
}

impl Place {
    const DOCKED: Place = Place { visible: true, floating: None };
}

/// Per-App panel state (View menu toggles and floating positions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panels {
    pub tool_box: Place,
    pub color_box: Place,
    pub status_bar: bool,
    /// Floating position of the text toolbar (`None` = docked).
    pub text_toolbar: Option<(i32, i32)>,
    /// The thumbnail window is open.
    pub thumbnail: bool,
    /// Screen position of the thumbnail window (remembered while closed).
    pub thumbnail_at: (i32, i32),
}

impl Panels {
    pub fn new(screen: (u32, u32)) -> Panels {
        Panels {
            tool_box: Place::DOCKED,
            color_box: Place::DOCKED,
            status_bar: true,
            text_toolbar: None,
            thumbnail: false,
            thumbnail_at: default_thumbnail_at(screen),
        }
    }
}

/// Where the thumbnail window first appears: near the top-right corner.
pub fn default_thumbnail_at(screen: (u32, u32)) -> (i32, i32) {
    (screen.0.saturating_sub(220).min(i32::MAX as u32) as i32, 80)
}

pub fn register(r: &mut Registry) {
    r.command("View > Tool Box", |st| toggle(st, Panel::ToolBox));
    r.command("View > Color Box", |st| toggle(st, Panel::ColorBox));
    r.command("View > Status Bar", |st| toggle(st, Panel::StatusBar));
    r.shortcut(Key::Char('t'), Mods::CTRL, "View > Tool Box");
    r.shortcut(Key::Char('l'), Mods::CTRL, "View > Color Box");
}

/// Whether the panel is currently shown (docked or floating).
pub fn visible(st: &State, panel: Panel) -> bool {
    match panel {
        Panel::ToolBox => st.panels.tool_box.visible,
        Panel::ColorBox => st.panels.color_box.visible,
        Panel::StatusBar => st.panels.status_bar,
        Panel::TextToolbar => st.text_toolbar,
        Panel::Thumbnail => st.panels.thumbnail,
        Panel::Help => false,
    }
}

/// Whether the panel is shown in its docked place (takes layout space).
pub fn docked_visible(st: &State, panel: Panel) -> bool {
    visible(st, panel) && floating_at(st, panel).is_none()
}

fn floating_at(st: &State, panel: Panel) -> Option<(i32, i32)> {
    match panel {
        Panel::ToolBox => st.panels.tool_box.floating,
        Panel::ColorBox => st.panels.color_box.floating,
        Panel::TextToolbar => st.panels.text_toolbar,
        Panel::Thumbnail => st.panels.thumbnail.then_some(st.panels.thumbnail_at),
        Panel::StatusBar | Panel::Help => None,
    }
}

/// `App::panel`.
pub fn panel_state(st: &State, panel: Panel) -> HResult<PanelState> {
    if panel == Panel::Help {
        let visible = crate::features::help_help_about::help_visible(st);
        return Ok(PanelState { visible, floating_at: None });
    }
    Ok(PanelState { visible: visible(st, panel), floating_at: floating_at(st, panel) })
}

/// Check state of the View items this feature owns; `None` for other paths.
pub fn check_state(st: &State, path: &str) -> Option<bool> {
    match path {
        "View > Tool Box" => Some(visible(st, Panel::ToolBox)),
        "View > Color Box" => Some(visible(st, Panel::ColorBox)),
        "View > Status Bar" => Some(visible(st, Panel::StatusBar)),
        "View > Text Toolbar" => Some(visible(st, Panel::TextToolbar)),
        "View > Zoom > Show Thumbnail" => Some(visible(st, Panel::Thumbnail)),
        _ => None,
    }
}

/// Show or hide a panel; the docked layout and the workspace follow.
pub fn set_visible(st: &mut State, panel: Panel, show: bool) {
    match panel {
        Panel::ToolBox => st.panels.tool_box.visible = show,
        Panel::ColorBox => st.panels.color_box.visible = show,
        Panel::StatusBar => st.panels.status_bar = show,
        Panel::TextToolbar => crate::features::tool_text::set_toolbar_visible(st, show),
        Panel::Thumbnail => st.panels.thumbnail = show,
        Panel::Help => return,
    }
    // Only the docked bars take workspace space; the thumbnail floats.
    if panel != Panel::Thumbnail {
        mw::refresh_workspace(st);
    }
}

fn toggle(st: &mut State, panel: Panel) -> HResult<()> {
    let show = !visible(st, panel);
    set_visible(st, panel, show);
    Ok(())
}

/// View > Text Toolbar body, for the owner of that command to call.
pub fn toggle_text_toolbar(st: &mut State) {
    let show = !st.text_toolbar;
    set_visible(st, Panel::TextToolbar, show);
}

/// `App::place_panel`: drag a panel to float at `at`, or back to its dock.
pub fn place(st: &mut State, panel: Panel, at: Option<(i32, i32)>) -> HResult<()> {
    match panel {
        Panel::ToolBox => st.panels.tool_box.floating = at,
        Panel::ColorBox => st.panels.color_box.floating = at,
        Panel::TextToolbar => st.panels.text_toolbar = at,
        Panel::Thumbnail => st.panels.thumbnail_at = at.unwrap_or_else(|| default_thumbnail_at(st.screen)),
        Panel::StatusBar => return Err(HarnessError::Invalid("the status bar cannot be moved".to_owned())),
        Panel::Help => return Err(HarnessError::Invalid("the Help window is not a dockable panel".to_owned())),
    }
    mw::refresh_workspace(st);
    Ok(())
}

/// `App::close_panel`: the close button of a floating panel or window.
pub fn close(st: &mut State, panel: Panel) -> HResult<()> {
    match panel {
        Panel::ToolBox | Panel::ColorBox | Panel::TextToolbar | Panel::Thumbnail => {
            set_visible(st, panel, false);
            Ok(())
        }
        Panel::StatusBar => Err(HarnessError::Invalid("the status bar has no close button".to_owned())),
        Panel::Help => {
            crate::features::help_help_about::close_help(st);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thumbnail_position_stays_on_screen() {
        assert_eq!(default_thumbnail_at((1920, 1080)), (1700, 80));
        assert_eq!(default_thumbnail_at((100, 100)).0, 0);
    }
}
