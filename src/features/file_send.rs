//! Feature file.send: File > Send hands the saved file to the mail service.
//!
//! A document without a path or with unsaved changes goes through the
//! save-changes question first (Yes saves, then sends; No and Cancel abandon
//! the send). Without a mail service a message says so.

use crate::features::file_save_changes_prompt::{ask, Pending};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("File > Send...", send_command);
}

fn send_command(st: &mut State) -> HResult<()> {
    if !st.services.mail {
        st.error_box("No e-mail program is available.");
        return Ok(());
    }
    if st.doc.path.is_none() || st.doc.dirty {
        return ask(st, Pending::Send);
    }
    send_now(st)
}

/// Pass the document's file to the mail service as the attachment.
pub(crate) fn send_now(st: &mut State) -> HResult<()> {
    if let Some(attachment) = st.doc.path.clone() {
        st.record_call(PlatformCall::Mail { attachment });
    }
    Ok(())
}
