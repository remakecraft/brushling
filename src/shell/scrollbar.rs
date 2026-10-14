//! The workspace scroll bars and the mouse wheel. The app owns the scroll
//! offset and its range; a bar maps it to a thumb on its track and turns
//! presses and drags back into an offset for [`brushling::App::set_scroll`].
//! The geometry is pure so it can be tested without a window.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use super::theme::{self, Bevel};

/// Shortest thumb, in points, so a huge image keeps a grabbable thumb.
const MIN_THUMB: f32 = 10.0;

/// One scrolling axis in screen pixels: `view` of `view + range` content is
/// visible, starting `offset` pixels in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Axis {
    pub view: i32,
    pub range: i32,
    pub offset: i32,
}

impl Axis {
    /// Thumb `(start, length)` along a track `track` points long.
    pub fn thumb(&self, track: f32) -> (f32, f32) {
        let content = (self.view.max(0) as f32 + self.range.max(0) as f32).max(1.0);
        let len = (track * self.view.max(0) as f32 / content).max(MIN_THUMB).min(track);
        let start = if self.range > 0 { (track - len) * self.offset as f32 / self.range as f32 } else { 0.0 };
        (start, len)
    }

    /// The offset that puts the thumb's start at `start` points.
    pub fn offset_at(&self, track: f32, start: f32) -> i32 {
        let free = track - self.thumb(track).1;
        if free <= 0.0 || self.range <= 0 {
            return 0;
        }
        ((start / free).clamp(0.0, 1.0) * self.range as f32).round() as i32
    }

    /// A press on the track at `at` points: one view towards it, or nothing
    /// on the thumb itself.
    pub fn page_towards(&self, track: f32, at: f32) -> i32 {
        let (start, len) = self.thumb(track);
        if at < start {
            (self.offset - self.view).max(0)
        } else if at >= start + len {
            (self.offset + self.view).min(self.range)
        } else {
            self.offset
        }
    }
}

/// Whole screen pixels to scroll for a wheel delta in points (positive =
/// content moves right / down, so the offset shrinks). The fraction carries
/// into the next frame so slow touchpad scrolling still moves.
pub fn wheel_step(carry: &mut (f32, f32), delta: Vec2, ppp: f32) -> (i32, i32) {
    let x = carry.0 - delta.x * ppp;
    let y = carry.1 - delta.y * ppp;
    let (wx, wy) = (x.trunc(), y.trunc());
    *carry = (x - wx, y - wy);
    (wx as i32, wy as i32)
}

/// Pixels one arrow click scrolls.
pub const LINE_STEP: i32 = 16;
/// Seconds an arrow is held before it repeats, and between repeats.
const REPEAT_DELAY: f64 = 0.4;
const REPEAT_EVERY: f64 = 0.05;

/// Length of each arrow button on a bar `len` long and `thick` wide: square,
/// or half the bar each when it is too short for two squares.
pub fn arrow_len(len: f32, thick: f32) -> f32 {
    thick.min(len / 2.0).max(0.0)
}

impl Axis {
    /// The offset after scrolling `delta` pixels, kept in range.
    pub fn step(&self, delta: i32) -> i32 {
        self.offset.saturating_add(delta).clamp(0, self.range.max(0))
    }
}

/// Whether a held arrow fires this frame: on the press, then after the
/// delay at the repeat rate. `next` holds when it fires again.
pub fn repeat_fires(pressed: bool, now: f64, next: &mut Option<f64>) -> bool {
    if pressed {
        *next = Some(now + REPEAT_DELAY);
        return true;
    }
    match *next {
        Some(t) if now >= t => {
            *next = Some(now + REPEAT_EVERY);
            true
        }
        _ => false,
    }
}

/// Which way an arrow button points.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// One arrow button: raised, or pressed while held, with a pixel-art
/// triangle.
fn arrow(ui: &egui::Ui, rect: Rect, dir: Dir, down: bool, enabled: bool) {
    let (u, ppp) = theme::metrics(ui);
    let p = ui.painter();
    let rect = theme::snap(rect, ppp);
    let inner = theme::panel(p, rect, if down { Bevel::Pressed } else { Bevel::Raised }, theme::FACE, u);
    let (w, h) = ((inner.width() / u) as i32, (inner.height() / u) as i32);
    let shift = i32::from(down);
    let px = theme::Pixels { painter: p, origin: inner.min, u };
    // A 7-wide, 4-deep triangle, centred: row `i` is 7 - 2i long.
    for i in 0..4 {
        let len = 7 - 2 * i;
        let (x, y, fw, fh) = match dir {
            Dir::Up => ((w - 7) / 2 + i, (h - 4) / 2 + 3 - i, len, 1),
            Dir::Down => ((w - 7) / 2 + i, (h - 4) / 2 + i, len, 1),
            Dir::Left => ((w - 4) / 2 + 3 - i, (h - 7) / 2 + i, 1, len),
            Dir::Right => ((w - 4) / 2 + i, (h - 7) / 2 + i, 1, len),
        };
        if enabled {
            px.fill(x + shift, y + shift, fw, fh, theme::BLACK);
        } else {
            px.fill(x + 1, y + 1, fw, fh, theme::WHITE);
            px.fill(x, y, fw, fh, theme::SHADOW);
        }
    }
}

/// Draw one bar in `rect` (arrow buttons at both ends, the thumb on a
/// dithered track) and return the offset the user asked for this frame, if
/// any. `grab` holds where on the thumb a drag started.
pub fn bar(ui: &mut egui::Ui, id: egui::Id, rect: Rect, horizontal: bool, axis: Axis, grab: &mut Option<f32>) -> Option<i32> {
    let (len, thick) = if horizontal { (rect.width(), rect.height()) } else { (rect.height(), rect.width()) };
    let a = arrow_len(len, thick);
    let span = |from: f32, to: f32| {
        if horizontal {
            Rect::from_min_max(Pos2::new(rect.min.x + from, rect.min.y), Pos2::new(rect.min.x + to, rect.max.y))
        } else {
            Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + from), Pos2::new(rect.max.x, rect.min.y + to))
        }
    };
    let track_rect = span(a, len - a);
    let enabled = axis.range > 0;
    let mut want = None;

    // Arrows: one line per press, repeating while held.
    let now = ui.input(|i| i.time);
    let arrows = [
        (span(0.0, a), -1, "dec", if horizontal { Dir::Left } else { Dir::Up }),
        (span(len - a, len), 1, "inc", if horizontal { Dir::Right } else { Dir::Down }),
    ];
    for (r, sign, key, dir) in arrows {
        let resp = ui.interact(r, id.with(key), Sense::click_and_drag());
        let held = enabled && resp.is_pointer_button_down_on();
        let repeat_id = id.with((key, "repeat"));
        let mut next = ui.data(|d| d.get_temp::<f64>(repeat_id));
        if held {
            let pressed = ui.input(|i| i.pointer.primary_pressed());
            if repeat_fires(pressed, now, &mut next) {
                want = Some(Axis { offset: want.unwrap_or(axis.offset), ..axis }.step(sign * LINE_STEP));
            }
        } else {
            next = None;
        }
        ui.data_mut(|d| match next {
            Some(t) => {
                d.insert_temp(repeat_id, t);
            }
            None => d.remove::<f64>(repeat_id),
        });
        arrow(ui, r, dir, held && resp.hovered(), enabled);
    }

    let resp = ui.interact(track_rect, id, Sense::click_and_drag());
    let track = (len - 2.0 * a).max(0.0);
    let along = |p: Pos2| if horizontal { p.x - track_rect.min.x } else { p.y - track_rect.min.y };
    let (start, tlen) = axis.thumb(track);
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());

    if resp.is_pointer_button_down_on() {
        if let Some(at) = pointer.map(along) {
            if ui.input(|i| i.pointer.primary_pressed()) {
                *grab = (at >= start && at < start + tlen).then_some(at - start);
                if grab.is_none() {
                    want = Some(axis.page_towards(track, at));
                }
            } else if let Some(g) = *grab {
                want = Some(axis.offset_at(track, at - g));
            }
        }
    } else {
        *grab = None;
    }

    let (u, ppp) = theme::metrics(ui);
    theme::dither_fill(ui, theme::snap(track_rect, ppp));
    if enabled && track > 0.0 {
        let thumb = if horizontal {
            Rect::from_min_size(Pos2::new(track_rect.min.x + start, track_rect.min.y), Vec2::new(tlen, track_rect.height()))
        } else {
            Rect::from_min_size(Pos2::new(track_rect.min.x, track_rect.min.y + start), Vec2::new(track_rect.width(), tlen))
        };
        theme::panel(ui.painter(), theme::snap(thumb, ppp), Bevel::Raised, theme::FACE, u);
    }
    want.filter(|&o| o != axis.offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AXIS: Axis = Axis { view: 400, range: 600, offset: 0 };

    #[test]
    fn thumb_is_the_visible_share_of_the_track() {
        assert_eq!(AXIS.thumb(200.0), (0.0, 80.0));
        assert_eq!(Axis { offset: 600, ..AXIS }.thumb(200.0), (120.0, 80.0));
        assert_eq!(Axis { offset: 300, ..AXIS }.thumb(200.0), (60.0, 80.0));
        // Nothing to scroll: the thumb fills the track.
        assert_eq!(Axis { view: 400, range: 0, offset: 0 }.thumb(200.0), (0.0, 200.0));
        // A tiny share keeps the minimum thumb.
        assert_eq!(Axis { view: 10, range: 100_000, offset: 0 }.thumb(200.0).1, MIN_THUMB);
    }

    #[test]
    fn dragging_the_thumb_maps_back_to_the_offset() {
        assert_eq!(AXIS.offset_at(200.0, 60.0), 300);
        assert_eq!(AXIS.offset_at(200.0, 120.0), 600);
        assert_eq!(AXIS.offset_at(200.0, 500.0), 600);
        assert_eq!(AXIS.offset_at(200.0, -20.0), 0);
        assert_eq!(Axis { range: 0, ..AXIS }.offset_at(200.0, 50.0), 0);
        for offset in [0, 1, 299, 600] {
            let a = Axis { offset, ..AXIS };
            assert_eq!(a.offset_at(200.0, a.thumb(200.0).0), offset);
        }
    }

    #[test]
    fn track_presses_page_by_one_view() {
        let a = Axis { offset: 300, ..AXIS };
        assert_eq!(a.page_towards(200.0, 10.0), 0);
        assert_eq!(a.page_towards(200.0, 190.0), 600);
        assert_eq!(a.page_towards(200.0, 70.0), 300);
        assert_eq!(Axis { offset: 500, ..AXIS }.page_towards(200.0, 10.0), 100);
    }

    #[test]
    fn arrows_are_square_until_the_bar_is_short() {
        assert_eq!(arrow_len(300.0, 16.0), 16.0);
        assert_eq!(arrow_len(20.0, 16.0), 10.0);
        assert_eq!(arrow_len(0.0, 16.0), 0.0);
    }

    #[test]
    fn arrow_steps_stay_in_range() {
        let a = Axis { offset: 10, ..AXIS };
        assert_eq!(a.step(-LINE_STEP), 0);
        assert_eq!(a.step(LINE_STEP), 26);
        assert_eq!(Axis { offset: 595, ..AXIS }.step(LINE_STEP), 600);
        assert_eq!(Axis { view: 400, range: 0, offset: 0 }.step(LINE_STEP), 0);
    }

    #[test]
    fn held_arrows_repeat_after_a_delay() {
        let mut next = None;
        assert!(repeat_fires(true, 1.0, &mut next));
        assert!(!repeat_fires(false, 1.2, &mut next));
        assert!(repeat_fires(false, 1.41, &mut next));
        assert!(!repeat_fires(false, 1.43, &mut next));
        assert!(repeat_fires(false, 1.47, &mut next));
    }

    #[test]
    fn wheel_moves_against_the_delta_and_carries_fractions() {
        let mut carry = (0.0, 0.0);
        // Wheel up (positive y) scrolls the view up, at 1.5 pixels per point.
        assert_eq!(wheel_step(&mut carry, Vec2::new(0.0, 40.0), 1.5), (0, -60));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (0, 0));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (0, 0));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (1, 0));
    }
}
