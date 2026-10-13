//! The shared selection model. One selection exists at a time
//! (`State::sel`). Features built on it (move/copy, transparency, resize,
//! image ops scope, clipboard) use the helpers below instead of touching
//! the fields ad hoc; see the "Selection model" section of `src/README.md`.

use crate::raster::in_bounds;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

/// The active selection.
#[derive(Debug, Clone)]
pub struct Sel {
    /// Inclusive bounds in image coordinates (may extend past the canvas
    /// once a floating selection has been moved).
    pub bounds: Rect,
    /// Row-major over `bounds`; true = selected. Free-form shapes use this.
    pub mask: Vec<bool>,
    /// `Some` once the content is lifted: pixels row-major over `bounds`
    /// (same indexing as `mask`) floating above the image. `None` while the
    /// selection only marks pixels that still live in the image.
    pub floating: Option<Vec<Rgb>>,
    /// Bounds the selection was created or lifted at (move/copy and
    /// resize builders use it as the source rectangle).
    pub origin: Rect,
}

/// A drag on the selection itself (not a new Select drag).
#[derive(Debug, Clone)]
pub enum SelGesture {
    /// Moving the body (selection.move_copy). `started` flips at the first
    /// move event, which records the undo step and lifts a fresh selection.
    Move { last: (i32, i32), ctrl: bool, started: bool },
    /// Dragging a handle (selection.resize). `base` is the content as it was
    /// when the drag began, captured at the first move event.
    Resize { handle: Handle, start: Rect, base: Option<crate::features::selection_image_ops_scope::Raster> },
}

/// An in-progress Select / Free-Form Select drag.
#[derive(Debug, Clone)]
pub struct SelDrag {
    pub freeform: bool,
    pub start: (i32, i32),
    pub cur: (i32, i32),
    /// Whether any move event arrived (a bare click selects nothing).
    pub moved: bool,
    /// Traced pixels for free-form (clamped, consecutive points connected).
    pub path: Vec<(i32, i32)>,
}

impl Sel {
    pub fn new(bounds: Rect, mask: Vec<bool>) -> Sel {
        Sel { bounds, mask, floating: None, origin: bounds }
    }

    pub fn rect(bounds: Rect) -> Sel {
        let n = (bounds.width() as usize) * (bounds.height() as usize);
        Sel::new(bounds, vec![true; n])
    }

    /// Index into `mask` / `floating` of image pixel (x,y), if inside bounds.
    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        let b = self.bounds;
        (x >= b.x0 && x <= b.x1 && y >= b.y0 && y <= b.y1)
            .then(|| (y - b.y0) as usize * b.width() as usize + (x - b.x0) as usize)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some_and(|i| self.mask[i])
    }

    pub fn is_floating(&self) -> bool {
        self.floating.is_some()
    }

    /// The public view returned by `App::selection`.
    pub fn public(&self) -> Selection {
        Selection { bounds: self.bounds, mask: self.mask.clone(), floating: self.is_floating() }
    }
}

impl State {
    pub fn has_selection(&self) -> bool {
        self.sel.is_some()
    }

    /// Replace the selection by an unlifted one, ending any previous
    /// selection first (floating pixels are stamped where they lie).
    pub fn set_selection(&mut self, sel: Sel) {
        self.end_selection();
        self.sel = Some(sel);
    }

    /// Lift the selected pixels off the image so they float. With
    /// `clear_source` the vacated pixels become the background colour.
    /// Does not push an undo step: the caller pushes one first. No-op when
    /// already floating or when nothing is selected.
    pub fn lift_selection(&mut self, clear_source: bool) {
        let bg = self.bg;
        let Some(sel) = self.sel.as_mut() else { return };
        if sel.floating.is_some() {
            return;
        }
        let b = sel.bounds;
        let w = b.width() as usize;
        let mut px = Vec::with_capacity(sel.mask.len());
        for y in b.y0..=b.y1 {
            for x in b.x0..=b.x1 {
                // Pixels outside the canvas lift as background.
                px.push(self.doc.get(x, y).unwrap_or(bg));
            }
        }
        if clear_source {
            for y in b.y0..=b.y1 {
                for x in b.x0..=b.x1 {
                    if sel.mask[(y - b.y0) as usize * w + (x - b.x0) as usize] {
                        self.doc.put(x, y, bg);
                    }
                }
            }
        }
        sel.origin = b;
        sel.floating = Some(px);
    }

    /// Whether a floating pixel of colour `c` is drawn: always in opaque
    /// mode, but background-coloured pixels are skipped in transparent mode
    /// (selection.transparency). Text frames use the same rule.
    pub fn draws_floating(&self, c: Rgb) -> bool {
        self.options.opaque || c != self.bg
    }

    /// Write the floating pixels of the masked area into the image where they
    /// lie (clipped to the canvas). Leaves the selection itself untouched;
    /// "stamp a copy" (Ctrl-drag) calls this and keeps floating.
    pub fn stamp_selection(&mut self) {
        let Some(sel) = self.sel.as_ref() else { return };
        let Some(px) = sel.floating.as_ref() else { return };
        let (opaque, bg) = (self.options.opaque, self.bg);
        let b = sel.bounds;
        let w = b.width() as usize;
        for y in b.y0..=b.y1 {
            for x in b.x0..=b.x1 {
                let i = (y - b.y0) as usize * w + (x - b.x0) as usize;
                if sel.mask[i] && (opaque || px[i] != bg) {
                    self.doc.put(x, y, px[i]);
                }
            }
        }
    }

    /// End the selection: floating pixels merge into the image where they
    /// lie, then the selection is gone. Pushes no undo step.
    pub fn end_selection(&mut self) {
        self.stamp_selection();
        self.sel = None;
    }

    /// Drop the selection, discarding floating content.
    pub fn discard_selection(&mut self) {
        self.sel = None;
    }

    /// Clear the pixels of an unlifted selection to the background colour
    /// (floating content is simply dropped). Pushes no undo step.
    pub fn erase_selection(&mut self) {
        let bg = self.bg;
        let Some(sel) = self.sel.take() else { return };
        if sel.floating.is_some() {
            return;
        }
        let b = sel.bounds;
        let w = b.width() as usize;
        for y in b.y0..=b.y1 {
            for x in b.x0..=b.x1 {
                if sel.mask[(y - b.y0) as usize * w + (x - b.x0) as usize] {
                    self.doc.put(x, y, bg);
                }
            }
        }
    }

    /// Drop all selection state (new document).
    pub fn reset_selection(&mut self) {
        self.sel = None;
        self.sel_drag = None;
        self.sel_gesture = None;
    }
}

// ----- render layers

const MARQUEE: [Rgb; 2] = [Rgb::K, Rgb::W];

fn set(img: &mut Image, x: i64, y: i64, c: Rgb) {
    if x >= 0 && y >= 0 && x < img.width as i64 && y < img.height as i64 {
        img.pixels[y as usize * img.width as usize + x as usize] = c;
    }
}

/// Composite layer: floating pixels over the image.
pub fn composite_layer(st: &State, img: &mut Image) {
    let Some(sel) = st.sel.as_ref() else { return };
    let Some(px) = sel.floating.as_ref() else { return };
    let b = sel.bounds;
    let w = b.width() as usize;
    for y in b.y0..=b.y1 {
        for x in b.x0..=b.x1 {
            let i = (y - b.y0) as usize * w + (x - b.x0) as usize;
            if sel.mask[i] && st.draws_floating(px[i]) && in_bounds(x, y, img.width, img.height) {
                set(img, x as i64, y as i64, px[i]);
            }
        }
    }
}

/// Dashed rectangle with the given screen-space corners (inclusive).
fn dashed_rect(img: &mut Image, x0: i64, y0: i64, x1: i64, y1: i64) {
    let c = |n: i64| MARQUEE[(n.rem_euclid(6) / 3) as usize];
    for x in x0..=x1 {
        set(img, x, y0, c(x - x0));
        set(img, x, y1, c(x - x0));
    }
    for y in y0..=y1 {
        set(img, x0, y, c(y - y0));
        set(img, x1, y, c(y - y0));
    }
}

/// Screen-space box (inclusive corners) of an image rectangle in the
/// workspace image, drawn one pixel outside the covered pixels.
pub fn screen_box(st: &State, r: Rect) -> (i64, i64, i64, i64) {
    let z = st.scale() as i64;
    let ox = crate::render::CANVAS_MARGIN as i64 - st.scroll.0 as i64;
    let oy = crate::render::CANVAS_MARGIN as i64 - st.scroll.1 as i64;
    (ox + r.x0 as i64 * z - 1, oy + r.y0 as i64 * z - 1, ox + (r.x1 as i64 + 1) * z, oy + (r.y1 as i64 + 1) * z)
}

/// The eight handle centres (screen space) of a screen box, in the order
/// TopLeft, Top, TopRight, Left, Right, BottomLeft, Bottom, BottomRight.
pub fn handle_centres(b: (i64, i64, i64, i64)) -> [(i64, i64); 8] {
    let (x0, y0, x1, y1) = b;
    let (mx, my) = ((x0 + x1) / 2, (y0 + y1) / 2);
    [(x0, y0), (mx, y0), (x1, y0), (x0, my), (x1, my), (x0, y1), (mx, y1), (x1, y1)]
}

/// Workspace layer: in-progress drag preview, dashed box and 5x5 handles.
pub fn workspace_layer(st: &State, img: &mut Image) {
    if let Some(d) = st.sel_drag.as_ref().filter(|d| d.moved) {
        if d.freeform {
            for &(x, y) in &d.path {
                let (a, b, _, _) = screen_box(st, Rect::new(x, y, x, y));
                set(img, a + 1, b + 1, Rgb::K);
            }
        } else {
            let r = crate::raster::rect_from_corners(d.start, d.cur);
            let (a, b, c, e) = screen_box(st, r);
            dashed_rect(img, a, b, c, e);
        }
    }
    if let Some(sel) = st.sel.as_ref() {
        let b = screen_box(st, sel.bounds);
        dashed_rect(img, b.0, b.1, b.2, b.3);
        for (cx, cy) in handle_centres(b) {
            for dy in -2..=2i64 {
                for dx in -2..=2i64 {
                    let edge = dx.abs() == 2 || dy.abs() == 2;
                    set(img, cx + dx, cy + dy, if edge { Rgb::K } else { Rgb::W });
                }
            }
        }
    }
}

pub fn register(r: &mut Registry) {
    r.composite_layer(composite_layer);
    r.workspace_layer(workspace_layer);
}
