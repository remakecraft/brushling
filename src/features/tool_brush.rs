//! Feature tool.brush: freehand painting that stamps the selected brush
//! footprint (12 shapes: round, square and both slants in several sizes) at
//! every pixel of the path between pointer positions. Left = foreground,
//! right = background. With Shift held at the press the first move locks the
//! axis with the larger distance and the stroke stays on that row or column.
//! One press-to-release is one undo step.

use std::cell::Cell;

use crate::raster::shapes::path_points;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

struct Brush;
static BRUSH: Brush = Brush;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Brush, &BRUSH);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lock {
    Free,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct Stroke {
    last: (i32, i32),
    start: (i32, i32),
    shift: bool,
    lock: Lock,
}

thread_local! {
    /// The stroke in progress (cleared on release).
    static STROKE: Cell<Option<Stroke>> = const { Cell::new(None) };
}

/// Footprint offsets `(dx, dy)` of brush shape 1..=12 relative to the stamp
/// position.
pub(crate) fn brush_offsets(shape: u32) -> Vec<(i32, i32)> {
    let round = |d: i32| {
        let r = (d - 1) / 2;
        let mut v = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    v.push((dx, dy));
                }
            }
        }
        v
    };
    let square = |n: i32| {
        let (lo, hi) = (-(n / 2), (n + 1) / 2 - 1);
        (lo..=hi).flat_map(|dy| (lo..=hi).map(move |dx| (dx, dy))).collect::<Vec<_>>()
    };
    let ascending = |n: i32| (0..n).map(|i| (i - n / 2, n / 2 - i)).collect::<Vec<_>>();
    let descending = |n: i32| (0..n).map(|i| (i - n / 2, i - n / 2)).collect::<Vec<_>>();
    match shape {
        1 => round(7),
        2 => round(5),
        3 => round(1),
        4 => square(8),
        5 => square(5),
        6 => square(2),
        7 => ascending(8),
        8 => ascending(5),
        9 => ascending(2),
        10 => descending(8),
        11 => descending(5),
        _ => descending(2),
    }
}

fn stamp(st: &mut State, offsets: &[(i32, i32)], p: (i32, i32), c: Rgb) {
    for &(dx, dy) in offsets {
        st.doc.put(p.0 + dx, p.1 + dy, c);
    }
}

fn stamp_path(st: &mut State, a: (i32, i32), b: (i32, i32), c: Rgb) {
    let offsets = brush_offsets(st.options.brush_shape);
    for p in path_points(a, b) {
        stamp(st, &offsets, p, c);
    }
}

fn colour(st: &State, button: Option<Button>) -> Rgb {
    if button == Some(Button::Right) {
        st.bg
    } else {
        st.fg
    }
}

/// Apply the Shift axis lock to a pointer position, fixing the lock on the
/// first move that leaves the press point.
fn constrain(s: &mut Stroke, p: (i32, i32)) -> (i32, i32) {
    if !s.shift {
        return p;
    }
    let (dx, dy) = ((p.0 as i64 - s.start.0 as i64).abs(), (p.1 as i64 - s.start.1 as i64).abs());
    if s.lock == Lock::Free && (dx > 0 || dy > 0) {
        s.lock = if dx >= dy { Lock::Horizontal } else { Lock::Vertical };
    }
    match s.lock {
        Lock::Free => p,
        Lock::Horizontal => (p.0, s.start.1),
        Lock::Vertical => (s.start.0, p.1),
    }
}

impl ToolHandler for Brush {
    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        STROKE.with(|s| s.set(None));
        let Some(p) = ev.image else { return Ok(()) };
        st.doc.push_undo();
        let offsets = brush_offsets(st.options.brush_shape);
        let c = colour(st, ev.button);
        stamp(st, &offsets, p, c);
        STROKE.with(|s| s.set(Some(Stroke { last: p, start: p, shift: ev.mods.shift, lock: Lock::Free })));
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        let Some(mut s) = STROKE.with(Cell::get) else { return Ok(()) };
        let Some(p) = ev.image else { return Ok(()) };
        let p = constrain(&mut s, p);
        let c = colour(st, ev.button);
        stamp_path(st, s.last, p, c);
        s.last = p;
        STROKE.with(|cell| cell.set(Some(s)));
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        self.drag(st, ev)?;
        STROKE.with(|s| s.set(None));
        Ok(())
    }
}
