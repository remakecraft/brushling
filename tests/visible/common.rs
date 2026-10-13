//! Shared helpers for the Brushling acceptance suites. Golden values are
//! computed from the spec's exact rules (line rule, stamps, footprints, BMP
//! layout); nothing here knows the implementation.
#![allow(dead_code)]

pub use brushling::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub type T = HResult<()>;
pub type Pts = BTreeSet<(i32, i32)>;

pub const K: Rgb = Rgb::K;
pub const W: Rgb = Rgb::W;
pub const RED: Rgb = Rgb::R;
pub const GREEN: Rgb = Rgb::G;
pub const BLUE: Rgb = Rgb::B;
pub const YELLOW: Rgb = Rgb::new(255, 255, 0);
pub const GREY: Rgb = Rgb::new(128, 128, 128);
pub const L: Button = Button::Left;
pub const RB: Button = Button::Right;
pub const NO: Mods = Mods::NONE;

pub fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb::new(r, g, b)
}

pub fn fail<X>(msg: impl Into<String>) -> HResult<X> {
    Err(HarnessError::Invalid(msg.into()))
}

// ---------------------------------------------------------------- temp files

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, empty, unique temp folder.
pub fn tmp(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let p = std::env::temp_dir()
        .join("brushling-tests")
        .join(format!("{}-{}-{}-{}", std::process::id(), nanos, n, tag));
    std::fs::create_dir_all(&p).expect("create temp dir");
    p
}

pub fn write(path: &Path, bytes: &[u8]) -> T {
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn read(path: &Path) -> HResult<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

pub fn ps(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Names of the files in a folder, sorted.
pub fn dir_names(dir: &Path) -> HResult<Vec<String>> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(dir)? {
        v.push(e?.file_name().to_string_lossy().into_owned());
    }
    v.sort();
    Ok(v)
}

// ---------------------------------------------------------------- app setup

pub fn opts() -> StartOptions {
    StartOptions::new(tmp("cfg"))
}

pub fn start() -> HResult<App> {
    App::start(opts())
}

/// The spec's 'fresh state': 32x32 W, fg K, bg W, defaults, 100%.
pub fn fresh() -> HResult<App> {
    fresh_sized(32, 32)
}

pub fn fresh_sized(w: u32, h: u32) -> HResult<App> {
    let mut a = start()?;
    a.new_document(w, h, W)?;
    Ok(a)
}

/// A fresh app whose document is `w` x `h` with the given row-major pixels.
pub fn image(w: u32, h: u32, px: &[Rgb]) -> HResult<App> {
    let mut a = fresh_sized(w, h)?;
    load(&mut a, w, h, px)?;
    Ok(a)
}

pub fn load(a: &mut App, w: u32, h: u32, px: &[Rgb]) -> T {
    a.new_document(w, h, W)?;
    for y in 0..h {
        for x in 0..w {
            a.set_pixel(x as i32, y as i32, px[(y * w + x) as usize])?;
        }
    }
    Ok(())
}

pub fn fill_rect(a: &mut App, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) -> T {
    for y in y0..=y1 {
        for x in x0..=x1 {
            a.set_pixel(x, y, c)?;
        }
    }
    Ok(())
}

pub fn set_bg(a: &mut App, bg: Rgb) -> T {
    let (fg, _) = a.colors()?;
    a.set_colors(fg, bg)
}

pub fn set_fg(a: &mut App, fg: Rgb) -> T {
    let (_, bg) = a.colors()?;
    a.set_colors(fg, bg)
}

// ---------------------------------------------------------------- pointer

pub fn press(a: &mut App, x: i32, y: i32) -> T {
    a.pointer(PointerKind::Press(L), Pos::Image(x, y), NO)
}
pub fn press_with(a: &mut App, x: i32, y: i32, b: Button, m: Mods) -> T {
    a.pointer(PointerKind::Press(b), Pos::Image(x, y), m)
}
pub fn mv(a: &mut App, x: i32, y: i32) -> T {
    a.pointer(PointerKind::Move, Pos::Image(x, y), NO)
}
pub fn mv_with(a: &mut App, x: i32, y: i32, m: Mods) -> T {
    a.pointer(PointerKind::Move, Pos::Image(x, y), m)
}
pub fn release(a: &mut App, x: i32, y: i32) -> T {
    a.pointer(PointerKind::Release(L), Pos::Image(x, y), NO)
}
pub fn release_with(a: &mut App, x: i32, y: i32, b: Button, m: Mods) -> T {
    a.pointer(PointerKind::Release(b), Pos::Image(x, y), m)
}

pub fn click(a: &mut App, x: i32, y: i32) -> T {
    click_with(a, x, y, L, NO)
}
pub fn rclick(a: &mut App, x: i32, y: i32) -> T {
    click_with(a, x, y, RB, NO)
}
pub fn click_with(a: &mut App, x: i32, y: i32, b: Button, m: Mods) -> T {
    press_with(a, x, y, b, m)?;
    release_with(a, x, y, b, m)
}

pub fn drag(a: &mut App, from: (i32, i32), to: (i32, i32)) -> T {
    drag_with(a, from, to, L, NO)
}
pub fn rdrag(a: &mut App, from: (i32, i32), to: (i32, i32)) -> T {
    drag_with(a, from, to, RB, NO)
}
pub fn drag_with(a: &mut App, from: (i32, i32), to: (i32, i32), b: Button, m: Mods) -> T {
    press_with(a, from.0, from.1, b, m)?;
    mv_with(a, to.0, to.1, m)?;
    release_with(a, to.0, to.1, b, m)
}

/// Press at pts[0], one move event per further point, release at the last.
pub fn trace(a: &mut App, pts: &[(i32, i32)]) -> T {
    trace_with(a, pts, L, NO)
}
pub fn trace_with(a: &mut App, pts: &[(i32, i32)], b: Button, m: Mods) -> T {
    let (x0, y0) = pts[0];
    press_with(a, x0, y0, b, m)?;
    for &(x, y) in &pts[1..] {
        mv_with(a, x, y, m)?;
    }
    let (xl, yl) = *pts.last().unwrap();
    release_with(a, xl, yl, b, m)
}

pub fn tool(a: &mut App, t: Tool) -> T {
    a.select_tool(t)
}

pub fn opt(a: &mut App, o: ToolOption) -> T {
    a.set_tool_option(o)
}

/// Select tool + drag.
pub fn select_rect(a: &mut App, from: (i32, i32), to: (i32, i32)) -> T {
    a.select_tool(Tool::Select)?;
    drag(a, from, to)
}

pub fn pencil_click(a: &mut App, x: i32, y: i32) -> T {
    a.select_tool(Tool::Pencil)?;
    click(a, x, y)
}

/// Drag a canvas resize handle so the handle ends at image (x,y).
pub fn drag_canvas_handle(a: &mut App, h: CanvasHandle, x: i32, y: i32) -> T {
    a.pointer(PointerKind::Press(L), Pos::CanvasHandle(h), NO)?;
    a.pointer(PointerKind::Move, Pos::Image(x, y), NO)?;
    a.pointer(PointerKind::Release(L), Pos::Image(x, y), NO)
}

pub fn drag_sel_handle(a: &mut App, h: Handle, x: i32, y: i32) -> T {
    a.pointer(PointerKind::Press(L), Pos::SelectionHandle(h), NO)?;
    a.pointer(PointerKind::Move, Pos::Image(x, y), NO)?;
    a.pointer(PointerKind::Release(L), Pos::Image(x, y), NO)
}

// ---------------------------------------------------------------- keys

pub fn key(a: &mut App, k: Key) -> T {
    a.key(k, NO)
}
pub fn ctrl(a: &mut App, c: char) -> T {
    a.key(Key::Char(c), Mods::CTRL)
}
pub fn esc(a: &mut App) -> T {
    a.key(Key::Escape, NO)
}
pub fn type_text(a: &mut App, s: &str) -> T {
    for c in s.chars() {
        a.key(Key::Char(c), NO)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- menus

pub fn item(a: &App, path: &str) -> HResult<MenuItem> {
    a.menu_item(path)
}
pub fn enabled(a: &App, path: &str) -> HResult<bool> {
    Ok(a.menu_item(path)?.enabled)
}
pub fn checked(a: &App, path: &str) -> HResult<bool> {
    match a.menu_item(path)?.checked {
        Some(c) => Ok(c),
        None => fail(format!("{path} is not a check item")),
    }
}
pub fn undo_enabled(a: &App) -> HResult<bool> {
    enabled(a, "Edit > Undo")
}
pub fn repeat_enabled(a: &App) -> HResult<bool> {
    enabled(a, "Edit > Repeat")
}
pub fn undo(a: &mut App) -> T {
    a.menu("Edit > Undo")
}
pub fn repeat(a: &mut App) -> T {
    a.menu("Edit > Repeat")
}
/// Non-separator labels of a menu.
pub fn labels(a: &App, path: &str) -> HResult<Vec<String>> {
    Ok(a.menu_items(path)?.into_iter().filter(|i| !i.separator).map(|i| i.label).collect())
}
/// Recent-file entries of the File menu ("<n> <path>").
pub fn recent(a: &App) -> HResult<Vec<String>> {
    Ok(labels(a, "File")?
        .into_iter()
        .filter(|l| l.len() > 2 && l.as_bytes()[0].is_ascii_digit() && l.as_bytes()[1] == b' ')
        .collect())
}

// ---------------------------------------------------------------- dialogs

pub fn dlg(a: &App) -> HResult<Dialog> {
    match a.dialog()? {
        Some(d) => Ok(d),
        None => fail("expected an open dialog"),
    }
}
pub fn dlg_kind(a: &App) -> HResult<Option<DialogKind>> {
    Ok(a.dialog()?.map(|d| d.kind))
}
pub fn expect_dialog(a: &App, kind: DialogKind) -> HResult<Dialog> {
    let d = dlg(a)?;
    if d.kind != kind {
        return fail(format!("expected {kind:?} dialog, got {:?}", d.kind));
    }
    Ok(d)
}
pub fn field_f64(d: &Dialog, name: &str) -> HResult<f64> {
    match d.field(name).and_then(|v| v.trim().parse::<f64>().ok()) {
        Some(v) => Ok(v),
        None => fail(format!("field {name} missing or not numeric")),
    }
}

/// File > Open..., pick `path`, press Open.
pub fn open_path(a: &mut App, path: &Path) -> T {
    a.menu("File > Open...")?;
    open_in_dialog(a, path)
}
pub fn open_in_dialog(a: &mut App, path: &Path) -> T {
    expect_dialog(a, DialogKind::FileOpen)?;
    a.dialog_set("File name", &ps(path))?;
    a.dialog_click("Open")
}

pub const T1: &str = "Monochrome bitmap (1-bit)";
pub const T4: &str = "16-color bitmap (4-bit)";
pub const T8: &str = "256-color bitmap (8-bit)";
pub const T24: &str = "24-bit bitmap";

/// File > Save As..., name + type, press Save. A colour warning, if any, is
/// left open for the caller.
pub fn save_as(a: &mut App, path: &Path, ty: &str) -> T {
    a.menu("File > Save As...")?;
    save_in_dialog(a, path, ty)
}
pub fn save_in_dialog(a: &mut App, path: &Path, ty: &str) -> T {
    expect_dialog(a, DialogKind::FileSave)?;
    a.dialog_set("File name", &ps(path))?;
    a.dialog_set("Save as type", ty)?;
    a.dialog_click("Save")
}
/// Save As and accept a colour-reduction warning if one appears.
pub fn save_as_ok(a: &mut App, path: &Path, ty: &str) -> T {
    save_as(a, path, ty)?;
    if let Some(d) = a.dialog()? {
        if d.kind == DialogKind::MessageBox && d.buttons.iter().any(|b| b == "OK") {
            a.dialog_click("OK")?;
        }
    }
    Ok(())
}

pub fn attributes(a: &mut App, w: &str, h: &str) -> T {
    a.menu("Image > Attributes...")?;
    expect_dialog(a, DialogKind::Attributes)?;
    a.dialog_set("Width", w)?;
    a.dialog_set("Height", h)?;
    a.dialog_click("OK")
}

pub fn flip_rotate(a: &mut App, action: &str, angle: &str) -> T {
    a.menu("Image > Flip/Rotate...")?;
    expect_dialog(a, DialogKind::FlipRotate)?;
    a.dialog_set("Action", action)?;
    if action == "Rotate by angle" {
        a.dialog_set("Angle", angle)?;
    }
    a.dialog_click("OK")
}

pub fn stretch_skew(a: &mut App, sh: &str, sv: &str, kh: &str, kv: &str) -> T {
    a.menu("Image > Stretch/Skew...")?;
    expect_dialog(a, DialogKind::StretchSkew)?;
    a.dialog_set("Stretch Horizontal", sh)?;
    a.dialog_set("Stretch Vertical", sv)?;
    a.dialog_set("Skew Horizontal", kh)?;
    a.dialog_set("Skew Vertical", kv)?;
    a.dialog_click("OK")
}

pub fn custom_zoom(a: &mut App, z: &str, button: &str) -> T {
    a.menu("View > Zoom > Custom...")?;
    expect_dialog(a, DialogKind::CustomZoom)?;
    a.dialog_set("Zoom", z)?;
    a.dialog_click(button)
}

/// Fill the expanded Edit Colors dialog's RGB fields.
pub fn edit_colors_rgb(a: &mut App, c: Rgb) -> T {
    expect_dialog(a, DialogKind::EditColors)?;
    a.dialog_click("Define Custom Colors >>")?;
    a.dialog_set("Red", &c.r.to_string())?;
    a.dialog_set("Green", &c.g.to_string())?;
    a.dialog_set("Blue", &c.b.to_string())
}

/// Double-click swatch n, choose colour c, OK.
pub fn redefine_swatch(a: &mut App, n: u32, c: Rgb) -> T {
    a.click_swatch(n, L, true)?;
    edit_colors_rgb(a, c)?;
    a.dialog_click("OK")
}

/// File > Exit (must exit without a prompt), then start again with `o`.
pub fn restart(a: &mut App, o: &StartOptions) -> HResult<App> {
    a.menu("File > Exit")?;
    if a.exit_status()?.is_none() {
        return fail("app did not exit");
    }
    App::start(o.clone())
}

// ---------------------------------------------------------------- pixels

pub fn px(a: &App, x: i32, y: i32) -> HResult<Rgb> {
    a.pixel(x, y)
}
pub fn size(a: &App) -> HResult<(u32, u32)> {
    let d = a.document()?;
    Ok((d.width, d.height))
}
/// All image pixels equal to c.
pub fn where_eq(img: &Image, c: Rgb) -> Pts {
    let mut s = Pts::new();
    for y in 0..img.height as i32 {
        for x in 0..img.width as i32 {
            if img.get(x, y) == Some(c) {
                s.insert((x, y));
            }
        }
    }
    s
}
pub fn where_ne(img: &Image, c: Rgb) -> Pts {
    let mut s = Pts::new();
    for y in 0..img.height as i32 {
        for x in 0..img.width as i32 {
            if img.get(x, y) != Some(c) {
                s.insert((x, y));
            }
        }
    }
    s
}
/// Committed pixels equal to c.
pub fn pts_of(a: &App, c: Rgb) -> HResult<Pts> {
    Ok(where_eq(&a.pixels()?, c))
}
pub fn k_pts(a: &App) -> HResult<Pts> {
    pts_of(a, K)
}
pub fn all_are(a: &App, c: Rgb) -> HResult<bool> {
    Ok(a.pixels()?.pixels.iter().all(|p| *p == c))
}
pub fn count_of(a: &App, c: Rgb) -> HResult<usize> {
    Ok(a.pixels()?.pixels.iter().filter(|p| **p == c).count())
}
pub fn rect_pts(x0: i32, y0: i32, x1: i32, y1: i32) -> Pts {
    let mut s = Pts::new();
    for y in y0.min(y1)..=y0.max(y1) {
        for x in x0.min(x1)..=x0.max(x1) {
            s.insert((x, y));
        }
    }
    s
}
/// Outer `w`-pixel ring of an inclusive rectangle.
pub fn border_pts(x0: i32, y0: i32, x1: i32, y1: i32, w: i32) -> Pts {
    let outer = rect_pts(x0, y0, x1, y1);
    if x1 - x0 + 1 <= 2 * w || y1 - y0 + 1 <= 2 * w {
        return outer;
    }
    let inner = rect_pts(x0 + w, y0 + w, x1 - w, y1 - w);
    outer.difference(&inner).copied().collect()
}
pub fn pts(list: &[(i32, i32)]) -> Pts {
    list.iter().copied().collect()
}
pub fn clip(s: &Pts, w: i32, h: i32) -> Pts {
    s.iter().copied().filter(|&(x, y)| x >= 0 && y >= 0 && x < w && y < h).collect()
}

// ---------------------------------------------------------------- spec geometry

/// The spec's 1-pixel path rule (tool.line behaviour 1).
pub fn line_path(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let n = dx.abs().max(dy.abs());
    if n == 0 {
        return vec![(x0, y0)];
    }
    let round_toward_zero_on_tie = |num: i32, den: i32| -> i32 {
        let q = num.abs() / den;
        let r = num.abs() % den;
        let m = if 2 * r > den { q + 1 } else { q };
        if num < 0 {
            -m
        } else {
            m
        }
    };
    (0..=n)
        .map(|t| {
            if dx.abs() >= dy.abs() {
                (x0 + dx.signum() * t, y0 + round_toward_zero_on_tie(dy * t, n))
            } else {
                (x0 + round_toward_zero_on_tie(dx * t, n), y0 + dy.signum() * t)
            }
        })
        .collect()
}

/// Offsets of an n x n square stamp: [-floor(n/2), ceil(n/2)-1].
pub fn square_offsets(n: i32) -> Vec<(i32, i32)> {
    let lo = -(n / 2);
    let hi = (n + 1) / 2 - 1;
    let mut v = Vec::new();
    for dy in lo..=hi {
        for dx in lo..=hi {
            v.push((dx, dy));
        }
    }
    v
}

/// Stamp offsets at every path pixel.
pub fn stamp(path: &[(i32, i32)], offs: &[(i32, i32)]) -> Pts {
    let mut s = Pts::new();
    for &(x, y) in path {
        for &(dx, dy) in offs {
            s.insert((x + dx, y + dy));
        }
    }
    s
}

/// Brush footprint offsets for shapes 1..=12 (tool.brush behaviour 1).
pub fn brush_offsets(shape: u32) -> Vec<(i32, i32)> {
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
    let asc = |n: i32| (0..n).map(|i| (i - n / 2, n / 2 - i)).collect::<Vec<_>>();
    let desc = |n: i32| (0..n).map(|i| (i - n / 2, i - n / 2)).collect::<Vec<_>>();
    match shape {
        1 => round(7),
        2 => round(5),
        3 => round(1),
        4 => square_offsets(8),
        5 => square_offsets(5),
        6 => square_offsets(2),
        7 => asc(8),
        8 => asc(5),
        9 => asc(2),
        10 => desc(8),
        11 => desc(5),
        _ => desc(2),
    }
}

pub fn disc(cx: i32, cy: i32, r: i32) -> Pts {
    let mut s = Pts::new();
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                s.insert((cx + dx, cy + dy));
            }
        }
    }
    s
}

// ---------------------------------------------------------------- BMP files

pub fn le16(b: &[u8], o: usize) -> u32 {
    u16::from_le_bytes([b[o], b[o + 1]]) as u32
}
pub fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
pub fn le32s(b: &[u8], o: usize) -> i32 {
    le32(b, o) as i32
}

fn put16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// A BMP with a 40-byte info header and arbitrary fields; `data` is the raw
/// pixel area, `extra` (bit masks) goes between header and palette.
pub fn bmp_raw(
    w: i32,
    h: i32,
    bpp: u16,
    compression: u32,
    palette: &[Rgb],
    extra: &[u8],
    data: &[u8],
) -> Vec<u8> {
    let off = 14 + 40 + extra.len() + palette.len() * 4;
    let total = off + data.len();
    let mut v = Vec::new();
    v.extend_from_slice(b"BM");
    put32(&mut v, total as u32);
    put32(&mut v, 0);
    put32(&mut v, off as u32);
    put32(&mut v, 40);
    put32(&mut v, w as u32);
    put32(&mut v, h as u32);
    put16(&mut v, 1);
    put16(&mut v, bpp);
    put32(&mut v, compression);
    put32(&mut v, data.len() as u32);
    put32(&mut v, 2835);
    put32(&mut v, 2835);
    put32(&mut v, palette.len() as u32);
    put32(&mut v, 0);
    v.extend_from_slice(extra);
    for c in palette {
        v.extend_from_slice(&[c.b, c.g, c.r, 0]);
    }
    v.extend_from_slice(data);
    v
}

fn row_bytes(w: u32, bpp: u32) -> usize {
    (((w * bpp + 31) / 32) * 4) as usize
}

/// Uncompressed 24-bit BMP (bottom-up unless `top_down`).
pub fn bmp24(w: u32, h: u32, px: &[Rgb], top_down: bool) -> Vec<u8> {
    let rb = row_bytes(w, 24);
    let mut data = Vec::new();
    for i in 0..h {
        let y = if top_down { i } else { h - 1 - i };
        let mut row = Vec::new();
        for x in 0..w {
            let c = px[(y * w + x) as usize];
            row.extend_from_slice(&[c.b, c.g, c.r]);
        }
        row.resize(rb, 0);
        data.extend_from_slice(&row);
    }
    let hh = if top_down { -(h as i32) } else { h as i32 };
    bmp_raw(w as i32, hh, 24, 0, &[], &[], &data)
}

/// Uncompressed indexed BMP (1, 4 or 8 bpp), bottom-up.
pub fn bmp_indexed(w: u32, h: u32, bpp: u32, palette: &[Rgb], idx: &[u8]) -> Vec<u8> {
    let rb = row_bytes(w, bpp);
    let mut data = Vec::new();
    for y in (0..h).rev() {
        let mut row = vec![0u8; rb];
        for x in 0..w {
            let v = idx[(y * w + x) as usize] as u32;
            let bit = x * bpp;
            let byte = (bit / 8) as usize;
            let shift = 8 - bpp - (bit % 8);
            row[byte] |= (v << shift) as u8;
        }
        data.extend_from_slice(&row);
    }
    bmp_raw(w as i32, h as i32, bpp as u16, 0, palette, &[], &data)
}

/// Decoded view of a BMP the app wrote (uncompressed, 40-byte header).
pub struct Bmp {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub palette: Vec<Rgb>,
    /// Palette index per pixel (indexed depths only), row-major from the top.
    pub index: Vec<u8>,
    pub image: Image,
}

pub fn decode(b: &[u8]) -> HResult<Bmp> {
    if b.len() < 54 || &b[0..2] != b"BM" {
        return fail("not a BMP");
    }
    let off = le32(b, 10) as usize;
    let w = le32s(b, 18);
    let h = le32s(b, 22);
    let bpp = le16(b, 28);
    let (w, hu) = (w as u32, h.unsigned_abs());
    let ncol = if bpp <= 8 {
        let n = le32(b, 46);
        if n == 0 {
            1 << bpp
        } else {
            n
        }
    } else {
        0
    };
    let mut palette = Vec::new();
    for i in 0..ncol as usize {
        let o = 14 + le32(b, 14) as usize + 4 * i;
        palette.push(Rgb::new(b[o + 2], b[o + 1], b[o]));
    }
    let rb = row_bytes(w, bpp);
    let mut pixels = Vec::new();
    let mut index = Vec::new();
    for y in 0..hu {
        let fy = if h < 0 { y } else { hu - 1 - y };
        let row = off + fy as usize * rb;
        for x in 0..w {
            if bpp == 24 {
                let o = row + 3 * x as usize;
                pixels.push(Rgb::new(b[o + 2], b[o + 1], b[o]));
            } else {
                let bit = x * bpp;
                let byte = b[row + (bit / 8) as usize];
                let shift = 8 - bpp - (bit % 8);
                let v = (byte >> shift) & ((1u16 << bpp) - 1) as u8;
                index.push(v);
                pixels.push(palette.get(v as usize).copied().unwrap_or(K));
            }
        }
    }
    Ok(Bmp { width: w, height: hu, bpp, palette, index, image: Image { width: w, height: hu, pixels } })
}

pub fn decode_file(p: &Path) -> HResult<Bmp> {
    decode(&read(p)?)
}

/// Write a 24-bit BMP and return its path.
pub fn bmp_file(dir: &Path, name: &str, w: u32, h: u32, px: &[Rgb]) -> HResult<PathBuf> {
    let p = dir.join(name);
    write(&p, &bmp24(w, h, px, false))?;
    Ok(p)
}

pub fn solid(w: u32, h: u32, c: Rgb) -> Vec<Rgb> {
    vec![c; (w * h) as usize]
}

/// A deterministic multi-colour pattern.
pub fn pattern(w: u32, h: u32, seed: u8) -> Vec<Rgb> {
    (0..w * h)
        .map(|i| {
            let i = i as u8;
            Rgb::new(i.wrapping_mul(37).wrapping_add(seed), i.wrapping_mul(91) ^ seed, 200u8.wrapping_sub(i.wrapping_mul(13)))
        })
        .collect()
}

/// The 16-colour palette of format.color_reduction.
pub const PAL16: [Rgb; 16] = [
    Rgb::new(0, 0, 0),
    Rgb::new(128, 0, 0),
    Rgb::new(0, 128, 0),
    Rgb::new(128, 128, 0),
    Rgb::new(0, 0, 128),
    Rgb::new(128, 0, 128),
    Rgb::new(0, 128, 128),
    Rgb::new(192, 192, 192),
    Rgb::new(128, 128, 128),
    Rgb::new(255, 0, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(0, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(0, 255, 255),
    Rgb::new(255, 255, 255),
];

/// The default 28 swatches of colors.color_box.
pub const SWATCHES: [Rgb; 28] = [
    Rgb::new(0, 0, 0),
    Rgb::new(128, 128, 128),
    Rgb::new(128, 0, 0),
    Rgb::new(128, 128, 0),
    Rgb::new(0, 128, 0),
    Rgb::new(0, 128, 128),
    Rgb::new(0, 0, 128),
    Rgb::new(128, 0, 128),
    Rgb::new(128, 128, 64),
    Rgb::new(0, 64, 64),
    Rgb::new(0, 128, 255),
    Rgb::new(0, 64, 128),
    Rgb::new(64, 0, 255),
    Rgb::new(128, 64, 0),
    Rgb::new(255, 255, 255),
    Rgb::new(192, 192, 192),
    Rgb::new(255, 0, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 255, 255),
    Rgb::new(0, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 255, 128),
    Rgb::new(0, 255, 128),
    Rgb::new(128, 255, 255),
    Rgb::new(128, 128, 255),
    Rgb::new(255, 0, 128),
    Rgb::new(255, 128, 64),
];

// ---------------------------------------------------------------- composite helpers

/// File > New, answering No if the save-changes prompt appears.
pub fn file_new_discard(a: &mut App) -> T {
    a.menu("File > New")?;
    if let Some(d) = a.dialog()? {
        if d.buttons.iter().any(|b| b == "No") {
            a.dialog_click("No")?;
        }
    }
    Ok(())
}

/// Observable state used to compare a shortcut with its menu item.
pub fn snapshot(a: &App) -> HResult<String> {
    Ok(format!(
        "{:?}",
        (
            a.pixels()?,
            a.selection()?,
            a.zoom()?,
            a.dialog()?.map(|d| (d.kind, d.buttons)),
            a.view_mode()?,
            a.panel(Panel::ToolBox)?,
            a.panel(Panel::ColorBox)?,
            a.clipboard()?,
            a.exit_status()?,
            a.document()?,
            a.menu_item("View > Zoom > Show Grid")?.checked,
            a.active_tool()?,
        )
    ))
}

/// Start state for shortcut comparisons: two pencil dots (dirty, undoable),
/// a 4x4 selection and a 2x2 red clipboard image.
pub fn shortcut_setup() -> HResult<App> {
    let mut a = fresh()?;
    pencil_click(&mut a, 1, 1)?;
    pencil_click(&mut a, 30, 30)?;
    select_rect(&mut a, (2, 2), (5, 5))?;
    a.set_clipboard(ClipboardContent::Image(Image { width: 2, height: 2, pixels: vec![RED; 4] }))?;
    Ok(a)
}

/// For each (key, mods, menu path, optional preparatory menu command):
/// the shortcut and the menu item give identical state, and that state
/// differs from the state before.
pub fn shortcut_table(cases: &[(Key, Mods, &str, Option<&str>)]) -> T {
    for &(k, m, path, pre) in cases {
        let mut by_key = shortcut_setup()?;
        let mut by_menu = shortcut_setup()?;
        if let Some(p) = pre {
            by_key.menu(p)?;
            by_menu.menu(p)?;
        }
        let before = snapshot(&by_key)?;
        by_key.key(k, m)?;
        by_menu.menu(path)?;
        let sk = snapshot(&by_key)?;
        let sm = snapshot(&by_menu)?;
        assert_eq!(sk, sm, "{k:?}+{m:?} differs from {path}");
        assert_ne!(sk, before, "{k:?}+{m:?} / {path} changed nothing");
    }
    Ok(())
}
