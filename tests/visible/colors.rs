//! colors.*

mod colors_color_box {
    use crate::common::*;

    #[test]
    fn swatch_values() -> T {
        let a = start()?;
        let s = a.swatches()?;
        assert_eq!(s.len(), 28);
        assert_eq!(s[10], rgb(0, 128, 255));
        assert_eq!(s[27], rgb(255, 128, 64));
        assert_eq!(s, SWATCHES);
        Ok(())
    }

    #[test]
    fn left_and_right_click() -> T {
        let mut a = fresh()?;
        a.click_swatch(17, L, false)?;
        assert_eq!(a.colors()?, (RED, W));
        a.click_swatch(21, RB, false)?;
        assert_eq!(a.colors()?, (RED, BLUE));
        Ok(())
    }

    #[test]
    fn first_start_colours() -> T {
        let a = start()?;
        assert_eq!(a.colors()?, (K, W));
        Ok(())
    }

    #[test]
    fn click_changes_no_pixels() -> T {
        let mut a = fresh()?;
        a.click_swatch(3, L, false)?;
        assert_eq!(a.colors()?.0, rgb(128, 0, 0));
        assert!(all_are(&a, W)?);
        assert!(!undo_enabled(&a)?);
        assert!(!a.document()?.dirty);
        Ok(())
    }
}

mod colors_edit_colors {
    use crate::common::*;

    #[test]
    fn rgb_to_hsl() -> T {
        let mut a = fresh()?;
        a.menu("Colors > Edit Colors...")?;
        edit_colors_rgb(&mut a, rgb(255, 0, 0))?;
        let d = dlg(&a)?;
        assert_eq!(field_f64(&d, "Hue")?, 0.0);
        assert_eq!(field_f64(&d, "Sat")?, 240.0);
        assert_eq!(field_f64(&d, "Lum")?, 120.0);
        Ok(())
    }

    #[test]
    fn double_click_ok() -> T {
        let mut a = fresh()?;
        redefine_swatch(&mut a, 5, rgb(10, 20, 30))?;
        assert_eq!(a.swatches()?[4], rgb(10, 20, 30));
        assert_eq!(a.colors()?.0, rgb(10, 20, 30));
        assert!(a.dialog()?.is_none());
        Ok(())
    }

    #[test]
    fn double_click_cancel() -> T {
        let mut a = fresh()?;
        a.click_swatch(5, L, true)?;
        expect_dialog(&a, DialogKind::EditColors)?;
        edit_colors_rgb(&mut a, rgb(10, 20, 30))?;
        a.dialog_click("Cancel")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(a.swatches()?[4], rgb(0, 128, 0));
        Ok(())
    }

    #[test]
    fn add_custom_three() -> T {
        let mut a = fresh()?;
        a.menu("Colors > Edit Colors...")?;
        let cs = [rgb(1, 2, 3), rgb(200, 100, 50), rgb(0, 255, 0)];
        expect_dialog(&a, DialogKind::EditColors)?;
        a.dialog_click("Define Custom Colors >>")?;
        for c in cs {
            a.dialog_set("Red", &c.r.to_string())?;
            a.dialog_set("Green", &c.g.to_string())?;
            a.dialog_set("Blue", &c.b.to_string())?;
            a.dialog_click("Add to Custom Colors")?;
        }
        let d = dlg(&a)?;
        for (i, c) in cs.iter().enumerate() {
            let f = d.field(&format!("Custom {}", i + 1)).map(str::to_string);
            assert_eq!(f, Some(format!("{},{},{}", c.r, c.g, c.b)));
        }
        assert_eq!(d.field("Custom 4").map(str::to_string), Some("255,255,255".to_string()));
        Ok(())
    }

    #[test]
    fn menu_edits_last_clicked_swatch() -> T {
        let mut a = fresh()?;
        a.click_swatch(9, RB, false)?;
        a.menu("Colors > Edit Colors...")?;
        edit_colors_rgb(&mut a, rgb(1, 1, 1))?;
        a.dialog_click("OK")?;
        let s = a.swatches()?;
        assert_eq!(s[8], rgb(1, 1, 1));
        assert_eq!(s[0], K);
        Ok(())
    }
}

mod colors_indexed_snap {
    use crate::common::*;

    fn two_colour_doc() -> HResult<App> {
        let dir = tmp("snap");
        let p = dir.join("kw.bmp");
        write(&p, &bmp_indexed(4, 4, 8, &[K, W], &[1; 16]))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        Ok(a)
    }

    #[test]
    fn red_snaps_to_black() -> T {
        let mut a = two_colour_doc()?;
        assert!(all_are(&a, W)?);
        set_fg(&mut a, RED)?;
        pencil_click(&mut a, 0, 0)?;
        assert_eq!(px(&a, 0, 0)?, K);
        Ok(())
    }

    #[test]
    fn light_grey_snaps_to_white() -> T {
        let mut a = two_colour_doc()?;
        a.set_pixel(0, 0, K)?;
        set_fg(&mut a, rgb(200, 200, 200))?;
        pencil_click(&mut a, 0, 0)?;
        assert_eq!(px(&a, 0, 0)?, W);
        assert!(undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn fill_snaps_in_16_colour() -> T {
        let dir = tmp("snap16");
        let p = dir.join("p16.bmp");
        write(&p, &bmp_indexed(4, 4, 4, &PAL16, &[15; 16]))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        set_fg(&mut a, rgb(140, 10, 10))?;
        a.select_tool(Tool::Fill)?;
        click(&mut a, 1, 1)?;
        assert!(all_are(&a, rgb(128, 0, 0))?);
        Ok(())
    }

    #[test]
    fn indicator_unsnapped() -> T {
        let mut a = two_colour_doc()?;
        a.click_swatch(17, L, false)?;
        assert_eq!(a.colors()?.0, RED);
        pencil_click(&mut a, 1, 1)?;
        assert_eq!(px(&a, 1, 1)?, K);
        assert_eq!(a.colors()?.0, RED);
        Ok(())
    }
}
