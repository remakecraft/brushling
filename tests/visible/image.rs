//! image.*, canvas.resize_handle

mod image_flip_rotate {
    use crate::common::*;

    #[test]
    fn flip_h_2x1() -> T {
        let mut a = image(2, 1, &[RED, GREEN])?;
        flip_rotate(&mut a, "Flip horizontal", "")?;
        assert_eq!(a.pixels()?.pixels, [GREEN, RED]);
        Ok(())
    }

    #[test]
    fn rotate90_2x1() -> T {
        let mut a = image(2, 1, &[RED, GREEN])?;
        flip_rotate(&mut a, "Rotate by angle", "90")?;
        assert_eq!(a.pixels()?, Image { width: 1, height: 2, pixels: vec![RED, GREEN] });
        Ok(())
    }

    #[test]
    fn rotate270_2x1() -> T {
        let mut a = image(2, 1, &[RED, GREEN])?;
        flip_rotate(&mut a, "Rotate by angle", "270")?;
        assert_eq!(a.pixels()?, Image { width: 1, height: 2, pixels: vec![GREEN, RED] });
        Ok(())
    }

    #[test]
    fn rotate180_undo() -> T {
        let p = pattern(3, 2, 8);
        let mut a = image(3, 2, &p)?;
        flip_rotate(&mut a, "Rotate by angle", "180")?;
        assert_eq!(px(&a, 0, 0)?, p[5]);
        assert_eq!(px(&a, 2, 1)?, p[0]);
        undo(&mut a)?;
        assert_eq!(a.pixels()?.pixels, p);
        Ok(())
    }

    #[test]
    fn cancel_changes_nothing() -> T {
        let p = pattern(3, 2, 8);
        let mut a = image(3, 2, &p)?;
        a.menu("Image > Flip/Rotate...")?;
        expect_dialog(&a, DialogKind::FlipRotate)?;
        a.dialog_set("Action", "Flip vertical")?;
        a.dialog_click("Cancel")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(a.pixels()?.pixels, p);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn dialog_defaults() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'r')?;
        let d = expect_dialog(&a, DialogKind::FlipRotate)?;
        assert_eq!(d.field("Action"), Some("Flip horizontal"));
        assert_eq!(d.field("Angle"), Some("90"));
        let angle_enabled = |d: &Dialog| d.fields.iter().find(|f| f.name == "Angle").map(|f| f.enabled);
        assert_eq!(angle_enabled(&d), Some(false));
        a.dialog_set("Action", "Rotate by angle")?;
        assert_eq!(angle_enabled(&dlg(&a)?), Some(true));
        Ok(())
    }
}

mod image_stretch_skew {
    use crate::common::*;

    #[test]
    fn shrink_4x1() -> T {
        let mut a = image(4, 1, &[RED, GREEN, BLUE, W])?;
        stretch_skew(&mut a, "50", "100", "0", "0")?;
        assert_eq!(a.pixels()?, Image { width: 2, height: 1, pixels: vec![GREEN, W] });
        Ok(())
    }

    #[test]
    fn grow_2x1() -> T {
        let mut a = image(2, 1, &[RED, GREEN])?;
        stretch_skew(&mut a, "200", "100", "0", "0")?;
        assert_eq!(a.pixels()?, Image { width: 4, height: 1, pixels: vec![RED, RED, GREEN, GREEN] });
        Ok(())
    }

    #[test]
    fn width_rounding_3x1() -> T {
        let mut a = image(3, 1, &[RED, GREEN, BLUE])?;
        stretch_skew(&mut a, "50", "100", "0", "0")?;
        assert_eq!(size(&a)?, (2, 1));
        Ok(())
    }

    #[test] // layer=pixel
    fn skew_h45() -> T {
        let mut a = image(3, 3, &[K; 9])?;
        stretch_skew(&mut a, "100", "100", "45", "0")?;
        assert_eq!(
            a.pixels()?,
            Image {
                width: 5,
                height: 3,
                pixels: vec![W, W, K, K, K, W, K, K, K, W, K, K, K, W, W]
            }
        );
        Ok(())
    }

    #[test] // layer=pixel
    fn skew_v45() -> T {
        let mut a = image(3, 3, &[K; 9])?;
        stretch_skew(&mut a, "100", "100", "0", "45")?;
        assert_eq!(size(&a)?, (3, 5));
        let col = |a: &App, x| -> HResult<Vec<Rgb>> { (0..5).map(|y| px(a, x, y)).collect() };
        assert_eq!(col(&a, 2)?, [K, K, K, W, W]);
        assert_eq!(col(&a, 0)?, [W, W, K, K, K]);
        Ok(())
    }

    #[test]
    fn invalid_stretch_negative() -> T {
        let p = pattern(4, 4, 1);
        let mut a = image(4, 4, &p)?;
        a.menu("Image > Stretch/Skew...")?;
        a.dialog_set("Stretch Horizontal", "-5")?;
        a.dialog_click("OK")?;
        let m = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(m.text.contains('1') && m.text.contains("500"), "{}", m.text);
        a.dialog_click("OK")?;
        expect_dialog(&a, DialogKind::StretchSkew)?;
        a.dialog_click("Cancel")?;
        assert_eq!(a.pixels()?.pixels, p);
        Ok(())
    }

    #[test]
    fn invalid_skew_90() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'w')?;
        a.dialog_set("Skew Vertical", "90")?;
        a.dialog_click("OK")?;
        let m = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(m.text.contains('0') && m.text.contains("89"), "{}", m.text);
        Ok(())
    }

    #[test]
    fn identity_no_undo() -> T {
        let mut a = fresh()?;
        a.menu("Image > Stretch/Skew...")?;
        let d = expect_dialog(&a, DialogKind::StretchSkew)?;
        assert_eq!(d.field("Stretch Horizontal"), Some("100"));
        assert_eq!(d.field("Stretch Vertical"), Some("100"));
        assert_eq!(d.field("Skew Horizontal"), Some("0"));
        assert_eq!(d.field("Skew Vertical"), Some("0"));
        a.dialog_click("OK")?;
        assert!(a.dialog()?.is_none());
        assert!(!undo_enabled(&a)?);
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }
}

mod image_invert {
    use crate::common::*;

    #[test]
    fn pixel_complement() -> T {
        let mut a = image(2, 1, &[rgb(10, 200, 255), K])?;
        ctrl(&mut a, 'i')?;
        assert_eq!(a.pixels()?.pixels, [rgb(245, 55, 0), W]);
        Ok(())
    }

    #[test]
    fn selection_only() -> T {
        let mut a = fresh()?;
        select_rect(&mut a, (0, 0), (1, 1))?;
        ctrl(&mut a, 'i')?;
        click(&mut a, 20, 20)?;
        assert_eq!(k_pts(&a)?, rect_pts(0, 0, 1, 1));
        assert_eq!(px(&a, 2, 2)?, W);
        Ok(())
    }

    #[test]
    fn invert_twice_restores() -> T {
        let p = pattern(6, 5, 3);
        let mut a = image(6, 5, &p)?;
        a.menu("Image > Invert Colors")?;
        assert_ne!(a.pixels()?.pixels, p);
        a.menu("Image > Invert Colors")?;
        assert_eq!(a.pixels()?.pixels, p);
        Ok(())
    }

    #[test]
    fn one_undo_step() -> T {
        let p = pattern(4, 4, 6);
        let mut a = image(4, 4, &p)?;
        ctrl(&mut a, 'i')?;
        assert!(undo_enabled(&a)?);
        undo(&mut a)?;
        assert_eq!(a.pixels()?.pixels, p);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }
}

mod image_attributes {
    use crate::common::*;

    #[test]
    fn units_display_512() -> T {
        let mut a = start()?;
        a.menu("Image > Attributes...")?;
        let d = expect_dialog(&a, DialogKind::Attributes)?;
        assert_eq!((d.field("Width"), d.field("Height")), (Some("512"), Some("384")));
        assert_eq!(d.field("Units"), Some("Pixels"));
        a.dialog_set("Units", "Inches")?;
        let d = dlg(&a)?;
        assert_eq!((d.field("Width"), d.field("Height")), (Some("5.33"), Some("4.00")));
        a.dialog_set("Units", "Cm")?;
        let d = dlg(&a)?;
        assert_eq!((d.field("Width"), d.field("Height")), (Some("13.55"), Some("10.16")));
        Ok(())
    }

    #[test]
    fn one_inch_96() -> T {
        let mut a = fresh()?;
        a.menu("Image > Attributes...")?;
        a.dialog_set("Units", "Inches")?;
        a.dialog_set("Width", "1")?;
        a.dialog_set("Height", "1")?;
        a.dialog_click("OK")?;
        assert_eq!(size(&a)?, (96, 96));
        Ok(())
    }

    #[test]
    fn grow_with_background() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, RED)?;
        attributes(&mut a, "40", "20")?;
        assert_eq!(size(&a)?, (40, 20));
        assert_eq!(pts_of(&a, RED)?, rect_pts(32, 0, 39, 19));
        Ok(())
    }

    #[test]
    fn bw_threshold() -> T {
        let mut a = image(2, 1, &[rgb(200, 100, 50), rgb(128, 128, 128)])?;
        a.menu("Image > Attributes...")?;
        a.dialog_set("Colors", "Black and white")?;
        a.dialog_click("OK")?;
        assert_eq!(a.pixels()?.pixels, [K, W]);
        assert_eq!(a.document()?.color_mode, ColorMode::BlackAndWhite);
        Ok(())
    }

    #[test]
    fn bw_paint_snaps() -> T {
        let mut a = fresh()?;
        a.menu("Image > Attributes...")?;
        a.dialog_set("Colors", "Black and white")?;
        a.dialog_click("OK")?;
        set_fg(&mut a, RED)?;
        pencil_click(&mut a, 1, 1)?;
        assert_eq!(px(&a, 1, 1)?, K);
        Ok(())
    }

    #[test]
    fn default_button() -> T {
        let mut a = fresh()?;
        a.menu("Image > Attributes...")?;
        a.dialog_click("Default")?;
        a.dialog_click("OK")?;
        assert_eq!(size(&a)?, (512, 384));
        Ok(())
    }

    #[test]
    fn invalid_size_message() -> T {
        let mut a = fresh()?;
        a.menu("Image > Attributes...")?;
        a.dialog_set("Width", "0")?;
        a.dialog_click("OK")?;
        let m = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(m.text.contains("Enter a size from 1 to 9999"), "{}", m.text);
        a.dialog_click("OK")?;
        expect_dialog(&a, DialogKind::Attributes)?;
        a.dialog_click("Cancel")?;
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }
}

mod image_clear_image {
    use crate::common::*;

    #[test]
    fn clear_to_background() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 3, 3)?;
        pencil_click(&mut a, 9, 4)?;
        set_bg(&mut a, BLUE)?;
        a.menu("Image > Clear Image")?;
        assert!(all_are(&a, BLUE)?);
        assert_eq!(a.colors()?, (K, BLUE));
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test]
    fn clear_then_undo() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 3, 3)?;
        pencil_click(&mut a, 9, 4)?;
        a.menu("Image > Clear Image")?;
        undo(&mut a)?;
        assert_eq!(k_pts(&a)?, pts(&[(3, 3), (9, 4)]));
        Ok(())
    }

    #[test]
    fn clear_discards_selection() -> T {
        let mut a = fresh()?;
        a.set_pixel(3, 3, K)?;
        select_rect(&mut a, (2, 2), (5, 5))?;
        drag(&mut a, (3, 3), (13, 13))?;
        set_bg(&mut a, GREEN)?;
        a.menu("Image > Clear Image")?;
        assert!(a.selection()?.is_none());
        assert!(all_are(&a, GREEN)?);
        Ok(())
    }

    #[test]
    fn clear_sets_dirty() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, RED)?;
        a.menu("Image > Clear Image")?;
        assert!(all_are(&a, RED)?);
        assert!(a.document()?.dirty);
        Ok(())
    }
}

mod canvas_resize_handle {
    use crate::common::*;

    #[test]
    fn corner_grow() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, GREEN)?;
        drag_canvas_handle(&mut a, CanvasHandle::Corner, 40, 36)?;
        assert_eq!(size(&a)?, (40, 36));
        let want: Pts = rect_pts(32, 0, 39, 35).union(&rect_pts(0, 32, 39, 35)).copied().collect();
        assert_eq!(pts_of(&a, GREEN)?, want);
        Ok(())
    }

    #[test]
    fn right_edge_crop() -> T {
        let mut a = fresh()?;
        drag_canvas_handle(&mut a, CanvasHandle::Right, 2, 26)?;
        assert_eq!(size(&a)?, (2, 32));
        Ok(())
    }

    #[test]
    fn undo_restores() -> T {
        let mut a = fresh()?;
        a.set_pixel(31, 31, RED)?;
        drag_canvas_handle(&mut a, CanvasHandle::Corner, 10, 10)?;
        assert_eq!(size(&a)?, (10, 10));
        assert!(a.document()?.dirty);
        undo(&mut a)?;
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(px(&a, 31, 31)?, RED);
        Ok(())
    }

    #[test]
    fn status_size_while_dragging() -> T {
        let mut a = fresh()?;
        a.pointer(PointerKind::Press(L), Pos::CanvasHandle(CanvasHandle::Corner), NO)?;
        mv(&mut a, 40, 36)?;
        assert_eq!(a.status_bar()?.size, "40,36");
        release(&mut a, 40, 36)?;
        assert_eq!(size(&a)?, (40, 36));
        Ok(())
    }
}
