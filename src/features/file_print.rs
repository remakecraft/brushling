//! Feature file.print: Print, Print Preview and Page Setup.
//!
//! The image prints at 96 pixels per inch from the top-left of the printable
//! area (paper minus margins) and is split into pages left to right, then top
//! to bottom. Page Setup values last for the session (`State::page_setup`)
//! and are never written to the image or the settings file. The platform
//! print dialog is a `Print` dialog whose OK records `PlatformCall::Print`.

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const PIXELS_PER_INCH: f64 = 96.0;
const PAPERS: [(&str, f64, f64); 3] = [("Letter", 8.5, 11.0), ("Legal", 8.5, 14.0), ("A4", 8.27, 11.69)];

/// Paper, orientation and margins (inches) chosen in Page Setup.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSetup {
    pub paper: String,
    pub landscape: bool,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Default for PageSetup {
    fn default() -> Self {
        PageSetup { paper: "Letter".into(), landscape: false, left: 0.75, right: 0.75, top: 1.0, bottom: 1.0 }
    }
}

impl PageSetup {
    /// Paper size in inches as (width, height), orientation applied.
    fn paper_inches(&self) -> (f64, f64) {
        let (w, h) = PAPERS
            .iter()
            .find(|(name, _, _)| *name == self.paper)
            .map_or((8.5, 11.0), |&(_, w, h)| (w, h));
        if self.landscape { (h, w) } else { (w, h) }
    }

    /// Printable area in pixels (paper minus margins); `None` if no pixel is left.
    fn printable_px(&self) -> Option<(u32, u32)> {
        let (pw, ph) = self.paper_inches();
        let w = to_px(pw - self.left - self.right);
        let h = to_px(ph - self.top - self.bottom);
        (w >= 1 && h >= 1).then_some((w, h))
    }
}

/// Inches to whole pixels; the epsilon keeps values like 7.0 in * 96 exact.
fn to_px(inches: f64) -> u32 {
    (inches * PIXELS_PER_INCH + 1e-6).floor().max(0.0) as u32
}

pub fn register(r: &mut Registry) {
    r.command("File > Print...", print_command);
    r.shortcut(Key::Char('p'), Mods::CTRL, "File > Print...");
    r.command("File > Print Preview", preview_command);
    r.command("File > Page Setup...", page_setup_command);
}

/// Pages the current document needs under the current page setup.
pub(crate) fn page_count(st: &State) -> u32 {
    let (pw, ph) = st.page_setup.printable_px().unwrap_or((1, 1));
    st.doc.width.div_ceil(pw) * st.doc.height.div_ceil(ph)
}

/// The view mode while Print Preview is shown.
pub(crate) fn preview_mode(st: &State) -> Option<ViewMode> {
    st.print_preview.map(|page| ViewMode::PrintPreview { page, pages: page_count(st) })
}

// ----- Print Preview (buttons: Print..., Next Page, Prev Page, Zoom In, Zoom Out, Close)

/// Preview magnifications in percent of the printed size; it opens on the
/// second so a whole sheet fits a normal workspace.
const PREVIEW_ZOOMS: [u32; 4] = [25, 50, 100, 200];
const PREVIEW_START_ZOOM: usize = 1;

const BTN_PRINT: &str = "Print...";
const BTN_NEXT: &str = "Next Page";
const BTN_PREV: &str = "Prev Page";
const BTN_ZOOM_IN: &str = "Zoom In";
const BTN_ZOOM_OUT: &str = "Zoom Out";
const BTN_CLOSE: &str = "Close";

fn preview_command(st: &mut State) -> HResult<()> {
    // The preview takes over input, so a held press can never be released
    // on the canvas: cancel it (pixels restored) and settle the gesture, so
    // Close finds the image unchanged and no tool keeps ticking.
    crate::features::input_escape::cancel_gesture(st);
    st.held = None;
    st.stroke_base = None;
    st.airbrush = Default::default();
    st.print_preview = Some(1);
    st.preview_zoom = PREVIEW_START_ZOOM;
    Ok(())
}

/// Whether the preview replaces the normal window. Canvas input, menus and
/// shortcuts are ignored while it does.
pub(crate) fn preview_active(st: &State) -> bool {
    st.print_preview.is_some()
}

/// Key press while the preview is shown: Esc closes it, Page Down / Page Up
/// turn the page; every other key is swallowed.
pub(crate) fn preview_key(st: &mut State, key: Key) {
    let button = match key {
        Key::Escape => BTN_CLOSE,
        Key::PageDown => BTN_NEXT,
        Key::PageUp => BTN_PREV,
        _ => return,
    };
    // The keys map onto fixed buttons that exist while the preview is shown.
    let _ = preview_click(st, button);
}

/// The toolbar as `App::preview_buttons` reports it.
pub(crate) fn preview_buttons(st: &State) -> HResult<Vec<PreviewButton>> {
    let Some(page) = st.print_preview else {
        return Err(HarnessError::Invalid("Print Preview is not shown".into()));
    };
    let button = |label: &str, enabled: bool| PreviewButton { label: label.to_owned(), enabled };
    Ok(vec![
        button(BTN_PRINT, true),
        button(BTN_NEXT, page < page_count(st)),
        button(BTN_PREV, page > 1),
        button(BTN_ZOOM_IN, st.preview_zoom + 1 < PREVIEW_ZOOMS.len()),
        button(BTN_ZOOM_OUT, st.preview_zoom > 0),
        button(BTN_CLOSE, true),
    ])
}

/// `App::preview_click`: a disabled button does nothing, like a greyed one.
pub(crate) fn preview_click(st: &mut State, label: &str) -> HResult<()> {
    let buttons = preview_buttons(st)?;
    let Some(btn) = buttons.iter().find(|b| b.label == label) else {
        return Err(HarnessError::Invalid(format!("Print Preview has no button {label:?}")));
    };
    if !btn.enabled || !st.dialogs.is_empty() {
        return Ok(());
    }
    match label {
        BTN_PRINT => return print_command(st),
        BTN_NEXT => st.print_preview = st.print_preview.map(|p| p + 1),
        BTN_PREV => st.print_preview = st.print_preview.map(|p| p - 1),
        BTN_ZOOM_IN => st.preview_zoom += 1,
        BTN_ZOOM_OUT => st.preview_zoom -= 1,
        _ => st.print_preview = None,
    }
    Ok(())
}

/// The workspace while the preview is shown: the sheet at the preview zoom
/// on the grey background, with the current page's slice of the image inside
/// the margins. `None` when the preview is not shown.
pub(crate) fn render_preview(st: &State) -> Option<Image> {
    let page = st.print_preview?;
    let (ww, wh) = st.workspace_size;
    let mut out = Image { width: ww, height: wh, pixels: vec![crate::render::WORKSPACE_GREY; ww as usize * wh as usize] };
    let zoom = u64::from(PREVIEW_ZOOMS[st.preview_zoom.min(PREVIEW_ZOOMS.len() - 1)]);
    let ps = &st.page_setup;
    let (paper_w, paper_h) = ps.paper_inches();
    let scaled = |inches: f64| u64::from(to_px(inches)) * zoom / 100;
    let origin = i64::from(crate::render::CANVAS_MARGIN);
    let mut put = |x: u64, y: u64, c: Rgb| {
        let (x, y) = (x as i64 + origin, y as i64 + origin);
        if x < i64::from(ww) && y < i64::from(wh) {
            out.pixels[y as usize * ww as usize + x as usize] = c;
        }
    };
    for y in 0..scaled(paper_h) {
        for x in 0..scaled(paper_w) {
            put(x, y, Rgb::new(255, 255, 255));
        }
    }
    // Page `page` covers the image block (col, row) of the left-to-right,
    // top-to-bottom split.
    let (pw, ph) = ps.printable_px().unwrap_or((1, 1));
    let cols = st.doc.width.div_ceil(pw);
    let (col, row) = ((page - 1) % cols, (page - 1) / cols);
    let (x0, y0) = (col * pw, row * ph);
    let (slice_w, slice_h) = (pw.min(st.doc.width - x0), ph.min(st.doc.height - y0));
    let (ml, mt) = (scaled(ps.left), scaled(ps.top));
    // The page shows the composite, so floating content prints too. At 100%
    // and 200% the sheet is larger than the workspace; it is clipped to the
    // workspace (`put`) rather than scrolled, the simpler of the two options.
    let comp = st.render_composite();
    for dy in 0..u64::from(slice_h) * zoom / 100 {
        for dx in 0..u64::from(slice_w) * zoom / 100 {
            let (sx, sy) = (x0 + (dx * 100 / zoom) as u32, y0 + (dy * 100 / zoom) as u32);
            if let Some(c) = comp.get(sx as i32, sy as i32) {
                put(ml + dx, mt + dy, c);
            }
        }
    }
    Some(out)
}

// ----- Print

fn print_command(st: &mut State) -> HResult<()> {
    open_print_dialog(st, print_dialog);
    Ok(())
}

/// Help > Options > Print: the same Print dialog, but the shown topic goes
/// out as one page whatever the document needs.
pub(crate) fn print_topic_command(st: &mut State) -> HResult<()> {
    open_print_dialog(st, print_topic_dialog);
    Ok(())
}

fn open_print_dialog(st: &mut State, handler: crate::dialogs::DialogFn) {
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::Print,
        title: "Print".into(),
        text: String::new(),
        buttons: vec!["OK".into(), "Cancel".into()],
        fields: Vec::new(),
    };
    st.open_dialog(OpenDialog::new(dialog, handler));
}

fn print_dialog(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    let pages = page_count(st);
    send_to_printer(st, od, action, pages)
}

fn print_topic_dialog(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    send_to_printer(st, od, action, 1)
}

/// The Print dialog's reaction: OK sends `pages` pages to the printer (or
/// reports there is none), Cancel closes.
fn send_to_printer(st: &mut State, _od: &mut OpenDialog, action: DialogAction<'_>, pages: u32) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, .. } => Err(HarnessError::Invalid(format!("Print dialog has no field {field:?}"))),
        DialogAction::Click("OK") => {
            if st.services.printer {
                st.record_call(PlatformCall::Print { pages });
            } else {
                st.error_box("No printer is installed.");
            }
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

// ----- Page Setup

fn page_setup_command(st: &mut State) -> HResult<()> {
    let ps = &st.page_setup;
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::PageSetup,
        title: "Page Setup".into(),
        text: String::new(),
        buttons: vec!["OK".into(), "Cancel".into()],
        fields: vec![
            OpenDialog::field("Paper", &ps.paper),
            OpenDialog::field("Orientation", if ps.landscape { "Landscape" } else { "Portrait" }),
            OpenDialog::field("Left", &ps.left.to_string()),
            OpenDialog::field("Right", &ps.right.to_string()),
            OpenDialog::field("Top", &ps.top.to_string()),
            OpenDialog::field("Bottom", &ps.bottom.to_string()),
        ],
    };
    st.open_dialog(OpenDialog::new(dialog, page_setup_dialog));
    Ok(())
}

fn page_setup_dialog(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("OK") => match read_setup(od) {
            Ok(ps) => {
                st.page_setup = ps;
                Ok(Outcome::Close)
            }
            Err(msg) => {
                st.error_box(&msg);
                Ok(Outcome::Keep)
            }
        },
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

/// Validate the dialog's fields; the error is the message to show.
fn read_setup(od: &OpenDialog) -> Result<PageSetup, String> {
    let get = |name: &str| od.dialog.field(name).unwrap_or("").trim().to_owned();
    let paper = PAPERS
        .iter()
        .map(|p| p.0)
        .find(|n| n.eq_ignore_ascii_case(&get("Paper")))
        .ok_or_else(|| "Choose a paper size from the list.".to_owned())?;
    let landscape = match get("Orientation").to_lowercase().as_str() {
        "portrait" => false,
        "landscape" => true,
        _ => return Err("Choose Portrait or Landscape.".into()),
    };
    let margin = |name: &str| {
        get(name)
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or_else(|| format!("The {} margin must be a number of inches, 0 or more.", name.to_lowercase()))
    };
    let ps = PageSetup {
        paper: paper.to_owned(),
        landscape,
        left: margin("Left")?,
        right: margin("Right")?,
        top: margin("Top")?,
        bottom: margin("Bottom")?,
    };
    if ps.printable_px().is_none() {
        return Err("The margins leave no room to print on the page.".into());
    }
    Ok(ps)
}

#[cfg(test)]
mod tests {
    use crate::{App, Key, Mods, PlatformCall, PointerKind, Rgb, StartOptions, Surface, ViewMode};

    fn app(tag: &str, w: u32, h: u32) -> App {
        let mut a =
            App::start(StartOptions::new(std::env::temp_dir().join(format!("brushling-print-{}-{tag}", std::process::id()))))
                .unwrap();
        a.new_document(w, h, Rgb::new(255, 255, 255)).unwrap();
        a
    }

    fn enabled(a: &App, label: &str) -> bool {
        a.preview_buttons().unwrap().iter().find(|b| b.label == label).unwrap().enabled
    }

    #[test]
    fn page_buttons_follow_the_page_and_close_restores_the_view() {
        let mut a = app("pages", 2000, 100);
        a.menu("File > Print Preview").unwrap();
        let labels: Vec<_> = a.preview_buttons().unwrap().into_iter().map(|b| b.label).collect();
        assert_eq!(labels, ["Print...", "Next Page", "Prev Page", "Zoom In", "Zoom Out", "Close"]);
        assert!(enabled(&a, "Next Page") && !enabled(&a, "Prev Page"));
        a.preview_click("Prev Page").unwrap();
        a.preview_click("Next Page").unwrap();
        a.preview_click("Next Page").unwrap();
        assert_eq!(a.view_mode().unwrap(), ViewMode::PrintPreview { page: 3, pages: 3 });
        assert!(!enabled(&a, "Next Page") && enabled(&a, "Prev Page"));
        a.preview_click("Next Page").unwrap();
        assert_eq!(a.view_mode().unwrap(), ViewMode::PrintPreview { page: 3, pages: 3 });
        a.preview_click("Close").unwrap();
        assert_eq!(a.view_mode().unwrap(), ViewMode::Normal);
        assert!(a.preview_buttons().is_err());
    }

    #[test]
    fn zoom_buttons_stop_at_the_ends() {
        let mut a = app("zoom", 100, 100);
        a.menu("File > Print Preview").unwrap();
        while enabled(&a, "Zoom In") {
            a.preview_click("Zoom In").unwrap();
        }
        assert!(enabled(&a, "Zoom Out"));
        while enabled(&a, "Zoom Out") {
            a.preview_click("Zoom Out").unwrap();
        }
        assert!(enabled(&a, "Zoom In"));
    }

    #[test]
    fn canvas_input_is_blocked_and_the_page_is_rendered() {
        let mut a = app("input", 100, 100);
        a.set_colors(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)).unwrap();
        a.select_tool(crate::Tool::Pencil).unwrap();
        a.set_pixel(0, 0, Rgb::new(255, 0, 0)).unwrap();
        a.menu("File > Print Preview").unwrap();
        a.pointer(PointerKind::Press(crate::Button::Left), crate::Pos::Image(20, 20), Mods::NONE).unwrap();
        a.pointer(PointerKind::Release(crate::Button::Left), crate::Pos::Image(20, 20), Mods::NONE).unwrap();
        a.key(Key::Char('a'), Mods::CTRL).unwrap();
        a.menu("Edit > Select All").unwrap();
        assert!(a.selection().unwrap().is_none());
        assert!(!a.document().unwrap().dirty);
        let img = a.render(Surface::Workspace).unwrap();
        // 50% preview: the sheet is white, the image sits inside the margins (0.75 in left, 1 in top).
        let at = |x: u32, y: u32| img.pixels[(y * img.width + x) as usize];
        let off = crate::render::CANVAS_MARGIN as u32;
        assert_eq!(at(off + 2, off + 2), Rgb::new(255, 255, 255));
        assert_eq!(at(off + 36, off + 48), Rgb::new(255, 0, 0));
        assert_eq!(at(img.width - 1, img.height - 1), crate::render::WORKSPACE_GREY);
        a.key(Key::Escape, Mods::NONE).unwrap();
        assert_eq!(a.view_mode().unwrap(), ViewMode::Normal);
    }

    #[test]
    fn print_button_opens_the_print_dialog_and_records_the_page_count() {
        let mut a = app("print", 2000, 100);
        a.menu("File > Print Preview").unwrap();
        a.preview_click("Print...").unwrap();
        a.dialog_click("OK").unwrap();
        assert_eq!(a.platform_calls().unwrap(), [PlatformCall::Print { pages: 3 }]);
    }

    #[test]
    fn preview_settles_a_held_airbrush_gesture() {
        let mut a = app("held", 100, 100);
        a.set_colors(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)).unwrap();
        a.select_tool(crate::Tool::Airbrush).unwrap();
        let before = a.document().unwrap();
        a.pointer(PointerKind::Press(crate::Button::Left), crate::Pos::Image(50, 50), Mods::NONE).unwrap();
        a.menu("File > Print Preview").unwrap();
        a.pointer(PointerKind::Release(crate::Button::Left), crate::Pos::Image(50, 50), Mods::NONE).unwrap();
        a.advance(1000).unwrap();
        a.preview_click("Close").unwrap();
        let white = Rgb::new(255, 255, 255);
        assert!((0..100).all(|y| (0..100).all(|x| a.pixel(x, y).unwrap() == white)), "image changed");
        assert_eq!(a.document().unwrap().dirty, before.dirty);
        // Nothing keeps spraying after Close.
        a.advance(1000).unwrap();
        assert!((0..100).all(|y| (0..100).all(|x| a.pixel(x, y).unwrap() == white)), "tool still ticking");
    }
}
