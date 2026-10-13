//! The document model: one RGB raster plus the metadata the spec records
//! (path, source bit depth, palette, colour mode, dirty flag) and the undo
//! snapshot stack features push onto before they change pixels.

use crate::raster::in_bounds;
use crate::types::*;
use std::path::PathBuf;

/// Number of most recent steps the undo history keeps.
pub const UNDO_DEPTH: usize = 3;
pub const MAX_DIM: u32 = 9999;
/// Size of the first new image when no size has ever been confirmed.
pub const DEFAULT_SIZE: (u32, u32) = (512, 384);

/// Validate a canvas dimension pair against the 1..=9999 range.
pub fn check_size(w: u32, h: u32) -> HResult<()> {
    if (1..=MAX_DIM).contains(&w) && (1..=MAX_DIM).contains(&h) {
        Ok(())
    } else {
        Err(HarnessError::Invalid(format!("size {w}x{h} outside 1..={MAX_DIM}")))
    }
}

/// Everything an undo step has to restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgb>,
    pub palette: Option<Vec<Rgb>>,
    pub color_mode: ColorMode,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    /// Row-major, `width * height` entries.
    pub pixels: Vec<Rgb>,
    pub path: Option<PathBuf>,
    /// 1, 4, 8 or 24.
    pub source_depth: u32,
    pub palette: Option<Vec<Rgb>>,
    pub color_mode: ColorMode,
    pub dirty: bool,
    /// Undo history, oldest first. Features push through
    /// [`Document::push_undo`] before changing pixels.
    pub undo: Vec<Snapshot>,
    /// Steps undone and still repeatable, most recently undone last.
    pub redo: Vec<Snapshot>,
}

impl Document {
    /// A clean untitled 24-bit image filled with `fill`.
    pub fn new(w: u32, h: u32, fill: Rgb) -> HResult<Document> {
        check_size(w, h)?;
        Ok(Document {
            width: w,
            height: h,
            pixels: vec![fill; w as usize * h as usize],
            path: None,
            source_depth: 24,
            palette: None,
            color_mode: ColorMode::Colors,
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// A clean document holding an already decoded image.
    pub fn from_image(img: Image, path: Option<PathBuf>, depth: u32, palette: Option<Vec<Rgb>>) -> HResult<Document> {
        check_size(img.width, img.height)?;
        if img.pixels.len() != img.width as usize * img.height as usize {
            return Err(HarnessError::Invalid("pixel buffer does not match its size".into()));
        }
        Ok(Document {
            width: img.width,
            height: img.height,
            pixels: img.pixels,
            path,
            source_depth: depth,
            palette,
            color_mode: ColorMode::Colors,
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        in_bounds(x, y, self.width, self.height).then(|| y as usize * self.width as usize + x as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Place one pixel; points outside the canvas are discarded. Returns
    /// whether the point was inside.
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.pixels[i] = c;
                true
            }
            None => false,
        }
    }

    pub fn image(&self) -> Image {
        Image { width: self.width, height: self.height, pixels: self.pixels.clone() }
    }

    pub fn info(&self) -> DocInfo {
        DocInfo {
            width: self.width,
            height: self.height,
            path: self.path.clone(),
            source_depth: self.source_depth,
            palette: self.palette.clone(),
            color_mode: self.color_mode,
            dirty: self.dirty,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
            palette: self.palette.clone(),
            color_mode: self.color_mode,
        }
    }

    /// The undo hook: record the current state as one undo step and mark the
    /// document dirty. Call once per user-visible edit, before changing it.
    pub fn push_undo(&mut self) {
        let s = self.snapshot();
        self.undo.push(s);
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    /// Replace the picture state with `s`, returning the state it replaced.
    pub fn restore(&mut self, s: Snapshot) -> Snapshot {
        let prev = self.snapshot();
        self.width = s.width;
        self.height = s.height;
        self.pixels = s.pixels;
        self.palette = s.palette;
        self.color_mode = s.color_mode;
        self.dirty = true;
        prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_clips() {
        let mut d = Document::new(4, 3, Rgb::W).unwrap();
        assert!(d.put(3, 2, Rgb::K));
        assert!(!d.put(4, 0, Rgb::K));
        assert!(!d.put(-1, 0, Rgb::K));
        assert_eq!(d.get(3, 2), Some(Rgb::K));
        assert_eq!(d.pixels.iter().filter(|p| **p == Rgb::K).count(), 1);
    }

    #[test]
    fn size_limits() {
        assert!(Document::new(0, 5, Rgb::W).is_err());
        assert!(Document::new(10000, 5, Rgb::W).is_err());
        assert!(Document::new(9999, 1, Rgb::W).is_ok());
    }
}
