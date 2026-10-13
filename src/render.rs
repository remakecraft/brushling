//! Rendering to pixels. `Composite` is the document as displayed 1:1;
//! `Workspace` places the zoomed composite on the grey workspace background.
//! Features draw extra content through registry layers.

use crate::registry::registry;
use crate::state::State;
use crate::types::*;

/// Workspace background grey.
pub const WORKSPACE_GREY: Rgb = Rgb::new(128, 128, 128);
/// The canvas sits this many pixels from the workspace's top-left corner.
pub const CANVAS_MARGIN: i32 = 4;

impl State {
    pub fn render(&self, surface: Surface) -> HResult<Image> {
        match surface {
            Surface::Composite => Ok(self.render_composite()),
            Surface::Workspace => Ok(self.render_workspace()),
            Surface::Thumbnail => crate::features::view_thumbnail::render(self),
            Surface::FullScreen => crate::features::view_view_bitmap::render(self),
        }
    }

    fn render_composite(&self) -> Image {
        let mut img = self.doc.image();
        for layer in registry().composite_layers() {
            layer(self, &mut img);
        }
        img
    }

    fn render_workspace(&self) -> Image {
        if let Some(preview) = crate::features::file_print::render_preview(self) {
            return preview;
        }
        let comp = self.render_composite();
        let (ww, wh) = self.workspace_size;
        let mut out = Image { width: ww, height: wh, pixels: vec![WORKSPACE_GREY; ww as usize * wh as usize] };
        let z = self.scale() as i64;
        let (ox, oy) = (CANVAS_MARGIN as i64 - self.scroll.0 as i64, CANVAS_MARGIN as i64 - self.scroll.1 as i64);
        // Only visit the visible part of the zoomed canvas.
        let x_lo = (-ox).max(0);
        let y_lo = (-oy).max(0);
        let x_hi = (ww as i64 - ox).min(comp.width as i64 * z);
        let y_hi = (wh as i64 - oy).min(comp.height as i64 * z);
        for sy in y_lo..y_hi {
            for sx in x_lo..x_hi {
                let c = comp.pixels[(sy / z) as usize * comp.width as usize + (sx / z) as usize];
                out.pixels[(oy + sy) as usize * ww as usize + (ox + sx) as usize] = c;
            }
        }
        for layer in registry().workspace_layers() {
            layer(self, &mut out);
        }
        out
    }
}
