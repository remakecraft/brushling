//! Feature image.invert: Image > Invert Colors (Ctrl+I) replaces each pixel by
//! its RGB complement, snapped to the palette in indexed / black-and-white
//! images. See `src/README.md`.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("Image > Invert Colors", invert);
    r.shortcut(Key::Char('i'), Mods::CTRL, "Image > Invert Colors");
}

fn complement(c: Rgb) -> Rgb {
    Rgb::new(255 - c.r, 255 - c.g, 255 - c.b)
}

fn dist2(a: Rgb, b: Rgb) -> i32 {
    let d = |x: u8, y: u8| (x as i32 - y as i32).pow(2);
    d(a.r, b.r) + d(a.g, b.g) + d(a.b, b.b)
}

/// Nearest entry of `palette` to `c`; `c` itself when the palette is empty.
fn snap(c: Rgb, palette: &[Rgb]) -> Rgb {
    palette.iter().copied().min_by_key(|p| dist2(*p, c)).unwrap_or(c)
}

fn invert(st: &mut State) -> HResult<()> {
    st.doc.push_undo();
    let palette: Option<Vec<Rgb>> = match (st.doc.color_mode, &st.doc.palette) {
        (ColorMode::BlackAndWhite, _) => Some(vec![Rgb::K, Rgb::W]),
        (_, Some(p)) if !p.is_empty() => Some(p.clone()),
        _ => None,
    };
    for px in st.doc.pixels.iter_mut() {
        let inv = complement(*px);
        *px = match &palette {
            Some(p) => snap(inv, p),
            None => inv,
        };
    }
    Ok(())
}
