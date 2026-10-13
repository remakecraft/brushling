//! Feature ui.menu_enable_state: greyed-out menu items.
//!
//! An item can act only when its command is registered and both its own
//! predicate (`Registry::command_if`, owned by the feature that provides the
//! command) and the central rule below allow it. The central rules cover
//! items that depend on state outside their own feature (document path, active
//! tool, zoom); they apply even before the owning feature registers the
//! command, so the listing is already right. A disabled command is a silent
//! no-op for the menu and for its shortcut alike (`State::menu`).

use crate::registry::{registry, Registry};
use crate::state::State;
use crate::types::*;

pub fn register(_r: &mut Registry) {}

/// The central rule of a menu path: `Some(allowed)` when one exists.
fn central_rule(st: &State, path: &str) -> Option<bool> {
    Some(match path {
        // Wallpaper needs a file on disk to point at.
        "File > Set As Wallpaper (Tiled)" | "File > Set As Wallpaper (Centered)" => st.doc.path.is_some(),
        // The text toolbar belongs to the Text tool.
        "View > Text Toolbar" => st.tool == Tool::Text,
        // A thumbnail only means something when the image is zoomed.
        "View > Zoom > Show Thumbnail" => st.zoom != 100,
        _ => return None,
    })
}

/// Whether the menu command at `path` is enabled right now. Commands that no
/// feature has registered (and that have no central rule) are disabled.
pub fn enabled(st: &State, path: &str) -> bool {
    let own = match registry().command_for(path) {
        Some(cmd) => cmd.enabled.is_none_or(|f| f(st)),
        None => central_rule(st, path).is_some(),
    };
    own && central_rule(st, path).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use crate::{App, StartOptions, Tool};

    fn app(tag: &str) -> App {
        App::start(StartOptions::new(std::env::temp_dir().join(format!("brushling-enable-{}-{tag}", std::process::id())))).unwrap()
    }

    fn on(a: &App, p: &str) -> bool {
        a.st.menu_entry(p).unwrap().enabled
    }

    #[test]
    fn unregistered_without_rule_is_disabled() {
        let a = app("unreg");
        assert!(!super::enabled(&a.st, "No > Such Thing"));
    }

    #[test]
    fn rules_follow_state() {
        let mut a = app("rules");
        assert!(!on(&a, "View > Text Toolbar") && !on(&a, "File > Set As Wallpaper (Tiled)"));
        assert!(!on(&a, "View > Zoom > Show Thumbnail"));
        a.select_tool(Tool::Text).unwrap();
        assert!(on(&a, "View > Text Toolbar"));
        a.select_tool(Tool::Pencil).unwrap();
        assert!(!on(&a, "View > Text Toolbar"));
        a.menu("View > Zoom > Large Size").unwrap();
        assert!(on(&a, "View > Zoom > Show Thumbnail"));
        a.st.doc.path = Some("x.bmp".into());
        assert!(on(&a, "File > Set As Wallpaper (Centered)"));
    }
}
