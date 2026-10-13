//! Feature image.stretch_skew: Image > Stretch/Skew... (Ctrl+W). Percent
//! stretch (1..=500) and degree skew (0..=89), applied in the order
//! horizontal stretch, vertical stretch, horizontal skew, vertical skew with
//! nearest-neighbour sampling. Scoping (selection or whole image) is handled
//! by `selection_image_ops_scope::with_target_pixels`.

use super::selection_image_ops_scope::{with_target_pixels, Raster};
use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::doc::MAX_DIM;
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const STRETCH_H: &str = "Stretch Horizontal";
const STRETCH_V: &str = "Stretch Vertical";
const SKEW_H: &str = "Skew Horizontal";
const SKEW_V: &str = "Skew Vertical";

const STRETCH_MESSAGE: &str = "Enter a whole number from 1 to 500.";
const SKEW_MESSAGE: &str = "Enter a whole number from 0 to 89.";
const TOO_LARGE_MESSAGE: &str = "There is not enough memory or resources to complete this operation.";

pub fn register(r: &mut Registry) {
    r.command("Image > Stretch/Skew...", open_dialog);
    r.shortcut(Key::Char('w'), Mods::CTRL, "Image > Stretch/Skew...");
}

fn open_dialog(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::StretchSkew,
        title: "Stretch and Skew".to_owned(),
        text: String::new(),
        buttons: vec!["OK".to_owned(), "Cancel".to_owned()],
        fields: vec![
            OpenDialog::field(STRETCH_H, "100"),
            OpenDialog::field(STRETCH_V, "100"),
            OpenDialog::field(SKEW_H, "0"),
            OpenDialog::field(SKEW_V, "0"),
        ],
    };
    st.open_dialog(OpenDialog::new(dialog, handle));
    Ok(())
}

fn handle(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("OK") => confirm(st, od),
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

/// Parse a whole number without sign or decimals and check its range.
fn parse_in(text: &str, lo: u32, hi: u32) -> Option<u32> {
    let t = text.trim();
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse::<u32>().ok().filter(|v| (lo..=hi).contains(v))
}

fn confirm(st: &mut State, od: &OpenDialog) -> HResult<Outcome> {
    let mut nums = [0u32; 4];
    let rules = [
        (STRETCH_H, 1, 500, STRETCH_MESSAGE),
        (STRETCH_V, 1, 500, STRETCH_MESSAGE),
        (SKEW_H, 0, 89, SKEW_MESSAGE),
        (SKEW_V, 0, 89, SKEW_MESSAGE),
    ];
    for (i, (name, lo, hi, msg)) in rules.into_iter().enumerate() {
        match parse_in(od.dialog.field(name).unwrap_or_default(), lo, hi) {
            Some(v) => nums[i] = v,
            None => {
                st.error_box(msg);
                return Ok(Outcome::Keep);
            }
        }
    }
    let [sh, sv, kh, kv] = nums;
    if (sh, sv, kh, kv) == (100, 100, 0, 0) {
        return Ok(Outcome::Close);
    }
    match apply(st, sh, sv, kh, kv) {
        Ok(()) => Ok(Outcome::Close),
        Err(HarnessError::Invalid(m)) if m == TOO_LARGE_MESSAGE => {
            st.error_box(TOO_LARGE_MESSAGE);
            Ok(Outcome::Keep)
        }
        Err(e) => Err(e),
    }
}

/// Stretched length: max(1, floor(len * percent / 100 + 0.5)).
fn stretched_len(len: u32, percent: u32) -> u64 {
    ((len as u64 * percent as u64 * 2 + 100) / 200).max(1)
}

/// Shift of a line `distance` lines away from the one that stays, for a
/// skew of `deg` degrees: floor(distance * tan(deg) + 0.5).
fn shift(distance: u32, deg: u32) -> u32 {
    // The epsilon keeps exact .5 boundaries (45 degrees) from falling the wrong way.
    (distance as f64 * f64::from(deg).to_radians().tan() + 0.5 + 1e-9).floor() as u32
}

fn apply(st: &mut State, sh: u32, sv: u32, kh: u32, kv: u32) -> HResult<()> {
    with_target_pixels(st, |r, bg| {
        // Work out every size first so an oversized result allocates nothing.
        let too_large = || HarnessError::Invalid(TOO_LARGE_MESSAGE.into());
        let max = u64::from(MAX_DIM);
        let w1 = stretched_len(r.width, sh);
        let h1 = stretched_len(r.height, sv);
        if w1 > max || h1 > max {
            return Err(too_large());
        }
        let (w1, h1) = (w1 as u32, h1 as u32);
        let w2 = u64::from(w1) + u64::from(shift(h1 - 1, kh));
        if w2 > max {
            return Err(too_large());
        }
        let h2 = u64::from(h1) + u64::from(shift(w2 as u32 - 1, kv));
        if h2 > max {
            return Err(too_large());
        }

        let mut out: Raster = r.stretched(w1, h1);
        if kh > 0 {
            out = skew_horizontal(&out, kh, bg);
        }
        if kv > 0 {
            out = skew_vertical(&out, kv, bg);
        }
        Ok(out)
    })
}

/// Row `y` moves right by floor((H-1-y) tan a); the bottom row stays.
fn skew_horizontal(src: &Raster, deg: u32, bg: Rgb) -> Raster {
    let (w, h) = (src.width, src.height);
    src.remap(w + shift(h - 1, deg), h, bg, |x, y| {
        let s = shift(h - 1 - y, deg);
        x.checked_sub(s).filter(|sx| *sx < w).map(|sx| (sx, y))
    })
}

/// Column `x` moves down by floor((W-1-x) tan a); the right column stays.
fn skew_vertical(src: &Raster, deg: u32, bg: Rgb) -> Raster {
    let (w, h) = (src.width, src.height);
    src.remap(w, h + shift(w - 1, deg), bg, |x, y| {
        let t = shift(w - 1 - x, deg);
        y.checked_sub(t).filter(|sy| *sy < h).map(|sy| (x, sy))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_rejects_signs_and_fractions() {
        assert_eq!(parse_in("250", 1, 500), Some(250));
        for bad in ["-5", "1.5", "", "abc", "0", "501", "+3"] {
            assert_eq!(parse_in(bad, 1, 500), None, "{bad}");
        }
        assert_eq!(parse_in("0", 0, 89), Some(0));
    }

    #[test]
    fn lengths_round_half_up() {
        assert_eq!(stretched_len(3, 50), 2);
        assert_eq!(stretched_len(50, 1), 1);
        assert_eq!(stretched_len(4, 50), 2);
    }

    #[test]
    fn forty_five_degrees_shifts_one_per_line() {
        assert_eq!((shift(0, 45), shift(1, 45), shift(2, 45)), (0, 1, 2));
    }
}
