//! Feature tools.mouse_colors: which colour the left and right buttons draw
//! with. Tools read `State::fg` / `State::bg`; put the button-to-colour rule
//! here as a helper the tools call. See `src/README.md`.

use crate::registry::Registry;

pub fn register(_r: &mut Registry) {}
