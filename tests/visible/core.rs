//! core.document, ui.*, input.escape, help.help_about

mod core_document {
    use crate::common::*;

    #[test]
    fn start_defaults() -> T {
        let a = start()?;
        assert_eq!(size(&a)?, (512, 384));
        assert!(all_are(&a, W)?);
        assert_eq!(a.title()?, "untitled - Brushling");
        let d = a.document()?;
        assert_eq!(d.path, None);
        assert!(!d.dirty);
        Ok(())
    }

    #[test] // layer=format
    fn open_a_then_b() -> T {
        let dir = tmp("open2");
        let pa = bmp_file(&dir, "a.bmp", 2, 2, &solid(2, 2, RED))?;
        let pb = bmp_file(&dir, "b.bmp", 3, 1, &[RED, GREEN, BLUE])?;
        let mut a = start()?;
        open_path(&mut a, &pa)?;
        open_path(&mut a, &pb)?;
        assert_eq!(a.pixels()?, Image { width: 3, height: 1, pixels: vec![RED, GREEN, BLUE] });
        assert_eq!(a.document()?.path, Some(pb));
        Ok(())
    }

    #[test]
    fn attributes_sets_new_size() -> T {
        let mut a = fresh()?;
        attributes(&mut a, "100", "50")?;
        file_new_discard(&mut a)?;
        assert_eq!(size(&a)?, (100, 50));
        Ok(())
    }

    #[test]
    fn new_size_survives_restart() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        attributes(&mut a, "100", "50")?;
        file_new_discard(&mut a)?;
        let mut b = restart(&mut a, &o)?;
        b.menu("File > New")?;
        assert_eq!(size(&b)?, (100, 50));
        Ok(())
    }

    #[test] // layer=pixel
    fn pencil_clips_to_canvas() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        drag(&mut a, (-5, 3), (40, 3))?;
        assert_eq!(k_pts(&a)?, rect_pts(0, 3, 31, 3));
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test] // layer=format
    fn startup_arg_opens_file() -> T {
        let dir = tmp("arg");
        let px = pattern(4, 3, 7);
        let p = bmp_file(&dir, "start.bmp", 4, 3, &px)?;
        let mut o = opts();
        o.args = vec![ps(&p)];
        let a = App::start(o)?;
        assert_eq!(a.pixels()?.pixels, px);
        assert_eq!(a.title()?, "start.bmp - Brushling");
        Ok(())
    }
}

mod ui_main_window {
    use crate::common::*;

    #[test]
    fn title_after_save() -> T {
        let mut a = fresh()?;
        let dir = tmp("title");
        save_as(&mut a, &dir.join("cat.bmp"), T24)?;
        assert_eq!(a.title()?, "cat.bmp - Brushling");
        Ok(())
    }

    #[test]
    fn title_untitled_and_after_open() -> T {
        let dir = tmp("title2");
        let p = bmp_file(&dir, "dog.bmp", 2, 2, &solid(2, 2, BLUE))?;
        let mut a = fresh()?;
        assert_eq!(a.title()?, "untitled - Brushling");
        open_path(&mut a, &p)?;
        assert_eq!(a.title()?, "dog.bmp - Brushling");
        a.menu("File > New")?;
        assert_eq!(a.title()?, "untitled - Brushling");
        Ok(())
    }

    #[test]
    fn min_size_50() -> T {
        let mut a = start()?;
        a.set_window_size(50, 50)?;
        let l = a.layout()?;
        assert!(l.color_box_docked.0 > 0 && l.tool_box_docked.1 > 0);
        assert!(l.client_width >= l.color_box_docked.0);
        assert!(l.client_height >= l.tool_box_docked.1);
        assert!(
            l.client_height
                >= l.menu_bar_height + l.tool_box_docked.1 + l.color_box_docked.1 + l.status_bar_height
        );
        Ok(())
    }

    #[test]
    fn scrollbars_large_and_small() -> T {
        let mut a = start()?;
        a.set_window_size(800, 600)?;
        a.new_document(2000, 2000, W)?;
        let l = a.layout()?;
        assert!(l.hscroll && l.vscroll);
        a.new_document(10, 10, W)?;
        let l = a.layout()?;
        assert!(!l.hscroll && !l.vscroll);
        Ok(())
    }

    #[test]
    fn scrollbars_wide_only() -> T {
        let mut a = start()?;
        a.set_window_size(800, 600)?;
        a.new_document(2000, 10, W)?;
        let l = a.layout()?;
        assert!(l.hscroll);
        assert!(!l.vscroll);
        Ok(())
    }

    #[test] // layer=pixel
    fn workspace_render_origin() -> T {
        let mut a = fresh()?;
        a.set_window_size(800, 600)?;
        a.set_pixel(0, 0, RED)?;
        let img = a.render(Surface::Workspace)?;
        assert_eq!(img.get(4, 4), Some(RED));
        assert_eq!(img.get(5, 4), Some(W));
        assert_eq!(img.get(2, 2), Some(GREY));
        assert_eq!(img.get(200, 200), Some(GREY));
        Ok(())
    }
}

mod ui_layout_persistence {
    use crate::common::*;

    #[test]
    fn status_bar_hidden_persists() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        assert!(checked(&a, "View > Status Bar")?);
        a.menu("View > Status Bar")?;
        let b = restart(&mut a, &o)?;
        assert!(!checked(&b, "View > Status Bar")?);
        assert!(!b.status_bar()?.visible);
        Ok(())
    }

    #[test]
    fn toolbox_float_persists() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        a.place_panel(Panel::ToolBox, Some((300, 200)))?;
        let b = restart(&mut a, &o)?;
        assert_eq!(b.panel(Panel::ToolBox)?, PanelState { visible: true, floating_at: Some((300, 200)) });
        Ok(())
    }

    #[test]
    fn offscreen_panel_reset() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        a.place_panel(Panel::ColorBox, Some((-5000, -5000)))?;
        let b = restart(&mut a, &o)?;
        assert_eq!(b.panel(Panel::ColorBox)?, PanelState { visible: true, floating_at: Some((0, 0)) });
        Ok(())
    }

    #[test]
    fn settings_written_in_brushling_subfolder() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        a.menu("File > Exit")?;
        assert_eq!(a.exit_status()?, Some(0));
        let files = dir_names(&o.config_dir.join("brushling"))?;
        assert!(!files.is_empty());
        Ok(())
    }

    #[test]
    fn corrupt_settings_ignored() -> T {
        let o = opts();
        let mut a = App::start(o.clone())?;
        a.menu("View > Status Bar")?;
        a.menu("File > Exit")?;
        let dir = o.config_dir.join("brushling");
        let files = dir_names(&dir)?;
        assert!(!files.is_empty());
        for f in files {
            let p = dir.join(f);
            if p.is_file() {
                write(&p, b"\x00\xffgarbage{{{[[[")?;
            }
        }
        let mut b = App::start(o)?;
        assert!(b.dialog()?.is_none());
        assert!(checked(&b, "View > Status Bar")?);
        b.menu("File > New")?;
        assert_eq!(size(&b)?, (512, 384));
        Ok(())
    }
}

mod ui_menu_bar {
    use crate::common::*;

    #[test]
    fn titles_in_order() -> T {
        let a = start()?;
        assert_eq!(labels(&a, "")?, ["File", "Edit", "View", "Image", "Colors", "Help"]);
        Ok(())
    }

    #[test]
    fn edit_items() -> T {
        let a = start()?;
        assert_eq!(
            labels(&a, "Edit")?,
            ["Undo", "Repeat", "Cut", "Copy", "Paste", "Clear Selection", "Select All", "Copy To...", "Paste From..."]
        );
        Ok(())
    }

    #[test]
    fn alt_f_v_print_preview() -> T {
        let mut a = fresh()?;
        a.key(Key::Char('f'), Mods::ALT)?;
        key(&mut a, Key::Char('v'))?;
        assert!(matches!(a.view_mode()?, ViewMode::PrintPreview { .. }));
        Ok(())
    }

    #[test]
    fn alt_i_i_inverts() -> T {
        let mut a = fresh()?;
        a.key(Key::Char('i'), Mods::ALT)?;
        key(&mut a, Key::Char('i'))?;
        assert!(all_are(&a, K)?);
        Ok(())
    }

    #[test]
    fn colors_menu_single_item() -> T {
        let a = start()?;
        assert_eq!(labels(&a, "Colors")?, ["Edit Colors..."]);
        Ok(())
    }

    #[test]
    fn file_items_order() -> T {
        let a = start()?;
        assert_eq!(
            labels(&a, "File")?,
            [
                "New",
                "Open...",
                "Save",
                "Save As...",
                "Print Preview",
                "Page Setup...",
                "Print...",
                "Send...",
                "Set As Wallpaper (Tiled)",
                "Set As Wallpaper (Centered)",
                "Exit"
            ]
        );
        let seps: Vec<bool> = a.menu_items("File")?.iter().map(|i| i.separator).collect();
        assert_eq!(
            seps,
            [false, false, false, false, true, false, false, false, true, false, true, false, false, true, false]
        );
        Ok(())
    }

    #[test]
    fn shortcut_texts() -> T {
        let a = start()?;
        for (p, s) in [
            ("File > New", "Ctrl+N"),
            ("File > Exit", "Alt+F4"),
            ("Edit > Repeat", "F4"),
            ("Edit > Clear Selection", "Del"),
            ("Image > Clear Image", "Ctrl+Shift+N"),
            ("Image > Attributes...", "Ctrl+E"),
            ("View > Zoom > Large Size", "Ctrl+PageUp"),
        ] {
            assert_eq!(item(&a, p)?.shortcut.as_deref(), Some(s), "{p}");
        }
        Ok(())
    }
}

mod ui_menu_enable_state {
    use crate::common::*;

    #[test]
    fn fresh_disabled_items() -> T {
        let a = fresh()?;
        for p in [
            "Edit > Cut",
            "Edit > Copy",
            "Edit > Clear Selection",
            "Edit > Copy To...",
            "Edit > Undo",
            "Edit > Repeat",
            "File > Set As Wallpaper (Tiled)",
            "File > Set As Wallpaper (Centered)",
            "View > Text Toolbar",
        ] {
            assert!(!enabled(&a, p)?, "{p} should be disabled");
        }
        assert!(enabled(&a, "Edit > Select All")?);
        assert!(enabled(&a, "File > New")?);
        Ok(())
    }

    #[test]
    fn repeat_enabled_after_undo() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        undo(&mut a)?;
        assert!(repeat_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn ctrl_x_without_selection_noop() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.set_clipboard(ClipboardContent::Text("sentinel".into()))?;
        ctrl(&mut a, 'x')?;
        assert_eq!(k_pts(&a)?, pts(&[(1, 1)]));
        assert_eq!(a.clipboard()?, ClipboardContent::Text("sentinel".into()));
        select_rect(&mut a, (0, 0), (3, 3))?;
        ctrl(&mut a, 'x')?;
        assert!(matches!(a.clipboard()?, ClipboardContent::Image(ref i) if i.width == 4 && i.height == 4));
        Ok(())
    }

    #[test]
    fn wallpaper_enabled_after_save() -> T {
        let mut a = fresh()?;
        save_as(&mut a, &tmp("wp").join("a.bmp"), T24)?;
        assert!(enabled(&a, "File > Set As Wallpaper (Tiled)")?);
        assert!(enabled(&a, "File > Set As Wallpaper (Centered)")?);
        Ok(())
    }

    #[test]
    fn text_toolbar_enabled_with_text() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Text)?;
        assert!(enabled(&a, "View > Text Toolbar")?);
        Ok(())
    }

    #[test]
    fn cut_copy_enabled_with_selection() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (1, 1), (4, 4))?;
        for p in ["Edit > Cut", "Edit > Copy", "Edit > Clear Selection", "Edit > Copy To..."] {
            assert!(enabled(&a, p)?, "{p}");
        }
        Ok(())
    }

    #[test]
    fn show_thumbnail_disabled_at_100() -> T {
        let mut a = fresh()?;
        assert!(!enabled(&a, "View > Zoom > Show Thumbnail")?);
        a.menu("View > Zoom > Large Size")?;
        assert!(enabled(&a, "View > Zoom > Show Thumbnail")?);
        Ok(())
    }
}

mod ui_keyboard_shortcuts {
    use crate::common::*;

    #[test]
    fn alt_backspace_undoes() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 5, 5)?;
        assert_eq!(px(&a, 5, 5)?, K);
        a.key(Key::Backspace, Mods::ALT)?;
        assert_eq!(px(&a, 5, 5)?, W);
        Ok(())
    }

    #[test]
    fn ctrl_shift_n_clears_without_prompt() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        set_bg(&mut a, RED)?;
        a.key(Key::Char('n'), Mods::CTRL_SHIFT)?;
        assert!(a.dialog()?.is_none());
        assert!(all_are(&a, RED)?);
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(a.colors()?, (K, RED));
        Ok(())
    }

    #[test]
    fn ctrl_i_inverts() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'i')?;
        assert!(all_are(&a, K)?);
        Ok(())
    }

    #[test]
    fn zoom_keys() -> T {
        let mut a = fresh()?;
        a.key(Key::PageUp, Mods::CTRL)?;
        assert_eq!(a.zoom()?, 400);
        a.key(Key::PageDown, Mods::CTRL)?;
        assert_eq!(a.zoom()?, 100);
        Ok(())
    }

    #[test]
    fn ctrl_g_toggles_grid() -> T {
        let mut a = fresh()?;
        assert!(!checked(&a, "View > Zoom > Show Grid")?);
        ctrl(&mut a, 'g')?;
        assert!(checked(&a, "View > Zoom > Show Grid")?);
        ctrl(&mut a, 'g')?;
        assert!(!checked(&a, "View > Zoom > Show Grid")?);
        Ok(())
    }

    #[test]
    fn shortcuts_ignored_in_dialog() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'e')?;
        expect_dialog(&a, DialogKind::Attributes)?;
        ctrl(&mut a, 'i')?;
        expect_dialog(&a, DialogKind::Attributes)?;
        assert!(all_are(&a, W)?);
        esc(&mut a)?;
        assert!(a.dialog()?.is_none());
        ctrl(&mut a, 'i')?;
        assert!(all_are(&a, K)?);
        Ok(())
    }

    #[test]
    fn shortcuts_match_menu_file_edit() -> T {
        let cases: &[(Key, Mods, &str, Option<&str>)] = &[
            (Key::Char('n'), Mods::CTRL, "File > New", None),
            (Key::Char('o'), Mods::CTRL, "File > Open...", None),
            (Key::Char('s'), Mods::CTRL, "File > Save", None),
            (Key::Char('p'), Mods::CTRL, "File > Print...", None),
            (Key::Char('z'), Mods::CTRL, "Edit > Undo", None),
            (Key::Backspace, Mods::ALT, "Edit > Undo", None),
            (Key::F4, Mods::NONE, "Edit > Repeat", Some("Edit > Undo")),
            (Key::Char('x'), Mods::CTRL, "Edit > Cut", None),
            (Key::Char('c'), Mods::CTRL, "Edit > Copy", None),
            (Key::Char('v'), Mods::CTRL, "Edit > Paste", None),
            (Key::Delete, Mods::NONE, "Edit > Clear Selection", None),
            (Key::Char('a'), Mods::CTRL, "Edit > Select All", None),
        ];
        crate::common::shortcut_table(cases)
    }
}

mod ui_status_bar {
    use crate::common::*;

    #[test]
    fn position_pane() -> T {
        let mut a = fresh()?;
        mv(&mut a, 12, 7)?;
        assert_eq!(a.status_bar()?.position, "12,7");
        Ok(())
    }

    #[test]
    fn outside_empty() -> T {
        let mut a = fresh()?;
        mv(&mut a, 3, 4)?;
        assert_eq!(a.status_bar()?.position, "3,4");
        a.pointer(PointerKind::Move, Pos::Outside, NO)?;
        assert_eq!(a.status_bar()?.position, "");
        Ok(())
    }

    #[test]
    fn size_pane_while_dragging() -> T {
        let mut a = fresh_sized(512, 384)?;
        a.select_tool(Tool::Rectangle)?;
        press(&mut a, 10, 10)?;
        mv(&mut a, 209, 209)?;
        assert_eq!(a.status_bar()?.size, "200,200");
        release(&mut a, 209, 209)?;
        Ok(())
    }

    #[test]
    fn idle_message() -> T {
        let a = fresh()?;
        let s = a.status_bar()?;
        assert!(s.visible);
        assert_eq!(s.message, "For help, open the Help menu.");
        Ok(())
    }

    #[test]
    fn menu_highlight_message() -> T {
        let mut a = fresh()?;
        let idle = a.status_bar()?.message;
        a.hover(Hover::MenuItem("File > New".into()))?;
        let m_new = a.status_bar()?.message;
        a.hover(Hover::MenuItem("File > Open...".into()))?;
        let m_open = a.status_bar()?.message;
        assert!(!m_new.is_empty() && m_new != idle);
        assert!(m_open != m_new && m_open != idle);
        a.hover(Hover::Nothing)?;
        assert_eq!(a.status_bar()?.message, idle);
        Ok(())
    }
}

mod input_escape {
    use crate::common::*;

    #[test]
    fn line_drag_cancel() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        press(&mut a, 2, 2)?;
        mv(&mut a, 20, 20)?;
        assert_eq!(a.render(Surface::Composite)?.get(10, 10), Some(K));
        esc(&mut a)?;
        release(&mut a, 20, 20)?;
        assert!(all_are(&a, W)?);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn esc_commits_moved_selection() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 2, 2)?;
        select_rect(&mut a, (1, 1), (3, 3))?;
        drag(&mut a, (2, 2), (12, 2))?;
        esc(&mut a)?;
        assert!(a.selection()?.is_none());
        assert_eq!(px(&a, 12, 2)?, K);
        Ok(())
    }

    #[test]
    fn pencil_stroke_cancel() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        press(&mut a, 2, 2)?;
        mv(&mut a, 10, 2)?;
        assert_eq!(a.render(Surface::Composite)?.get(5, 2), Some(K));
        esc(&mut a)?;
        release(&mut a, 10, 2)?;
        assert!(all_are(&a, W)?);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn handle_drag_cancel() -> T {
        let mut a = fresh()?;
        a.pointer(PointerKind::Press(L), Pos::CanvasHandle(CanvasHandle::Corner), NO)?;
        mv(&mut a, 40, 40)?;
        esc(&mut a)?;
        release(&mut a, 40, 40)?;
        assert_eq!(size(&a)?, (32, 32));
        drag_canvas_handle(&mut a, CanvasHandle::Corner, 36, 36)?;
        assert_eq!(size(&a)?, (36, 36));
        Ok(())
    }
}

mod help_help_about {
    use crate::common::*;

    #[test]
    fn curve_survives_help() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (2, 2), (20, 2))?;
        a.menu("Help > Help Topics")?;
        assert!(a.panel(Panel::Help)?.visible);
        a.close_panel(Panel::Help)?;
        click(&mut a, 10, 10)?;
        click(&mut a, 10, 10)?;
        assert_eq!(px(&a, 2, 2)?, K);
        assert_eq!(px(&a, 20, 2)?, K);
        assert!(undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn selection_survives_help() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (3, 3), (9, 9))?;
        a.menu("Help > Help Topics")?;
        assert!(a.panel(Panel::Help)?.visible);
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(3, 3, 9, 9)));
        Ok(())
    }

    #[test]
    fn about_text() -> T {
        let mut a = fresh()?;
        a.menu("Help > About Brushling")?;
        let d = expect_dialog(&a, DialogKind::About)?;
        assert!(d.text.contains("Brushling"));
        assert!(d.text.contains(brushling::VERSION));
        Ok(())
    }

    #[test]
    fn help_twice_single_window() -> T {
        let mut a = fresh()?;
        a.menu("Help > Help Topics")?;
        a.menu("Help > Help Topics")?;
        assert!(a.panel(Panel::Help)?.visible);
        a.close_panel(Panel::Help)?;
        assert!(!a.panel(Panel::Help)?.visible);
        Ok(())
    }
}
