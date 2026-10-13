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

fn preview_command(st: &mut State) -> HResult<()> {
    st.print_preview = Some(1);
    Ok(())
}

/// Next Page; a no-op on the last page (the button is disabled there).
#[allow(dead_code)]
pub(crate) fn preview_next(st: &mut State) {
    let pages = page_count(st);
    if let Some(page) = st.print_preview.as_mut() {
        *page = (*page + 1).min(pages);
    }
}

/// Prev Page; a no-op on the first page.
#[allow(dead_code)]
pub(crate) fn preview_prev(st: &mut State) {
    if let Some(page) = st.print_preview.as_mut() {
        *page = page.saturating_sub(1).max(1);
    }
}

/// Close: back to the normal view, the image untouched.
#[allow(dead_code)]
pub(crate) fn preview_close(st: &mut State) {
    st.print_preview = None;
}

/// The preview's Print... button.
#[allow(dead_code)]
pub(crate) fn preview_print(st: &mut State) -> HResult<()> {
    print_command(st)
}

// ----- Print

fn print_command(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        kind: DialogKind::Print,
        title: "Print".into(),
        text: String::new(),
        buttons: vec!["OK".into(), "Cancel".into()],
        fields: Vec::new(),
    };
    st.open_dialog(OpenDialog::new(dialog, print_dialog));
    Ok(())
}

fn print_dialog(st: &mut State, _od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, .. } => Err(HarnessError::Invalid(format!("Print dialog has no field {field:?}"))),
        DialogAction::Click("OK") => {
            if st.services.printer {
                let pages = page_count(st);
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
