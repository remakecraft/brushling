//! Feature selection.image_ops_scope: Flip/Rotate, Stretch/Skew and Invert
//! Colors act on the selection when one exists and on the whole image
//! otherwise. This module owns the shared scoping helpers; the commands
//! themselves live in their own feature files and call into it:
//!
//! * [`with_target_pixels`] for commands that produce a new raster (flip,
//!   rotate, stretch, skew). Whole image: the canvas takes the result size.
//!   Selection: the selection is lifted (fresh source filled with the
//!   background colour), its content and mask are replaced by the result, the
//!   bounds keep their top-left and take the result size, the canvas size is
//!   unchanged, and the selection stays floating.
//! * [`map_target_pixels`] for per-pixel recolouring (Invert Colors). Whole
//!   image: every pixel. Selection: only masked pixels, in place (floating
//!   pixels if lifted, image pixels otherwise); the selection does not float.
//!
//! Both push exactly one undo step. A command that does nothing (e.g.
//! Stretch/Skew 100/100/0/0) must not call them.

use crate::doc::check_size;
use crate::state::State;
use crate::types::*;

/// A rectangular block of pixels with a selection mask, row-major.
#[derive(Debug, Clone)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgb>,
    /// Which pixels belong to the selection (all true for the whole image).
    pub mask: Vec<bool>,
}

impl Raster {
    fn at(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// A new `nw` x `nh` raster where destination `(x, y)` takes the pixel
    /// and mask bit of the source coordinate `f(x, y)`. `None` marks an area
    /// the transform exposes: it becomes `bg` and counts as selected, so the
    /// opaque / transparent rule decides whether it shows.
    pub fn remap(&self, nw: u32, nh: u32, bg: Rgb, f: impl Fn(u32, u32) -> Option<(u32, u32)>) -> Raster {
        let mut pixels = Vec::with_capacity(nw as usize * nh as usize);
        let mut mask = Vec::with_capacity(pixels.capacity());
        for y in 0..nh {
            for x in 0..nw {
                match f(x, y) {
                    Some((sx, sy)) => {
                        let i = self.at(sx, sy);
                        pixels.push(self.pixels[i]);
                        mask.push(self.mask[i]);
                    }
                    None => {
                        pixels.push(bg);
                        mask.push(true);
                    }
                }
            }
        }
        Raster { width: nw, height: nh, pixels, mask }
    }

    /// Nearest-neighbour resample to `nw` x `nh` (both at least 1) with the
    /// stretch rule of image.stretch_skew: source = floor((x' + 0.5) * W / W').
    pub fn stretched(&self, nw: u32, nh: u32) -> Raster {
        let (nw, nh) = (nw.max(1), nh.max(1));
        let src = |d: u32, n: u32, s: u32| ((2 * d as u64 + 1) * s as u64 / (2 * n as u64)) as u32;
        self.remap(nw, nh, Rgb::W, |x, y| Some((src(x, nw, self.width), src(y, nh, self.height))))
    }
}

/// The raster a command works on: the selection's content (pixels outside
/// its mask read as `bg`) or the whole image.
fn target_raster(st: &State) -> Raster {
    let bg = st.bg_snapped();
    match st.sel.as_ref() {
        None => Raster {
            width: st.doc.width,
            height: st.doc.height,
            pixels: st.doc.pixels.clone(),
            mask: vec![true; st.doc.pixels.len()],
        },
        Some(sel) => {
            let b = sel.bounds;
            let mut pixels = Vec::with_capacity(sel.mask.len());
            for y in b.y0..=b.y1 {
                for x in b.x0..=b.x1 {
                    let i = (y - b.y0) as usize * b.width() as usize + (x - b.x0) as usize;
                    let c = match &sel.floating {
                        Some(f) => f[i],
                        None => st.doc.get(x, y).unwrap_or(bg),
                    };
                    pixels.push(if sel.mask[i] { c } else { bg });
                }
            }
            Raster { width: b.width() as u32, height: b.height() as u32, pixels, mask: sel.mask.clone() }
        }
    }
}

/// Run `f(current content, background colour)` on the selection (or the whole
/// image) and install its result as described in the module docs. `f` may
/// fail (e.g. result too large); nothing changes then. The result must have
/// `width * height` pixels and mask bits and a size within the document limits.
pub fn with_target_pixels(st: &mut State, f: impl FnOnce(Raster, Rgb) -> HResult<Raster>) -> HResult<()> {
    let out = f(target_raster(st), st.bg_snapped())?;
    check_size(out.width, out.height)?;
    let n = out.width as usize * out.height as usize;
    if out.pixels.len() != n || out.mask.len() != n {
        return Err(HarnessError::Invalid("transformed raster does not match its size".into()));
    }
    st.push_undo_sel();
    if st.sel.is_none() {
        st.doc.width = out.width;
        st.doc.height = out.height;
        st.doc.pixels = out.pixels;
        crate::features::ui_main_window::refresh_workspace(st);
        return Ok(());
    }
    st.lift_selection(true);
    if let Some(sel) = st.sel.as_mut() {
        let b = sel.bounds;
        sel.bounds = Rect::new(b.x0, b.y0, b.x0 + out.width as i32 - 1, b.y0 + out.height as i32 - 1);
        // Pixels that were outside a free-form mask are background by now, so
        // the transformed block is whole: the mask no longer excludes anything.
        sel.mask = vec![true; n];
        sel.floating = Some(out.pixels);
    }
    Ok(())
}

/// Recolour every pixel of the target (whole image, or the masked pixels of
/// the selection in place) with `f`.
pub fn map_target_pixels(st: &mut State, mut f: impl FnMut(Rgb) -> Rgb) {
    st.push_undo_sel();
    // Recoloured pixels are painted: they snap like any other painted colour.
    let (mode, palette) = (st.doc.color_mode, st.doc.palette.clone());
    let mut f = |c: Rgb| crate::doc::snap_colour(mode, palette.as_deref(), f(c));
    let Some(sel) = st.sel.as_mut() else {
        st.doc.pixels.iter_mut().for_each(|p| *p = f(*p));
        return;
    };
    let b = sel.bounds;
    let w = b.width() as usize;
    if let Some(fl) = sel.floating.as_mut() {
        for (p, m) in fl.iter_mut().zip(&sel.mask) {
            if *m {
                *p = f(*p);
            }
        }
        return;
    }
    for y in b.y0..=b.y1 {
        for x in b.x0..=b.x1 {
            if sel.mask[(y - b.y0) as usize * w + (x - b.x0) as usize] {
                if let Some(c) = st.doc.get(x, y) {
                    st.doc.put(x, y, f(c));
                }
            }
        }
    }
}

pub fn register(_r: &mut crate::registry::Registry) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-scope-{}", std::process::id()));
        State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap()
    }

    /// A 2x2 free-form selection that leaves the bottom-right pixel out.
    fn select_l_shape(st: &mut State) {
        st.doc = crate::doc::Document::new(4, 4, Rgb::W).unwrap();
        st.bg = Rgb::new(10, 20, 30);
        for (x, y, c) in [(0, 0, Rgb::R), (1, 0, Rgb::G), (0, 1, Rgb::B), (1, 1, Rgb::K)] {
            st.doc.put(x, y, c);
        }
        let b = Rect::new(0, 0, 1, 1);
        st.sel = Some(crate::selection::Sel { bounds: b, mask: vec![true, true, true, false], floating: None, origin: b });
    }

    #[test]
    fn transformed_free_form_selection_is_whole_with_background_outside_the_mask() {
        let mut st = state();
        select_l_shape(&mut st);
        with_target_pixels(&mut st, |r, bg| Ok(r.remap(2, 2, bg, |x, y| Some((1 - x, y))))).unwrap();
        let sel = st.sel.as_ref().unwrap();
        assert_eq!(sel.mask, vec![true; 4]);
        // The excluded pixel (1,1) mirrors to (0,1) and reads as background.
        assert_eq!(sel.floating.as_deref(), Some(&[Rgb::G, Rgb::R, st.bg, Rgb::B][..]));
    }

    #[test]
    fn background_fill_follows_the_snapping_rule() {
        let mut st = state();
        select_l_shape(&mut st);
        st.doc.color_mode = ColorMode::BlackAndWhite;
        st.bg = Rgb::new(200, 0, 0);
        let snapped = st.bg_snapped();
        assert_eq!(snapped, Rgb::K);
        with_target_pixels(&mut st, |r, bg| Ok(r.remap(3, 3, bg, |_, _| None))).unwrap();
        let floating = st.sel.as_ref().and_then(|s| s.floating.clone()).unwrap();
        assert!(floating.iter().all(|c| *c == snapped));
        // Lifting cleared the source with the snapped colour as well.
        assert_eq!(st.doc.get(0, 0), Some(snapped));
    }
}
