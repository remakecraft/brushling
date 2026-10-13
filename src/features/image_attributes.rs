//! Feature image.attributes: Image > Attributes... (Ctrl+E) sets the canvas
//! size (pixels, inches or cm at 96 dpi) and the colour mode, and remembers
//! the size for File > New. Dialog fields: `Width`, `Height`, `Units`,
//! `Colors`; buttons OK, Cancel, Default.

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::doc::MAX_DIM;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const DPI: u64 = 96;
const CM_PER_INCH: f64 = 2.54;
const DEFAULT_PX: (u32, u32) = (512, 384);
const INVALID_SIZE: &str = "Enter a size from 1 to 9999 pixels.";

pub fn register(r: &mut Registry) {
    r.command("Image > Attributes...", open_attributes);
    r.shortcut(Key::Char('e'), Mods::CTRL, "Image > Attributes...");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Unit {
    Inches,
    Cm,
    #[default]
    Pixels,
}

impl Unit {
    fn label(self) -> &'static str {
        match self {
            Unit::Inches => "Inches",
            Unit::Cm => "Cm",
            Unit::Pixels => "Pixels",
        }
    }

    fn parse(s: &str) -> Option<Unit> {
        [Unit::Inches, Unit::Cm, Unit::Pixels].into_iter().find(|u| u.label() == s)
    }

    /// Text shown for a pixel count: integers in pixels, half-up 2 decimals otherwise.
    fn show(self, px: u32) -> String {
        let px = px as u64;
        let hundredths = match self {
            Unit::Pixels => return px.to_string(),
            // px / 96 * 100, rounded half up.
            Unit::Inches => (px * 200 + DPI) / (2 * DPI),
            // px / 96 * 2.54 * 100 = px * 254 / 96, rounded half up.
            Unit::Cm => (px * 508 + DPI) / (2 * DPI),
        };
        format!("{}.{:02}", hundredths / 100, hundredths % 100)
    }

    /// Pixels for a typed value, or `None` when it is not a valid size.
    fn to_px(self, text: &str) -> Option<u32> {
        let v: f64 = text.trim().parse().ok()?;
        if !v.is_finite() || v <= 0.0 {
            return None;
        }
        let raw = match self {
            // A size in pixels is a whole number.
            Unit::Pixels if v.fract() != 0.0 => return None,
            Unit::Pixels => v,
            Unit::Inches => v * DPI as f64,
            Unit::Cm => v / CM_PER_INCH * DPI as f64,
        };
        let px = (raw + 0.5 + 1e-9).floor();
        if px > MAX_DIM as f64 {
            return None;
        }
        // Physical units never round a positive value down to nothing.
        match self {
            Unit::Pixels if px < 1.0 => None,
            _ => Some((px as u32).max(1)),
        }
    }
}

/// Exact pixel size behind the dialog, so changing units never loses it.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttrDraft {
    w: u32,
    h: u32,
    /// The Width / Height text was typed since it was last derived from `w` / `h`.
    w_edited: bool,
    h_edited: bool,
}

fn field_text(od: &OpenDialog, name: &str) -> String {
    od.dialog.fields.iter().find(|f| f.name == name).map(|f| f.value.clone()).unwrap_or_default()
}

fn unit_of(od: &OpenDialog) -> Unit {
    Unit::parse(&field_text(od, "Units")).unwrap_or_default()
}

fn show_size(od: &mut OpenDialog, d: &AttrDraft, unit: Unit) {
    if let Some(f) = od.field_mut("Width") {
        f.value = unit.show(d.w);
    }
    if let Some(f) = od.field_mut("Height") {
        f.value = unit.show(d.h);
    }
}

/// Take typed text into the draft. Returns false when a typed value is not a
/// valid size (the draft keeps its previous value for that side).
fn sync(st: &mut State, od: &OpenDialog) -> bool {
    let unit = unit_of(od);
    let d = &mut st.attr_draft;
    let mut ok = true;
    for (edited, px, name) in [(&mut d.w_edited, &mut d.w, "Width"), (&mut d.h_edited, &mut d.h, "Height")] {
        if !*edited {
            continue;
        }
        // Only a valid entry settles the side; an invalid one stays pending
        // so the next OK reports it again instead of applying the old size.
        match unit.to_px(&field_text(od, name)) {
            Some(v) => {
                *px = v;
                *edited = false;
            }
            None => ok = false,
        }
    }
    ok
}

fn open_attributes(st: &mut State) -> HResult<()> {
    let (w, h) = (st.doc.width, st.doc.height);
    st.attr_draft = AttrDraft { w, h, w_edited: false, h_edited: false };
    let mode = match st.doc.color_mode {
        ColorMode::BlackAndWhite => "Black and white",
        ColorMode::Colors => "Colors",
    };
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::Attributes,
        title: "Attributes".to_owned(),
        text: String::new(),
        buttons: ["OK", "Cancel", "Default"].map(str::to_owned).to_vec(),
        fields: vec![
            OpenDialog::field("Width", &w.to_string()),
            OpenDialog::field("Height", &h.to_string()),
            OpenDialog::field("Units", Unit::Pixels.label()),
            OpenDialog::field("Colors", mode),
        ],
    };
    st.open_dialog(OpenDialog::new(dialog, attributes_handler));
    Ok(())
}

fn attributes_handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            match field {
                "Units" => {
                    let Some(new) = Unit::parse(value) else {
                        return Err(HarnessError::Invalid(format!("unknown units {value:?}")));
                    };
                    // Pending typed values are read in the old units first; an
                    // invalid one is reported and the units stay as they are.
                    if !sync(st, od) {
                        st.error_box(INVALID_SIZE);
                        return Ok(Outcome::Keep);
                    }
                    store_field(od, field, value)?;
                    let d = st.attr_draft;
                    show_size(od, &d, new);
                }
                "Colors" => {
                    if !matches!(value, "Black and white" | "Colors") {
                        return Err(HarnessError::Invalid(format!("unknown colour mode {value:?}")));
                    }
                    store_field(od, field, value)?;
                }
                "Width" | "Height" => {
                    store_field(od, field, value)?;
                    if field == "Width" {
                        st.attr_draft.w_edited = true;
                    } else {
                        st.attr_draft.h_edited = true;
                    }
                }
                _ => store_field(od, field, value)?,
            }
            Ok(Outcome::Keep)
        }
        DialogAction::Click("Cancel") => Ok(Outcome::Close),
        DialogAction::Click("Default") => {
            st.attr_draft = AttrDraft { w: DEFAULT_PX.0, h: DEFAULT_PX.1, w_edited: false, h_edited: false };
            store_field(od, "Units", Unit::Pixels.label())?;
            store_field(od, "Colors", "Colors")?;
            let d = st.attr_draft;
            show_size(od, &d, Unit::Pixels);
            Ok(Outcome::Keep)
        }
        DialogAction::Click(_) => {
            if !sync(st, od) {
                st.error_box(INVALID_SIZE);
                return Ok(Outcome::Keep);
            }
            let mode = if field_text(od, "Colors") == "Black and white" {
                ColorMode::BlackAndWhite
            } else {
                ColorMode::Colors
            };
            let (w, h) = (st.attr_draft.w, st.attr_draft.h);
            apply(st, w, h, mode)?;
            Ok(Outcome::Close)
        }
    }
}

/// The pixels of a `ow` x `oh` image resized to `w` x `h`: cropped at the
/// right/bottom, or extended there with `fill`; the top-left stays in place.
pub(crate) fn resized_pixels(src: &[Rgb], ow: u32, oh: u32, w: u32, h: u32, fill: Rgb) -> Vec<Rgb> {
    let (ow, oh, w, h) = (ow as usize, oh as usize, w as usize, h as usize);
    let mut px = vec![fill; w * h];
    let n = ow.min(w);
    for y in 0..oh.min(h) {
        px[y * w..y * w + n].copy_from_slice(&src[y * ow..y * ow + n]);
    }
    px
}

/// Change size and mode as one undo step (none when neither changes) and
/// remember the size for File > New.
fn apply(st: &mut State, w: u32, h: u32, mode: ColorMode) -> HResult<()> {
    crate::doc::check_size(w, h)?;
    st.remember_new_size(w, h)?;
    let size_changed = (w, h) != (st.doc.width, st.doc.height);
    if !size_changed && mode == st.doc.color_mode {
        return Ok(());
    }
    st.end_selection();
    st.doc.push_undo();
    let fill = crate::doc::snap_colour(mode, st.doc.palette.as_deref(), st.bg);
    if size_changed {
        st.doc.pixels = resized_pixels(&st.doc.pixels, st.doc.width, st.doc.height, w, h, fill);
        st.doc.width = w;
        st.doc.height = h;
        crate::features::ui_main_window::refresh_workspace(st);
    }
    st.doc.color_mode = mode;
    if mode == ColorMode::BlackAndWhite {
        reduce_to_black_and_white(&mut st.doc.pixels);
    }
    Ok(())
}

/// Map every pixel through the 1-bit threshold rule of format.color_reduction.
fn reduce_to_black_and_white(pixels: &mut [Rgb]) {
    use crate::features::format_color_reduction::{fixed_palette, reduce};
    let bw = fixed_palette(1);
    let idx = reduce(pixels, &bw, true);
    for (p, i) in pixels.iter_mut().zip(idx) {
        *p = bw[i as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_display_and_parse() {
        assert_eq!(Unit::Inches.show(512), "5.33");
        assert_eq!(Unit::Cm.show(512), "13.55");
        assert_eq!(Unit::Cm.show(384), "10.16");
        assert_eq!(Unit::Inches.to_px("1"), Some(96));
        assert_eq!(Unit::Cm.to_px("2.54"), Some(96));
        assert_eq!(Unit::Pixels.to_px("0"), None);
        assert_eq!(Unit::Pixels.to_px("10000"), None);
        assert_eq!(Unit::Inches.to_px("0.001"), Some(1));
        assert_eq!(Unit::Pixels.to_px("10.6"), None);
        assert_eq!(Unit::Pixels.to_px("12"), Some(12));
    }

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-attr-{}", std::process::id()));
        let mut st = State::new(crate::settings::Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        st.new_default_document().unwrap();
        st
    }

    #[test]
    fn invalid_size_is_not_accepted_on_a_second_ok() {
        let mut st = state();
        let (w, h) = (st.doc.width, st.doc.height);
        open_attributes(&mut st).unwrap();
        st.dialog_set("Width", "10000").unwrap();
        st.dialog_click("OK").unwrap();
        // The error box is on top; dismiss it and press OK again.
        st.dialog_click("OK").unwrap();
        assert_eq!(st.top_dialog().map(|d| d.kind), Some(DialogKind::Attributes));
        // The second OK must report the error again, not apply the old size.
        st.dialog_click("OK").unwrap();
        assert_eq!(st.top_dialog().map(|d| d.kind), Some(DialogKind::MessageBox));
        assert_eq!((st.doc.width, st.doc.height), (w, h));
    }

    #[test]
    fn invalid_entry_blocks_a_units_switch() {
        let mut st = state();
        open_attributes(&mut st).unwrap();
        st.dialog_set("Width", "abc").unwrap();
        st.dialog_set("Units", "Inches").unwrap();
        st.dialog_click("OK").unwrap();
        let d = st.top_dialog().unwrap();
        assert_eq!(d.field("Units"), Some("Pixels"));
    }
}
