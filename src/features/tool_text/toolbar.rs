//! The text toolbar model and the frame's right-click context menu.
//!
//! The toolbar is headless state: lists (`font_names`, `FONT_SIZES`) and the
//! choices in `State::text_style`. It is shown while a frame is active and
//! View > Text Toolbar is checked (`view_panels`), and every change restyles
//! the whole frame because `paint` reads `text_style` on each render.
//! Only the size and the Bold / Italic / Underline toggles change the
//! rendering; the bundled font is the only font that is drawn. Installed
//! fonts come from the platform; the headless build has none, so choosing a
//! font has no visible effect.

use super::{frame_active, BUNDLED_FONT, FONT_SIZES};
use crate::features::view_panels;
use crate::state::State;
use crate::types::*;
use crate::App;

const CUT: &str = "Cut";
const COPY: &str = "Copy";
const PASTE: &str = "Paste";
const TOOLBAR: &str = "Text Toolbar";

/// Entries of the font list: the bundled font first, then installed fonts.
pub fn font_names() -> Vec<String> {
    let mut names = vec![BUNDLED_FONT.to_owned()];
    names.extend(platform_font_names().into_iter().filter(|n| n != BUNDLED_FONT));
    names
}

/// Installed fonts come from the platform; the headless build has none.
fn platform_font_names() -> Vec<String> {
    Vec::new()
}

pub fn size_list_has_focus(st: &State) -> bool {
    st.text.as_ref().is_some_and(|s| s.size_list_focus)
}

/// Esc in the size list: focus goes back to the frame, which stays open.
pub fn return_focus_to_frame(st: &mut State) {
    if let Some(s) = st.text.as_mut() {
        s.size_list_focus = false;
    }
}

/// Give the toolbar's size list keyboard focus.
pub fn focus_size_list(st: &mut State) -> HResult<()> {
    require_shown(st)?;
    if let Some(s) = st.text.as_mut() {
        s.size_list_focus = true;
    }
    Ok(())
}

pub fn state(st: &State) -> TextToolbarState {
    let s = &st.text_style;
    TextToolbarState {
        fonts: font_names(),
        sizes: FONT_SIZES.to_vec(),
        font: s.font.clone(),
        size_pt: s.size_pt,
        bold: s.bold,
        italic: s.italic,
        underline: s.underline,
    }
}

/// The controls only exist while the toolbar is shown.
fn require_shown(st: &State) -> HResult<()> {
    if view_panels::visible(st, Panel::TextToolbar) {
        Ok(())
    } else {
        Err(HarnessError::Invalid("the text toolbar is not shown".to_owned()))
    }
}

pub fn set_font(st: &mut State, font: &str) -> HResult<()> {
    require_shown(st)?;
    if !font_names().iter().any(|f| f == font) {
        return Err(HarnessError::Invalid(format!("no font named {font:?} in the font list")));
    }
    st.text_style.font = font.to_owned();
    Ok(())
}

pub fn set_size(st: &mut State, pt: u32) -> HResult<()> {
    require_shown(st)?;
    if !FONT_SIZES.contains(&pt) {
        return Err(HarnessError::Invalid(format!("{pt} is not in the size list")));
    }
    st.text_style.size_pt = pt;
    // Choosing a size leaves keyboard focus in the size list.
    focus_size_list(st)?;
    // A bigger or smaller font re-wraps the text, so the frame follows.
    super::fit(st);
    Ok(())
}

pub fn set_bold(st: &mut State, on: bool) -> HResult<()> {
    require_shown(st)?;
    st.text_style.bold = on;
    super::fit(st);
    Ok(())
}

pub fn set_italic(st: &mut State, on: bool) -> HResult<()> {
    require_shown(st)?;
    st.text_style.italic = on;
    Ok(())
}

pub fn set_underline(st: &mut State, on: bool) -> HResult<()> {
    require_shown(st)?;
    st.text_style.underline = on;
    Ok(())
}

// ----- context menu

pub fn menu_is_open(st: &State) -> bool {
    st.text.as_ref().is_some_and(|s| s.menu_open)
}

pub fn close_context_menu(st: &mut State) {
    if let Some(s) = st.text.as_mut() {
        s.menu_open = false;
    }
}

/// A right press on the active frame (its inside or its border ring) opens
/// the menu; anywhere else it only closes it.
pub fn open_context_menu(st: &mut State, at: Option<(i32, i32)>) {
    let Some(s) = st.text.as_mut() else { return };
    let r = s.rect;
    s.menu_open = at.is_some_and(|p| p.0 >= r.x0 - 1 && p.0 <= r.x1 + 1 && p.1 >= r.y0 - 1 && p.1 <= r.y1 + 1);
}

/// Items of the open context menu: Cut, Copy, Paste, Text Toolbar.
pub fn context_menu(st: &State) -> Option<Vec<MenuItem>> {
    if !menu_is_open(st) {
        return None;
    }
    let edit = |label: &str| MenuItem {
        label: label.to_owned(),
        enabled: crate::features::ui_menu_enable_state::enabled(st, &format!("Edit > {label}")),
        ..MenuItem::default()
    };
    Some(vec![
        edit(CUT),
        edit(COPY),
        edit(PASTE),
        MenuItem { label: TOOLBAR.to_owned(), enabled: true, checked: Some(st.text_toolbar), ..MenuItem::default() },
    ])
}

/// Choose an item of the open context menu; a disabled item only closes it.
pub fn context_click(st: &mut State, label: &str) -> HResult<()> {
    let Some(items) = context_menu(st) else {
        return Err(HarnessError::Invalid("no text context menu is open".to_owned()));
    };
    let Some(item) = items.iter().find(|i| i.label == label) else {
        return Err(HarnessError::Invalid(format!("the text context menu has no item {label:?}")));
    };
    let enabled = item.enabled;
    close_context_menu(st);
    if !enabled || !frame_active(st) {
        return Ok(());
    }
    if label == TOOLBAR {
        view_panels::toggle_text_toolbar(st);
        Ok(())
    } else {
        st.menu(&format!("Edit > {label}"))
    }
}

impl App {
    /// The text toolbar's lists and current choices (tool.text).
    pub fn text_toolbar(&self) -> HResult<TextToolbarState> {
        require_shown(&self.st)?;
        Ok(state(&self.st))
    }

    /// Put keyboard focus in the toolbar's font size list; Esc then returns
    /// it to the frame without committing (tool.text).
    pub fn focus_text_size_list(&mut self) -> HResult<()> {
        focus_size_list(&mut self.st)
    }

    pub fn set_text_font(&mut self, font: &str) -> HResult<()> {
        set_font(&mut self.st, font)?;
        return_focus_to_frame(&mut self.st);
        Ok(())
    }

    /// Choose a size of the list, in points.
    pub fn set_text_size(&mut self, pt: u32) -> HResult<()> {
        set_size(&mut self.st, pt)
    }

    pub fn set_text_bold(&mut self, on: bool) -> HResult<()> {
        set_bold(&mut self.st, on)?;
        return_focus_to_frame(&mut self.st);
        Ok(())
    }

    pub fn set_text_italic(&mut self, on: bool) -> HResult<()> {
        set_italic(&mut self.st, on)?;
        return_focus_to_frame(&mut self.st);
        Ok(())
    }

    pub fn set_text_underline(&mut self, on: bool) -> HResult<()> {
        set_underline(&mut self.st, on)?;
        return_focus_to_frame(&mut self.st);
        Ok(())
    }

    /// The text frame's context menu while open (right-click on the frame).
    pub fn text_context_menu(&self) -> HResult<Option<Vec<MenuItem>>> {
        Ok(context_menu(&self.st))
    }

    /// Choose an item of the open context menu by label.
    pub fn text_context_click(&mut self, label: &str) -> HResult<()> {
        context_click(&mut self.st, label)
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, Button, ClipboardContent, Key, Mods, Panel, PointerKind, Pos, StartOptions, Tool};

    fn app(tag: &str) -> App {
        let dir = std::env::temp_dir().join(format!("brushling-ttb-{}-{tag}", std::process::id()));
        let mut a = App::start(StartOptions::new(dir)).unwrap();
        a.select_tool(Tool::Text).unwrap();
        a
    }

    fn make_frame(a: &mut App) {
        a.pointer(PointerKind::Press(Button::Left), Pos::Image(10, 10), Mods::NONE).unwrap();
        a.pointer(PointerKind::Release(Button::Left), Pos::Image(109, 49), Mods::NONE).unwrap();
    }

    fn right_click(a: &mut App, x: i32, y: i32) {
        a.pointer(PointerKind::Press(Button::Right), Pos::Image(x, y), Mods::NONE).unwrap();
        a.pointer(PointerKind::Release(Button::Right), Pos::Image(x, y), Mods::NONE).unwrap();
    }

    fn paste_enabled(a: &App) -> bool {
        a.st.menu_entry("Edit > Paste").unwrap().enabled
    }

    #[test]
    fn toolbar_shows_with_a_frame_and_lists_the_bundled_font_first() {
        let mut a = app("show");
        assert!(!a.panel(Panel::TextToolbar).unwrap().visible);
        assert!(a.set_text_bold(true).is_err());
        make_frame(&mut a);
        assert!(a.panel(Panel::TextToolbar).unwrap().visible);
        let t = a.text_toolbar().unwrap();
        assert_eq!(t.fonts[0], "Brushling Sans");
        assert_eq!((t.sizes.len(), t.size_pt, t.bold, t.italic, t.underline), (16, 10, false, false, false));
        a.set_text_size(24).unwrap();
        a.set_text_italic(true).unwrap();
        assert!(a.set_text_size(13).is_err() && a.set_text_font("Nope").is_err());
        let t = a.text_toolbar().unwrap();
        assert_eq!((t.size_pt, t.italic), (24, true));
    }

    #[test]
    fn esc_in_the_size_list_returns_focus_without_committing() {
        let mut a = app("sizefocus");
        assert!(a.text_toolbar().is_err() && a.focus_text_size_list().is_err());
        make_frame(&mut a);
        a.focus_text_size_list().unwrap();
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert!(a.panel(Panel::TextToolbar).unwrap().visible);
        a.set_text_size(12).unwrap();
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert!(a.panel(Panel::TextToolbar).unwrap().visible);
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert!(!a.panel(Panel::TextToolbar).unwrap().visible);
    }

    #[test]
    fn typing_after_a_size_change_returns_focus_so_esc_commits() {
        let mut a = app("sizefocus2");
        make_frame(&mut a);
        a.set_text_size(12).unwrap();
        a.key(Key::Char('A'), Mods::NONE).unwrap();
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert!(!a.panel(Panel::TextToolbar).unwrap().visible);
    }

    #[test]
    fn style_change_after_a_size_change_returns_focus_so_esc_commits() {
        let mut a = app("sizefocus3");
        make_frame(&mut a);
        a.set_text_size(12).unwrap();
        a.set_text_bold(true).unwrap();
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert!(!a.panel(Panel::TextToolbar).unwrap().visible);
    }

    #[test]
    fn context_menu_toggles_the_toolbar() {
        let mut a = app("ctx");
        right_click(&mut a, 20, 20);
        assert!(a.text_context_menu().unwrap().is_none());
        make_frame(&mut a);
        right_click(&mut a, 20, 20);
        let items = a.text_context_menu().unwrap().unwrap();
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, ["Cut", "Copy", "Paste", "Text Toolbar"]);
        assert_eq!(items[3].checked, Some(true));
        a.text_context_click("Text Toolbar").unwrap();
        assert!(a.text_context_menu().unwrap().is_none());
        assert!(!a.panel(Panel::TextToolbar).unwrap().visible);
        assert_eq!(a.st.menu_entry("View > Text Toolbar").unwrap().checked, Some(false));
    }

    #[test]
    fn paste_never_inserts_an_image_under_the_text_tool() {
        let mut a = app("paste");
        let img = crate::Image { width: 1, height: 1, pixels: vec![crate::Rgb::K] };
        a.set_clipboard(ClipboardContent::Image(img)).unwrap();
        assert!(!paste_enabled(&a));
        a.menu("Edit > Paste").unwrap();
        assert_eq!(a.selection().unwrap(), None);
        a.set_clipboard(ClipboardContent::Text("hi".into())).unwrap();
        assert!(!paste_enabled(&a));
        make_frame(&mut a);
        assert!(paste_enabled(&a));
        a.key(Key::Escape, Mods::NONE).unwrap();
    }
}
