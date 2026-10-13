//! Feature file.open: File > Open... (Ctrl+O) and open errors.
//!
//! The dialog only collects a path; decoding and the error boxes live in
//! `formats.rs`. The save-changes check in front of the dialog and the
//! recent-file list belong to other features and hook into [`open_file`].

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::features::file_save_changes_prompt::{guard, Pending};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;
use std::path::{Path, PathBuf};

const FILTER_BMP: &str = "Bitmap files (*.bmp;*.dib)";
const FILTER_ALL: &str = "All files (*.*)";

pub fn register(r: &mut Registry) {
    r.command("File > Open...", open_command);
    r.shortcut(Key::Char('o'), Mods::CTRL, "File > Open...");
}

/// File > Open...: the save-changes check, then the dialog.
fn open_command(st: &mut State) -> HResult<()> {
    guard(st, Pending::Open)
}

/// Open the platform open-file dialog.
pub(crate) fn show_open_dialog(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        disabled_buttons: Vec::new(),
        kind: DialogKind::FileOpen,
        title: "Open".to_owned(),
        text: String::new(),
        buttons: vec!["Open".to_owned(), "Cancel".to_owned()],
        fields: vec![OpenDialog::field("File name", ""), OpenDialog::field("Files of type", FILTER_BMP)],
    };
    st.open_dialog(OpenDialog::new(dialog, handler));
    Ok(())
}

/// Replace the document with the file at `path` (re-reading it even when it
/// is already open). Unreadable or undecodable files show their message box
/// and keep the current document; returns whether the file was opened. The
/// recent-file list (file.recent) is updated from here on success.
pub(crate) fn open_file(st: &mut State, path: &Path) -> HResult<bool> {
    let opened = st.open_path_checked(path)?;
    if opened {
        crate::features::file_recent::add(st, path);
    }
    Ok(opened)
}

fn handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
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
            open_file(st, &PathBuf::from(name))?;
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}
