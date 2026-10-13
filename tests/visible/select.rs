//! tool.select, selection.*

mod tool_select {
    use crate::common::*;

    #[test]
    fn rect_drag_reversed() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (5, 6), (2, 3))?;
        let s = a.selection()?.expect("selection");
        assert_eq!(s.bounds, Rect::new(2, 3, 5, 6));
        assert_eq!(s.mask, vec![true; 16]);
        Ok(())
    }

    #[test]
    fn freeform_square() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::FreeFormSelect)?;
        trace(&mut a, &[(2, 2), (12, 2), (12, 12), (2, 12), (2, 2)])?;
        let s = a.selection()?.expect("selection");
        assert_eq!(s.bounds, Rect::new(2, 2, 12, 12));
        assert!(s.contains(7, 7));
        assert!(s.contains(2, 2));
        assert!(!s.contains(13, 13));
        Ok(())
    }

    #[test]
    fn click_outside_ends() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (2, 2), (5, 5))?;
        assert!(a.selection()?.is_some());
        click(&mut a, 20, 20)?;
        assert!(a.selection()?.is_none());
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test]
    fn no_undo_step() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (1, 1), (9, 9))?;
        assert!(a.selection()?.is_some());
        assert!(!undo_enabled(&a)?);
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test]
    fn clamped_to_canvas() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (-5, -5), (3, 3))?;
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(0, 0, 3, 3)));
        Ok(())
    }

    #[test]
    fn click_selects_nothing() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Select)?;
        click(&mut a, 5, 5)?;
        assert!(a.selection()?.is_none());
        drag(&mut a, (5, 5), (8, 8))?;
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(5, 5, 8, 8)));
        Ok(())
    }
}

mod selection_transparency {
    use crate::common::*;

    fn setup(opaque: bool) -> HResult<App> {
        let mut a = fresh()?;
        fill_rect(&mut a, 10, 0, 19, 9, RED)?;
        pencil_click(&mut a, 5, 5)?;
        select_rect(&mut a, (0, 0), (9, 9))?;
        opt(&mut a, ToolOption::Opaque(opaque))?;
        drag(&mut a, (3, 3), (13, 3))?;
        click(&mut a, 25, 25)?;
        Ok(a)
    }

    #[test]
    fn draw_opaque_checked_at_start() -> T {
        let a = start()?;
        assert!(checked(&a, "Image > Draw Opaque")?);
        assert!(a.tool_options()?.opaque);
        Ok(())
    }

    #[test]
    fn transparent_option_unchecks() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Select)?;
        opt(&mut a, ToolOption::Opaque(false))?;
        assert!(!checked(&a, "Image > Draw Opaque")?);
        opt(&mut a, ToolOption::Opaque(true))?;
        assert!(checked(&a, "Image > Draw Opaque")?);
        Ok(())
    }

    #[test] // layer=pixel
    fn transparent_move() -> T {
        let a = setup(false)?;
        assert_eq!(px(&a, 15, 5)?, K);
        assert_eq!(px(&a, 16, 6)?, RED);
        Ok(())
    }

    #[test] // layer=pixel
    fn opaque_move() -> T {
        let a = setup(true)?;
        assert_eq!(px(&a, 15, 5)?, K);
        assert_eq!(px(&a, 16, 6)?, W);
        Ok(())
    }
}

mod selection_move_copy {
    use crate::common::*;

    fn dot_and_select(bg: Rgb) -> HResult<App> {
        let mut a = fresh()?;
        pencil_click(&mut a, 2, 2)?;
        select_rect(&mut a, (1, 1), (3, 3))?;
        set_bg(&mut a, bg)?;
        Ok(a)
    }

    #[test]
    fn move_leaves_background_white() -> T {
        let mut a = dot_and_select(W)?;
        drag(&mut a, (2, 2), (12, 2))?;
        click(&mut a, 25, 25)?;
        assert_eq!(k_pts(&a)?, pts(&[(12, 2)]));
        assert_eq!(px(&a, 2, 2)?, W);
        Ok(())
    }

    #[test]
    fn move_leaves_background_colour() -> T {
        let mut a = dot_and_select(RED)?;
        drag(&mut a, (2, 2), (12, 2))?;
        click(&mut a, 25, 25)?;
        assert_eq!(pts_of(&a, RED)?, rect_pts(1, 1, 3, 3));
        assert_eq!(px(&a, 12, 2)?, K);
        Ok(())
    }

    #[test]
    fn ctrl_stamps_copy() -> T {
        let mut a = dot_and_select(W)?;
        drag_with(&mut a, (2, 2), (12, 2), L, Mods::CTRL)?;
        click(&mut a, 25, 25)?;
        assert_eq!(k_pts(&a)?, pts(&[(2, 2), (12, 2)]));
        Ok(())
    }

    #[test]
    fn shift_trail() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 2, 2)?;
        select_rect(&mut a, (2, 2), (2, 2))?;
        press_with(&mut a, 2, 2, L, Mods::SHIFT)?;
        for x in 3..=6 {
            mv_with(&mut a, x, 2, Mods::SHIFT)?;
        }
        release_with(&mut a, 6, 2, L, Mods::SHIFT)?;
        click(&mut a, 25, 25)?;
        for x in 3..=6 {
            assert_eq!(px(&a, x, 2)?, K, "({x},2)");
        }
        Ok(())
    }

    #[test]
    fn move_is_one_undo_step() -> T {
        let mut a = dot_and_select(W)?;
        drag(&mut a, (2, 2), (12, 2))?;
        undo(&mut a)?;
        assert_eq!(px(&a, 2, 2)?, K);
        assert_eq!(px(&a, 12, 2)?, W);
        Ok(())
    }
}

mod selection_resize {
    use crate::common::*;

    #[test] // layer=pixel
    fn right_handle_stretch() -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1, 0, GREEN)?;
        select_rect(&mut a, (0, 0), (1, 0))?;
        drag_sel_handle(&mut a, Handle::Right, 3, 0)?;
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(0, 0, 3, 0)));
        click(&mut a, 20, 20)?;
        let row: Vec<Rgb> = (0..4).map(|x| px(&a, x, 0)).collect::<HResult<_>>()?;
        assert_eq!(row, [RED, RED, GREEN, GREEN]);
        Ok(())
    }

    #[test] // layer=pixel
    fn corner_handle_stretch() -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1, 0, GREEN)?;
        a.set_pixel(0, 1, BLUE)?;
        a.set_pixel(1, 1, K)?;
        select_rect(&mut a, (0, 0), (1, 1))?;
        drag_sel_handle(&mut a, Handle::BottomRight, 3, 3)?;
        click(&mut a, 20, 20)?;
        assert_eq!(pts_of(&a, RED)?, rect_pts(0, 0, 1, 1));
        assert_eq!(pts_of(&a, GREEN)?, rect_pts(2, 0, 3, 1));
        assert_eq!(pts_of(&a, BLUE)?, rect_pts(0, 2, 1, 3));
        assert_eq!(k_pts(&a)?, rect_pts(2, 2, 3, 3));
        Ok(())
    }

    #[test]
    fn resize_one_undo_step() -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1, 0, GREEN)?;
        select_rect(&mut a, (0, 0), (1, 0))?;
        drag_sel_handle(&mut a, Handle::Right, 3, 0)?;
        undo(&mut a)?;
        assert_eq!(px(&a, 0, 0)?, RED);
        assert_eq!(px(&a, 1, 0)?, GREEN);
        assert_eq!(px(&a, 2, 0)?, W);
        Ok(())
    }
}

mod selection_image_ops_scope {
    use crate::common::*;

    #[test]
    fn flip_selection_h() -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1, 0, GREEN)?;
        select_rect(&mut a, (0, 0), (1, 0))?;
        flip_rotate(&mut a, "Flip horizontal", "")?;
        click(&mut a, 20, 20)?;
        assert_eq!((px(&a, 0, 0)?, px(&a, 1, 0)?), (GREEN, RED));
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test]
    fn rotate_selection_bounds() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (0, 0), (3, 1))?;
        flip_rotate(&mut a, "Rotate by angle", "90")?;
        let s = a.selection()?.expect("selection");
        assert_eq!(s.bounds, Rect::new(0, 0, 1, 3));
        assert!(s.floating);
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test]
    fn invert_selection_not_floating() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (2, 2), (4, 4))?;
        a.menu("Image > Invert Colors")?;
        let s = a.selection()?.expect("selection");
        assert!(!s.floating);
        assert_eq!(k_pts(&a)?, rect_pts(2, 2, 4, 4));
        Ok(())
    }

    #[test]
    fn stretch_selection() -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1, 0, GREEN)?;
        select_rect(&mut a, (0, 0), (1, 0))?;
        stretch_skew(&mut a, "200", "100", "0", "0")?;
        assert_eq!(a.selection()?.map(|s| s.bounds), Some(Rect::new(0, 0, 3, 0)));
        click(&mut a, 20, 20)?;
        let row: Vec<Rgb> = (0..4).map(|x| px(&a, x, 0)).collect::<HResult<_>>()?;
        assert_eq!(row, [RED, RED, GREEN, GREEN]);
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }
}
