//! The workspace scroll bars and the mouse wheel. The app owns the scroll
//! offset and its range; a bar maps it to a thumb on its track and turns
//! presses and drags back into an offset for [`brushling::App::set_scroll`].
//! The geometry is pure so it can be tested without a window.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

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

/// Draw one bar in `rect` and return the offset the user asked for this
/// frame, if any. `grab` holds where on the thumb a drag started.
pub fn bar(ui: &mut egui::Ui, id: egui::Id, rect: Rect, horizontal: bool, axis: Axis, grab: &mut Option<f32>) -> Option<i32> {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let track = if horizontal { rect.width() } else { rect.height() };
    let along = |p: Pos2| if horizontal { p.x - rect.min.x } else { p.y - rect.min.y };
    let (start, len) = axis.thumb(track);
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let mut want = None;

    if resp.is_pointer_button_down_on() {
        if let Some(a) = pointer.map(along) {
            if ui.input(|i| i.pointer.primary_pressed()) {
                *grab = (a >= start && a < start + len).then_some(a - start);
                if grab.is_none() {
                    want = Some(axis.page_towards(track, a));
                }
            } else if let Some(g) = *grab {
                want = Some(axis.offset_at(track, a - g));
            }
        }
    } else {
        *grab = None;
    }

    let v = ui.visuals();
    let thumb_rect = if horizontal {
        Rect::from_min_size(Pos2::new(rect.min.x + start, rect.min.y), Vec2::new(len, rect.height()))
    } else {
        Rect::from_min_size(Pos2::new(rect.min.x, rect.min.y + start), Vec2::new(rect.width(), len))
    };
    let over_thumb = pointer.is_some_and(|p| thumb_rect.contains(p));
    let style = if grab.is_some() {
        &v.widgets.active
    } else if over_thumb {
        &v.widgets.hovered
    } else {
        &v.widgets.inactive
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, v.extreme_bg_color);
    painter.rect_filled(thumb_rect.shrink(1.0), 2.0, style.bg_fill);
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
    fn wheel_moves_against_the_delta_and_carries_fractions() {
        let mut carry = (0.0, 0.0);
        // Wheel up (positive y) scrolls the view up, at 1.5 pixels per point.
        assert_eq!(wheel_step(&mut carry, Vec2::new(0.0, 40.0), 1.5), (0, -60));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (0, 0));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (0, 0));
        assert_eq!(wheel_step(&mut carry, Vec2::new(-0.4, 0.0), 1.0), (1, 0));
    }
}
