//! Feature tool.curve: a straight line (drag) bent by up to two control
//! points (two further press-drags) into a cubic Bezier in the current width.
//! Everything is a live preview in the composite layer until the curve is
//! finished, which is one undo step.
//!
//! Stages: drag the line, press-drag the first bend (C1 = C2 = pointer),
//! press-drag the second bend (C2 = pointer, then commit). A double-click in
//! the last stage finishes with C2 = C1. Esc discards; another tool commits
//! what is shown.

use crate::features::mouse_colors::paint_colour;
use crate::raster::shapes::{path_points, stamp_square, thick_line};
use crate::raster::snap_line_end;
use crate::registry::Registry;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;

type P = (i32, i32);

/// Where the curve gesture stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Idle,
    /// Button down, dragging the straight line.
    Line,
    /// Line fixed, waiting for the first bend press.
    AwaitBend1,
    /// Button down, placing the first bend.
    Bend1,
    /// First bend fixed, waiting for the second bend press.
    AwaitBend2,
    /// Button down, placing the second bend.
    Bend2,
}

/// Per-gesture curve data (a field of [`State`]).
#[derive(Debug, Clone, Default)]
pub struct CurveState {
    pub stage: Stage,
    pub p0: P,
    pub p3: P,
    pub c1: P,
    pub c2: P,
    pub button: Option<Button>,
    /// The press that finished the curve must not act again on its release.
    skip_release: bool,
}

struct Curve;
static CURVE: Curve = Curve;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Curve, &CURVE);
    r.composite_layer(preview);
}

/// Cubic Bezier sampled at t = i/64, rounded half up, joined by the
/// 1-pixel path rule.
fn curve_path(p0: P, c1: P, c2: P, p3: P) -> Vec<P> {
    let f = |a: i32, b: i32, c: i32, d: i32, t: f64| -> i32 {
        let u = 1.0 - t;
        let v = u * u * u * a as f64 + 3.0 * u * u * t * b as f64 + 3.0 * u * t * t * c as f64 + t * t * t * d as f64;
        (v + 0.5).floor() as i32
    };
    let samples: Vec<P> = (0..=64)
        .map(|i| {
            let t = i as f64 / 64.0;
            (f(p0.0, c1.0, c2.0, p3.0, t), f(p0.1, c1.1, c2.1, p3.1, t))
        })
        .collect();
    let mut out = vec![samples[0]];
    for w in samples.windows(2) {
        out.extend(path_points(w[0], w[1]));
    }
    out
}

/// Pixels of the curve as it currently looks (stamped with width `w`).
fn shown_pixels(c: &CurveState, w: i32) -> Vec<P> {
    match c.stage {
        Stage::Idle => Vec::new(),
        Stage::Line | Stage::AwaitBend1 => thick_line(c.p0, c.p3, w),
        _ => stamp_square(&curve_path(c.p0, c.c1, c.c2, c.p3), w),
    }
}

fn preview(st: &State, img: &mut Image) {
    if st.tool != Tool::Curve || st.curve.stage == Stage::Idle {
        return;
    }
    let c = paint_colour(st, st.curve.button);
    for (x, y) in shown_pixels(&st.curve, st.options.line_width as i32) {
        if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
            img.pixels[y as usize * img.width as usize + x as usize] = c;
        }
    }
}

/// Draw the curve into the document (one undo step) and end the gesture.
fn commit(st: &mut State) {
    let px = shown_pixels(&st.curve, st.options.line_width as i32);
    let c = paint_colour(st, st.curve.button);
    st.curve.stage = Stage::Idle;
    if px.is_empty() {
        return;
    }
    st.doc.push_undo();
    for (x, y) in px {
        st.doc.put(x, y, c);
    }
}

fn pointer_of(st: &State, ev: &PointerEvent) -> Option<P> {
    ev.image.or(st.pointer_image)
}

/// Move the point being placed to the pointer.
fn follow(st: &mut State, ev: &PointerEvent) {
    let Some(p) = pointer_of(st, ev) else { return };
    let c = &mut st.curve;
    match c.stage {
        Stage::Line => c.p3 = if ev.mods.shift { snap_line_end(c.p0, p) } else { p },
        Stage::Bend1 => {
            c.c1 = p;
            c.c2 = p;
        }
        Stage::Bend2 => c.c2 = p,
        _ => {}
    }
}

impl ToolHandler for Curve {
    fn deactivate(&self, st: &mut State) -> HResult<()> {
        match st.curve.stage {
            Stage::AwaitBend1 | Stage::Bend1 | Stage::AwaitBend2 | Stage::Bend2 => commit(st),
            _ => st.curve.stage = Stage::Idle,
        }
        Ok(())
    }

    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        st.curve.skip_release = false;
        let Some(p) = pointer_of(st, ev) else { return Ok(()) };
        match st.curve.stage {
            Stage::Idle => {
                let c = &mut st.curve;
                c.p0 = p;
                c.p3 = p;
                c.c1 = p;
                c.c2 = p;
                c.button = ev.button;
                c.stage = Stage::Line;
            }
            Stage::AwaitBend1 => {
                st.curve.stage = Stage::Bend1;
                follow(st, ev);
            }
            Stage::AwaitBend2 if ev.double_click => {
                st.curve.c2 = st.curve.c1;
                st.curve.skip_release = true;
                commit(st);
            }
            Stage::AwaitBend2 => {
                st.curve.stage = Stage::Bend2;
                follow(st, ev);
            }
            _ => {}
        }
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        follow(st, ev);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if st.curve.skip_release {
            st.curve.skip_release = false;
            return Ok(());
        }
        follow(st, ev);
        match st.curve.stage {
            Stage::Line => st.curve.stage = Stage::AwaitBend1,
            Stage::Bend1 => st.curve.stage = Stage::AwaitBend2,
            Stage::Bend2 => commit(st),
            _ => {}
        }
        Ok(())
    }

    fn key(&self, st: &mut State, key: Key, _mods: Mods) -> HResult<bool> {
        if key == Key::Escape && st.curve.stage != Stage::Idle {
            st.curve.stage = Stage::Idle;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_bezier_stays_on_the_line() {
        let p = curve_path((10, 20), (20, 20), (20, 20), (30, 20));
        assert!(p.iter().all(|&(_, y)| y == 20));
        assert_eq!(p.first(), Some(&(10, 20)));
        assert_eq!(p.last(), Some(&(30, 20)));
    }
}
