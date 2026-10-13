//! file.*

mod file_new {
    use crate::common::*;

    #[test]
    fn new_fills_background_and_resets_colours() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, BLUE)?;
        a.menu("File > New")?;
        assert_eq!(size(&a)?, (512, 384));
        assert!(all_are(&a, BLUE)?);
        assert_eq!(a.colors()?, (K, W));
        Ok(())
    }

    #[test]
    fn no_prompt_when_clean() -> T {
        let mut a = fresh()?;
        a.menu("File > New")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(size(&a)?, (512, 384));
        Ok(())
    }

    #[test]
    fn cancel_keeps_document() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > New")?;
        expect_dialog(&a, DialogKind::MessageBox)?;
        a.dialog_click("Cancel")?;
        assert_eq!(px(&a, 1, 1)?, K);
        assert_eq!(size(&a)?, (32, 32));
        Ok(())
    }

    #[test]
    fn swatch_redefinition_survives() -> T {
        let mut a = fresh()?;
        redefine_swatch(&mut a, 3, rgb(1, 2, 3))?;
        file_new_discard(&mut a)?;
        assert_eq!(size(&a)?, (512, 384));
        assert_eq!(a.swatches()?[2], rgb(1, 2, 3));
        Ok(())
    }

    #[test]
    fn undo_disabled_after_new() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        assert!(undo_enabled(&a)?);
        file_new_discard(&mut a)?;
        assert_eq!(size(&a)?, (512, 384));
        assert!(!undo_enabled(&a)?);
        assert!(!repeat_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn tool_and_options_kept() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        opt(&mut a, ToolOption::BrushShape(6))?;
        a.menu("File > New")?;
        assert_eq!(size(&a)?, (512, 384));
        assert_eq!(a.active_tool()?, Tool::Brush);
        assert_eq!(a.tool_options()?.brush_shape, 6);
        Ok(())
    }
}

mod file_save_changes_prompt {
    use crate::common::*;

    #[test]
    fn prompt_has_three_buttons() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > New")?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert_eq!(d.buttons, ["Yes", "No", "Cancel"]);
        assert!(d.text.contains("Save changes to untitled?"));
        Ok(())
    }

    #[test]
    fn no_discards() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > New")?;
        a.dialog_click("No")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(size(&a)?, (512, 384));
        assert!(all_are(&a, W)?);
        assert_eq!(a.document()?.path, None);
        Ok(())
    }

    #[test]
    fn yes_then_cancel_keeps() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > New")?;
        a.dialog_click("Yes")?;
        expect_dialog(&a, DialogKind::FileSave)?;
        a.dialog_click("Cancel")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(px(&a, 1, 1)?, K);
        Ok(())
    }

    #[test]
    fn exit_clean_no_prompt() -> T {
        let mut a = fresh()?;
        a.menu("File > Exit")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(a.exit_status()?, Some(0));
        Ok(())
    }

    #[test] // layer=format
    fn yes_saves_to_path() -> T {
        let dir = tmp("yes");
        let p = bmp_file(&dir, "a.bmp", 4, 4, &solid(4, 4, W))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > New")?;
        a.dialog_click("Yes")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(size(&a)?, (512, 384));
        assert_eq!(decode_file(&p)?.image.get(1, 1), Some(K));
        Ok(())
    }
}

mod file_open {
    use crate::common::*;

    #[test] // layer=format
    fn open_24bit_3x2() -> T {
        let dir = tmp("open");
        let px = pattern(3, 2, 1);
        let p = bmp_file(&dir, "three.bmp", 3, 2, &px)?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        assert_eq!(size(&a)?, (3, 2));
        assert_eq!(a.pixels()?.pixels, px);
        assert_eq!(a.title()?, "three.bmp - Brushling");
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn text_file_rejected() -> T {
        let dir = tmp("txt");
        let p = dir.join("x.bmp");
        write(&p, b"hello, this is plain text\r\n")?;
        let mut a = fresh()?;
        a.set_pixel(1, 1, K)?;
        open_path(&mut a, &p)?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert_eq!(d.title, "Brushling");
        assert!(d.text.contains(&ps(&p)), "message lacks path: {}", d.text);
        assert!(d.text.contains("This file cannot be opened"));
        a.dialog_click("OK")?;
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(k_pts(&a)?, pts(&[(1, 1)]));
        Ok(())
    }

    #[test]
    fn compression_4_rejected_memory() -> T {
        let dir = tmp("jpegbmp");
        let p = dir.join("j.bmp");
        write(&p, &bmp_raw(2, 2, 24, 4, &[], &[], &[0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1]))?;
        let mut a = fresh()?;
        a.set_pixel(3, 3, RED)?;
        open_path(&mut a, &p)?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(d.text.contains("There is not enough memory or resources to open this image."));
        a.dialog_click("OK")?;
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(px(&a, 3, 3)?, RED);
        Ok(())
    }

    #[test]
    fn open_keeps_colours_and_zoom() -> T {
        let dir = tmp("keep");
        let p = bmp_file(&dir, "k.bmp", 40, 30, &solid(40, 30, BLUE))?;
        let mut a = fresh()?;
        a.set_colors(RED, GREEN)?;
        a.menu("View > Zoom > Large Size")?;
        select_rect(&mut a, (1, 1), (5, 5))?;
        open_path(&mut a, &p)?;
        assert_eq!(size(&a)?, (40, 30));
        assert_eq!(a.colors()?, (RED, GREEN));
        assert_eq!(a.zoom()?, 400);
        assert!(a.selection()?.is_none());
        let d = a.document()?;
        assert!(!d.dirty);
        assert_eq!(d.source_depth, 24);
        assert_eq!(d.path, Some(p));
        Ok(())
    }

    #[test]
    fn open_dialog_filter() -> T {
        let mut a = fresh()?;
        ctrl(&mut a, 'o')?;
        let d = expect_dialog(&a, DialogKind::FileOpen)?;
        assert_eq!(d.field("Files of type"), Some("Bitmap files (*.bmp;*.dib)"));
        a.dialog_click("Cancel")?;
        assert!(a.dialog()?.is_none());
        Ok(())
    }
}

mod file_save {
    use crate::common::*;

    #[test]
    fn ctrl_s_new_shows_save_as() -> T {
        let mut a = start()?;
        ctrl(&mut a, 's')?;
        let d = expect_dialog(&a, DialogKind::FileSave)?;
        assert!(d.field("File name").unwrap_or("").contains("untitled"));
        assert_eq!(d.field("Save as type"), Some(T24));
        Ok(())
    }

    #[test] // layer=format
    fn save_as_cat_single_file() -> T {
        let dir = tmp("cat");
        let mut a = fresh()?;
        save_as(&mut a, &dir.join("cat"), T24)?;
        assert_eq!(dir_names(&dir)?, ["cat.bmp"]);
        Ok(())
    }

    #[test]
    fn title_and_undo_after_save() -> T {
        let dir = tmp("cat2");
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        save_as(&mut a, &dir.join("cat"), T24)?;
        assert!(undo_enabled(&a)?);
        assert_eq!(a.title()?, "cat.bmp - Brushling");
        assert!(!a.document()?.dirty);
        Ok(())
    }

    #[test] // layer=format
    fn four_bit_save_keeps_depth() -> T {
        let dir = tmp("4bit");
        let p = dir.join("four.bmp");
        write(&p, &bmp_indexed(2, 2, 4, &PAL16, &[15, 15, 15, 15]))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        set_fg(&mut a, PAL16[9])?;
        pencil_click(&mut a, 0, 0)?;
        a.menu("File > Save")?;
        assert!(a.dialog()?.is_none());
        let b = read(&p)?;
        assert_eq!(le16(&b, 28), 4);
        assert_eq!(decode(&b)?.image.get(0, 0), Some(RED));
        Ok(())
    }

    #[test]
    fn readonly_save_fails() -> T {
        let dir = tmp("ro");
        let p = dir.join("ro.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        let before = read(&p)?;
        let mut perm = std::fs::metadata(&p)?.permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&p, perm.clone())?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > Save")?;
        let d = a.dialog()?;
        let after = read(&p)?;
        perm.set_readonly(false);
        std::fs::set_permissions(&p, perm)?;
        let d = match d {
            Some(d) => d,
            None => return fail("no error message"),
        };
        assert!(d.text.contains("The file could not be saved"));
        assert_eq!(before, after);
        Ok(())
    }

    #[test] // layer=format
    fn save_roundtrip_pixels() -> T {
        let dir = tmp("rt");
        let p = dir.join("rt.bmp");
        let px = pattern(5, 3, 9);
        let mut a = image(5, 3, &px)?;
        save_as(&mut a, &p, T24)?;
        assert_eq!(decode_file(&p)?.image.pixels, px);
        let mut b = start()?;
        open_path(&mut b, &p)?;
        assert_eq!(b.pixels()?.pixels, px);
        Ok(())
    }
}

mod file_recent {
    use crate::common::*;
    use std::path::PathBuf;

    fn files(n: usize) -> HResult<Vec<PathBuf>> {
        let dir = tmp("recent");
        let mut v = Vec::new();
        for (i, name) in ["a", "b", "c", "d", "e", "f"].iter().take(n).enumerate() {
            v.push(bmp_file(&dir, &format!("{name}.bmp"), 1, 1, &[rgb(i as u8, 0, 0)])?);
        }
        Ok(v)
    }

    fn entries(paths: &[&PathBuf]) -> Vec<String> {
        paths.iter().enumerate().map(|(i, p)| format!("{} {}", i + 1, ps(p))).collect()
    }

    #[test]
    fn five_opens_keep_four() -> T {
        let f = files(5)?;
        let mut a = start()?;
        for p in &f {
            open_path(&mut a, p)?;
        }
        assert_eq!(recent(&a)?, entries(&[&f[4], &f[3], &f[2], &f[1]]));
        Ok(())
    }

    #[test]
    fn reopen_moves_to_top() -> T {
        let f = files(5)?;
        let mut a = start()?;
        for p in &f {
            open_path(&mut a, p)?;
        }
        open_path(&mut a, &f[2])?;
        assert_eq!(recent(&a)?, entries(&[&f[2], &f[4], &f[3], &f[1]]));
        Ok(())
    }

    #[test]
    fn missing_entry_removed() -> T {
        let f = files(5)?;
        let mut a = start()?;
        for p in &f {
            open_path(&mut a, p)?;
        }
        std::fs::remove_file(&f[3])?;
        a.menu(&format!("File > 2 {}", ps(&f[3])))?;
        expect_dialog(&a, DialogKind::MessageBox)?;
        a.dialog_click("OK")?;
        let r = recent(&a)?;
        assert_eq!(r.len(), 3);
        assert!(r.iter().all(|e| !e.ends_with(&ps(&f[3]))));
        Ok(())
    }

    #[test]
    fn save_adds_entry() -> T {
        let dir = tmp("recent-save");
        let p = dir.join("s.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        assert_eq!(recent(&a)?, entries(&[&p]));
        Ok(())
    }

    #[test]
    fn persists_across_restart() -> T {
        let f = files(2)?;
        let o = opts();
        let mut a = App::start(o.clone())?;
        open_path(&mut a, &f[0])?;
        open_path(&mut a, &f[1])?;
        let b = restart(&mut a, &o)?;
        assert_eq!(recent(&b)?, entries(&[&f[1], &f[0]]));
        Ok(())
    }
}

mod file_exit {
    use crate::common::*;

    #[test]
    fn alt_f4_exits_clean() -> T {
        let mut a = fresh()?;
        a.key(Key::F4, Mods::ALT)?;
        assert!(a.dialog()?.is_none());
        assert_eq!(a.exit_status()?, Some(0));
        Ok(())
    }

    #[test]
    fn cancel_keeps_running() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > Exit")?;
        expect_dialog(&a, DialogKind::MessageBox)?;
        a.dialog_click("Cancel")?;
        assert_eq!(a.exit_status()?, None);
        assert_eq!(px(&a, 1, 1)?, K);
        Ok(())
    }

    #[test]
    fn no_exits_without_saving() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > Exit")?;
        a.dialog_click("No")?;
        assert_eq!(a.exit_status()?, Some(0));
        Ok(())
    }

    #[test] // layer=format
    fn yes_saves_then_exits() -> T {
        let dir = tmp("exit-yes");
        let p = bmp_file(&dir, "e.bmp", 3, 3, &solid(3, 3, W))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        pencil_click(&mut a, 2, 2)?;
        a.menu("File > Exit")?;
        a.dialog_click("Yes")?;
        assert_eq!(a.exit_status()?, Some(0));
        assert_eq!(decode_file(&p)?.image.get(2, 2), Some(K));
        Ok(())
    }
}

mod file_print {
    use crate::common::*;

    fn preview_pages(a: &mut App) -> HResult<u32> {
        a.menu("File > Print Preview")?;
        match a.view_mode()? {
            ViewMode::PrintPreview { pages, .. } => Ok(pages),
            m => fail(format!("not in print preview: {m:?}")),
        }
    }

    #[test]
    fn preview_one_page_default() -> T {
        let mut a = start()?;
        assert_eq!(size(&a)?, (512, 384));
        a.menu("File > Print Preview")?;
        assert_eq!(a.view_mode()?, ViewMode::PrintPreview { page: 1, pages: 1 });
        Ok(())
    }

    #[test]
    fn preview_three_pages_wide() -> T {
        let mut a = fresh_sized(2000, 100)?;
        assert_eq!(preview_pages(&mut a)?, 3);
        Ok(())
    }

    #[test]
    fn landscape_three_pages() -> T {
        let mut a = fresh_sized(2000, 100)?;
        a.menu("File > Page Setup...")?;
        expect_dialog(&a, DialogKind::PageSetup)?;
        a.dialog_set("Orientation", "Landscape")?;
        a.dialog_click("OK")?;
        assert_eq!(preview_pages(&mut a)?, 3);
        Ok(())
    }

    #[test]
    fn landscape_changes_page_count() -> T {
        // 673 px wide: 2 portrait columns (672 px), 1 landscape column (912 px).
        let mut a = fresh_sized(673, 100)?;
        assert_eq!(preview_pages(&mut a)?, 2);
        let mut b = fresh_sized(673, 100)?;
        b.menu("File > Page Setup...")?;
        b.dialog_set("Orientation", "Landscape")?;
        b.dialog_click("OK")?;
        assert_eq!(preview_pages(&mut b)?, 1);
        Ok(())
    }

    #[test]
    fn print_job_matches_preview() -> T {
        let mut a = fresh_sized(2000, 100)?;
        a.menu("File > Print...")?;
        expect_dialog(&a, DialogKind::Print)?;
        a.dialog_click("OK")?;
        assert_eq!(a.platform_calls()?, [PlatformCall::Print { pages: 3 }]);
        assert_eq!(preview_pages(&mut a)?, 3);
        Ok(())
    }

    #[test]
    fn page_setup_defaults() -> T {
        let mut a = fresh()?;
        a.menu("File > Page Setup...")?;
        let d = expect_dialog(&a, DialogKind::PageSetup)?;
        assert_eq!(d.field("Paper"), Some("Letter"));
        assert_eq!(d.field("Orientation"), Some("Portrait"));
        assert_eq!(field_f64(&d, "Left")?, 0.75);
        assert_eq!(field_f64(&d, "Right")?, 0.75);
        assert_eq!(field_f64(&d, "Top")?, 1.0);
        assert_eq!(field_f64(&d, "Bottom")?, 1.0);
        a.dialog_click("Cancel")?;
        assert!(a.dialog()?.is_none());
        Ok(())
    }
}

mod file_send {
    use crate::common::*;

    #[test]
    fn send_records_attachment() -> T {
        let dir = tmp("send");
        let p = dir.join("a.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        a.menu("File > Send...")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(a.platform_calls()?, [PlatformCall::Mail { attachment: p }]);
        Ok(())
    }

    #[test]
    fn unsaved_no_aborts() -> T {
        let mut a = fresh()?;
        a.menu("File > Send...")?;
        expect_dialog(&a, DialogKind::MessageBox)?;
        a.dialog_click("No")?;
        assert!(a.dialog()?.is_none());
        assert!(a.platform_calls()?.is_empty());
        Ok(())
    }

    #[test] // layer=format
    fn dirty_yes_saves_then_sends() -> T {
        let dir = tmp("send2");
        let p = dir.join("m.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        pencil_click(&mut a, 1, 1)?;
        a.menu("File > Send...")?;
        a.dialog_click("Yes")?;
        assert_eq!(a.platform_calls()?, [PlatformCall::Mail { attachment: p.clone() }]);
        assert_eq!(decode_file(&p)?.image.get(1, 1), Some(K));
        Ok(())
    }
}

mod file_wallpaper {
    use crate::common::*;

    #[test]
    fn tiled_and_centered_record() -> T {
        let p = tmp("wall").join("a.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        a.menu("File > Set As Wallpaper (Tiled)")?;
        a.menu("File > Set As Wallpaper (Centered)")?;
        assert_eq!(
            a.platform_calls()?,
            [
                PlatformCall::Wallpaper { path: p.clone(), style: WallpaperStyle::Tiled },
                PlatformCall::Wallpaper { path: p, style: WallpaperStyle::Centered }
            ]
        );
        Ok(())
    }

    #[test]
    fn dirty_no_prompt_file_unchanged() -> T {
        let p = tmp("wall2").join("a.bmp");
        let mut a = fresh()?;
        save_as(&mut a, &p, T24)?;
        let before = read(&p)?;
        pencil_click(&mut a, 3, 3)?;
        a.menu("File > Set As Wallpaper (Tiled)")?;
        assert!(a.dialog()?.is_none());
        assert_eq!(read(&p)?, before);
        assert_eq!(a.platform_calls()?.len(), 1);
        Ok(())
    }

    #[test]
    fn disabled_without_service() -> T {
        let mut o = opts();
        o.services.wallpaper = false;
        let mut a = App::start(o)?;
        let p = tmp("wall3").join("a.bmp");
        save_as(&mut a, &p, T24)?;
        assert_eq!(a.document()?.path, Some(p));
        assert!(!enabled(&a, "File > Set As Wallpaper (Tiled)")?);
        assert!(!enabled(&a, "File > Set As Wallpaper (Centered)")?);
        Ok(())
    }

    #[test]
    fn opened_file_path_used() -> T {
        let dir = tmp("wall4");
        let p = bmp_file(&dir, "o.bmp", 2, 2, &solid(2, 2, GREEN))?;
        let mut a = start()?;
        open_path(&mut a, &p)?;
        a.menu("File > Set As Wallpaper (Centered)")?;
        assert_eq!(a.platform_calls()?, [PlatformCall::Wallpaper { path: p, style: WallpaperStyle::Centered }]);
        Ok(())
    }
}
