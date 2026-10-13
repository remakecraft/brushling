//! Feature tool.fill: implement a `ToolHandler` here and register it with
//! `r.tool(Tool::Fill, &FILL)`. See `src/README.md`.
//!
//! Flood fill over the 4-connected region of exactly the clicked colour.

use crate::features::mouse_colors::paint_colour;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Fill;

static FILL: Fill = Fill;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Fill, &FILL);
}

impl ToolHandler for Fill {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some((x, y)) = ev.image else { return Ok(()) };
        let Some(target) = st.doc.get(x, y) else { return Ok(()) };
        let chosen = paint_colour(st, ev.button);
        let paint = st.doc.snap(chosen);
        if paint == target {
            return Ok(());
        }
        st.doc.push_undo();
        let (w, h) = (st.doc.width as i32, st.doc.height as i32);
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            if cx < 0 || cy < 0 || cx >= w || cy >= h || st.doc.get(cx, cy) != Some(target) {
                continue;
            }
            st.doc.put(cx, cy, paint);
            stack.extend([(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn black_and_white_fill_with_unsnapped_paint_terminates() {
        let dir = std::env::temp_dir().join(format!("brushling-fillbw-{}", std::process::id()));
        let mut st = State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.replace_document(crate::doc::Document::new(8, 8, Rgb::K).unwrap());
        st.doc.color_mode = ColorMode::BlackAndWhite;
        st.fg = Rgb::new(30, 30, 30);
        let ev = PointerEvent {
            pos: Pos::Image(2, 2),
            image: Some((2, 2)),
            button: Some(Button::Left),
            mods: Mods::NONE,
            double_click: false,
        };
        FILL.press(&mut st, &ev).unwrap();
        assert!(st.doc.undo.is_empty(), "snapped paint equals target: no-op");
        assert!(st.doc.pixels.iter().all(|p| *p == Rgb::K));
    }
}
