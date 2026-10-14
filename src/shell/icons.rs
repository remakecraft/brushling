//! Original pixel-art tool icons, drawn for this project as 16x16 character
//! grids (one character per pixel, see [`color_of`]) and uploaded once as
//! egui textures.

use brushling::Tool;
use eframe::egui::{self, Color32};

/// Icon side in pixels.
pub const SIZE: usize = 16;

type Grid = [&'static str; SIZE];

const FREE_FORM_SELECT: Grid = [
    "................",
    ".....kk.k.......",
    "...k.....k......",
    "..k........kk...",
    "................",
    ".k............k.",
    "..k...........k.",
    "................",
    "...k........k...",
    "..k........k....",
    "................",
    ".k..........k...",
    "..k..........k..",
    "....kk.kk.kk....",
    "................",
    "................",
];

const SELECT: Grid = [
    "................",
    "................",
    ".kk.kk.kk.kk.kk.",
    ".k............k.",
    "................",
    ".k............k.",
    ".k............k.",
    "................",
    ".k............k.",
    ".k............k.",
    "................",
    ".k............k.",
    ".k............k.",
    ".kk.kk.kk.kk.kk.",
    "................",
    "................",
];

const ERASER: Grid = [
    "................",
    "................",
    "................",
    "................",
    "......kkkkkkkkk.",
    ".....kwwwwwwwkk.",
    "....kwwwwwwwkpk.",
    "...kwwwwwwwkppk.",
    "..kkkkkkkkkppk..",
    "..kpppppppkpk...",
    "..kpppppppkk....",
    "..kkkkkkkkk.....",
    "................",
    "................",
    "................",
    "................",
];

const FILL: Grid = [
    "................",
    "......kkk.......",
    ".....k...k......",
    "....k.....k.....",
    "....k..kkkkk....",
    "...kkkkwwwwwk...",
    "..kwwwwwwwwwkr..",
    ".kwwwwwwwwwkkrr.",
    "kwwwwwwwwwwk.rr.",
    ".kwwwwwwwwk..rr.",
    "..kwwwwwwk...rr.",
    "...kwwwwk....rr.",
    "....kwwk......r.",
    ".....kk.........",
    "................",
    "................",
];

const PICK_COLOR: Grid = [
    "................",
    "...........kkk..",
    "..........kkkkk.",
    "..........kkkkk.",
    ".........kkkkkk.",
    "........kwkkkk..",
    ".......kwwwk....",
    "......kwwwk.....",
    ".....kwwwk......",
    "....kwwwk.......",
    "...kwwwk........",
    "..kwwwk.........",
    "..kwwk..........",
    ".k.kk...........",
    "k...............",
    "................",
];

const MAGNIFIER: Grid = [
    "................",
    "...kkkkk........",
    "..k.....k.......",
    ".k.ww....k......",
    ".k.w.....k......",
    ".k.......k......",
    ".k.......k......",
    ".k.......k......",
    "..k.....kk......",
    "...kkkkkkkk.....",
    ".........kkk....",
    "..........kkk...",
    "...........kkk..",
    "............kk..",
    "................",
    "................",
];

const PENCIL: Grid = [
    "................",
    "............kk..",
    "...........kppk.",
    "..........kpppk.",
    ".........kyyyk..",
    "........kyyyk...",
    ".......kyyyk....",
    "......kyyyk.....",
    ".....kyyyk......",
    "....kyyyk.......",
    "...kwwwk........",
    "..kwwk..........",
    "..kk............",
    "................",
    "................",
    "................",
];

const BRUSH: Grid = [
    "................",
    ".............kk.",
    "............kRk.",
    "...........kRk..",
    "..........kRk...",
    ".........kRk....",
    "........kRk.....",
    ".......kgk......",
    "......kgk.......",
    "....kkkk........",
    "...kkkkk........",
    "..kkkkk.........",
    "..kkkk..........",
    ".kkk............",
    "k...............",
    "................",
];

const AIRBRUSH: Grid = [
    "................",
    ".k..............",
    "k..k............",
    "..k..k..kkk.....",
    ".k..k..kkgk.....",
    "..k..k.kkkkk....",
    "k..k...kwbbk....",
    ".k.....kwbbk....",
    ".......kwbbk....",
    ".......kwbbk....",
    ".......kwbbk....",
    ".......kwbbk....",
    ".......kwbbk....",
    ".......kkkkk....",
    "................",
    "................",
];

const TEXT: Grid = [
    "................",
    "................",
    "................",
    ".......kk.......",
    ".......kk.......",
    "......kkkk......",
    "......k.kk......",
    ".....kk..kk.....",
    ".....k...kk.....",
    "....kkkkkkkk....",
    "....k.....kk....",
    "...kk......kk...",
    "...k.......kk...",
    ".kkkk.....kkkk..",
    "................",
    "................",
];

const LINE: Grid = [
    "................",
    ".............k..",
    "............k...",
    "...........k....",
    "..........k.....",
    ".........k......",
    "........k.......",
    ".......k........",
    "......k.........",
    ".....k..........",
    "....k...........",
    "...k............",
    "..k.............",
    ".k..............",
    "................",
    "................",
];

const CURVE: Grid = [
    "................",
    "................",
    "................",
    "....kk..........",
    "...k..k.........",
    "..k....k........",
    "..k....k........",
    ".k.....k......k.",
    ".k......k.....k.",
    "........k....k..",
    ".........k..k...",
    "..........kk....",
    "................",
    "................",
    "................",
    "................",
];

const RECTANGLE: Grid = [
    "................",
    "................",
    "................",
    ".kkkkkkkkkkkkkk.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".kkkkkkkkkkkkkk.",
    "................",
    "................",
    "................",
    "................",
];

const POLYGON: Grid = [
    "................",
    "................",
    "....kkkkk.......",
    "....k...k.......",
    "....k....k......",
    "...k......k.....",
    "...k.......kkkk.",
    "..k...........k.",
    "..k...........k.",
    ".k...........k..",
    ".k..........k...",
    ".kkkkkkkkkkkk...",
    "................",
    "................",
    "................",
    "................",
];

const ELLIPSE: Grid = [
    "................",
    "................",
    "................",
    ".....kkkkkk.....",
    "...kk......kk...",
    "..k..........k..",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    "..k..........k..",
    "...kk......kk...",
    ".....kkkkkk.....",
    "................",
    "................",
    "................",
    "................",
];

const ROUNDED_RECTANGLE: Grid = [
    "................",
    "................",
    "................",
    "...kkkkkkkkkk...",
    "..k..........k..",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    ".k............k.",
    "..k..........k..",
    "...kkkkkkkkkk...",
    "................",
    "................",
    "................",
    "................",
];

pub fn grid(tool: Tool) -> &'static Grid {
    match tool {
        Tool::FreeFormSelect => &FREE_FORM_SELECT,
        Tool::Select => &SELECT,
        Tool::Eraser => &ERASER,
        Tool::Fill => &FILL,
        Tool::PickColor => &PICK_COLOR,
        Tool::Magnifier => &MAGNIFIER,
        Tool::Pencil => &PENCIL,
        Tool::Brush => &BRUSH,
        Tool::Airbrush => &AIRBRUSH,
        Tool::Text => &TEXT,
        Tool::Line => &LINE,
        Tool::Curve => &CURVE,
        Tool::Rectangle => &RECTANGLE,
        Tool::Polygon => &POLYGON,
        Tool::Ellipse => &ELLIPSE,
        Tool::RoundedRectangle => &ROUNDED_RECTANGLE,
    }
}

/// The icon palette; `None` for anything that is not a palette character.
pub fn color_of(c: char) -> Option<Color32> {
    Some(match c {
        '.' => Color32::TRANSPARENT,
        'k' => Color32::BLACK,
        'g' => Color32::from_gray(128),
        'w' => Color32::WHITE,
        'r' => Color32::from_rgb(255, 0, 0),
        'R' => Color32::from_rgb(128, 0, 0),
        'y' => Color32::from_rgb(255, 255, 0),
        'b' => Color32::from_rgb(0, 0, 255),
        'p' => Color32::from_rgb(255, 150, 200),
        _ => return None,
    })
}

/// The icon's pixels, row by row. Unknown characters (caught by the tests)
/// show as transparent.
pub fn pixels(tool: Tool) -> Vec<Color32> {
    grid(tool).iter().flat_map(|row| row.chars()).map(|c| color_of(c).unwrap_or(Color32::TRANSPARENT)).collect()
}

/// The tool's icon texture, uploaded on first use and kept in egui's memory.
pub fn texture(ctx: &egui::Context, tool: Tool) -> egui::TextureId {
    let id = egui::Id::new(("brushling-tool-icon", tool));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t.id();
    }
    let img = egui::ColorImage::new([SIZE, SIZE], pixels(tool));
    let t = ctx.load_texture(format!("brushling-icon-{tool:?}"), img, egui::TextureOptions::NEAREST);
    let tid = t.id();
    ctx.data_mut(|d| d.insert_temp(id, t));
    tid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_square_icon_in_the_palette() {
        for tool in Tool::ALL {
            let g = grid(tool);
            for (y, row) in g.iter().enumerate() {
                assert_eq!(row.chars().count(), SIZE, "{tool:?} row {y}");
                for c in row.chars() {
                    assert!(color_of(c).is_some(), "{tool:?} row {y} has {c:?}");
                }
            }
            assert_eq!(pixels(tool).len(), SIZE * SIZE);
        }
    }

    #[test]
    fn icons_are_distinct_and_not_blank() {
        let all: Vec<Vec<Color32>> = Tool::ALL.iter().map(|t| pixels(*t)).collect();
        for (i, a) in all.iter().enumerate() {
            let inked = a.iter().filter(|c| c.a() > 0).count();
            assert!(inked >= 12, "{:?} has only {inked} pixels", Tool::ALL[i]);
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn icons_keep_a_transparent_border_pixel() {
        // Some breathing room inside the button: no icon fills the whole grid.
        for tool in Tool::ALL {
            let p = pixels(tool);
            assert!(p.iter().any(|c| c.a() == 0), "{tool:?}");
            assert!(p[..SIZE].iter().all(|c| c.a() == 0), "{tool:?} top row is inked");
        }
    }
}
