//! edit.*

mod edit_undo_repeat {
    use crate::common::*;

    #[test]
    fn three_step_limit() -> T {
        let mut a = fresh()?;
        for i in 1..=4 {
            pencil_click(&mut a, i, i)?;
        }
        for _ in 0..4 {
            undo(&mut a)?;
        }
        assert_eq!(k_pts(&a)?, pts(&[(1, 1)]));
        Ok(())
    }

    #[test]
    fn undo_then_repeat() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        undo(&mut a)?;
        assert_eq!(px(&a, 1, 1)?, W);
        repeat(&mut a)?;
        assert_eq!(px(&a, 1, 1)?, K);
        Ok(())
    }

    #[test]
    fn new_step_clears_repeat() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        undo(&mut a)?;
        assert!(repeat_enabled(&a)?);
        pencil_click(&mut a, 2, 2)?;
        assert!(!repeat_enabled(&a)?);
        assert_eq!(k_pts(&a)?, pts(&[(2, 2)]));
        Ok(())
    }

    #[test] // layer=pixel
    fn undo_discards_unfinished_curve() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Rectangle)?;
        drag(&mut a, (2, 2), (8, 8))?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (10, 10), (20, 10))?;
        undo(&mut a)?;
        assert!(k_pts(&a)?.is_empty());
        assert!(where_eq(&a.render(Surface::Composite)?, K).is_empty());
        repeat(&mut a)?;
        assert_eq!(k_pts(&a)?, border_pts(2, 2, 8, 8, 1));
        assert!(where_eq(&a.render(Surface::Composite)?, K).iter().all(|&(_, y)| y != 10));
        Ok(())
    }

    #[test]
    fn colour_change_not_undoable() -> T {
        let mut a = fresh()?;
        a.click_swatch(17, L, false)?;
        assert_eq!(a.colors()?.0, RED);
        assert!(!undo_enabled(&a)?);
        pencil_click(&mut a, 3, 3)?;
        assert!(undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn undo_restores_size() -> T {
        let mut a = fresh()?;
        attributes(&mut a, "10", "10")?;
        assert_eq!(size(&a)?, (10, 10));
        undo(&mut a)?;
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test]
    fn undo_rotate_restores() -> T {
        let px0 = pattern(32, 16, 2);
        let mut a = image(32, 16, &px0)?;
        flip_rotate(&mut a, "Rotate by angle", "90")?;
        assert_eq!(size(&a)?, (16, 32));
        undo(&mut a)?;
        assert_eq!(a.pixels()?, Image { width: 32, height: 16, pixels: px0 });
        Ok(())
    }
}

mod edit_clipboard {
    use crate::common::*;

    #[test]
    fn copy_3x3() -> T {
        let mut a = fresh()?;
        a.set_pixel(5, 5, RED)?;
        select_rect(&mut a, (4, 4), (6, 6))?;
        a.menu("Edit > Copy")?;
        let mut want = vec![W; 9];
        want[4] = RED;
        assert_eq!(a.clipboard()?, ClipboardContent::Image(Image { width: 3, height: 3, pixels: want }));
        assert_eq!(pts_of(&a, RED)?, pts(&[(5, 5)]));
        assert_eq!(count_of(&a, W)?, 32 * 32 - 1);
        Ok(())
    }

    #[test]
    fn cut_fills_background() -> T {
        let mut a = fresh()?;
        a.set_pixel(5, 5, RED)?;
        select_rect(&mut a, (4, 4), (6, 6))?;
        set_bg(&mut a, GREEN)?;
        a.menu("Edit > Cut")?;
        assert_eq!(pts_of(&a, GREEN)?, rect_pts(4, 4, 6, 6));
        assert!(a.selection()?.is_none());
        assert!(matches!(a.clipboard()?, ClipboardContent::Image(ref i) if i.width == 3 && i.pixels[4] == RED));
        Ok(())
    }

    #[test]
    fn paste_floating_at_origin() -> T {
        let mut a = fresh()?;
        a.set_clipboard(ClipboardContent::Image(Image { width: 2, height: 2, pixels: vec![RED; 4] }))?;
        a.menu("Edit > Paste")?;
        assert_eq!(a.active_tool()?, Tool::Select);
        let s = a.selection()?.expect("floating selection");
        assert_eq!(s.bounds, Rect::new(0, 0, 1, 1));
        assert!(s.floating);
        click(&mut a, 20, 20)?;
        assert!(a.selection()?.is_none());
        assert_eq!(pts_of(&a, RED)?, rect_pts(0, 0, 1, 1));
        Ok(())
    }

    #[test]
    fn paste_enlarge_yes() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, RED)?;
        a.set_clipboard(ClipboardContent::Image(Image { width: 40, height: 10, pixels: vec![BLUE; 400] }))?;
        ctrl(&mut a, 'v')?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert_eq!(d.title, "Brushling");
        assert_eq!(d.buttons, ["Yes", "No", "Cancel"]);
        a.dialog_click("Yes")?;
        assert_eq!(size(&a)?, (40, 32));
        let img = a.pixels()?;
        assert!(rect_pts(32, 10, 39, 31).iter().all(|&(x, y)| img.get(x, y) == Some(RED)));
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(0, 0, 39, 9)));
        Ok(())
    }

    #[test]
    fn paste_enlarge_cancel() -> T {
        let mut a = fresh()?;
        a.set_clipboard(ClipboardContent::Image(Image { width: 40, height: 10, pixels: vec![BLUE; 400] }))?;
        ctrl(&mut a, 'v')?;
        expect_dialog(&a, DialogKind::MessageBox)?;
        a.dialog_click("Cancel")?;
        assert_eq!(size(&a)?, (32, 32));
        assert!(a.selection()?.is_none());
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test]
    fn freeform_copy_outside_is_background() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::FreeFormSelect)?;
        trace(&mut a, &[(2, 2), (12, 2), (2, 12), (2, 2)])?;
        set_bg(&mut a, RED)?;
        ctrl(&mut a, 'c')?;
        let img = match a.clipboard()? {
            ClipboardContent::Image(i) => i,
            c => return fail(format!("clipboard {c:?}")),
        };
        assert_eq!((img.width, img.height), (11, 11));
        assert_eq!(img.get(10, 10), Some(RED));
        assert_eq!(img.get(0, 0), Some(W));
        assert_eq!(img.get(2, 2), Some(W));
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test]
    fn empty_clipboard_paste_disabled() -> T {
        let mut a = fresh()?;
        assert!(!enabled(&a, "Edit > Paste")?);
        ctrl(&mut a, 'v')?;
        assert!(a.selection()?.is_none());
        a.set_clipboard(ClipboardContent::Image(Image { width: 1, height: 1, pixels: vec![RED] }))?;
        assert!(enabled(&a, "Edit > Paste")?);
        Ok(())
    }
}

mod edit_clear_select_all {
    use crate::common::*;

    #[test]
    fn del_clears_to_background() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        click(&mut a, 10, 10)?;
        assert_eq!(px(&a, 10, 10)?, K);
        select_rect(&mut a, (8, 8), (12, 12))?;
        set_bg(&mut a, BLUE)?;
        key(&mut a, Key::Delete)?;
        assert_eq!(pts_of(&a, BLUE)?, rect_pts(8, 8, 12, 12));
        assert!(k_pts(&a)?.is_empty());
        assert!(a.selection()?.is_none());
        Ok(())
    }

    #[test]
    fn ctrl_a_selects_all() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'a')?;
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(0, 0, 31, 31)));
        assert_eq!(a.active_tool()?, Tool::Select);
        Ok(())
    }

    #[test]
    fn clear_then_undo() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 8, 8)?;
        pencil_click(&mut a, 10, 10)?;
        select_rect(&mut a, (7, 7), (11, 11))?;
        a.menu("Edit > Clear Selection")?;
        assert!(k_pts(&a)?.is_empty());
        undo(&mut a)?;
        assert_eq!(k_pts(&a)?, pts(&[(8, 8), (10, 10)]));
        Ok(())
    }

    #[test]
    fn freeform_clear_only_mask() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::FreeFormSelect)?;
        trace(&mut a, &[(2, 2), (12, 2), (2, 12), (2, 2)])?;
        set_bg(&mut a, RED)?;
        key(&mut a, Key::Delete)?;
        assert_eq!(px(&a, 3, 3)?, RED);
        assert_eq!(px(&a, 2, 2)?, RED);
        assert_eq!(px(&a, 12, 12)?, W);
        assert_eq!(px(&a, 11, 11)?, W);
        Ok(())
    }
}

mod edit_copy_to_paste_from {
    use crate::common::*;

    #[test] // layer=format
    fn copy_to_writes_selection() -> T {
        let dir = tmp("copyto");
        let mut a = fresh()?;
        select_rect(&mut a, (0, 0), (3, 1))?;
        a.menu("Edit > Copy To...")?;
        save_in_dialog(&mut a, &dir.join("part"), T24)?;
        assert_eq!(dir_names(&dir)?, ["part.bmp"]);
        let b = decode_file(&dir.join("part.bmp"))?;
        assert_eq!((b.width, b.height), (4, 2));
        assert!(b.image.pixels.iter().all(|&c| c == W));
        assert_eq!(a.title()?, "untitled - Brushling");
        assert_eq!(a.document()?.path, None);
        Ok(())
    }

    #[test]
    fn paste_from_file() -> T {
        let dir = tmp("pastefrom");
        let p = bmp_file(&dir, "blue.bmp", 3, 3, &solid(3, 3, BLUE))?;
        let mut a = fresh()?;
        a.menu("Edit > Paste From...")?;
        open_in_dialog(&mut a, &p)?;
        let s = a.selection()?.expect("floating selection");
        assert_eq!(s.bounds, Rect::new(0, 0, 2, 2));
        click(&mut a, 20, 20)?;
        assert_eq!(pts_of(&a, BLUE)?, rect_pts(0, 0, 2, 2));
        Ok(())
    }

    #[test]
    fn copy_to_cancel_writes_nothing() -> T {
        let dir = tmp("copyto-cancel");
        let mut a = fresh()?;
        select_rect(&mut a, (0, 0), (3, 3))?;
        a.menu("Edit > Copy To...")?;
        expect_dialog(&a, DialogKind::FileSave)?;
        a.dialog_set("File name", &ps(&dir.join("never")))?;
        a.dialog_click("Cancel")?;
        assert!(dir_names(&dir)?.is_empty());
        Ok(())
    }

    #[test]
    fn paste_from_bad_file_shows_message() -> T {
        let dir = tmp("pastefrom-bad");
        let p = dir.join("bad.bmp");
        write(&p, b"definitely not a bitmap")?;
        let mut a = fresh()?;
        a.menu("Edit > Paste From...")?;
        open_in_dialog(&mut a, &p)?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(d.text.contains("This file cannot be opened"));
        a.dialog_click("OK")?;
        assert!(a.selection()?.is_none());
        Ok(())
    }
}
