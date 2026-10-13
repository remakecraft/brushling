//! Feature file.exit: File > Exit and Alt+F4.

use crate::features::file_save_changes_prompt::{guard, Pending};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("File > Exit", exit_command);
    r.shortcut(Key::F4, Mods::ALT, "File > Exit");
}

/// File > Exit: the save-changes check, then [`exit`].
fn exit_command(st: &mut State) -> HResult<()> {
    // An active text frame is committed first, which dirties the document so
    // the save-changes check sees the typed text.
    crate::features::tool_text::commit_frame(st);
    guard(st, Pending::Exit)
}

/// Persist the settings and mark the application as exited with code 0.
pub(crate) fn exit(st: &mut State) -> HResult<()> {
    st.exit_code = Some(0);
    st.settings.save()
}
