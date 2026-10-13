//! Menu dispatch. Commands are looked up by their full menu path in the
//! registry; this file registers the ones that belong to the shared core
//! (File > New and File > Exit). Every other command is registered by the
//! feature that owns it.

use crate::registry::{registry, Registry};
use crate::state::State;
use crate::types::*;

pub fn register_core(r: &mut Registry) {
    r.command("File > New", file_new);
    r.command("File > Exit", file_exit);
    r.shortcut(Key::Char('n'), Mods::CTRL, "File > New");
}

/// File > New: a clean untitled image of the default size. The save-changes
/// prompt in front of it belongs to file.save_changes_prompt.
fn file_new(st: &mut State) -> HResult<()> {
    st.new_default_document()
}

/// File > Exit: persist the settings and mark the application as exited.
fn file_exit(st: &mut State) -> HResult<()> {
    st.exit_code = Some(0);
    st.settings.save()
}

impl State {
    /// Invoke a menu command by path. A disabled command does nothing; an
    /// unknown path is an error. Menus are unreachable behind a modal dialog.
    pub fn menu(&mut self, path: &str) -> HResult<()> {
        let Some(cmd) = registry().command_for(path) else {
            return Err(HarnessError::Invalid(format!("unknown or unavailable menu command {path:?}")));
        };
        if !self.dialogs.is_empty() || self.exit_code.is_some() {
            return Ok(());
        }
        if cmd.enabled.is_some_and(|f| !f(self)) {
            return Ok(());
        }
        (cmd.run)(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, HarnessError, Rgb, StartOptions};

    fn opts(tag: &str) -> StartOptions {
        StartOptions::new(std::env::temp_dir().join(format!("brushling-cmd-{}-{tag}", std::process::id())))
    }

    #[test]
    fn default_size_survives_restart() {
        let o = opts("restart");
        let mut a = App::start(o.clone()).unwrap();
        let d = a.document().unwrap();
        assert_eq!((d.width, d.height), (512, 384));
        assert!(a.pixels().unwrap().pixels.iter().all(|p| *p == Rgb::W));
        a.st.remember_new_size(100, 50).unwrap();
        a.menu("File > New").unwrap();
        assert_eq!((a.document().unwrap().width, a.document().unwrap().height), (100, 50));
        a.menu("File > Exit").unwrap();
        assert_eq!(a.exit_status().unwrap(), Some(0));
        let b = App::start(o.clone()).unwrap();
        assert_eq!((b.document().unwrap().width, b.document().unwrap().height), (100, 50));
        let _ = std::fs::remove_dir_all(&o.config_dir);
    }

    #[test]
    fn unknown_menu_path_is_invalid_and_bad_pixel_rejected() {
        let mut a = App::start(opts("invalid")).unwrap();
        assert!(matches!(a.menu("No > Such"), Err(HarnessError::Invalid(_))));
        assert!(a.set_pixel(-1, 0, Rgb::K).is_err());
        a.new_document(4, 4, Rgb::W).unwrap();
        a.set_pixel(2, 3, Rgb::R).unwrap();
        assert_eq!(a.pixel(2, 3).unwrap(), Rgb::R);
        assert!(!a.document().unwrap().dirty);
    }

    #[test]
    fn unreadable_startup_file_shows_message_and_keeps_untitled() {
        let mut o = opts("arg");
        o.args = vec!["Z:/definitely/missing.bmp".into()];
        let mut a = App::start(o).unwrap();
        let d = a.dialog().unwrap().unwrap();
        assert!(d.text.contains("missing.bmp") && d.text.contains("This file cannot be opened"));
        a.dialog_click("OK").unwrap();
        assert!(a.dialog().unwrap().is_none());
        assert_eq!(a.document().unwrap().path, None);
    }

    #[test]
    fn workspace_render_places_canvas_at_margin() {
        let mut a = App::start(opts("render")).unwrap();
        a.new_document(32, 32, Rgb::W).unwrap();
        a.set_pixel(0, 0, Rgb::R).unwrap();
        let img = a.render(crate::Surface::Workspace).unwrap();
        assert_eq!(img.get(4, 4), Some(Rgb::R));
        assert_eq!(img.get(5, 4), Some(Rgb::W));
        assert_eq!(img.get(2, 2), Some(Rgb::new(128, 128, 128)));
        assert_eq!(a.render(crate::Surface::Composite).unwrap().get(0, 0), Some(Rgb::R));
    }
}
