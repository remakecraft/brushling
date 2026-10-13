//! Feature colors.color_box: swatch state and behaviour. The `App` methods
//! `swatches` / `click_swatch` in `harness.rs` delegate to code written here;
//! add colour box state to `State` and register menu items/dialogs (Edit
//! Colors) from `register`. See `src/README.md`.

use crate::registry::Registry;
use crate::state::State;
use crate::types::{Button, HResult, HarnessError, Rgb};

/// Number of swatches in the colour box (2 rows of 14).
pub const SWATCH_COUNT: usize = 28;

/// Default palette: top row (swatches 1-14) then bottom row (15-28).
pub const DEFAULT_SWATCHES: [Rgb; SWATCH_COUNT] = [
    Rgb::new(0, 0, 0),
    Rgb::new(128, 128, 128),
    Rgb::new(128, 0, 0),
    Rgb::new(128, 128, 0),
    Rgb::new(0, 128, 0),
    Rgb::new(0, 128, 128),
    Rgb::new(0, 0, 128),
    Rgb::new(128, 0, 128),
    Rgb::new(128, 128, 64),
    Rgb::new(0, 64, 64),
    Rgb::new(0, 128, 255),
    Rgb::new(0, 64, 128),
    Rgb::new(64, 0, 255),
    Rgb::new(128, 64, 0),
    Rgb::new(255, 255, 255),
    Rgb::new(192, 192, 192),
    Rgb::new(255, 0, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 255, 255),
    Rgb::new(0, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 255, 128),
    Rgb::new(0, 255, 128),
    Rgb::new(128, 255, 255),
    Rgb::new(128, 128, 255),
    Rgb::new(255, 0, 128),
    Rgb::new(255, 128, 64),
];

pub fn register(_r: &mut Registry) {}

/// The current swatch colours.
pub fn swatches(st: &State) -> Vec<Rgb> {
    st.swatches.to_vec()
}

/// Click swatch `n` (1-based): left sets the foreground, right the
/// background. No pixels change and no undo step is made. A double-click
/// also opens Edit Colors for that swatch.
pub fn click_swatch(st: &mut State, n: u32, button: Button, double: bool) -> HResult<()> {
    let index = n
        .checked_sub(1)
        .map(|i| i as usize)
        .filter(|i| *i < SWATCH_COUNT)
        .ok_or_else(|| HarnessError::Invalid(format!("no swatch {n}")))?;
    let colour = st.swatches[index];
    st.last_swatch = index;
    match button {
        Button::Left => st.fg = colour,
        Button::Right => st.bg = colour,
    }
    if double {
        super::colors_edit_colors::open(st);
    }
    Ok(())
}
