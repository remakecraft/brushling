//! Pure translation from egui's input and geometry to the harness's event
//! model: keys, pointer positions (including the handle hit tests) and the
//! window size that makes the app's workspace match the egui central area.
//! Kept free of egui state so it can be unit tested without a window.

use brushling::{Button, CanvasHandle, Handle, Key, Layout, Mods, Pos, Rect};
use eframe::egui;

/// The canvas's top-left sits this many pixels into the workspace image
/// (minus scroll), matching the library's render.
pub const CANVAS_MARGIN: i64 = 4;

pub fn mods(m: egui::Modifiers) -> Mods {
    Mods { shift: m.shift, ctrl: m.ctrl, alt: m.alt }
}

pub fn button(b: egui::PointerButton) -> Option<Button> {
    match b {
        egui::PointerButton::Primary => Some(Button::Left),
        egui::PointerButton::Secondary => Some(Button::Right),
        _ => None,
    }
}

/// A key press as the harness sees it. Typed text arrives separately as
/// `Event::Text` (see [`text_keys`]), so letters only map here as Ctrl
/// shortcuts. Ctrl+Alt is AltGr on Windows and produces text instead. Alt
/// combinations other than Alt+F4 / Alt+Backspace are dropped: the app's
/// keyboard menu navigation is not mirrored by the egui menu bar.
pub fn map_key(key: egui::Key, m: egui::Modifiers) -> Option<(Key, Mods)> {
    use egui::Key as E;
    let altgr = m.ctrl && m.alt;
    let k = match key {
        E::Escape => Key::Escape,
        E::Enter => Key::Enter,
        E::Backspace => Key::Backspace,
        E::Delete => Key::Delete,
        E::Tab => Key::Tab,
        E::ArrowLeft => Key::Left,
        E::ArrowRight => Key::Right,
        E::ArrowUp => Key::Up,
        E::ArrowDown => Key::Down,
        E::Home => Key::Home,
        E::End => Key::End,
        E::PageUp => Key::PageUp,
        E::PageDown => Key::PageDown,
        E::F1 => Key::F1,
        E::F4 => Key::F4,
        // egui does not tell the numpad apart; the app only binds Ctrl+Num+/-.
        E::Plus if m.ctrl && !altgr => Key::NumPlus,
        E::Minus if m.ctrl && !altgr => Key::NumMinus,
        _ => match letter(key) {
            Some(c) if m.ctrl && !altgr => Key::Char(c),
            _ => return None,
        },
    };
    if m.alt && !m.ctrl && !matches!(k, Key::F4 | Key::Backspace) {
        return None;
    }
    Some((k, mods(m)))
}

/// Lower-case ASCII letter of a letter key.
fn letter(key: egui::Key) -> Option<char> {
    let mut chars = key.name().chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// Typed text as individual `Key::Char` presses.
pub fn text_keys(text: &str) -> Vec<Key> {
    text.chars().filter(|c| !c.is_control()).map(Key::Char).collect()
}

/// egui turns Ctrl+X/C/V into clipboard events before any key event; these
/// give the app its shortcuts back.
pub fn clipboard_shortcut(ev: &egui::Event) -> Option<(Key, Mods)> {
    let c = match ev {
        egui::Event::Cut => 'x',
        egui::Event::Copy => 'c',
        egui::Event::Paste(_) => 'v',
        _ => return None,
    };
    Some((Key::Char(c), Mods::CTRL))
}

/// Point (egui points, relative to the workspace origin) to a workspace
/// image pixel.
pub fn to_pixel(dx: f32, dy: f32, ppp: f32) -> (i32, i32) {
    ((dx * ppp).floor() as i32, (dy * ppp).floor() as i32)
}

/// What the hit tests need to know about the canvas. The harness exposes no
/// scroll offset, so handle positions assume an unscrolled canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// Screen pixels per image pixel.
    pub scale: i64,
    pub doc: (u32, u32),
    pub selection: Option<Rect>,
    pub text_frame: Option<Rect>,
}

/// A workspace pixel as an ordinary canvas position.
pub fn canvas_pos(px: (i32, i32)) -> Pos {
    Pos::Canvas(px.0 - CANVAS_MARGIN as i32, px.1 - CANVAS_MARGIN as i32)
}

/// The position a button press at workspace pixel `px` means: a text-frame
/// or selection handle, a canvas resize handle, or the canvas itself.
pub fn press_pos(px: (i32, i32), g: &Geometry) -> Pos {
    for r in [g.text_frame, g.selection].into_iter().flatten() {
        if let Some(h) = selection_handle_at(px, r, g.scale) {
            return Pos::SelectionHandle(h);
        }
    }
    if let Some(h) = canvas_handle_at(px, g) {
        return Pos::CanvasHandle(h);
    }
    canvas_pos(px)
}

fn near(px: (i32, i32), c: (i64, i64), radius: i64) -> bool {
    (px.0 as i64 - c.0).abs() <= radius && (px.1 as i64 - c.1).abs() <= radius
}

/// The 3x3 resize handles 2 px outside the right and bottom canvas edges.
fn canvas_handle_at(px: (i32, i32), g: &Geometry) -> Option<CanvasHandle> {
    let (w, h) = (g.doc.0 as i64 * g.scale, g.doc.1 as i64 * g.scale);
    let o = CANVAS_MARGIN;
    [
        (CanvasHandle::Corner, (o + w + 2, o + h + 2)),
        (CanvasHandle::Right, (o + w + 2, o + h / 2)),
        (CanvasHandle::Bottom, (o + w / 2, o + h + 2)),
    ]
    .into_iter()
    .find(|(_, c)| near(px, *c, 2))
    .map(|(handle, _)| handle)
}

/// The 5x5 handles on the dashed box drawn one pixel outside `r`.
fn selection_handle_at(px: (i32, i32), r: Rect, scale: i64) -> Option<Handle> {
    let o = CANVAS_MARGIN;
    let x0 = o + r.x0 as i64 * scale - 1;
    let y0 = o + r.y0 as i64 * scale - 1;
    let x1 = o + (r.x1 as i64 + 1) * scale;
    let y1 = o + (r.y1 as i64 + 1) * scale;
    let (mx, my) = ((x0 + x1) / 2, (y0 + y1) / 2);
    [
        (Handle::TopLeft, (x0, y0)),
        (Handle::Top, (mx, y0)),
        (Handle::TopRight, (x1, y0)),
        (Handle::Left, (x0, my)),
        (Handle::Right, (x1, my)),
        (Handle::BottomLeft, (x0, y1)),
        (Handle::Bottom, (mx, y1)),
        (Handle::BottomRight, (x1, y1)),
    ]
    .into_iter()
    .find(|(_, c)| near(px, *c, 2))
    .map(|(handle, _)| handle)
}

/// Which bars the app subtracts from its client area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bars {
    pub tool_box_docked: bool,
    pub color_box_docked: bool,
    pub status_bar: bool,
}

/// The client size to request so that the app's workspace (before scroll
/// bars) equals `central`, the egui central area in physical pixels.
pub fn window_size_for(central: (u32, u32), layout: &Layout, bars: Bars) -> (u32, u32) {
    let tool_w = if bars.tool_box_docked { layout.tool_box_docked.0 } else { 0 };
    let color_h = if bars.color_box_docked { layout.color_box_docked.1 } else { 0 };
    let status_h = if bars.status_bar { layout.status_bar_height } else { 0 };
    (central.0 + tool_w, central.1 + layout.menu_bar_height + color_h + status_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key as E;

    const CTRL: egui::Modifiers = egui::Modifiers::CTRL;

    #[test]
    fn named_keys_map_without_modifiers() {
        let none = egui::Modifiers::NONE;
        assert_eq!(map_key(E::Escape, none), Some((Key::Escape, Mods::NONE)));
        assert_eq!(map_key(E::ArrowLeft, none), Some((Key::Left, Mods::NONE)));
        assert_eq!(map_key(E::PageDown, CTRL), Some((Key::PageDown, Mods::CTRL)));
        assert_eq!(map_key(E::F4, none), Some((Key::F4, Mods::NONE)));
        assert_eq!(map_key(E::Delete, none), Some((Key::Delete, Mods::NONE)));
    }

    #[test]
    fn letters_are_ctrl_shortcuts_only() {
        assert_eq!(map_key(E::Z, CTRL), Some((Key::Char('z'), Mods::CTRL)));
        let ctrl_shift = egui::Modifiers { ctrl: true, shift: true, ..Default::default() };
        assert_eq!(map_key(E::N, ctrl_shift), Some((Key::Char('n'), Mods::CTRL_SHIFT)));
        // Plain letters come through Event::Text; AltGr (Ctrl+Alt) is text too.
        assert_eq!(map_key(E::A, egui::Modifiers::NONE), None);
        let altgr = egui::Modifiers { ctrl: true, alt: true, ..Default::default() };
        assert_eq!(map_key(E::Q, altgr), None);
    }

    #[test]
    fn alt_only_keeps_exit_and_undo() {
        let alt = egui::Modifiers::ALT;
        assert_eq!(map_key(E::F4, alt), Some((Key::F4, Mods::ALT)));
        assert_eq!(map_key(E::Backspace, alt), Some((Key::Backspace, Mods::ALT)));
        assert_eq!(map_key(E::F, alt), None);
        assert_eq!(map_key(E::Enter, alt), None);
    }

    #[test]
    fn ctrl_plus_minus_are_numpad_keys() {
        assert_eq!(map_key(E::Plus, CTRL), Some((Key::NumPlus, Mods::CTRL)));
        assert_eq!(map_key(E::Minus, CTRL), Some((Key::NumMinus, Mods::CTRL)));
        assert_eq!(map_key(E::Minus, egui::Modifiers::NONE), None);
    }

    #[test]
    fn text_and_clipboard_events() {
        assert_eq!(text_keys("Hi!\u{7}"), vec![Key::Char('H'), Key::Char('i'), Key::Char('!')]);
        assert_eq!(clipboard_shortcut(&egui::Event::Copy), Some((Key::Char('c'), Mods::CTRL)));
        assert_eq!(clipboard_shortcut(&egui::Event::Paste("x".into())), Some((Key::Char('v'), Mods::CTRL)));
        assert_eq!(clipboard_shortcut(&egui::Event::PointerGone), None);
    }

    #[test]
    fn points_to_pixels_floor() {
        assert_eq!(to_pixel(10.0, 3.9, 1.0), (10, 3));
        assert_eq!(to_pixel(10.0, 3.9, 1.5), (15, 5));
        assert_eq!(to_pixel(-0.2, 0.0, 2.0), (-1, 0));
    }

    fn geom(scale: i64) -> Geometry {
        Geometry { scale, doc: (100, 50), selection: None, text_frame: None }
    }

    #[test]
    fn plain_press_is_canvas_offset() {
        assert_eq!(press_pos((4, 4), &geom(1)), Pos::Canvas(0, 0));
        assert_eq!(press_pos((50, 20), &geom(1)), Pos::Canvas(46, 16));
        assert_eq!(press_pos((0, 0), &geom(1)), Pos::Canvas(-4, -4));
    }

    #[test]
    fn canvas_handles_scale_with_zoom() {
        // 100x50 at zoom 1: corner centre at (4+100+2, 4+50+2).
        assert_eq!(press_pos((106, 56), &geom(1)), Pos::CanvasHandle(CanvasHandle::Corner));
        assert_eq!(press_pos((107, 29), &geom(1)), Pos::CanvasHandle(CanvasHandle::Right));
        assert_eq!(press_pos((54, 55), &geom(1)), Pos::CanvasHandle(CanvasHandle::Bottom));
        // At 4x the corner moves to (4+400+2, 4+200+2).
        assert_eq!(press_pos((406, 206), &geom(4)), Pos::CanvasHandle(CanvasHandle::Corner));
        assert_eq!(press_pos((106, 56), &geom(4)), Pos::Canvas(102, 52));
    }

    #[test]
    fn selection_handles_win_over_canvas() {
        let mut g = geom(1);
        g.selection = Some(Rect::new(10, 10, 19, 19));
        // Box from (4+10-1, 4+10-1) = (13,13) to (4+20, 4+20) = (24,24).
        assert_eq!(press_pos((13, 13), &g), Pos::SelectionHandle(Handle::TopLeft));
        assert_eq!(press_pos((24, 24), &g), Pos::SelectionHandle(Handle::BottomRight));
        assert_eq!(press_pos((18, 14), &g), Pos::SelectionHandle(Handle::Top));
        assert_eq!(press_pos((18, 18), &g), Pos::Canvas(14, 14));
        g.text_frame = Some(Rect::new(40, 10, 49, 19));
        assert_eq!(press_pos((43, 18), &g), Pos::SelectionHandle(Handle::Left));
    }

    #[test]
    fn button_mapping() {
        assert_eq!(button(egui::PointerButton::Primary), Some(Button::Left));
        assert_eq!(button(egui::PointerButton::Secondary), Some(Button::Right));
        assert_eq!(button(egui::PointerButton::Middle), None);
    }

    fn layout() -> Layout {
        Layout {
            menu_bar_height: 20,
            status_bar_height: 22,
            tool_box_docked: (56, 272),
            color_box_docked: (284, 54),
            ..Default::default()
        }
    }

    #[test]
    fn window_size_adds_only_the_shown_bars() {
        let all = Bars { tool_box_docked: true, color_box_docked: true, status_bar: true };
        assert_eq!(window_size_for((800, 600), &layout(), all), (856, 696));
        let none = Bars { tool_box_docked: false, color_box_docked: false, status_bar: false };
        assert_eq!(window_size_for((800, 600), &layout(), none), (800, 620));
    }

    #[test]
    fn requested_size_gives_the_app_that_workspace() {
        let dir = std::env::temp_dir().join(format!("brushling-shell-test-{}", std::process::id()));
        let mut app = brushling::App::start(brushling::StartOptions::new(&dir)).unwrap();
        app.new_document(100, 100, brushling::Rgb::W).unwrap();
        let bars = Bars { tool_box_docked: true, color_box_docked: true, status_bar: true };
        let want = window_size_for((900, 500), &app.layout().unwrap(), bars);
        app.set_window_size(want.0, want.1).unwrap();
        let l = app.layout().unwrap();
        assert_eq!((l.workspace.2, l.workspace.3), (900, 500));
        let img = app.render(brushling::Surface::Workspace).unwrap();
        assert_eq!((img.width, img.height), (900, 500));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
