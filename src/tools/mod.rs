//! Pointer/key routing for the canvas tools. The routing here is complete;
//! each tool is a [`ToolHandler`] registered in `features/<tool>.rs`. A tool
//! with no registered handler simply ignores pointer input.

use crate::registry::registry;
use crate::state::{Held, LastPress, State};
use crate::types::*;

/// Maximum gap between two presses that form a double-click.
pub const DOUBLE_CLICK_MS: u64 = 500;
/// Maximum distance (screen pixels, per axis) between those presses.
pub const DOUBLE_CLICK_PX: i32 = 4;

/// One pointer event as a tool sees it.
#[derive(Debug, Clone, Copy)]
pub struct PointerEvent {
    pub pos: Pos,
    /// Image coordinates, when the pointer addresses the canvas (may lie
    /// outside the image).
    pub image: Option<(i32, i32)>,
    /// The button that was pressed/released, or the one held during a drag;
    /// `None` for a plain hover.
    pub button: Option<Button>,
    pub mods: Mods,
    /// Set on the second press of a double-click.
    pub double_click: bool,
}

/// Behaviour of one tool. Every method has a do-nothing default, so a tool
/// implements only what it needs. Per-gesture data lives in fields the tool
/// adds to [`State`].
pub trait ToolHandler: Send + Sync {
    /// The tool became the active tool.
    fn activate(&self, _st: &mut State) -> HResult<()> {
        Ok(())
    }
    /// The tool is about to stop being the active tool (commit pending work).
    fn deactivate(&self, _st: &mut State) -> HResult<()> {
        Ok(())
    }
    fn press(&self, _st: &mut State, _ev: &PointerEvent) -> HResult<()> {
        Ok(())
    }
    /// Pointer moved while a button is held.
    fn drag(&self, _st: &mut State, _ev: &PointerEvent) -> HResult<()> {
        Ok(())
    }
    /// Pointer moved with no button held.
    fn hover(&self, _st: &mut State, _ev: &PointerEvent) -> HResult<()> {
        Ok(())
    }
    fn release(&self, _st: &mut State, _ev: &PointerEvent) -> HResult<()> {
        Ok(())
    }
    /// A key press that reached the tool. Return `true` if consumed (then no
    /// shortcut sees it).
    fn key(&self, _st: &mut State, _key: Key, _mods: Mods) -> HResult<bool> {
        Ok(false)
    }
    /// The virtual clock advanced by `ms` (airbrush spray).
    fn tick(&self, _st: &mut State, _ms: u64) -> HResult<()> {
        Ok(())
    }
}

impl State {
    /// Click a tool box button.
    pub fn select_tool(&mut self, tool: Tool) -> HResult<()> {
        if tool == self.tool {
            return Ok(());
        }
        if let Some(h) = registry().tool_handler(self.tool) {
            h.deactivate(self)?;
        }
        self.prev_tool = self.tool;
        self.tool = tool;
        self.held = None;
        self.stroke_base = None;
        if let Some(h) = registry().tool_handler(tool) {
            h.activate(self)?;
        }
        Ok(())
    }

    /// Click an option in the options box. Values outside the spec's sets
    /// are rejected.
    pub fn set_tool_option(&mut self, option: ToolOption) -> HResult<()> {
        let bad = |what: &str, v: u32| Err(HarnessError::Invalid(format!("{what} {v} is not an available option")));
        match option {
            ToolOption::Opaque(o) => self.options.opaque = o,
            ToolOption::EraserSize(v) if matches!(v, 4 | 6 | 8 | 10) => self.options.eraser_size = v,
            ToolOption::EraserSize(v) => return bad("eraser size", v),
            ToolOption::MagnifierLevel(v) if matches!(v, 1 | 2 | 6 | 8) => crate::features::view_zoom::set_zoom(self, v * 100)?,
            ToolOption::MagnifierLevel(v) => return bad("magnifier level", v),
            ToolOption::BrushShape(v) if (1..=12).contains(&v) => self.options.brush_shape = v,
            ToolOption::BrushShape(v) => return bad("brush shape", v),
            ToolOption::AirbrushRadius(v) if matches!(v, 4 | 8 | 12) => self.options.airbrush_radius = v,
            ToolOption::AirbrushRadius(v) => return bad("airbrush radius", v),
            ToolOption::LineWidth(v) if (1..=5).contains(&v) => self.options.line_width = v,
            ToolOption::LineWidth(v) => return bad("line width", v),
            ToolOption::FillStyle(v) if (1..=3).contains(&v) => self.options.fill_style = v,
            ToolOption::FillStyle(v) => return bad("fill style", v),
        }
        Ok(())
    }

    /// Route a pointer event. Ignored while a modal dialog is open.
    pub fn pointer(&mut self, kind: PointerKind, pos: Pos, mods: Mods) -> HResult<()> {
        if !self.dialogs.is_empty() {
            return Ok(());
        }
        // Print Preview replaces the workspace: the canvas gets no pointer input.
        if crate::features::file_print::preview_active(self) {
            return Ok(());
        }
        // View Bitmap: a button press ends the full-screen view and nothing else.
        if crate::features::view_view_bitmap::swallow(self, matches!(kind, PointerKind::Press(_))) {
            return Ok(());
        }
        let image = self.image_point(pos);
        if let Some(p) = image {
            self.pointer_image = Some(p);
        }
        // The status bar's pointer tracker is updated before any hook runs, so
        // hooks that consume the event (selection and handle drags) cannot
        // freeze the position pane. It is cleared off the canvas.
        self.ptr_image = image;
        self.ptr_mods = mods;
        let mut ev = PointerEvent { pos, image, button: None, mods, double_click: false };
        match kind {
            PointerKind::Press(b) => {
                if let Some(h) = self.held {
                    // A second button while one is held never reaches the
                    // tool; if it interrupts a stroke it cancels the stroke.
                    if h.button != b && !h.cancelled {
                        crate::features::mouse_colors::cancel_stroke(self);
                    }
                    return Ok(());
                }
                ev.button = Some(b);
                ev.double_click = self.register_press(pos, b);
                self.held = Some(Held { button: b, start: pos, start_image: image, mods, cancelled: false });
            }
            PointerKind::Move => ev.button = self.held.map(|h| h.button),
            PointerKind::Release(b) => {
                if self.held.map(|h| h.button) != Some(b) {
                    return Ok(());
                }
                ev.button = Some(b);
            }
        }

        if self.held.is_some_and(|h| h.cancelled) {
            // Cancelled stroke: only the release (which ends the gesture) matters.
            if matches!(kind, PointerKind::Release(_)) {
                self.held = None;
            }
            return Ok(());
        }
        self.ptr_kind = Some(kind);
        let hooks = registry().pointer_hooks();
        for hook in hooks {
            if hook(self, &ev)? {
                if matches!(kind, PointerKind::Release(_)) {
                    self.held = None;
                }
                return Ok(());
            }
        }
        if matches!(kind, PointerKind::Press(_)) {
            crate::features::mouse_colors::begin_stroke(self);
        }
        let handler = registry().tool_handler(self.tool);
        if let Some(h) = handler {
            match kind {
                PointerKind::Press(_) => h.press(self, &ev)?,
                PointerKind::Move if ev.button.is_some() => h.drag(self, &ev)?,
                PointerKind::Move => h.hover(self, &ev)?,
                PointerKind::Release(_) => {
                    let r = h.release(self, &ev);
                    self.held = None;
                    self.stroke_base = None;
                    return r;
                }
            }
        }
        if matches!(kind, PointerKind::Release(_)) {
            self.held = None;
            self.stroke_base = None;
        }
        Ok(())
    }

    /// Track presses; true when this press completes a double-click.
    fn register_press(&mut self, pos: Pos, button: Button) -> bool {
        let screen = self.screen_point(pos);
        let double = self.last_press.is_some_and(|p| {
            p.button == button
                && self.now_ms.saturating_sub(p.at_ms) <= DOUBLE_CLICK_MS
                && (p.screen.0 as i64 - screen.0 as i64).abs() <= DOUBLE_CLICK_PX as i64
                && (p.screen.1 as i64 - screen.1 as i64).abs() <= DOUBLE_CLICK_PX as i64
        });
        // A double-click consumes both presses; the next press starts afresh.
        self.last_press = if double { None } else { Some(LastPress { at_ms: self.now_ms, screen, button }) };
        double
    }

    /// Route a key press: dialog first, then the active tool, then the
    /// registered shortcuts.
    pub fn key(&mut self, key: Key, mods: Mods) -> HResult<()> {
        if !self.dialogs.is_empty() {
            return match key {
                Key::Escape => self.dialog_escape(),
                Key::Enter => self.dialog_enter(),
                _ => Ok(()),
            };
        }
        if crate::features::file_print::preview_active(self) {
            crate::features::file_print::preview_key(self, key);
            return Ok(());
        }
        // View Bitmap: any key press ends the full-screen view and nothing else.
        if crate::features::view_view_bitmap::swallow(self, true) {
            return Ok(());
        }
        if self.exit_code.is_none() && self.menu_key(key, mods)? {
            return Ok(());
        }
        if key == Key::Escape {
            // Cancels the gesture in progress, then the tool's own Esc.
            crate::features::input_escape::escape(self, mods)?;
            return Ok(());
        }
        if let Some(h) = registry().tool_handler(self.tool) {
            if h.key(self, key, mods)? {
                return Ok(());
            }
        }
        // A shortcut whose command no feature has registered yet does nothing.
        if let Some(path) = registry().shortcut_for(key, mods).filter(|p| registry().command_for(p).is_some()) {
            return self.menu(path);
        }
        Ok(())
    }

    /// Advance the virtual clock and let the active tool react.
    pub fn advance(&mut self, ms: u64) -> HResult<()> {
        self.now_ms = self.now_ms.saturating_add(ms);
        if !self.dialogs.is_empty() {
            return Ok(());
        }
        match registry().tool_handler(self.tool) {
            Some(h) => h.tick(self, ms),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-tools-{}", std::process::id()));
        State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap()
    }

    #[test]
    fn double_click_needs_time_and_distance() {
        let mut st = state();
        assert!(!st.register_press(Pos::Image(5, 5), Button::Left));
        st.now_ms = 400;
        assert!(st.register_press(Pos::Image(5, 5), Button::Left));
        // consumed: the next press starts afresh
        st.now_ms = 450;
        assert!(!st.register_press(Pos::Image(5, 5), Button::Left));
        st.now_ms = 1000;
        assert!(!st.register_press(Pos::Image(5, 5), Button::Left));
        st.now_ms = 1400;
        assert!(!st.register_press(Pos::Image(50, 5), Button::Left));
    }

    #[test]
    fn pointer_position_is_tracked_before_hooks_and_cleared_off_canvas() {
        let mut st = state();
        st.pointer(PointerKind::Move, Pos::Image(3, 4), Mods::NONE).unwrap();
        assert_eq!(st.ptr_image, Some((3, 4)));
        st.pointer(PointerKind::Move, Pos::Outside, Mods::NONE).unwrap();
        assert_eq!(st.ptr_image, None);
    }

    #[test]
    fn options_validate() {
        let mut st = state();
        assert!(st.set_tool_option(ToolOption::EraserSize(7)).is_err());
        st.set_tool_option(ToolOption::EraserSize(10)).unwrap();
        st.set_tool_option(ToolOption::MagnifierLevel(6)).unwrap();
        assert_eq!(st.zoom, 600);
        assert_eq!(st.tool_options().magnifier_highlight, Some(6));
        assert_eq!(st.tool_options().eraser_size, 10);
    }

    #[test]
    fn canvas_position_floors_by_zoom() {
        let mut st = state();
        st.zoom = 400;
        assert_eq!(st.image_point(Pos::Canvas(7, 8)), Some((1, 2)));
        assert_eq!(st.image_point(Pos::Outside), None);
    }
}
