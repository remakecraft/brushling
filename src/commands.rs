//! Menu dispatch. Commands are looked up by their full menu path in the
//! registry; every command is registered by the feature that owns it. The
//! recent-file entries of the File menu are dynamic and resolved here through
//! `file_recent`.

use crate::registry::registry;
use crate::state::State;
use crate::types::*;

impl State {
    /// Invoke a menu command by path. A disabled command does nothing; an
    /// unknown path is an error. Menus are unreachable behind a modal dialog.
    pub fn menu(&mut self, path: &str) -> HResult<()> {
        if let Some(recent) = crate::features::file_recent::entry_path(self, path) {
            if self.dialogs.is_empty() && self.exit_code.is_none() {
                crate::features::file_save_changes_prompt::guard(
                    self,
                    crate::features::file_save_changes_prompt::Pending::Recent(recent),
                )?;
            }
            return Ok(());
        }
        let Some(cmd) = registry().command_for(path) else {
            return Err(HarnessError::Invalid(format!("unknown or unavailable menu command {path:?}")));
        };
        if !self.dialogs.is_empty() || self.exit_code.is_some() || self.view_bitmap {
            return Ok(());
        }
        if !crate::features::ui_menu_enable_state::enabled(self, path) {
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
