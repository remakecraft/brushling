//! Feature edit.copy_to_paste_from: Edit > Copy To... writes the selection as
//! a BMP through the Save As rules without touching the document; Edit >
//! Paste From... reads a file and pastes it like a clipboard image.

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::doc::Document;
use crate::features::edit_clipboard::{paste_image, selection_image};
use crate::features::file_save::{plan, resolve, type_depth, type_label, write_atomically};
use crate::features::format_bmp_write::{encode_bmp, encode_bmp_indexed};
use crate::features::format_color_reduction::WARNING;
use crate::formats::{decode, CANNOT_OPEN};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;
use std::path::{Path, PathBuf};

const FILTER_BMP: &str = "Bitmap files (*.bmp;*.dib)";
const FILTER_ALL: &str = "All files (*.*)";

pub fn register(r: &mut Registry) {
    r.command_if("Edit > Copy To...", show_copy_to, Some(State::has_selection));
    r.command("Edit > Paste From...", show_paste_from);
}

fn show_copy_to(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::FileSave,
        title: "Copy To".to_owned(),
        text: String::new(),
        buttons: vec!["Save".to_owned(), "Cancel".to_owned()],
        fields: vec![
            OpenDialog::field("File name", "untitled"),
            OpenDialog::field("Save as type", type_label(st.doc.source_depth)),
        ],
    };
    st.open_dialog(OpenDialog::new(dialog, copy_to_handler));
    Ok(())
}

fn copy_to_handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            if field == "Save as type" && type_depth(value).is_none() {
                return Err(HarnessError::Invalid(format!("unknown save type {value:?}")));
            }
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("Save") => {
            let typed = od.dialog.field("File name").unwrap_or("").trim().to_owned();
            if typed.trim_matches('"').trim().is_empty() {
                return Err(HarnessError::Invalid("no file name entered".into()));
            }
            let depth = od.dialog.field("Save as type").and_then(type_depth).unwrap_or(24);
            let path = resolve(st, &typed);
            if let Some(img) = selection_image(st) {
                request_copy_to(st, path, depth, img);
            }
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

/// The selection image as a throwaway document, so the Save As colour rules
/// (own palette at the source depth, fixed palette otherwise) apply to it.
fn scratch_document(st: &State, img: &Image) -> HResult<Document> {
    Document::from_image(img.clone(), None, st.doc.source_depth, st.doc.palette.clone())
}

fn request_copy_to(st: &mut State, path: PathBuf, depth: u32, img: Image) {
    let lossy = scratch_document(st, &img).ok().and_then(|d| plan(&d, depth)).is_some_and(|p| p.lossy);
    if lossy {
        st.pending_copy_to = Some((path, depth, img));
        st.message_box("Brushling", WARNING, &["OK", "Cancel"], Some(warning_answered));
        return;
    }
    write_copy(st, &path, depth, &img);
}

fn warning_answered(st: &mut State, button: &str) -> HResult<()> {
    let pending = st.pending_copy_to.take();
    if let ("OK", Some((path, depth, img))) = (button, pending) {
        write_copy(st, &path, depth, &img);
    }
    Ok(())
}

/// Encode and write; the document (path, title, dirty flag, pixels) is left
/// alone. Failures open the same box as Save.
fn write_copy(st: &mut State, path: &Path, depth: u32, img: &Image) {
    let bytes = scratch_document(st, img).and_then(|d| match plan(&d, depth) {
        None => encode_bmp(&img.pixels, img.width, img.height, 24, &[]),
        Some(p) => encode_bmp_indexed(&p.indices, img.width, img.height, depth, &p.palette),
    });
    let result = bytes.map_err(|e| format!("{e:?}")).and_then(|b| write_atomically(path, &b));
    if let Err(e) = result {
        st.error_box(&format!("The file could not be saved: {e}"));
    }
}

fn show_paste_from(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::FileOpen,
        title: "Paste From".to_owned(),
        text: String::new(),
        buttons: vec!["Open".to_owned(), "Cancel".to_owned()],
        fields: vec![OpenDialog::field("File name", ""), OpenDialog::field("Files of type", FILTER_BMP)],
    };
    st.open_dialog(OpenDialog::new(dialog, paste_from_handler));
    Ok(())
}

fn paste_from_handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            if field == "Files of type" && ![FILTER_BMP, FILTER_ALL].contains(&value) {
                return Err(HarnessError::Invalid(format!("unknown file type {value:?}")));
            }
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("Open") => {
            let name = od.dialog.field("File name").unwrap_or("").trim().trim_matches('"').trim().to_owned();
            if name.is_empty() {
                return Err(HarnessError::Invalid("no file name entered".into()));
            }
            paste_from_file(st, &PathBuf::from(name))?;
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

/// Read `path` with the registered decoders and paste it; read errors show
/// the same boxes as File > Open and leave everything as it was.
fn paste_from_file(st: &mut State, path: &Path) -> HResult<()> {
    let shown = path.display().to_string();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => {
            st.error_box(&format!("{shown}\n{CANNOT_OPEN}"));
            return Ok(());
        }
    };
    match decode(&bytes) {
        Ok(d) => paste_image(st, d.image),
        Err(e) => {
            st.error_box(&format!("{shown}\n{}", e.message));
            Ok(())
        }
    }
}
