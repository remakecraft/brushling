//! Feature ui.status_bar: message pane, pointer position pane and size pane.
//!
//! - Message: the idle text, or a one-line description of the highlighted
//!   menu item / tool button (`State::hover`).
//! - Position: `x,y` of the image pixel under the pointer; empty off the
//!   canvas. `State::pointer` records it (`ptr_image`) for every pointer
//!   event before the hooks run; the screen-to-image mapping (zoom, scroll)
//!   is `State::image_point`.
//! - Size: `w,h` while a shape or selection is dragged out, the selection's
//!   size while it is moved or resized, or the new canvas size while a canvas
//!   handle is dragged; empty otherwise.

use crate::features::canvas_resize_handle::drag_size_text;
use crate::features::ui_main_window::panel_visible;
use crate::raster::shapes::square_end;
use crate::registry::Registry;
use crate::selection::SelGesture;
use crate::state::State;
use crate::tools::PointerEvent;
use crate::types::*;

/// Message shown when nothing is highlighted.
pub const IDLE_MESSAGE: &str = "For help, open the Help menu.";

pub fn register(r: &mut Registry) {
    r.pointer_hook(track_pointer);
}

/// Never consumes the event. Any pointer activity means the pointer has left
/// the tool box / menu. (The pointer position itself is tracked by
/// `State::pointer`, ahead of every hook.)
fn track_pointer(st: &mut State, _ev: &PointerEvent) -> HResult<bool> {
    if st.hover != Hover::Nothing {
        st.hover = Hover::Nothing;
        st.hover_since = st.now_ms;
    }
    Ok(false)
}

pub fn status_bar(st: &State) -> StatusBar {
    StatusBar {
        visible: panel_visible(st, Panel::StatusBar),
        message: message(st),
        position: position(st),
        size: size(st),
    }
}

fn message(st: &State) -> String {
    match &st.hover {
        Hover::Nothing => IDLE_MESSAGE.to_owned(),
        Hover::ToolButton(t) => crate::features::tools_toolbox::hint(*t).to_owned(),
        Hover::MenuItem(path) => menu_hint(path),
    }
}

fn position(st: &State) -> String {
    match st.ptr_image {
        Some((x, y)) if st.doc.get(x, y).is_some() => format!("{x},{y}"),
        _ => String::new(),
    }
}

/// Inclusive extent of a drag from `a` to `b`.
fn extent(a: (i32, i32), b: (i32, i32)) -> String {
    format!("{},{}", (a.0 as i64 - b.0 as i64).abs() + 1, (a.1 as i64 - b.1 as i64).abs() + 1)
}

fn size(st: &State) -> String {
    if let Some(t) = drag_size_text(st) {
        return t;
    }
    let Some(held) = st.held.filter(|h| !h.cancelled) else { return String::new() };
    if let Some(SelGesture::Resize { .. } | SelGesture::Move { .. }) = st.sel_gesture {
        if let Some(sel) = &st.sel {
            return format!("{},{}", sel.bounds.width(), sel.bounds.height());
        }
    }
    if let Some(d) = st.sel_drag.as_ref().filter(|_| matches!(st.tool, Tool::Select | Tool::FreeFormSelect)) {
        return extent(d.start, d.cur);
    }
    if matches!(st.tool, Tool::Rectangle | Tool::Ellipse | Tool::RoundedRectangle) {
        let (Some(start), Some(cur)) = (held.start_image, st.ptr_image.or(st.pointer_image)) else {
            return String::new();
        };
        let cur = if st.ptr_mods.shift { square_end(start, cur) } else { cur };
        return extent(start, cur);
    }
    String::new()
}

/// One-line description of a menu item, in the remake's own words.
fn menu_hint(path: &str) -> String {
    let text = match path {
        "File" => "Open, save, print and exit.",
        "Edit" => "Undo, copy, paste and select.",
        "View" => "Choose what is shown and how large.",
        "Image" => "Change the whole picture or the selection.",
        "Colors" => "Change the palette.",
        "Help" => "Get help and information about Brushling.",
        "File > New" => "Starts a new, empty picture.",
        "File > Open..." => "Opens an existing picture file.",
        "File > Save" => "Saves the picture under its current name.",
        "File > Save As..." => "Saves the picture under a new name or format.",
        "File > Print Preview" => "Shows the picture as it would print.",
        "File > Page Setup..." => "Sets paper, orientation and margins.",
        "File > Print..." => "Prints the picture.",
        "File > Send..." => "Mails the picture as an attachment.",
        "File > Set As Wallpaper (Tiled)" => "Repeats the picture across the desktop background.",
        "File > Set As Wallpaper (Centered)" => "Puts the picture in the middle of the desktop background.",
        "File > Exit" => "Closes Brushling.",
        "Edit > Undo" => "Takes back the last change.",
        "Edit > Repeat" => "Does the last undone change again.",
        "Edit > Cut" => "Moves the selection to the clipboard.",
        "Edit > Copy" => "Copies the selection to the clipboard.",
        "Edit > Paste" => "Inserts the clipboard contents into the picture.",
        "Edit > Clear Selection" => "Deletes the selected part of the picture.",
        "Edit > Select All" => "Selects the whole picture.",
        "Edit > Copy To..." => "Writes the selection to a picture file.",
        "Edit > Paste From..." => "Inserts a picture file as a selection.",
        "View > Tool Box" => "Shows or hides the tool box.",
        "View > Color Box" => "Shows or hides the color box.",
        "View > Status Bar" => "Shows or hides the status bar.",
        "View > Text Toolbar" => "Shows or hides the text formatting bar.",
        "View > Zoom" => "Changes the magnification of the picture.",
        "View > Zoom > Normal Size" => "Shows the picture at actual size.",
        "View > Zoom > Large Size" => "Shows the picture magnified four times.",
        "View > Zoom > Custom..." => "Chooses a magnification from a list.",
        "View > Zoom > Show Grid" => "Draws a grid over magnified pixels.",
        "View > Zoom > Show Thumbnail" => "Shows a small actual-size view of the picture.",
        "View > View Bitmap" => "Shows the picture on the whole screen.",
        "Image > Flip/Rotate..." => "Mirrors or turns the picture or selection.",
        "Image > Stretch/Skew..." => "Resizes or slants the picture or selection.",
        "Image > Invert Colors" => "Swaps every color for its opposite.",
        "Image > Attributes..." => "Sets the picture size and color mode.",
        "Image > Clear Image" => "Fills the whole picture with the background color.",
        "Image > Draw Opaque" => "Chooses whether selections hide what is beneath them.",
        "Colors > Edit Colors..." => "Mixes a color and stores it in the color box.",
        "Help > Help Topics" => "Opens the list of help topics.",
        "Help > About Brushling" => "Shows the version and licence of Brushling.",
        _ => "",
    };
    if !text.is_empty() {
        return text.to_owned();
    }
    // A recent-file entry ("File > 1 C:\a.bmp").
    match path.strip_prefix("File > ") {
        Some(label) => format!("Opens {label}."),
        None => format!("{path}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::collections::HashSet;

    fn walk(st: &State, parent: &str, out: &mut Vec<String>) {
        for it in st.menu_listing(parent).unwrap() {
            if it.separator {
                continue;
            }
            let path = if parent.is_empty() { it.label.clone() } else { format!("{parent} > {}", it.label) };
            if it.submenu {
                walk(st, &path, out);
            }
            out.push(path);
        }
    }

    #[test]
    fn every_menu_item_has_its_own_hint() {
        let dir = std::env::temp_dir().join(format!("brushling-status-{}", std::process::id()));
        let st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        let mut paths = Vec::new();
        walk(&st, "", &mut paths);
        let hints: HashSet<String> = paths.iter().map(|p| menu_hint(p)).collect();
        assert_eq!(hints.len(), paths.len());
        assert!(!hints.contains(IDLE_MESSAGE));
        assert!(paths.iter().all(|p| !menu_hint(p).ends_with(" > .") && !menu_hint(p).is_empty()));
    }
}
