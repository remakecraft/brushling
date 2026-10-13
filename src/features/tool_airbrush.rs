//! Feature tool.airbrush: while the button is held, sprays random single
//! pixels inside a disc around the pointer, once at the press and then every
//! 50 ms of virtual time. Each tick paints 2r dots drawn uniformly from the
//! disc of the chosen radius (rejection sampling on the seeded generator);
//! dots off the canvas are dropped but still count. Press to release is one
//! undo step.

use crate::features::mouse_colors::paint_colour;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

/// Virtual milliseconds between two sprays while the button is held.
pub const TICK_MS: u64 = 50;

/// Per-gesture airbrush data (a field of [`State`]).
#[derive(Debug, Clone, Default)]
pub struct AirbrushState {
    active: bool,
    /// Time collected towards the next tick.
    acc_ms: u64,
}

struct Airbrush;
static AIRBRUSH: Airbrush = Airbrush;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Airbrush, &AIRBRUSH);
}

/// One tick: 2r dots at the pointer in the held button's colour.
fn spray(st: &mut State, button: Button) {
    let Some((cx, cy)) = st.pointer_image else { return };
    let r = st.options.airbrush_radius as i32;
    let colour = paint_colour(st, Some(button));
    let span = (2 * r + 1) as u32;
    for _ in 0..2 * r {
        let (dx, dy) = loop {
            let dx = st.rng.below(span) as i32 - r;
            let dy = st.rng.below(span) as i32 - r;
            if dx * dx + dy * dy <= r * r {
                break (dx, dy);
            }
        };
        st.doc.paint(cx + dx, cy + dy, colour);
    }
}

impl ToolHandler for Airbrush {
    fn deactivate(&self, st: &mut State) -> HResult<()> {
        st.airbrush.active = false;
        Ok(())
    }

    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(button) = ev.button else { return Ok(()) };
        if ev.image.is_none() {
            return Ok(());
        }
        st.airbrush.active = true;
        st.airbrush.acc_ms = 0;
        st.doc.push_undo();
        spray(st, button);
        Ok(())
    }

    fn release(&self, st: &mut State, _ev: &PointerEvent) -> HResult<()> {
        st.airbrush.active = false;
        Ok(())
    }

    fn tick(&self, st: &mut State, ms: u64) -> HResult<()> {
        let held = match st.held {
            Some(h) if st.airbrush.active && !h.cancelled => h,
            _ => {
                st.airbrush.active = false;
                return Ok(());
            }
        };
        st.airbrush.acc_ms += ms;
        while st.airbrush.acc_ms >= TICK_MS {
            st.airbrush.acc_ms -= TICK_MS;
            spray(st, held.button);
        }
        Ok(())
    }
}
