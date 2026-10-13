//! Feature ui.layout_persistence: window size, the View toggles, docked or
//! floating placement of the tool box, colour box and text toolbar and the
//! thumbnail position survive a restart, through `settings.ini`.
//!
//! [`store`] runs when the application exits (just before the settings file is
//! written); [`restore`] runs once while the state is built. Unreadable values
//! fall back to the defaults. A floating panel that would lie fully outside
//! the primary screen is moved to its top-left corner.
//!
//! The settings file has no window position because the headless window has
//! none; the size is stored as `window=<w>x<h>`.

use crate::features::ui_main_window as mw;
use crate::features::view_panels::{self, Place, TEXT_TOOLBAR_SIZE};
use crate::registry::Registry;
use crate::settings::Settings;
use crate::state::State;

pub fn register(_r: &mut Registry) {}

const KEY_WINDOW: &str = "window";
const THUMBNAIL_SIZE: (u32, u32) = crate::features::view_thumbnail::SIZE;

fn flag(settings: &Settings, key: &str) -> Option<bool> {
    match settings.get(key)? {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

fn pair(text: &str, sep: char) -> Option<(i32, i32)> {
    let (a, b) = text.split_once(sep)?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// A stored placement: `docked` or `x,y`.
fn place(settings: &Settings, key: &str) -> Option<Option<(i32, i32)>> {
    match settings.get(key)? {
        "docked" => Some(None),
        v => pair(v, ',').map(Some),
    }
}

/// Whether a panel of `size` at `at` would lie fully outside the screen.
fn off_screen(at: (i32, i32), size: (u32, u32), screen: (u32, u32)) -> bool {
    let (x, y) = (at.0 as i64, at.1 as i64);
    x + size.0 as i64 <= 0 || y + size.1 as i64 <= 0 || x >= screen.0 as i64 || y >= screen.1 as i64
}

fn on_screen(at: (i32, i32), size: (u32, u32), screen: (u32, u32)) -> (i32, i32) {
    if off_screen(at, size, screen) {
        (0, 0)
    } else {
        at
    }
}

/// Load the layout from the settings into a freshly built state.
pub fn restore(st: &mut State) {
    let screen = st.screen;
    let s = &st.settings;
    let mut panels = view_panels::Panels::new(screen);
    let load = |key: &str, size: (u32, u32), target: &mut Place| {
        if let Some(v) = flag(s, &format!("view.{key}")) {
            target.visible = v;
        }
        if let Some(p) = place(s, &format!("place.{key}")) {
            target.floating = p.map(|at| on_screen(at, size, screen));
        }
    };
    load("tool_box", mw::TOOL_BOX_DOCKED, &mut panels.tool_box);
    load("color_box", mw::COLOR_BOX_DOCKED, &mut panels.color_box);
    if let Some(v) = flag(s, "view.status_bar") {
        panels.status_bar = v;
    }
    if let Some(p) = place(s, "place.text_toolbar") {
        panels.text_toolbar = p.map(|at| on_screen(at, TEXT_TOOLBAR_SIZE, screen));
    }
    if let Some(at) = s.get("place.thumbnail").and_then(|v| pair(v, ',')) {
        panels.thumbnail_at = on_screen(at, THUMBNAIL_SIZE, screen);
    }
    let text_toolbar = flag(s, "view.text_toolbar");
    let window = s
        .get(KEY_WINDOW)
        .and_then(|v| v.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));

    st.panels = panels;
    if let Some(v) = text_toolbar {
        st.text_toolbar = v;
    }
    if let Some((w, h)) = window {
        mw::set_window_size(st, w, h);
    }
    mw::refresh_workspace(st);
}

fn put(settings: &mut Settings, key: &str, value: &str) {
    // Keys and values are built here and never contain line breaks.
    let _ = settings.set(key, value);
}

fn flag_text(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

fn place_text(at: Option<(i32, i32)>) -> String {
    at.map_or_else(|| "docked".to_owned(), |(x, y)| format!("{x},{y}"))
}

/// Copy the current layout into the in-memory settings; the caller saves.
pub fn store(st: &mut State) {
    let (w, h) = st.window;
    let p = st.panels.clone();
    let text_toolbar = st.text_toolbar;
    let s = &mut st.settings;
    put(s, KEY_WINDOW, &format!("{w}x{h}"));
    put(s, "view.tool_box", flag_text(p.tool_box.visible));
    put(s, "place.tool_box", &place_text(p.tool_box.floating));
    put(s, "view.color_box", flag_text(p.color_box.visible));
    put(s, "place.color_box", &place_text(p.color_box.floating));
    put(s, "view.status_bar", flag_text(p.status_bar));
    put(s, "view.text_toolbar", flag_text(text_toolbar));
    put(s, "place.text_toolbar", &place_text(p.text_toolbar));
    put(s, "place.thumbnail", &format!("{},{}", p.thumbnail_at.0, p.thumbnail_at.1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_screen_means_fully_outside() {
        let screen = (1920, 1080);
        assert!(off_screen((-5000, -5000), (56, 272), screen));
        assert!(off_screen((-56, 10), (56, 272), screen));
        assert!(!off_screen((-55, 10), (56, 272), screen));
        assert!(off_screen((1920, 0), (56, 272), screen));
        assert!(!off_screen((1919, 1079), (56, 272), screen));
        assert_eq!(on_screen((-5000, 3), (56, 272), screen), (0, 0));
        assert_eq!(on_screen((300, 200), (56, 272), screen), (300, 200));
    }

    #[test]
    fn pairs_parse_strictly() {
        assert_eq!(pair("300,200", ','), Some((300, 200)));
        assert_eq!(pair("-5,7", ','), Some((-5, 7)));
        assert_eq!(pair("300;200", ','), None);
        assert_eq!(pair("a,b", ','), None);
    }
}
