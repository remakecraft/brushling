//! Feature file.wallpaper: Set As Wallpaper (Tiled / Centered).
//!
//! Both items hand the document's file path, as it is on disk, to the
//! wallpaper service. Unsaved changes are neither saved nor asked about.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command_if("File > Set As Wallpaper (Tiled)", tiled, Some(enabled));
    r.command_if("File > Set As Wallpaper (Centered)", centered, Some(enabled));
}

/// Available only with a wallpaper service and a document that has a file.
fn enabled(st: &State) -> bool {
    st.services.wallpaper && st.doc.path.is_some()
}

fn tiled(st: &mut State) -> HResult<()> {
    set(st, WallpaperStyle::Tiled)
}

fn centered(st: &mut State) -> HResult<()> {
    set(st, WallpaperStyle::Centered)
}

fn set(st: &mut State, style: WallpaperStyle) -> HResult<()> {
    if let Some(path) = st.doc.path.clone() {
        st.record_call(PlatformCall::Wallpaper { path, style });
    }
    Ok(())
}
