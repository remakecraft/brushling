//! Feature image.flip_rotate: Image > Flip/Rotate... (Ctrl+R) opens a dialog
//! that mirrors the image or rotates it clockwise by 90, 180 or 270 degrees.
//! See `src/README.md`.

use super::selection_image_ops_scope::with_target_pixels;
use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const ACTIONS: [&str; 3] = ["Flip horizontal", "Flip vertical", "Rotate by angle"];
const ANGLES: [&str; 3] = ["90", "180", "270"];

pub fn register(r: &mut Registry) {
    r.command("Image > Flip/Rotate...", open_dialog);
    r.shortcut(Key::Char('r'), Mods::CTRL, "Image > Flip/Rotate...");
}

fn open_dialog(st: &mut State) -> HResult<()> {
    let mut angle = OpenDialog::field("Angle", ANGLES[0]);
    angle.enabled = false;
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::FlipRotate,
        title: "Flip and Rotate".to_owned(),
        text: String::new(),
        buttons: vec!["OK".to_owned(), "Cancel".to_owned()],
        fields: vec![OpenDialog::field("Action", ACTIONS[0]), angle],
    };
    st.open_dialog(OpenDialog::new(dialog, handle));
    Ok(())
}

fn handle(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            let allowed: &[&str] = match field {
                "Action" => &ACTIONS,
                "Angle" => &ANGLES,
                _ => &[],
            };
            if !allowed.is_empty() && !allowed.contains(&value) {
                return Err(HarnessError::Invalid(format!("{value:?} is not a choice of {field:?}")));
            }
            store_field(od, field, value)?;
            if field == "Action" {
                let rotate = value == ACTIONS[2];
                if let Some(a) = od.field_mut("Angle") {
                    a.enabled = rotate;
                }
            }
            Ok(Outcome::Keep)
        }
        DialogAction::Click("OK") => {
            let value = |name: &str| od.dialog.field(name).unwrap_or_default().to_owned();
            let transform = match value("Action").as_str() {
                "Flip horizontal" => Transform::FlipH,
                "Flip vertical" => Transform::FlipV,
                _ => match value("Angle").as_str() {
                    "180" => Transform::Rotate180,
                    "270" => Transform::Rotate270,
                    _ => Transform::Rotate90,
                },
            };
            apply(st, transform)?;
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

#[derive(Clone, Copy)]
enum Transform {
    FlipH,
    FlipV,
    Rotate90,
    Rotate180,
    Rotate270,
}

/// Source coordinate of destination pixel `(x, y)` for a `w` x `h` source.
fn source(t: Transform, x: u32, y: u32, w: u32, h: u32) -> (u32, u32) {
    match t {
        Transform::FlipH => (w - 1 - x, y),
        Transform::FlipV => (x, h - 1 - y),
        Transform::Rotate90 => (y, h - 1 - x),
        Transform::Rotate180 => (w - 1 - x, h - 1 - y),
        Transform::Rotate270 => (w - 1 - y, x),
    }
}

/// Transform the selection's content (it stays floating, canvas size
/// unchanged) or, without a selection, the whole image.
fn apply(st: &mut State, t: Transform) -> HResult<()> {
    with_target_pixels(st, |r, bg| {
        let (w, h) = (r.width, r.height);
        let (nw, nh) = match t {
            Transform::Rotate90 | Transform::Rotate270 => (h, w),
            _ => (w, h),
        };
        Ok(r.remap(nw, nh, bg, |x, y| Some(source(t, x, y, w, h))))
    })
}
