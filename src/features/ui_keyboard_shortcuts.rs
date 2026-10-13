//! Feature ui.keyboard_shortcuts: global keyboard shortcuts. Each shortcut
//! runs exactly the same command as its menu item (`State::key` resolves it
//! through the registry and calls `State::menu`, so enabled state, dialogs and
//! the save prompts behave identically).
//!
//! Most commands bind their own key where they are registered. This file
//! binds the ones whose commands live in features that register no key of
//! their own (Print, the View toggles, View Bitmap). A shortcut whose command
//! is not registered yet does nothing. The first matching binding wins, so a
//! duplicate registration elsewhere is harmless.
//!
//! Complete table (spec ui.keyboard_shortcuts):
//! Ctrl+N New, Ctrl+O Open, Ctrl+S Save, Ctrl+P Print, Alt+F4 Exit,
//! Ctrl+Z / Alt+Backspace Undo, F4 Repeat, Ctrl+X Cut, Ctrl+C Copy,
//! Ctrl+V Paste, Del Clear Selection, Ctrl+A Select All, Ctrl+T Tool Box,
//! Ctrl+L Color Box, Ctrl+F View Bitmap, Ctrl+R Flip/Rotate, Ctrl+W
//! Stretch/Skew, Ctrl+I Invert Colors, Ctrl+E Attributes, Ctrl+Shift+N Clear
//! Image, Ctrl+PageUp Large Size, Ctrl+PageDown Normal Size, Ctrl+G Show Grid.

use crate::registry::Registry;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.shortcut(Key::Char('p'), Mods::CTRL, "File > Print...");
    r.shortcut(Key::Char('t'), Mods::CTRL, "View > Tool Box");
    r.shortcut(Key::Char('l'), Mods::CTRL, "View > Color Box");
    r.shortcut(Key::Char('f'), Mods::CTRL, "View > View Bitmap");
}

#[cfg(test)]
mod tests {
    use crate::registry::registry;
    use crate::types::*;

    #[test]
    fn every_spec_shortcut_is_bound_to_its_menu_path() {
        let table: &[(Key, Mods, &str)] = &[
            (Key::Char('n'), Mods::CTRL, "File > New"),
            (Key::Char('o'), Mods::CTRL, "File > Open..."),
            (Key::Char('s'), Mods::CTRL, "File > Save"),
            (Key::Char('p'), Mods::CTRL, "File > Print..."),
            (Key::F4, Mods::ALT, "File > Exit"),
            (Key::Char('z'), Mods::CTRL, "Edit > Undo"),
            (Key::Backspace, Mods::ALT, "Edit > Undo"),
            (Key::F4, Mods::NONE, "Edit > Repeat"),
            (Key::Char('x'), Mods::CTRL, "Edit > Cut"),
            (Key::Char('c'), Mods::CTRL, "Edit > Copy"),
            (Key::Char('v'), Mods::CTRL, "Edit > Paste"),
            (Key::Delete, Mods::NONE, "Edit > Clear Selection"),
            (Key::Char('a'), Mods::CTRL, "Edit > Select All"),
            (Key::Char('t'), Mods::CTRL, "View > Tool Box"),
            (Key::Char('l'), Mods::CTRL, "View > Color Box"),
            (Key::Char('f'), Mods::CTRL, "View > View Bitmap"),
            (Key::Char('r'), Mods::CTRL, "Image > Flip/Rotate..."),
            (Key::Char('w'), Mods::CTRL, "Image > Stretch/Skew..."),
            (Key::Char('i'), Mods::CTRL, "Image > Invert Colors"),
            (Key::Char('e'), Mods::CTRL, "Image > Attributes..."),
            (Key::Char('n'), Mods::CTRL_SHIFT, "Image > Clear Image"),
            (Key::PageUp, Mods::CTRL, "View > Zoom > Large Size"),
            (Key::PageDown, Mods::CTRL, "View > Zoom > Normal Size"),
            (Key::Char('g'), Mods::CTRL, "View > Zoom > Show Grid"),
        ];
        for &(k, m, path) in table {
            assert_eq!(registry().shortcut_for(k, m), Some(path), "{k:?}+{m:?}");
        }
    }
}
