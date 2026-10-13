//! One module per feature. Each exposes `register(&mut Registry)`, called
//! from `register_all` below, so a feature only edits its own file.

use crate::registry::Registry;

pub mod color_box;
pub mod eraser;
pub mod fill;
pub mod flip_rotate;
pub mod invert;
pub mod line;
pub mod mouse_colors;
pub mod pencil;
pub mod pick_color;

/// Register every feature with the registry (called once, on first use).
pub fn register_all(r: &mut Registry) {
    color_box::register(r);
    eraser::register(r);
    fill::register(r);
    flip_rotate::register(r);
    invert::register(r);
    line::register(r);
    mouse_colors::register(r);
    pencil::register(r);
    pick_color::register(r);
}
