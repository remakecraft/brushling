//! tools.*, freehand and utility tools

mod tools_toolbox {
    use crate::common::*;

    pub const NAMES: [&str; 16] = [
        "Free-Form Select",
        "Select",
        "Eraser/Color Eraser",
        "Fill With Color",
        "Pick Color",
        "Magnifier",
        "Pencil",
        "Brush",
        "Airbrush",
        "Text",
        "Line",
        "Curve",
        "Rectangle",
        "Polygon",
        "Ellipse",
        "Rounded Rectangle",
    ];

    #[test]
    fn pencil_at_start() -> T {
        let a = start()?;
        assert_eq!(a.active_tool()?, Tool::Pencil);
        Ok(())
    }

    #[test]
    fn sixteen_names() -> T {
        let a = start()?;
        let b = a.tool_buttons()?;
        assert_eq!(b.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), NAMES);
        assert_eq!(b.iter().map(|t| t.tool).collect::<Vec<_>>(), Tool::ALL);
        Ok(())
    }

    #[test]
    fn option_kept_brush() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        opt(&mut a, ToolOption::BrushShape(6))?;
        a.select_tool(Tool::Line)?;
        a.select_tool(Tool::Brush)?;
        assert_eq!(a.tool_options()?.brush_shape, 6);
        Ok(())
    }

    #[test]
    fn tooltip_fill() -> T {
        let mut a = start()?;
        a.hover(Hover::ToolButton(Tool::Fill))?;
        a.advance(600)?;
        assert_eq!(a.tooltip()?.as_deref(), Some("Fill With Color"));
        Ok(())
    }

    #[test]
    fn tooltip_delay() -> T {
        let mut a = start()?;
        a.hover(Hover::ToolButton(Tool::Eraser))?;
        a.advance(300)?;
        assert_eq!(a.tooltip()?, None);
        a.advance(300)?;
        assert_eq!(a.tooltip()?.as_deref(), Some("Eraser/Color Eraser"));
        Ok(())
    }

    #[test]
    fn tool_change_commits_selection() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 2, 2)?;
        select_rect(&mut a, (1, 1), (3, 3))?;
        drag(&mut a, (2, 2), (12, 2))?;
        a.select_tool(Tool::Pencil)?;
        assert!(a.selection()?.is_none());
        assert_eq!(k_pts(&a)?, pts(&[(12, 2)]));
        Ok(())
    }
}

mod tools_mouse_colors {
    use crate::common::*;

    #[test]
    fn pencil_right_drag_background() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, RED)?;
        a.select_tool(Tool::Pencil)?;
        rdrag(&mut a, (2, 2), (6, 2))?;
        assert_eq!(pts_of(&a, RED)?, rect_pts(2, 2, 6, 2));
        Ok(())
    }

    #[test]
    fn ctrl_click_same_as_plain() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        click_with(&mut a, 3, 3, L, Mods::CTRL)?;
        assert_eq!(k_pts(&a)?, pts(&[(3, 3)]));
        Ok(())
    }

    #[test]
    fn other_button_cancels_stroke() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        press(&mut a, 1, 1)?;
        mv(&mut a, 5, 1)?;
        assert_eq!(a.render(Surface::Composite)?.get(3, 1), Some(K));
        press_with(&mut a, 5, 1, RB, NO)?;
        release_with(&mut a, 5, 1, RB, NO)?;
        release(&mut a, 5, 1)?;
        assert!(all_are(&a, W)?);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn line_right_drag_background() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, GREEN)?;
        a.select_tool(Tool::Line)?;
        rdrag(&mut a, (1, 1), (5, 1))?;
        assert_eq!(pts_of(&a, GREEN)?, rect_pts(1, 1, 5, 1));
        Ok(())
    }

    #[test]
    fn right_click_select_tool_nothing() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Select)?;
        rdrag(&mut a, (2, 2), (8, 8))?;
        assert!(a.selection()?.is_none());
        drag(&mut a, (2, 2), (8, 8))?;
        assert!(a.selection()?.is_some());
        Ok(())
    }
}

mod tool_eraser {
    use crate::common::*;

    fn black(size: Option<u32>) -> HResult<App> {
        let mut a = start()?;
        a.new_document(32, 32, K)?;
        a.select_tool(Tool::Eraser)?;
        if let Some(s) = size {
            opt(&mut a, ToolOption::EraserSize(s))?;
        }
        Ok(a)
    }

    #[test] // layer=pixel
    fn default_size_click() -> T {
        let mut a = black(None)?;
        assert_eq!(a.tool_options()?.eraser_size, 8);
        click(&mut a, 10, 10)?;
        assert_eq!(pts_of(&a, W)?, rect_pts(6, 6, 13, 13));
        assert_eq!(px(&a, 5, 10)?, K);
        assert_eq!(px(&a, 14, 10)?, K);
        Ok(())
    }

    #[test] // layer=pixel
    fn size4_click() -> T {
        let mut a = black(Some(4))?;
        click(&mut a, 10, 10)?;
        assert_eq!(pts_of(&a, W)?, rect_pts(8, 8, 11, 11));
        Ok(())
    }

    #[test] // layer=pixel
    fn colour_eraser_only_fg() -> T {
        let mut a = fresh()?;
        a.set_pixel(5, 5, K)?;
        a.set_pixel(6, 5, RED)?;
        a.select_tool(Tool::Eraser)?;
        opt(&mut a, ToolOption::EraserSize(10))?;
        rdrag(&mut a, (5, 5), (6, 5))?;
        assert_eq!(px(&a, 5, 5)?, W);
        assert_eq!(px(&a, 6, 5)?, RED);
        Ok(())
    }

    #[test] // layer=pixel
    fn drag_stamps_along_path() -> T {
        let mut a = black(Some(4))?;
        drag(&mut a, (5, 20), (15, 20))?;
        assert_eq!(pts_of(&a, W)?, rect_pts(3, 18, 16, 21));
        Ok(())
    }

    #[test]
    fn one_undo_step() -> T {
        let mut a = black(None)?;
        trace(&mut a, &[(5, 5), (20, 5), (20, 20)])?;
        assert!(count_of(&a, W)? > 0);
        undo(&mut a)?;
        assert!(all_are(&a, K)?);
        Ok(())
    }
}

mod tool_fill {
    use crate::common::*;

    #[test] // layer=pixel
    fn diagonal_barrier() -> T {
        let mut a = fresh_sized(10, 10)?;
        for i in 0..10 {
            a.set_pixel(i, 9 - i, K)?;
        }
        set_fg(&mut a, RED)?;
        a.select_tool(Tool::Fill)?;
        click(&mut a, 0, 0)?;
        let img = a.pixels()?;
        for y in 0..10 {
            for x in 0..10 {
                let want = if x + y < 9 { RED } else if x + y == 9 { K } else { W };
                assert_eq!(img.get(x, y), Some(want), "({x},{y})");
            }
        }
        Ok(())
    }

    #[test] // layer=pixel
    fn tolerance_zero() -> T {
        let mut a = fresh_sized(10, 10)?;
        a.set_pixel(5, 5, rgb(254, 255, 255))?;
        set_fg(&mut a, RED)?;
        a.select_tool(Tool::Fill)?;
        click(&mut a, 0, 0)?;
        assert_eq!(px(&a, 5, 5)?, rgb(254, 255, 255));
        assert_eq!(count_of(&a, RED)?, 99);
        Ok(())
    }

    #[test]
    fn right_click_background() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, BLUE)?;
        a.select_tool(Tool::Fill)?;
        rclick(&mut a, 3, 3)?;
        assert!(all_are(&a, BLUE)?);
        Ok(())
    }

    #[test]
    fn same_colour_no_undo() -> T {
        let mut a = fresh()?;
        set_fg(&mut a, W)?;
        a.select_tool(Tool::Fill)?;
        click(&mut a, 3, 3)?;
        assert!(!undo_enabled(&a)?);
        set_fg(&mut a, RED)?;
        click(&mut a, 3, 3)?;
        assert!(undo_enabled(&a)?);
        assert!(all_are(&a, RED)?);
        Ok(())
    }

    #[test] // layer=pixel
    fn four_connected_only() -> T {
        let mut a = fresh_sized(6, 6)?;
        a.set_pixel(1, 0, K)?;
        a.set_pixel(0, 1, K)?;
        set_fg(&mut a, BLUE)?;
        a.select_tool(Tool::Fill)?;
        click(&mut a, 0, 0)?;
        assert_eq!(pts_of(&a, BLUE)?, pts(&[(0, 0)]));
        Ok(())
    }
}

mod tool_pick_color {
    use crate::common::*;

    #[test]
    fn pick_foreground_returns_to_brush() -> T {
        let mut a = fresh()?;
        a.set_pixel(4, 4, rgb(12, 34, 56))?;
        a.select_tool(Tool::Brush)?;
        a.select_tool(Tool::PickColor)?;
        click(&mut a, 4, 4)?;
        assert_eq!(a.colors()?, (rgb(12, 34, 56), W));
        assert_eq!(a.active_tool()?, Tool::Brush);
        assert!(!undo_enabled(&a)?);
        assert_eq!(count_of(&a, W)?, 32 * 32 - 1);
        Ok(())
    }

    #[test]
    fn pick_background_right() -> T {
        let mut a = fresh()?;
        a.set_pixel(4, 4, rgb(12, 34, 56))?;
        a.select_tool(Tool::Brush)?;
        a.select_tool(Tool::PickColor)?;
        rclick(&mut a, 4, 4)?;
        assert_eq!(a.colors()?, (K, rgb(12, 34, 56)));
        Ok(())
    }

    #[test]
    fn pick_at_start_returns_pencil() -> T {
        let mut a = fresh()?;
        a.set_pixel(1, 1, GREEN)?;
        a.select_tool(Tool::PickColor)?;
        click(&mut a, 1, 1)?;
        assert_eq!(a.colors()?.0, GREEN);
        assert_eq!(a.active_tool()?, Tool::Pencil);
        Ok(())
    }

    #[test]
    fn outside_click_keeps_tool() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        a.select_tool(Tool::PickColor)?;
        click(&mut a, 40, 40)?;
        assert_eq!(a.active_tool()?, Tool::PickColor);
        assert_eq!(a.colors()?, (K, W));
        a.set_pixel(2, 2, BLUE)?;
        click(&mut a, 2, 2)?;
        assert_eq!(a.colors()?.0, BLUE);
        Ok(())
    }
}

mod tool_magnifier {
    use crate::common::*;

    #[test]
    fn click_zooms_and_reverts() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        a.select_tool(Tool::Magnifier)?;
        click(&mut a, 10, 10)?;
        assert_eq!(a.zoom()?, 400);
        assert_eq!(a.active_tool()?, Tool::Pencil);
        Ok(())
    }

    #[test]
    fn second_click_returns_100() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Magnifier)?;
        click(&mut a, 10, 10)?;
        assert_eq!(a.zoom()?, 400);
        a.select_tool(Tool::Magnifier)?;
        click(&mut a, 10, 10)?;
        assert_eq!(a.zoom()?, 100);
        Ok(())
    }

    #[test]
    fn option_6x() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Magnifier)?;
        opt(&mut a, ToolOption::MagnifierLevel(6))?;
        assert_eq!(a.zoom()?, 600);
        assert_eq!(a.tool_options()?.magnifier_highlight, Some(6));
        Ok(())
    }

    #[test]
    fn no_pixels_no_undo() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Magnifier)?;
        click(&mut a, 10, 10)?;
        a.select_tool(Tool::Magnifier)?;
        click(&mut a, 10, 10)?;
        a.select_tool(Tool::Magnifier)?;
        opt(&mut a, ToolOption::MagnifierLevel(6))?;
        assert_eq!(a.zoom()?, 600);
        assert!(all_are(&a, W)?);
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn right_click_same() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Magnifier)?;
        rclick(&mut a, 5, 5)?;
        assert_eq!(a.zoom()?, 400);
        Ok(())
    }
}

mod tool_pencil {
    use crate::common::*;

    #[test] // layer=pixel
    fn click_one_pixel() -> T {
        let mut a = fresh()?;
        pencil_click(&mut a, 7, 7)?;
        assert_eq!(k_pts(&a)?, pts(&[(7, 7)]));
        Ok(())
    }

    #[test] // layer=pixel
    fn drag_line_rule() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        drag(&mut a, (0, 0), (6, 2))?;
        assert_eq!(k_pts(&a)?, pts(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 2), (6, 2)]));
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_snaps_horizontal() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        drag_with(&mut a, (10, 10), (30, 13), L, Mods::SHIFT)?;
        assert_eq!(k_pts(&a)?, rect_pts(10, 10, 30, 10));
        Ok(())
    }

    #[test] // layer=pixel
    fn multi_segment_stroke() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        trace(&mut a, &[(2, 2), (5, 2), (5, 6)])?;
        assert_eq!(k_pts(&a)?, rect_pts(2, 2, 5, 2).union(&rect_pts(5, 2, 5, 6)).copied().collect());
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test] // layer=pixel
    fn leaves_and_returns() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Pencil)?;
        trace(&mut a, &[(28, 5), (40, 5), (40, 9), (28, 9)])?;
        assert_eq!(k_pts(&a)?, rect_pts(28, 5, 31, 5).union(&rect_pts(28, 9, 31, 9)).copied().collect());
        Ok(())
    }
}

mod tool_brush {
    use crate::common::*;

    fn brush_click(shape: Option<u32>, x: i32, y: i32) -> HResult<App> {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        if let Some(s) = shape {
            opt(&mut a, ToolOption::BrushShape(s))?;
        }
        click(&mut a, x, y)?;
        Ok(a)
    }

    #[test] // layer=pixel
    fn round7_click() -> T {
        let a = brush_click(Some(1), 10, 10)?;
        let k = k_pts(&a)?;
        assert_eq!(k.len(), 37);
        assert_eq!(k, stamp(&[(10, 10)], &brush_offsets(1)));
        assert!(k.iter().all(|&(x, y)| (7..=13).contains(&x) && (7..=13).contains(&y)));
        assert_eq!(px(&a, 12, 12)?, K);
        assert_eq!(px(&a, 13, 12)?, W);
        Ok(())
    }

    #[test] // layer=pixel
    fn round5_default() -> T {
        let a = brush_click(None, 10, 10)?;
        assert_eq!(a.tool_options()?.brush_shape, 2);
        let k = k_pts(&a)?;
        assert_eq!(k.len(), 21);
        assert_eq!(k, stamp(&[(10, 10)], &brush_offsets(2)));
        assert_eq!(px(&a, 12, 11)?, K);
        assert_eq!(px(&a, 12, 12)?, W);
        Ok(())
    }

    #[test] // layer=pixel
    fn square8_click() -> T {
        let a = brush_click(Some(4), 10, 10)?;
        assert_eq!(k_pts(&a)?, rect_pts(6, 6, 13, 13));
        Ok(())
    }

    #[test] // layer=pixel
    fn slant_ascending8() -> T {
        let a = brush_click(Some(7), 10, 10)?;
        assert_eq!(
            k_pts(&a)?,
            pts(&[(6, 14), (7, 13), (8, 12), (9, 11), (10, 10), (11, 9), (12, 8), (13, 7)])
        );
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_locks_horizontal() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        opt(&mut a, ToolOption::BrushShape(3))?;
        drag_with(&mut a, (5, 5), (20, 8), L, Mods::SHIFT)?;
        assert_eq!(k_pts(&a)?, rect_pts(5, 5, 20, 5));
        Ok(())
    }

    #[test] // layer=pixel
    fn drag_stamps_square2() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        opt(&mut a, ToolOption::BrushShape(6))?;
        drag(&mut a, (5, 5), (10, 5))?;
        assert_eq!(k_pts(&a)?, rect_pts(4, 4, 10, 5));
        Ok(())
    }

    #[test] // layer=pixel
    fn slant_parallel_drag() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        opt(&mut a, ToolOption::BrushShape(9))?;
        drag(&mut a, (10, 10), (14, 6))?;
        let k = k_pts(&a)?;
        assert_eq!(k, stamp(&line_path(10, 10, 14, 6), &brush_offsets(9)));
        assert!(k.iter().all(|&(x, y)| x + y == 20));
        assert_eq!(k.len(), 6);
        Ok(())
    }
}

mod tool_airbrush {
    use crate::common::*;

    fn spray(seed: u64, radius: Option<u32>, at: (i32, i32), hold_ms: u64) -> HResult<Pts> {
        let mut a = fresh()?;
        a.seed(seed)?;
        a.select_tool(Tool::Airbrush)?;
        if let Some(r) = radius {
            opt(&mut a, ToolOption::AirbrushRadius(r))?;
        }
        press(&mut a, at.0, at.1)?;
        if hold_ms > 0 {
            a.advance(hold_ms)?;
        }
        release(&mut a, at.0, at.1)?;
        k_pts(&a)
    }

    #[test]
    fn single_tick() -> T {
        let a = fresh()?;
        assert_eq!(a.tool_options()?.airbrush_radius, 4);
        let d = disc(15, 15, 4);
        for seed in 0..20 {
            let k = spray(seed, None, (15, 15), 0)?;
            assert!((1..=8).contains(&k.len()), "seed {seed}: {} pixels", k.len());
            assert!(k.is_subset(&d), "seed {seed}");
        }
        Ok(())
    }

    #[test]
    fn one_second_hold() -> T {
        let d = disc(15, 15, 4);
        for seed in 0..10 {
            let k = spray(seed, None, (15, 15), 1000)?;
            assert!(!k.is_empty() && k.len() <= 168, "seed {seed}");
            assert!(k.is_subset(&d), "seed {seed}");
        }
        Ok(())
    }

    #[test]
    fn ten_seconds_fills_disc() -> T {
        let d = disc(15, 15, 4);
        assert_eq!(d.len(), 49);
        for seed in 0..100 {
            assert_eq!(spray(seed, None, (15, 15), 10_000)?, d, "seed {seed}");
        }
        Ok(())
    }

    #[test]
    fn radius12_bounds() -> T {
        let d = disc(15, 15, 12);
        assert_eq!(d.len(), 441);
        for seed in 0..10 {
            let k = spray(seed, Some(12), (15, 15), 500)?;
            assert!(!k.is_empty() && k.len() <= 11 * 24, "seed {seed}");
            assert!(k.is_subset(&d), "seed {seed}");
        }
        Ok(())
    }

    #[test]
    fn moves_do_not_tick() -> T {
        let d = disc(15, 15, 4);
        for seed in 0..10 {
            let mut a = fresh()?;
            a.seed(seed)?;
            a.select_tool(Tool::Airbrush)?;
            press(&mut a, 15, 15)?;
            for i in 0..10 {
                mv(&mut a, 15 + (i % 2), 15)?;
            }
            mv(&mut a, 15, 15)?;
            release(&mut a, 15, 15)?;
            let k = k_pts(&a)?;
            assert!((1..=8).contains(&k.len()), "seed {seed}: {}", k.len());
            assert!(k.is_subset(&d));
        }
        Ok(())
    }
}

mod tool_text {
    use crate::common::*;

    /// 128x64 canvas, text frame over (10,10)-(109,49) with "A" typed.
    fn frame_with_a(bg: Rgb, opaque: bool) -> HResult<App> {
        let mut a = fresh_sized(128, 64)?;
        set_bg(&mut a, bg)?;
        a.select_tool(Tool::Text)?;
        opt(&mut a, ToolOption::Opaque(opaque))?;
        drag(&mut a, (10, 10), (109, 49))?;
        type_text(&mut a, "A")?;
        Ok(a)
    }

    fn check_frame(a: &App, inside: &[Rgb], ink: Rgb) -> T {
        let img = a.pixels()?;
        let frame = rect_pts(10, 10, 109, 49);
        let mut inked = 0;
        for y in 0..64 {
            for x in 0..128 {
                let c = img.get(x, y).unwrap_or(K);
                if frame.contains(&(x, y)) {
                    assert!(inside.contains(&c), "({x},{y}) = {c:?}");
                    inked += (c == ink) as usize;
                } else {
                    assert_eq!(c, W, "outside ({x},{y})");
                }
            }
        }
        assert!(inked > 0, "no text pixels");
        Ok(())
    }

    #[test] // layer=pixel
    fn opaque_frame() -> T {
        let mut a = frame_with_a(GREEN, true)?;
        click(&mut a, 0, 0)?;
        check_frame(&a, &[K, GREEN], K)
    }

    #[test] // layer=pixel
    fn transparent_frame() -> T {
        let mut a = frame_with_a(GREEN, false)?;
        click(&mut a, 0, 0)?;
        check_frame(&a, &[K, W], K)
    }

    #[test] // layer=pixel
    fn recolour_on_fg_change() -> T {
        let mut a = frame_with_a(W, true)?;
        set_fg(&mut a, RED)?;
        click(&mut a, 0, 0)?;
        assert!(k_pts(&a)?.is_empty());
        assert!(count_of(&a, RED)? > 0);
        Ok(())
    }

    #[test]
    fn disabled_when_zoomed() -> T {
        let mut a = fresh()?;
        let text_enabled = |a: &App| -> HResult<bool> {
            Ok(a.tool_buttons()?.into_iter().find(|b| b.tool == Tool::Text).map(|b| b.enabled).unwrap_or(false))
        };
        assert!(text_enabled(&a)?);
        a.menu("View > Zoom > Large Size")?;
        assert!(!text_enabled(&a)?);
        Ok(())
    }

    #[test]
    fn paste_undo_restores() -> T {
        let mut a = fresh_sized(128, 64)?;
        a.select_tool(Tool::Text)?;
        drag(&mut a, (10, 10), (109, 49))?;
        type_text(&mut a, "abc")?;
        key(&mut a, Key::Left)?;
        key(&mut a, Key::Left)?;
        a.key(Key::Right, Mods::SHIFT)?;
        a.set_clipboard(ClipboardContent::Text("XY".into()))?;
        ctrl(&mut a, 'v')?;
        assert_eq!(a.text_frame()?.map(|f| f.text), Some("aXYc".to_string()));
        ctrl(&mut a, 'z')?;
        assert_eq!(a.text_frame()?.map(|f| f.text), Some("abc".to_string()));
        Ok(())
    }

    #[test]
    fn commit_undo_single_step() -> T {
        let mut a = fresh_sized(128, 64)?;
        a.select_tool(Tool::Text)?;
        drag(&mut a, (10, 10), (109, 49))?;
        type_text(&mut a, "Hi")?;
        click(&mut a, 0, 0)?;
        assert!(k_pts(&a)?.len() > 0);
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }
}

mod tool_line_width {
    use crate::common::*;

    #[test]
    fn default_width() -> T {
        let mut a = fresh()?;
        assert_eq!(a.tool_options()?.line_width, 1);
        a.select_tool(Tool::Line)?;
        drag(&mut a, (2, 2), (9, 2))?;
        assert_eq!(k_pts(&a)?, rect_pts(2, 2, 9, 2));
        Ok(())
    }

    #[test]
    fn keys_step_and_clamp() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        for _ in 0..2 {
            a.key(Key::NumPlus, Mods::CTRL)?;
        }
        assert_eq!(a.tool_options()?.line_width, 3);
        for _ in 0..5 {
            a.key(Key::NumMinus, Mods::CTRL)?;
        }
        assert_eq!(a.tool_options()?.line_width, 1);
        Ok(())
    }

    #[test]
    fn other_tools_ignore_keys() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Brush)?;
        a.key(Key::NumPlus, Mods::CTRL)?;
        assert_eq!(a.tool_options()?.line_width, 1);
        a.select_tool(Tool::Line)?;
        a.key(Key::NumPlus, Mods::CTRL)?;
        assert_eq!(a.tool_options()?.line_width, 2);
        Ok(())
    }

    #[test] // layer=pixel
    fn key_during_drag() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        press(&mut a, 5, 5)?;
        mv(&mut a, 15, 5)?;
        a.key(Key::NumPlus, Mods::CTRL)?;
        release(&mut a, 15, 5)?;
        assert_eq!(k_pts(&a)?, rect_pts(4, 4, 15, 5));
        Ok(())
    }
}
