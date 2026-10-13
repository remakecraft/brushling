//! Feature tool.pencil: 1-pixel freehand drawing. Without Shift every pointer
//! move paints the line from the previous to the new position; with Shift
//! held at the press the stroke is one live straight line from the press
//! point, snapped to 0/45/90 degrees, redrawn over the pre-stroke pixels.
//!
//! Left paints the foreground, right the background colour (see
//! `mouse_colors`). A stroke is one undo step, recorded when its first pixel
//! lands on the canvas, so a stroke that never touches it leaves no step.
//! The pointer may leave the canvas (pixels outside are discarded, and while
//! it is over a non-canvas area the stroke is simply interrupted) and come
//! back. Esc, or the other mouse button, cancels the stroke.

use crate::features::mouse_colors::{cancel_stroke, paint_colour};
use crate::raster::{in_bounds, line_points, snap_line_end};
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Pencil;
static PENCIL: Pencil = Pencil;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Pencil, &PENCIL);
}

/// Record the stroke's undo step once, just before its first pixel lands on
/// the canvas.
fn ensure_undo(st: &mut State, points: &[(i32, i32)]) {
    if st.pencil_pushed {
        return;
    }
    let (w, h) = (st.doc.width, st.doc.height);
    if points.iter().any(|&(x, y)| in_bounds(x, y, w, h)) {
        st.doc.push_undo();
        st.pencil_pushed = true;
    }
}

fn paint_line(st: &mut State, a: (i32, i32), b: (i32, i32), c: Rgb) {
    let pts = line_points(a, b);
    ensure_undo(st, &pts);
    for (x, y) in pts {
        st.doc.put(x, y, c);
    }
}

/// Put the pre-stroke pixels back (Shift preview redraw).
fn restore_base(st: &mut State) {
    if let Some(base) = st.pencil_base.as_ref() {
        if base.len() == st.doc.pixels.len() {
            st.doc.pixels.clone_from(base);
        }
    }
}

fn end_stroke(st: &mut State) {
    st.pencil_last = None;
    st.pencil_base = None;
    st.pencil_active = false;
    st.pencil_pushed = false;
}

/// Extend the stroke to `p` (`None` = the pointer is over a non-canvas area).
fn extend(st: &mut State, p: Option<(i32, i32)>, button: Option<Button>) {
    let Some(p) = p else {
        // Position unknown: break the path so no line is invented across the
        // canvas when the pointer comes back.
        if st.pencil_base.is_none() {
            st.pencil_last = None;
        }
        return;
    };
    let c = paint_colour(st, button);
    if st.pencil_base.is_some() {
        let Some(start) = st.held.and_then(|h| h.start_image) else { return };
        restore_base(st);
        paint_line(st, start, snap_line_end(start, p), c);
    } else {
        let from = st.pencil_last.unwrap_or(p);
        paint_line(st, from, p, c);
    }
    st.pencil_last = Some(p);
}

impl ToolHandler for Pencil {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        end_stroke(st);
        let Some(p) = ev.image else { return Ok(()) };
        st.pencil_active = true;
        if ev.mods.shift {
            st.pencil_base = Some(st.doc.pixels.clone());
        }
        let c = paint_colour(st, ev.button);
        paint_line(st, p, p, c);
        st.pencil_last = Some(p);
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if st.pencil_active {
            extend(st, ev.image, ev.button);
        }
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if st.pencil_active {
            extend(st, ev.image, ev.button);
        }
        end_stroke(st);
        Ok(())
    }

    fn key(&self, st: &mut State, key: Key, _mods: Mods) -> HResult<bool> {
        if key == Key::Escape && st.pencil_active {
            cancel_stroke(st);
            end_stroke(st);
            return Ok(true);
        }
        Ok(false)
    }

    fn deactivate(&self, st: &mut State) -> HResult<()> {
        end_stroke(st);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-pencil-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.replace_document(crate::doc::Document::new(32, 32, Rgb::W).unwrap());
        st
    }

    fn at(x: i32, y: i32) -> Pos {
        Pos::Image(x, y)
    }

    fn black(st: &State) -> usize {
        st.doc.pixels.iter().filter(|p| **p == Rgb::K).count()
    }

    #[test]
    fn click_is_one_pixel_one_undo_step() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), at(4, 4), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), at(4, 4), Mods::NONE).unwrap();
        assert_eq!(black(&st), 1);
        assert_eq!(st.doc.undo.len(), 1);
        assert!(st.doc.dirty);
    }

    #[test]
    fn stroke_entirely_outside_leaves_no_step() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), at(-9, -9), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, at(-2, -9), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), at(-2, -9), Mods::NONE).unwrap();
        assert_eq!(black(&st), 0);
        assert!(st.doc.undo.is_empty());
        assert!(!st.doc.dirty);
    }

    #[test]
    fn right_button_paints_background_even_with_shift() {
        let mut st = state();
        st.bg = Rgb { r: 255, g: 0, b: 0 };
        st.pointer(PointerKind::Press(Button::Right), at(2, 2), Mods::SHIFT).unwrap();
        st.pointer(PointerKind::Move, at(9, 3), Mods::SHIFT).unwrap();
        st.pointer(PointerKind::Release(Button::Right), at(9, 3), Mods::SHIFT).unwrap();
        let red = st.doc.pixels.iter().filter(|p| **p == st.bg).count();
        assert_eq!(red, 8);
        assert_eq!(st.doc.get(9, 2), Some(st.bg));
        assert_eq!(black(&st), 0);
    }

    #[test]
    fn shift_stroke_is_redrawn_not_accumulated() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), at(10, 10), Mods::SHIFT).unwrap();
        st.pointer(PointerKind::Move, at(20, 20), Mods::SHIFT).unwrap();
        st.pointer(PointerKind::Move, at(10, 3), Mods::SHIFT).unwrap();
        st.pointer(PointerKind::Release(Button::Left), at(10, 3), Mods::SHIFT).unwrap();
        assert_eq!(black(&st), 8);
        assert_eq!(st.doc.undo.len(), 1);
    }

    #[test]
    fn escape_cancels_stroke_without_undo_step() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), at(1, 1), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, at(8, 1), Mods::NONE).unwrap();
        st.key(Key::Escape, Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, at(8, 8), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), at(8, 8), Mods::NONE).unwrap();
        assert_eq!(black(&st), 0);
        assert!(st.doc.undo.is_empty());
        assert!(!st.doc.dirty);
    }

    #[test]
    fn leaving_over_non_canvas_does_not_connect_across() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), at(2, 2), Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, Pos::Outside, Mods::NONE).unwrap();
        st.pointer(PointerKind::Move, at(2, 20), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), at(2, 20), Mods::NONE).unwrap();
        assert_eq!(black(&st), 2);
    }
}
