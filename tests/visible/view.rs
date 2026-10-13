//! view.*

mod view_panels {
    use crate::common::*;

    #[test]
    fn first_start_checked() -> T {
        let a = start()?;
        assert!(checked(&a, "View > Tool Box")?);
        assert!(checked(&a, "View > Color Box")?);
        assert!(checked(&a, "View > Status Bar")?);
        Ok(())
    }

    #[test]
    fn first_start_panels_visible() -> T {
        let a = start()?;
        assert!(a.panel(Panel::ToolBox)?.visible);
        assert!(a.panel(Panel::ColorBox)?.visible);
        assert!(a.status_bar()?.visible);
        Ok(())
    }

    #[test]
    fn ctrl_t_toggles_toolbox() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        ctrl(&mut a, 't')?;
        assert!(!checked(&a, "View > Tool Box")?);
        assert!(!a.panel(Panel::ToolBox)?.visible);
        assert_eq!(a.active_tool()?, Tool::Brush);
        ctrl(&mut a, 't')?;
        assert!(checked(&a, "View > Tool Box")?);
        assert!(a.panel(Panel::ToolBox)?.visible);
        Ok(())
    }

    #[test]
    fn status_bar_menu_twice() -> T {
        let mut a = fresh()?;
        a.menu("View > Status Bar")?;
        assert!(!a.status_bar()?.visible);
        a.menu("View > Status Bar")?;
        assert!(a.status_bar()?.visible);
        Ok(())
    }

    #[test]
    fn toggle_while_floating_keeps_position() -> T {
        let mut a = fresh()?;
        a.place_panel(Panel::ToolBox, Some((400, 300)))?;
        ctrl(&mut a, 't')?;
        assert!(!a.panel(Panel::ToolBox)?.visible);
        ctrl(&mut a, 't')?;
        assert_eq!(a.panel(Panel::ToolBox)?, PanelState { visible: true, floating_at: Some((400, 300)) });
        Ok(())
    }
}

mod view_zoom {
    use crate::common::*;

    #[test] // layer=pixel
    fn large_size_render() -> T {
        let mut a = fresh()?;
        a.set_window_size(800, 600)?;
        a.set_pixel(0, 0, GREEN)?;
        a.set_pixel(1, 0, RED)?;
        a.menu("View > Zoom > Large Size")?;
        assert_eq!(a.zoom()?, 400);
        let img = a.render(Surface::Workspace)?;
        for x in 8..=11 {
            for y in 4..=7 {
                assert_eq!(img.get(x, y), Some(RED), "({x},{y})");
            }
        }
        assert_eq!(img.get(7, 4), Some(GREEN));
        assert_eq!(img.get(12, 4), Some(W));
        Ok(())
    }

    #[test]
    fn custom_600_ok_cancel() -> T {
        let mut a = fresh()?;
        custom_zoom(&mut a, "600%", "OK")?;
        assert_eq!(a.zoom()?, 600);
        custom_zoom(&mut a, "200%", "Cancel")?;
        assert_eq!(a.zoom()?, 600);
        Ok(())
    }

    #[test]
    fn screen_to_image_400() -> T {
        let mut a = fresh()?;
        a.menu("View > Zoom > Large Size")?;
        a.select_tool(Tool::Pencil)?;
        a.pointer(PointerKind::Press(L), Pos::Canvas(9, 13), NO)?;
        a.pointer(PointerKind::Release(L), Pos::Canvas(9, 13), NO)?;
        assert_eq!(k_pts(&a)?, pts(&[(2, 3)]));
        Ok(())
    }

    #[test]
    fn zoom_not_dirty() -> T {
        let mut a = fresh()?;
        a.menu("View > Zoom > Large Size")?;
        custom_zoom(&mut a, "800%", "OK")?;
        assert_eq!(a.zoom()?, 800);
        a.menu("View > Zoom > Normal Size")?;
        assert_eq!(a.zoom()?, 100);
        assert!(!a.document()?.dirty);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn normal_size_unchecks_grid() -> T {
        let mut a = fresh()?;
        a.menu("View > Zoom > Large Size")?;
        a.menu("View > Zoom > Show Grid")?;
        assert!(checked(&a, "View > Zoom > Show Grid")?);
        a.menu("View > Zoom > Normal Size")?;
        assert_eq!(a.zoom()?, 100);
        assert!(!checked(&a, "View > Zoom > Show Grid")?);
        Ok(())
    }
}

mod view_grid {
    use crate::common::*;

    #[test] // layer=pixel
    fn grid_drawn_at_400() -> T {
        let mut a = fresh()?;
        a.set_window_size(800, 600)?;
        a.menu("View > Zoom > Large Size")?;
        a.menu("View > Zoom > Show Grid")?;
        let img = a.render(Surface::Workspace)?;
        assert_eq!(img.get(16, 16), Some(GREY));
        assert_eq!(img.get(18, 16), Some(GREY));
        assert_eq!(img.get(16, 18), Some(GREY));
        assert_eq!(img.get(17, 17), Some(W));
        Ok(())
    }

    #[test] // layer=pixel
    fn grid_off_by_default() -> T {
        let mut a = fresh()?;
        a.set_window_size(800, 600)?;
        a.menu("View > Zoom > Large Size")?;
        let img = a.render(Surface::Workspace)?;
        assert_eq!(img.get(16, 16), Some(W));
        assert_eq!(img.get(17, 17), Some(W));
        Ok(())
    }

    #[test] // layer=pixel
    fn no_grid_at_200() -> T {
        let mut a = fresh()?;
        a.set_window_size(800, 600)?;
        custom_zoom(&mut a, "200%", "OK")?;
        a.menu("View > Zoom > Show Grid")?;
        assert!(checked(&a, "View > Zoom > Show Grid")?);
        let img = a.render(Surface::Workspace)?;
        assert_eq!(img.get(4, 4), Some(W));
        assert!(rect_pts(4, 4, 67, 67).iter().all(|&(x, y)| img.get(x, y) == Some(W)));
        Ok(())
    }

    #[test] // layer=format
    fn grid_not_saved() -> T {
        let mut a = fresh()?;
        a.menu("View > Zoom > Large Size")?;
        a.menu("View > Zoom > Show Grid")?;
        let p = tmp("grid").join("g.bmp");
        save_as(&mut a, &p, T24)?;
        let b = decode_file(&p)?;
        assert_eq!((b.width, b.height), (32, 32));
        assert!(b.image.pixels.iter().all(|&c| c == W));
        Ok(())
    }
}

mod view_thumbnail {
    use crate::common::*;

    fn shown(a: &mut App) -> T {
        a.menu("View > Zoom > Large Size")?;
        a.menu("View > Zoom > Show Thumbnail")
    }

    #[test] // layer=pixel
    fn window_and_pixel() -> T {
        let mut a = fresh()?;
        a.set_pixel(5, 5, RED)?;
        shown(&mut a)?;
        assert!(a.panel(Panel::Thumbnail)?.visible);
        let t = a.render(Surface::Thumbnail)?;
        assert_eq!((t.width, t.height), (200, 150));
        assert_eq!(t.get(5, 5), Some(RED));
        assert_eq!(t.get(4, 5), Some(W));
        Ok(())
    }

    #[test] // layer=pixel
    fn live_update() -> T {
        let mut a = fresh()?;
        shown(&mut a)?;
        pencil_click(&mut a, 5, 5)?;
        assert_eq!(a.render(Surface::Thumbnail)?.get(5, 5), Some(K));
        Ok(())
    }

    #[test]
    fn normal_size_hides() -> T {
        let mut a = fresh()?;
        shown(&mut a)?;
        assert!(checked(&a, "View > Zoom > Show Thumbnail")?);
        a.menu("View > Zoom > Normal Size")?;
        assert!(!a.panel(Panel::Thumbnail)?.visible);
        assert!(!checked(&a, "View > Zoom > Show Thumbnail")?);
        Ok(())
    }

    #[test] // layer=pixel
    fn beyond_image_grey() -> T {
        let mut a = fresh()?;
        shown(&mut a)?;
        let t = a.render(Surface::Thumbnail)?;
        assert_eq!(t.get(31, 31), Some(W));
        assert_eq!(t.get(100, 100), Some(GREY));
        assert_eq!(t.get(32, 0), Some(GREY));
        Ok(())
    }

    #[test]
    fn close_button_unchecks() -> T {
        let mut a = fresh()?;
        shown(&mut a)?;
        assert!(a.panel(Panel::Thumbnail)?.visible);
        a.close_panel(Panel::Thumbnail)?;
        assert!(!a.panel(Panel::Thumbnail)?.visible);
        assert!(!checked(&a, "View > Zoom > Show Thumbnail")?);
        Ok(())
    }
}

mod view_view_bitmap {
    use crate::common::*;

    fn app_on_screen(w: u32, h: u32) -> HResult<App> {
        let mut o = opts();
        o.screen = (w, h);
        App::start(o)
    }

    #[test] // layer=pixel
    fn centred_small_image() -> T {
        let mut a = app_on_screen(1024, 768)?;
        a.new_document(100, 100, RED)?;
        a.menu("View > View Bitmap")?;
        assert_eq!(a.view_mode()?, ViewMode::ViewBitmap);
        let f = a.render(Surface::FullScreen)?;
        assert_eq!((f.width, f.height), (1024, 768));
        assert_eq!(f.get(462, 334), Some(RED));
        assert_eq!(f.get(561, 433), Some(RED));
        assert_eq!(f.get(0, 0), Some(K));
        assert_eq!(f.get(461, 334), Some(K));
        assert_eq!(f.get(562, 433), Some(K));
        Ok(())
    }

    #[test]
    fn key_returns() -> T {
        let mut a = app_on_screen(1024, 768)?;
        a.new_document(100, 100, RED)?;
        ctrl(&mut a, 'f')?;
        assert_eq!(a.view_mode()?, ViewMode::ViewBitmap);
        key(&mut a, Key::Char('a'))?;
        assert_eq!(a.view_mode()?, ViewMode::Normal);
        assert!(all_are(&a, RED)?);
        assert!(!a.document()?.dirty);
        Ok(())
    }

    #[test] // layer=pixel
    fn large_image_top_left() -> T {
        let mut a = app_on_screen(1024, 768)?;
        a.new_document(2000, 1000, W)?;
        a.set_pixel(0, 0, RED)?;
        a.set_pixel(1023, 767, GREEN)?;
        a.menu("View > View Bitmap")?;
        let f = a.render(Surface::FullScreen)?;
        assert_eq!(f.get(0, 0), Some(RED));
        assert_eq!(f.get(1023, 767), Some(GREEN));
        Ok(())
    }

    #[test]
    fn esc_returns() -> T {
        let mut a = app_on_screen(1024, 768)?;
        a.menu("View > View Bitmap")?;
        assert_eq!(a.view_mode()?, ViewMode::ViewBitmap);
        esc(&mut a)?;
        assert_eq!(a.view_mode()?, ViewMode::Normal);
        Ok(())
    }
}
