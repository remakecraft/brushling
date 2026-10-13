//! Feature edit.clipboard: Edit > Cut / Copy / Paste (Ctrl+X / C / V) over
//! the in-process system clipboard (`State::clipboard`).
//!
//! [`selection_image`] and [`paste_image`] are shared with
//! edit.copy_to_paste_from, which writes / reads the same bounding-box image.

use crate::doc::MAX_DIM;
use crate::registry::Registry;
use crate::selection::Sel;
use crate::state::State;
use crate::types::*;

const ENLARGE_TEXT: &str = "The pasted image is larger than the canvas. Enlarge the canvas?";

pub fn register(r: &mut Registry) {
    r.command_if("Edit > Cut", cut, Some(State::has_selection));
    r.command_if("Edit > Copy", copy, Some(State::has_selection));
    r.command_if("Edit > Paste", paste, Some(can_paste));
    r.shortcut(Key::Char('x'), Mods::CTRL, "Edit > Cut");
    r.shortcut(Key::Char('c'), Mods::CTRL, "Edit > Copy");
    r.shortcut(Key::Char('v'), Mods::CTRL, "Edit > Paste");
}

/// Paste needs an image; clipboard text only matters to an active text frame
/// (tool.text), so it enables Paste while the Text tool is selected.
fn can_paste(st: &State) -> bool {
    match st.clipboard {
        ClipboardContent::Image(_) => true,
        ClipboardContent::Text(_) => st.tool == Tool::Text,
        ClipboardContent::Empty => false,
    }
}

/// The selection's bounding-box image as shown: floating pixels as they lie,
/// otherwise the document pixels; masked-out pixels and pixels beyond the
/// canvas become the background colour.
pub(crate) fn selection_image(st: &State) -> Option<Image> {
    let sel = st.sel.as_ref()?;
    let b = sel.bounds;
    let w = b.width() as usize;
    let mut pixels = Vec::with_capacity(sel.mask.len());
    for y in b.y0..=b.y1 {
        for x in b.x0..=b.x1 {
            let i = (y - b.y0) as usize * w + (x - b.x0) as usize;
            let c = if !sel.mask[i] {
                st.bg
            } else if let Some(f) = sel.floating.as_ref() {
                f[i]
            } else {
                st.doc.get(x, y).unwrap_or(st.bg)
            };
            pixels.push(c);
        }
    }
    Some(Image { width: b.width() as u32, height: b.height() as u32, pixels })
}

fn copy(st: &mut State) -> HResult<()> {
    if let Some(img) = selection_image(st) {
        st.clipboard = ClipboardContent::Image(img);
    }
    Ok(())
}

fn cut(st: &mut State) -> HResult<()> {
    let Some(img) = selection_image(st) else { return Ok(()) };
    st.clipboard = ClipboardContent::Image(img);
    // A floating selection's source was vacated when it was lifted, so only
    // an unlifted one edits pixels (and so needs an undo step).
    if st.sel.as_ref().is_some_and(|s| !s.is_floating()) {
        st.doc.push_undo();
    }
    st.erase_selection();
    st.discard_selection();
    Ok(())
}

fn paste(st: &mut State) -> HResult<()> {
    let ClipboardContent::Image(img) = st.clipboard.clone() else { return Ok(()) };
    paste_image(st, img)
}

/// Insert `img` as a floating selection at the first visible pixel: commit
/// the current selection, switch to Select, and ask first when the image is
/// bigger than the canvas.
pub(crate) fn paste_image(st: &mut State, img: Image) -> HResult<()> {
    if img.width == 0 || img.height == 0 || img.pixels.len() != img.width as usize * img.height as usize {
        return Ok(());
    }
    st.select_tool(Tool::Select)?;
    st.end_selection();
    if img.width > st.doc.width || img.height > st.doc.height {
        st.pending_paste = Some(img);
        st.message_box("Brushling", ENLARGE_TEXT, &["Yes", "No", "Cancel"], Some(enlarge_answered));
        return Ok(());
    }
    insert(st, img);
    Ok(())
}

fn enlarge_answered(st: &mut State, button: &str) -> HResult<()> {
    let Some(img) = st.pending_paste.take() else { return Ok(()) };
    match button {
        "Yes" => {
            st.doc.push_undo();
            grow_canvas(st, img.width, img.height);
            insert_floating(st, img);
        }
        "No" => insert(st, img),
        _ => {}
    }
    Ok(())
}

/// Grow the canvas to at least `w` x `h`; new area is the background colour.
fn grow_canvas(st: &mut State, w: u32, h: u32) {
    let (nw, nh) = (st.doc.width.max(w).min(MAX_DIM), st.doc.height.max(h).min(MAX_DIM));
    let (ow, bg) = (st.doc.width as usize, st.bg);
    let mut pixels = vec![bg; nw as usize * nh as usize];
    for y in 0..st.doc.height.min(nh) as usize {
        for x in 0..ow.min(nw as usize) {
            pixels[y * nw as usize + x] = st.doc.pixels[y * ow + x];
        }
    }
    st.doc.pixels = pixels;
    st.doc.width = nw;
    st.doc.height = nh;
}

/// Undo step plus the floating insert.
fn insert(st: &mut State, img: Image) {
    st.doc.push_undo();
    insert_floating(st, img);
}

/// The pasted selection's top-left is the first image pixel visible in the
/// workspace (0,0 when not scrolled).
fn insert_floating(st: &mut State, img: Image) {
    let z = st.scale() as i32;
    let x = st.scroll.0.max(0) / z;
    let y = st.scroll.1.max(0) / z;
    let bounds = Rect::new(x, y, x + img.width as i32 - 1, y + img.height as i32 - 1);
    let mut sel = Sel::rect(bounds);
    sel.floating = Some(img.pixels);
    st.sel = Some(sel);
}
