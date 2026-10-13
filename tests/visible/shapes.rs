//! tool.line, tool.curve, tool.shapes, tool.polygon

/// Golden generator for tool.curve: cubic Bezier sampled at t = i/64,
/// rounded half-up, joined by the 1-pixel path rule.
pub(crate) fn curve_pts(p0: (i32, i32), c1: (i32, i32), c2: (i32, i32), p3: (i32, i32)) -> crate::common::Pts {
    let f = |a: i32, b: i32, c: i32, d: i32, t: f64| -> i32 {
        let u = 1.0 - t;
        let v = u * u * u * a as f64 + 3.0 * u * u * t * b as f64 + 3.0 * u * t * t * c as f64 + t * t * t * d as f64;
        (v + 0.5).floor() as i32
    };
    let samples: Vec<(i32, i32)> = (0..=64)
        .map(|i| {
            let t = i as f64 / 64.0;
            (f(p0.0, c1.0, c2.0, p3.0, t), f(p0.1, c1.1, c2.1, p3.1, t))
        })
        .collect();
    let mut out = crate::common::Pts::new();
    out.insert(samples[0]);
    for w in samples.windows(2) {
        for p in crate::common::line_path(w[0].0, w[0].1, w[1].0, w[1].1) {
            out.insert(p);
        }
    }
    out
}

mod tool_line {
    use crate::common::*;

    #[test] // layer=pixel
    fn drag_line_rule() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        drag(&mut a, (0, 0), (6, 2))?;
        assert_eq!(k_pts(&a)?, pts(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 2), (6, 2)]));
        Ok(())
    }

    #[test] // layer=pixel
    fn width3_horizontal() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        opt(&mut a, ToolOption::LineWidth(3))?;
        drag(&mut a, (5, 10), (15, 10))?;
        let k = k_pts(&a)?;
        assert_eq!(k.len(), 39);
        assert_eq!(k, rect_pts(4, 9, 16, 11));
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_snaps_diagonal() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        drag_with(&mut a, (10, 10), (30, 25), L, Mods::SHIFT)?;
        let want: Pts = (0..=15).map(|k| (10 + k, 10 + k)).collect();
        assert_eq!(k_pts(&a)?, want);
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_snaps_vertical() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        drag_with(&mut a, (10, 10), (12, 30), L, Mods::SHIFT)?;
        assert_eq!(k_pts(&a)?, rect_pts(10, 10, 10, 30));
        Ok(())
    }

    #[test]
    fn right_drag_background() -> T {
        let mut a = fresh()?;
        set_bg(&mut a, RED)?;
        a.select_tool(Tool::Line)?;
        rdrag(&mut a, (1, 1), (1, 5))?;
        assert_eq!(pts_of(&a, RED)?, rect_pts(1, 1, 1, 5));
        assert!(k_pts(&a)?.is_empty());
        Ok(())
    }

    #[test]
    fn preview_then_commit_one_step() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        press(&mut a, 2, 2)?;
        mv(&mut a, 8, 2)?;
        assert_eq!(where_eq(&a.render(Surface::Composite)?, K), rect_pts(2, 2, 8, 2));
        assert!(all_are(&a, W)?);
        release(&mut a, 8, 2)?;
        assert_eq!(k_pts(&a)?, rect_pts(2, 2, 8, 2));
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }

    #[test] // layer=pixel
    fn same_pixel_square() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Line)?;
        opt(&mut a, ToolOption::LineWidth(3))?;
        click(&mut a, 10, 10)?;
        assert_eq!(k_pts(&a)?, rect_pts(9, 9, 11, 11));
        Ok(())
    }
}

mod tool_curve {
    use crate::common::*;

    #[test] // layer=pixel
    fn one_bend_upward() -> T {
        let mut a = fresh_sized(64, 32)?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (10, 20), (50, 20))?;
        click(&mut a, 30, 0)?;
        click(&mut a, 30, 0)?;
        let k = k_pts(&a)?;
        assert!(k.contains(&(10, 20)) && k.contains(&(50, 20)) && k.contains(&(30, 5)));
        assert!(k.iter().all(|&(_, y)| y >= 5));
        Ok(())
    }

    #[test] // layer=pixel
    fn straight_when_bend_on_line() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (10, 20), (30, 20))?;
        click(&mut a, 20, 20)?;
        click(&mut a, 20, 20)?;
        assert_eq!(k_pts(&a)?, rect_pts(10, 20, 30, 20));
        Ok(())
    }

    #[test]
    fn esc_discards() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (10, 20), (30, 20))?;
        assert!(!where_eq(&a.render(Surface::Composite)?, K).is_empty());
        esc(&mut a)?;
        assert!(all_are(&a, W)?);
        assert!(where_eq(&a.render(Surface::Composite)?, K).is_empty());
        assert!(!undo_enabled(&a)?);
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_step1_preview() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Curve)?;
        drag_with(&mut a, (5, 5), (25, 9), L, Mods::SHIFT)?;
        assert_eq!(where_eq(&a.render(Surface::Composite)?, K), rect_pts(5, 5, 25, 5));
        Ok(())
    }

    #[test] // layer=pixel
    fn two_bends_golden() -> T {
        let mut a = fresh_sized(64, 64)?;
        a.select_tool(Tool::Curve)?;
        drag(&mut a, (4, 40), (60, 40))?;
        drag(&mut a, (20, 4), (20, 4))?;
        drag(&mut a, (44, 60), (44, 60))?;
        let want = crate::shapes::curve_pts((4, 40), (20, 4), (44, 60), (60, 40));
        assert_eq!(k_pts(&a)?, want);
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }
}

mod tool_shapes {
    use crate::common::*;

    fn shape(t: Tool, style: Option<u32>, width: Option<u32>) -> HResult<App> {
        let mut a = fresh()?;
        a.select_tool(t)?;
        if let Some(s) = style {
            opt(&mut a, ToolOption::FillStyle(s))?;
        }
        if let Some(w) = width {
            opt(&mut a, ToolOption::LineWidth(w))?;
        }
        Ok(a)
    }

    #[test] // layer=pixel
    fn rect_outline() -> T {
        let mut a = shape(Tool::Rectangle, None, None)?;
        drag(&mut a, (2, 2), (7, 5))?;
        let k = k_pts(&a)?;
        assert_eq!(k.len(), 16);
        assert_eq!(k, border_pts(2, 2, 7, 5, 1));
        assert_eq!(px(&a, 4, 3)?, W);
        Ok(())
    }

    #[test] // layer=pixel
    fn rect_width2() -> T {
        let mut a = shape(Tool::Rectangle, None, Some(2))?;
        drag(&mut a, (0, 0), (9, 9))?;
        assert_eq!(k_pts(&a)?.len(), 64);
        assert_eq!(k_pts(&a)?, border_pts(0, 0, 9, 9, 2));
        Ok(())
    }

    #[test] // layer=pixel
    fn rect_style2_fill_background() -> T {
        let mut a = shape(Tool::Rectangle, Some(2), None)?;
        set_bg(&mut a, RED)?;
        drag(&mut a, (0, 0), (9, 9))?;
        assert_eq!(k_pts(&a)?, border_pts(0, 0, 9, 9, 1));
        assert_eq!(pts_of(&a, RED)?, rect_pts(1, 1, 8, 8));
        Ok(())
    }

    #[test] // layer=pixel
    fn rect_style3_solid() -> T {
        let mut a = shape(Tool::Rectangle, Some(3), None)?;
        drag(&mut a, (0, 0), (9, 9))?;
        assert_eq!(k_pts(&a)?, rect_pts(0, 0, 9, 9));
        Ok(())
    }

    #[test] // layer=pixel
    fn rect_style2_right_drag() -> T {
        let mut a = shape(Tool::Rectangle, Some(2), None)?;
        set_bg(&mut a, RED)?;
        rdrag(&mut a, (0, 0), (9, 9))?;
        assert_eq!(pts_of(&a, RED)?, border_pts(0, 0, 9, 9, 1));
        assert_eq!(k_pts(&a)?, rect_pts(1, 1, 8, 8));
        Ok(())
    }

    #[test] // layer=pixel
    fn ellipse_symmetric() -> T {
        let mut a = shape(Tool::Ellipse, None, None)?;
        drag(&mut a, (0, 0), (20, 10))?;
        for p in [(10, 0), (10, 10), (0, 5), (20, 5)] {
            assert_eq!(px(&a, p.0, p.1)?, K, "{p:?}");
        }
        for p in [(10, 5), (0, 0), (20, 0), (0, 10), (20, 10)] {
            assert_eq!(px(&a, p.0, p.1)?, W, "{p:?}");
        }
        let k = k_pts(&a)?;
        assert!(k.iter().all(|&(x, y)| k.contains(&(20 - x, y)) && k.contains(&(x, 10 - y))));
        assert!(k.iter().all(|&(x, y)| x <= 20 && y <= 10));
        Ok(())
    }

    #[test] // layer=pixel
    fn rounded_rect_corners() -> T {
        let mut a = start()?;
        a.new_document(64, 64, W)?;
        a.select_tool(Tool::RoundedRectangle)?;
        drag(&mut a, (0, 0), (39, 29))?;
        assert_eq!(px(&a, 0, 0)?, W);
        assert_eq!(px(&a, 39, 29)?, W);
        assert_eq!(px(&a, 20, 0)?, K);
        assert_eq!(px(&a, 0, 15)?, K);
        assert!(k_pts(&a)?.iter().all(|&(x, y)| x <= 39 && y <= 29));
        Ok(())
    }

    #[test] // layer=pixel
    fn shift_square() -> T {
        let mut a = shape(Tool::Rectangle, None, None)?;
        drag_with(&mut a, (2, 2), (12, 7), L, Mods::SHIFT)?;
        assert_eq!(k_pts(&a)?, border_pts(2, 2, 7, 7, 1));
        Ok(())
    }

    #[test] // layer=pixel
    fn one_pixel_wide_ellipse_is_line() -> T {
        let mut a = shape(Tool::Ellipse, None, None)?;
        drag(&mut a, (5, 2), (5, 9))?;
        assert_eq!(k_pts(&a)?, rect_pts(5, 2, 5, 9));
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }
}

mod tool_polygon {
    use crate::common::*;

    fn square_poly(style: Option<u32>, bg: Rgb) -> HResult<App> {
        let mut a = fresh()?;
        set_bg(&mut a, bg)?;
        a.select_tool(Tool::Polygon)?;
        if let Some(s) = style {
            opt(&mut a, ToolOption::FillStyle(s))?;
        }
        drag(&mut a, (2, 2), (12, 2))?;
        click(&mut a, 12, 12)?;
        click(&mut a, 2, 12)?;
        click(&mut a, 2, 12)?;
        Ok(a)
    }

    #[test] // layer=pixel
    fn double_click_closes() -> T {
        let a = square_poly(None, W)?;
        assert_eq!(k_pts(&a)?, border_pts(2, 2, 12, 12, 1));
        Ok(())
    }

    #[test] // layer=pixel
    fn style2_interior_background() -> T {
        let a = square_poly(Some(2), RED)?;
        assert_eq!(px(&a, 7, 7)?, RED);
        assert_eq!(pts_of(&a, RED)?, rect_pts(3, 3, 11, 11));
        Ok(())
    }

    #[test]
    fn click_first_vertex_closes() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Polygon)?;
        drag(&mut a, (2, 2), (12, 2))?;
        click(&mut a, 12, 12)?;
        a.advance(1000)?;
        click(&mut a, 2, 2)?;
        assert_eq!(px(&a, 2, 7)?, K);
        assert!(undo_enabled(&a)?);
        drag(&mut a, (20, 20), (28, 20))?;
        let comp = a.render(Surface::Composite)?;
        assert_eq!(comp.get(24, 20), Some(K));
        assert_eq!(comp.get(16, 11), Some(W));
        Ok(())
    }

    #[test]
    fn shift_click_snaps() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Polygon)?;
        drag(&mut a, (2, 2), (12, 2))?;
        click_with(&mut a, 20, 9, L, Mods::SHIFT)?;
        let comp = a.render(Surface::Composite)?;
        assert_eq!(comp.get(19, 9), Some(K));
        assert_eq!(comp.get(15, 5), Some(K));
        assert_eq!(comp.get(20, 9), Some(W));
        Ok(())
    }

    #[test]
    fn esc_discards() -> T {
        let mut a = fresh()?;
        a.select_tool(Tool::Polygon)?;
        drag(&mut a, (2, 2), (12, 2))?;
        assert_eq!(a.render(Surface::Composite)?.get(7, 2), Some(K));
        esc(&mut a)?;
        assert!(all_are(&a, W)?);
        assert!(where_eq(&a.render(Surface::Composite)?, K).is_empty());
        Ok(())
    }

    #[test]
    fn close_is_one_undo_step() -> T {
        let mut a = square_poly(None, W)?;
        assert!(!k_pts(&a)?.is_empty());
        undo(&mut a)?;
        assert!(all_are(&a, W)?);
        Ok(())
    }
}
