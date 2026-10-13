//! Feature tool.text: Text tool and text toolbar state. A drag (or click)
//! creates a frame; typed text is laid out with the bundled bitmap font
//! (`font.rs`), word-wrapped to the frame width, drawn live by the composite
//! layer in the foreground colour and turned into ordinary pixels (one undo
//! step) when the frame is committed.
//!
//! Hooks for other features (clipboard, zoom, panels, undo):
//! [`insert_text`], [`selected_text`], [`delete_selected`], [`undo_text_edit`],
//! [`frame_active`], [`commit_frame`], [`on_zoom_changed`], and the toolbar
//! state in `State::text_style` / `State::text_toolbar`.

mod font;

use crate::raster::{clip_rect, rect_from_corners};
use crate::registry::Registry;
use crate::selection::{handle_centres, screen_box};
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;
pub use font::BUNDLED_FONT;
use font::{glyph, Metrics};

/// Width of a frame made by a click without drag (spec assumption).
const CLICK_FRAME_WIDTH: i32 = 100;
/// Edit steps remembered by the frame's own undo.
const TEXT_UNDO_DEPTH: usize = 100;

/// Sizes offered by the toolbar's size list.
pub const FONT_SIZES: [u32; 16] = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

/// Toolbar choices; they restyle all text of the active frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    pub font: String,
    pub size_pt: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle { font: BUNDLED_FONT.to_string(), size_pt: 10, bold: false, italic: false, underline: false }
    }
}

impl TextStyle {
    fn metrics(&self) -> Metrics {
        Metrics::new(self.size_pt, self.bold, self.italic)
    }
}

type EditState = (Vec<char>, usize, Option<usize>);

/// The active (uncommitted) frame.
#[derive(Debug, Clone)]
pub struct TextSession {
    pub rect: Rect,
    pub chars: Vec<char>,
    /// Caret position as a char index (0..=len).
    pub cursor: usize,
    /// Other end of the selected text, when any.
    pub anchor: Option<usize>,
    undo: Vec<EditState>,
}

/// An in-progress pointer gesture on the Text tool.
#[derive(Debug, Clone, Copy)]
pub enum TextDrag {
    Create { start: (i32, i32), cur: (i32, i32) },
    Select,
    Move { dx: i32, dy: i32 },
    Resize(Handle),
    /// The press committed the old frame; the rest of the gesture is ignored.
    Swallow,
}

struct TextTool;
static HANDLER: TextTool = TextTool;

pub fn register(r: &mut Registry) {
    r.tool(Tool::Text, &HANDLER);
    r.composite_layer(composite_layer);
    r.workspace_layer(workspace_layer);
}

// ----- public hooks

/// Whether a frame is currently active.
pub fn frame_active(st: &State) -> bool {
    st.text.is_some()
}

/// The harness view of the active frame.
pub fn frame(st: &State) -> Option<TextFrame> {
    st.text.as_ref().map(|s| TextFrame { rect: s.rect, text: s.chars.iter().collect() })
}

/// Insert text at the caret, replacing selected text (Paste into a frame).
/// One text-undo step. Returns false when no frame is active.
pub fn insert_text(st: &mut State, s: &str) -> bool {
    let chars: Vec<char> = s
        .replace("\r\n", "\n")
        .chars()
        .filter_map(|c| match c {
            '\r' => Some('\n'),
            '\t' => Some(' '),
            c if c == '\n' || !c.is_control() => Some(c),
            _ => None,
        })
        .collect();
    if st.text.is_none() {
        return false;
    }
    if chars.is_empty() && !has_selection(st) {
        return true;
    }
    edit(st, |sess| {
        remove_selection(sess);
        let at = sess.cursor;
        sess.chars.splice(at..at, chars.iter().copied());
        sess.cursor = at + chars.len();
    });
    true
}

fn has_selection(st: &State) -> bool {
    st.text.as_ref().is_some_and(|s| s.anchor.is_some_and(|a| a != s.cursor))
}

/// The selected text of the frame, if any (Copy / Cut).
pub fn selected_text(st: &State) -> Option<String> {
    let s = st.text.as_ref()?;
    let a = s.anchor.filter(|a| *a != s.cursor)?;
    let (lo, hi) = (a.min(s.cursor), a.max(s.cursor));
    Some(s.chars[lo..hi].iter().collect())
}

/// Delete the selected text (second half of Cut). Returns whether any text
/// was removed.
pub fn delete_selected(st: &mut State) -> bool {
    if !has_selection(st) {
        return false;
    }
    edit(st, |sess| {
        remove_selection(sess);
    });
    true
}

/// Undo the last text edit of the active frame. Returns false when there is
/// no frame (the caller then undoes the document instead).
pub fn undo_text_edit(st: &mut State) -> bool {
    let Some(sess) = st.text.as_mut() else { return false };
    if let Some((chars, cursor, anchor)) = sess.undo.pop() {
        sess.chars = chars;
        sess.cursor = cursor;
        sess.anchor = anchor;
    }
    true
}

/// Commit the active frame: its text becomes pixels in one undo step. An
/// empty frame commits nothing.
pub fn commit_frame(st: &mut State) {
    let Some(sess) = st.text.take() else { return };
    st.text_drag = None;
    if sess.chars.is_empty() {
        return;
    }
    st.doc.push_undo();
    let (fill, ink) = paint(st, &sess);
    let bg = st.bg;
    let fg = st.fg;
    if let Some(r) = fill {
        for y in r.y0..=r.y1 {
            for x in r.x0..=r.x1 {
                st.doc.paint(x, y, bg);
            }
        }
    }
    for (x, y) in ink {
        st.doc.paint(x, y, fg);
    }
}

/// The zoom changed: above 100% the Text tool is disabled, so any frame is
/// committed and the previous tool (or Pencil) takes over.
pub fn on_zoom_changed(st: &mut State) -> HResult<()> {
    if st.zoom > 100 && st.tool == Tool::Text {
        commit_frame(st);
        let back = if st.prev_tool == Tool::Text { Tool::Pencil } else { st.prev_tool };
        st.select_tool(back)?;
    }
    Ok(())
}

/// Show or hide the text toolbar (View > Text Toolbar and the context menu).
pub fn set_toolbar_visible(st: &mut State, visible: bool) {
    st.text_toolbar = visible;
}

// ----- layout

#[derive(Debug, Clone, Copy)]
struct Line {
    start: usize,
    /// Exclusive; for a hard line break this is the index of the '\n'.
    end: usize,
}

/// Word-wrap `chars` to `width` pixels of fixed-pitch text.
fn layout(chars: &[char], adv: i32, width: i32) -> Vec<Line> {
    let cap = (width / adv.max(1)).max(1) as usize;
    let mut lines = Vec::new();
    let mut ps = 0;
    loop {
        let pe = chars[ps..].iter().position(|c| *c == '\n').map_or(chars.len(), |i| ps + i);
        let mut pos = ps;
        loop {
            if pe - pos <= cap {
                lines.push(Line { start: pos, end: pe });
                break;
            }
            let brk = (pos..=(pos + cap).min(pe - 1)).rev().find(|&j| chars[j] == ' ');
            match brk {
                Some(j) => {
                    lines.push(Line { start: pos, end: j + 1 });
                    pos = j + 1;
                }
                None => {
                    lines.push(Line { start: pos, end: pos + cap });
                    pos += cap;
                }
            }
        }
        if pe >= chars.len() {
            break;
        }
        ps = pe + 1;
    }
    lines
}

fn line_of(lines: &[Line], idx: usize) -> usize {
    lines.iter().rposition(|l| l.start <= idx).unwrap_or(0)
}

/// Where the caret may rest at the end of a line (not past a soft wrap).
fn line_end(lines: &[Line], i: usize) -> usize {
    let l = lines[i];
    if i + 1 < lines.len() && lines[i + 1].start == l.end && l.end > l.start {
        l.end - 1
    } else {
        l.end
    }
}

/// Char index nearest to image point `p`.
fn index_at(sess: &TextSession, style: &TextStyle, p: (i32, i32)) -> usize {
    let m = style.metrics();
    let lines = layout(&sess.chars, m.adv, sess.rect.width());
    let li = ((p.1 - sess.rect.y0).max(0) / m.lh.max(1)) as usize;
    let li = li.min(lines.len() - 1);
    let col = ((p.0 - sess.rect.x0) + m.adv / 2).div_euclid(m.adv.max(1)).max(0) as usize;
    let l = lines[li];
    (l.start + col).min(line_end(&lines, li))
}

/// Grow the frame downward to hold all lines, stopping at the canvas bottom.
fn fit(st: &mut State) {
    let m = st.text_style.metrics();
    let bottom = st.doc.height as i32 - 1;
    let Some(sess) = st.text.as_mut() else { return };
    let n = layout(&sess.chars, m.adv, sess.rect.width()).len() as i32;
    let need = sess.rect.y0 + n * m.lh - 1;
    if need > sess.rect.y1 {
        sess.rect.y1 = need.min(bottom.max(sess.rect.y1));
    }
}

// ----- rendering

/// (opaque background rectangle clipped to the canvas, ink pixels clipped to
/// the frame and canvas) of a session as it would be drawn now.
fn paint(st: &State, sess: &TextSession) -> (Option<Rect>, Vec<(i32, i32)>) {
    let style = &st.text_style;
    let m = style.metrics();
    let r = sess.rect;
    let fill = if st.options.opaque { clip_rect(r, st.doc.width, st.doc.height) } else { None };
    let mut ink = Vec::new();
    let mut put = |x: i32, y: i32| {
        if x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1 && x >= 0 && y >= 0 && x < st.doc.width as i32 && y < st.doc.height as i32 {
            ink.push((x, y));
        }
    };
    for (li, l) in layout(&sess.chars, m.adv, r.width()).iter().enumerate() {
        let top = r.y0 + li as i32 * m.lh;
        let mut pen = r.x0;
        for &c in &sess.chars[l.start..l.end] {
            if c != ' ' {
                let g = glyph(c, &m);
                let gy = top + m.ascent - m.gh as i32;
                for yy in 0..g.h {
                    for xx in 0..g.w {
                        if g.on[yy * g.w + xx] {
                            put(pen + xx as i32, gy + yy as i32);
                        }
                    }
                }
            }
            pen += m.adv;
        }
        if style.underline && l.end > l.start {
            for x in r.x0..pen {
                put(x, top + m.ascent + 1);
            }
        }
    }
    (fill, ink)
}

fn composite_layer(st: &State, img: &mut Image) {
    let Some(sess) = st.text.as_ref() else { return };
    let (fill, ink) = paint(st, sess);
    let w = img.width as usize;
    if let Some(r) = fill {
        for y in r.y0..=r.y1 {
            for x in r.x0..=r.x1 {
                img.pixels[y as usize * w + x as usize] = st.bg;
            }
        }
    }
    for (x, y) in ink {
        img.pixels[y as usize * w + x as usize] = st.fg;
    }
}

fn set(img: &mut Image, x: i64, y: i64, c: Rgb) {
    if x >= 0 && y >= 0 && x < img.width as i64 && y < img.height as i64 {
        img.pixels[y as usize * img.width as usize + x as usize] = c;
    }
}

fn dashed_box(img: &mut Image, b: (i64, i64, i64, i64)) {
    let c = |n: i64| if (n.rem_euclid(6) / 3) == 0 { Rgb::K } else { Rgb::W };
    for x in b.0..=b.2 {
        set(img, x, b.1, c(x - b.0));
        set(img, x, b.3, c(x - b.0));
    }
    for y in b.1..=b.3 {
        set(img, b.0, y, c(y - b.1));
        set(img, b.2, y, c(y - b.1));
    }
}

fn workspace_layer(st: &State, img: &mut Image) {
    if let Some(TextDrag::Create { start, cur }) = st.text_drag {
        if start != cur {
            dashed_box(img, screen_box(st, rect_from_corners(start, cur)));
        }
    }
    let Some(sess) = st.text.as_ref() else { return };
    let b = screen_box(st, sess.rect);
    dashed_box(img, b);
    for (cx, cy) in handle_centres(b) {
        for dy in -2..=2i64 {
            for dx in -2..=2i64 {
                let edge = dx.abs() == 2 || dy.abs() == 2;
                set(img, cx + dx, cy + dy, if edge { Rgb::K } else { Rgb::W });
            }
        }
    }
}

// ----- editing

/// Run one text edit as a single undoable step, then regrow the frame.
fn edit(st: &mut State, f: impl FnOnce(&mut TextSession)) {
    if let Some(sess) = st.text.as_mut() {
        sess.undo.push((sess.chars.clone(), sess.cursor, sess.anchor));
        if sess.undo.len() > TEXT_UNDO_DEPTH {
            sess.undo.remove(0);
        }
        f(sess);
    }
    fit(st);
}

fn remove_selection(sess: &mut TextSession) -> bool {
    match sess.anchor.take() {
        Some(a) if a != sess.cursor => {
            let (lo, hi) = (a.min(sess.cursor), a.max(sess.cursor));
            sess.chars.drain(lo..hi);
            sess.cursor = lo;
            true
        }
        _ => false,
    }
}

fn move_caret(st: &mut State, shift: bool, target: impl FnOnce(&TextSession, &[Line]) -> usize) {
    let m = st.text_style.metrics();
    let Some(sess) = st.text.as_mut() else { return };
    let lines = layout(&sess.chars, m.adv, sess.rect.width());
    let to = target(sess, &lines).min(sess.chars.len());
    if shift {
        sess.anchor.get_or_insert(sess.cursor);
    } else {
        sess.anchor = None;
    }
    sess.cursor = to;
}

impl ToolHandler for TextTool {
    fn deactivate(&self, st: &mut State) -> HResult<()> {
        commit_frame(st);
        st.text_drag = None;
        Ok(())
    }

    fn press(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        if ev.button != Some(Button::Left) {
            return Ok(());
        }
        if let (Some(sess), Pos::SelectionHandle(h)) = (st.text.as_ref(), ev.pos) {
            let _ = sess;
            st.text_drag = Some(TextDrag::Resize(h));
            return Ok(());
        }
        let Some(p) = ev.image else {
            commit_frame(st);
            st.text_drag = Some(TextDrag::Swallow);
            return Ok(());
        };
        if let Some(sess) = st.text.as_ref() {
            let r = sess.rect;
            let inside = p.0 >= r.x0 && p.0 <= r.x1 && p.1 >= r.y0 && p.1 <= r.y1;
            let ring = p.0 >= r.x0 - 1 && p.0 <= r.x1 + 1 && p.1 >= r.y0 - 1 && p.1 <= r.y1 + 1;
            if inside {
                let idx = index_at(sess, &st.text_style, p);
                if let Some(s) = st.text.as_mut() {
                    s.cursor = idx;
                    s.anchor = Some(idx);
                }
                st.text_drag = Some(TextDrag::Select);
            } else if ring {
                st.text_drag = Some(TextDrag::Move { dx: p.0 - r.x0, dy: p.1 - r.y0 });
            } else {
                // Clicking outside commits; that click starts nothing new.
                commit_frame(st);
                st.text_drag = Some(TextDrag::Swallow);
            }
            return Ok(());
        }
        st.text_drag = Some(TextDrag::Create { start: p, cur: p });
        Ok(())
    }

    fn drag(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        track(st, ev);
        Ok(())
    }

    fn release(&self, st: &mut State, ev: &PointerEvent) -> HResult<()> {
        track(st, ev);
        let Some(drag) = st.text_drag.take() else { return Ok(()) };
        match drag {
            TextDrag::Create { start, cur } => create_frame(st, start, cur),
            TextDrag::Select => {
                if let Some(s) = st.text.as_mut() {
                    if s.anchor == Some(s.cursor) {
                        s.anchor = None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn key(&self, st: &mut State, key: Key, mods: Mods) -> HResult<bool> {
        if st.text.is_none() {
            return Ok(false);
        }
        if mods.ctrl {
            return Ok(match key {
                Key::Char(c) if c.eq_ignore_ascii_case(&'a') => {
                    let n = st.text.as_ref().map_or(0, |s| s.chars.len());
                    if let Some(s) = st.text.as_mut() {
                        s.anchor = Some(0);
                        s.cursor = n;
                    }
                    true
                }
                Key::Char(c) if c.eq_ignore_ascii_case(&'z') => undo_text_edit(st),
                _ => false,
            });
        }
        if mods.alt {
            return Ok(false);
        }
        let shift = mods.shift;
        match key {
            Key::Escape => commit_frame(st),
            Key::Char(c) if !c.is_control() => {
                edit(st, |s| {
                    remove_selection(s);
                    s.chars.insert(s.cursor, c);
                    s.cursor += 1;
                });
            }
            Key::Enter => edit(st, |s| {
                remove_selection(s);
                s.chars.insert(s.cursor, '\n');
                s.cursor += 1;
            }),
            Key::Backspace => {
                let can = has_selection(st) || st.text.as_ref().is_some_and(|s| s.cursor > 0);
                if can {
                    edit(st, |s| {
                        if !remove_selection(s) {
                            s.cursor -= 1;
                            s.chars.remove(s.cursor);
                        }
                    });
                }
            }
            Key::Delete => {
                let can = has_selection(st) || st.text.as_ref().is_some_and(|s| s.cursor < s.chars.len());
                if can {
                    edit(st, |s| {
                        if !remove_selection(s) {
                            s.chars.remove(s.cursor);
                        }
                    });
                }
            }
            Key::Left => {
                let collapse = !shift && has_selection(st);
                move_caret(st, shift, |s, _| match (collapse, s.anchor) {
                    (true, Some(a)) => a.min(s.cursor),
                    _ => s.cursor.saturating_sub(1),
                });
            }
            Key::Right => {
                let collapse = !shift && has_selection(st);
                move_caret(st, shift, |s, _| match (collapse, s.anchor) {
                    (true, Some(a)) => a.max(s.cursor),
                    _ => s.cursor + 1,
                });
            }
            Key::Up | Key::Down => {
                let up = key == Key::Up;
                move_caret(st, shift, |s, lines| {
                    let li = line_of(lines, s.cursor);
                    let col = s.cursor - lines[li].start;
                    let to = if up { li.checked_sub(1) } else { (li + 1 < lines.len()).then_some(li + 1) };
                    match to {
                        Some(t) => (lines[t].start + col).min(line_end(lines, t)),
                        None if up => 0,
                        None => s.chars.len(),
                    }
                });
            }
            Key::Home => move_caret(st, shift, |s, lines| lines[line_of(lines, s.cursor)].start),
            Key::End => move_caret(st, shift, |s, lines| line_end(lines, line_of(lines, s.cursor))),
            Key::PageUp | Key::PageDown | Key::Tab => {}
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// Apply a pointer move/release to the gesture in progress.
fn track(st: &mut State, ev: &PointerEvent) {
    let Some(drag) = st.text_drag else { return };
    let Some(p) = ev.image else { return };
    match drag {
        TextDrag::Create { start, .. } => st.text_drag = Some(TextDrag::Create { start, cur: p }),
        TextDrag::Select => {
            let idx = st.text.as_ref().map(|s| index_at(s, &st.text_style, p));
            if let (Some(s), Some(i)) = (st.text.as_mut(), idx) {
                s.cursor = i;
            }
        }
        TextDrag::Move { dx, dy } => {
            if let Some(s) = st.text.as_mut() {
                let (w, h) = (s.rect.width(), s.rect.height());
                s.rect = Rect::new(p.0 - dx, p.1 - dy, p.0 - dx + w - 1, p.1 - dy + h - 1);
            }
        }
        TextDrag::Resize(h) => {
            if let Some(s) = st.text.as_mut() {
                let mut r = s.rect;
                match h {
                    Handle::TopLeft | Handle::Left | Handle::BottomLeft => r.x0 = p.0,
                    Handle::TopRight | Handle::Right | Handle::BottomRight => r.x1 = p.0,
                    _ => {}
                }
                match h {
                    Handle::TopLeft | Handle::Top | Handle::TopRight => r.y0 = p.1,
                    Handle::BottomLeft | Handle::Bottom | Handle::BottomRight => r.y1 = p.1,
                    _ => {}
                }
                if r.x0 > r.x1 {
                    std::mem::swap(&mut r.x0, &mut r.x1);
                }
                if r.y0 > r.y1 {
                    std::mem::swap(&mut r.y0, &mut r.y1);
                }
                s.rect = r;
            }
            fit(st);
        }
        TextDrag::Swallow => {}
    }
}

/// Make a frame from a finished creation gesture.
fn create_frame(st: &mut State, start: (i32, i32), end: (i32, i32)) {
    let (w, h) = (st.doc.width as i32, st.doc.height as i32);
    let clamp = |p: (i32, i32)| (p.0.clamp(0, w - 1), p.1.clamp(0, h - 1));
    let rect = if start == end {
        let (x, y) = clamp(start);
        let lh = st.text_style.metrics().lh;
        Rect::new(x, y, (x + CLICK_FRAME_WIDTH - 1).min(w - 1), (y + lh - 1).min(h - 1))
    } else {
        rect_from_corners(clamp(start), clamp(end))
    };
    st.text = Some(TextSession { rect, chars: Vec::new(), cursor: 0, anchor: None, undo: Vec::new() });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_words_and_breaks_long_words() {
        let t: Vec<char> = "ab cd efgh".chars().collect();
        let l = layout(&t, 1, 5);
        let rows: Vec<String> = l.iter().map(|l| t[l.start..l.end].iter().collect()).collect();
        assert_eq!(rows, ["ab cd ", "efgh"]);
        let t: Vec<char> = "abcdefg".chars().collect();
        assert_eq!(layout(&t, 1, 3).len(), 3);
    }

    #[test]
    fn hard_breaks_make_lines() {
        let t: Vec<char> = "a\n\nb".chars().collect();
        assert_eq!(layout(&t, 8, 100).len(), 3);
    }

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-text-{}", std::process::id()));
        let mut st = State::new(crate::settings::Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.new_default_document().unwrap();
        st.select_tool(Tool::Text).unwrap();
        st
    }

    fn typed(st: &mut State, s: &str) {
        for c in s.chars() {
            st.key(Key::Char(c), Mods::NONE).unwrap();
        }
    }

    #[test]
    fn paste_over_selection_undoes_to_original() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), Pos::Image(10, 10), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), Pos::Image(109, 49), Mods::NONE).unwrap();
        typed(&mut st, "abc");
        st.key(Key::Left, Mods::NONE).unwrap();
        st.key(Key::Left, Mods::NONE).unwrap();
        st.key(Key::Right, Mods::SHIFT).unwrap();
        assert_eq!(selected_text(&st).as_deref(), Some("b"));
        assert!(insert_text(&mut st, "XY"));
        assert_eq!(frame(&st).unwrap().text, "aXYc");
        st.key(Key::Char('z'), Mods::CTRL).unwrap();
        assert_eq!(frame(&st).unwrap().text, "abc");
    }

    #[test]
    fn click_makes_default_frame_and_text_wraps_and_grows() {
        let mut st = state();
        st.pointer(PointerKind::Press(Button::Left), Pos::Image(5, 5), Mods::NONE).unwrap();
        st.pointer(PointerKind::Release(Button::Left), Pos::Image(5, 5), Mods::NONE).unwrap();
        let f = frame(&st).unwrap();
        assert_eq!((f.rect.width(), f.rect.height()), (100, 13));
        typed(&mut st, "hello world again and again and again");
        assert!(frame(&st).unwrap().rect.height() > 13);
        commit_frame(&mut st);
        assert!(frame(&st).is_none());
        assert!(st.doc.pixels.iter().any(|p| *p == Rgb::K));
        assert_eq!(st.doc.undo.len(), 1);
    }

    #[test]
    fn styles_change_ink() {
        let ink = |bold: bool, italic: bool, underline: bool| {
            let mut st = state();
            st.text_style.bold = bold;
            st.text_style.italic = italic;
            st.text_style.underline = underline;
            st.pointer(PointerKind::Press(Button::Left), Pos::Image(5, 5), Mods::NONE).unwrap();
            st.pointer(PointerKind::Release(Button::Left), Pos::Image(5, 5), Mods::NONE).unwrap();
            typed(&mut st, "Hi");
            commit_frame(&mut st);
            st.doc.pixels.iter().filter(|p| **p == Rgb::K).count()
        };
        let plain = ink(false, false, false);
        assert!(plain > 0);
        assert!(ink(true, false, false) > plain);
        assert!(ink(false, false, true) > plain);
    }

    #[test]
    fn point_sizes_convert_at_96_dpi() {
        assert_eq!(font::px_for_pt(10), 13);
        assert_eq!(font::px_for_pt(72), 96);
    }
}
