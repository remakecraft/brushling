//! Feature file.save_changes_prompt: the Yes/No/Cancel question in front of
//! every action that discards a dirty document.
//!
//! An action goes through [`guard`]. A clean document runs it at once; a
//! dirty one stores it in `State::pending_action` and asks. Yes starts a
//! save with the action parked in `State::after_save`; `file_save` calls
//! [`continue_after_save`] when the file was written and clears the slot when
//! the save is cancelled or fails, so the action is then abandoned.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;
use std::path::PathBuf;

/// An action that replaces the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    New,
    Open,
    Recent(PathBuf),
    Exit,
}

pub fn register(_r: &mut Registry) {}

/// Run `action` now, or after the user answered the save-changes question.
pub(crate) fn guard(st: &mut State, action: Pending) -> HResult<()> {
    if !st.doc.dirty {
        return run(st, action);
    }
    let name = st
        .doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(|| "untitled".to_owned(), |n| n.to_string_lossy().into_owned());
    st.pending_action = Some(action);
    st.message_box("Brushling", &format!("Save changes to {name}?"), &["Yes", "No", "Cancel"], Some(answered));
    Ok(())
}

fn answered(st: &mut State, button: &str) -> HResult<()> {
    let Some(action) = st.pending_action.take() else { return Ok(()) };
    match button {
        "Yes" => {
            st.after_save = Some(action);
            crate::features::file_save::save_document(st)
        }
        "No" => run(st, action),
        _ => Ok(()),
    }
}

/// Called by `file_save` after a successful write: carry on with the action
/// the prompt interrupted, if any.
pub(crate) fn continue_after_save(st: &mut State) -> HResult<()> {
    match st.after_save.take() {
        Some(action) => run(st, action),
        None => Ok(()),
    }
}

fn run(st: &mut State, action: Pending) -> HResult<()> {
    match action {
        Pending::New => crate::features::file_new::new_document(st),
        Pending::Open => crate::features::file_open::show_open_dialog(st),
        Pending::Recent(path) => crate::features::file_recent::open_recent(st, &path),
        Pending::Exit => crate::features::file_exit::exit(st),
    }
}
