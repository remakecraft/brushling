//! Feature help.help_about: Help > Help Topics and Help > About Brushling.
//!
//! Help Topics opens the separate, non-modal help window (`Panel::Help`); the
//! window is a single instance, so opening it twice changes nothing. Opening
//! it never touches the document, selection, text frame or an unfinished
//! curve/polygon. About is a modal box with the name, version and licence.

use crate::dialogs::{message_box_handler, OpenDialog};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("Help > Help Topics", help_topics);
    r.command("Help > About Brushling", about);
}

/// The licence notice shown in the About box (matches `Cargo.toml`).
const LICENCE: &str = "Released under the MIT licence.";

/// Whether the help window is open.
pub fn help_visible(st: &State) -> bool {
    st.help_open
}

/// Press the help window's close button.
pub fn close_help(st: &mut State) {
    st.help_open = false;
}

fn help_topics(st: &mut State) -> HResult<()> {
    st.help_open = true;
    Ok(())
}

/// The text of the About box.
pub fn about_text() -> String {
    format!("Brushling\nVersion {}\n{}", crate::VERSION, LICENCE)
}

fn about(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        kind: DialogKind::About,
        title: "About Brushling".to_owned(),
        text: about_text(),
        buttons: vec!["OK".to_owned()],
        fields: Vec::new(),
    };
    st.open_dialog(OpenDialog::new(dialog, message_box_handler));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{App, DialogKind, StartOptions};

    fn app(tag: &str) -> App {
        App::start(StartOptions::new(std::env::temp_dir().join(format!("brushling-help-{}-{tag}", std::process::id())))).unwrap()
    }

    #[test]
    fn help_opens_once_and_closes() {
        let mut a = app("once");
        a.menu("Help > Help Topics").unwrap();
        a.menu("Help > Help Topics").unwrap();
        assert!(super::help_visible(&a.st));
        super::close_help(&mut a.st);
        assert!(!super::help_visible(&a.st));
    }

    #[test]
    fn about_box_has_name_version_licence_and_ok() {
        let mut a = app("about");
        a.menu("Help > About Brushling").unwrap();
        let d = a.dialog().unwrap().unwrap();
        assert_eq!(d.kind, DialogKind::About);
        assert!(d.text.contains("Brushling") && d.text.contains(crate::VERSION) && d.text.contains("licence"));
        a.dialog_click("OK").unwrap();
        assert!(a.dialog().unwrap().is_none());
    }
}
