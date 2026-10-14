//! The bars around the workspace, drawn from the app's state each frame:
//! menu bar, tool box with its options, colour box, status bar and the Print
//! Preview toolbar. Clicks become [`Action`]s the shell applies afterwards,
//! so drawing only ever needs a shared borrow of the app.

use brushling::{App, Button, Hover, Rgb, Tool, ToolOption, ToolOptions, ViewMode};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Ui, Vec2};

use super::theme::{self, Bevel, Pixels};
use super::{icons, options, Action};

/// What the panels report back besides their actions.
#[derive(Default)]
pub struct PanelOutput {
    pub actions: Vec<Action>,
    /// The tool button or menu item under the pointer, for `App::hover`.
    pub hover: Option<Hover>,
    pub error: Option<String>,
}

impl PanelOutput {
    fn fail(&mut self, e: brushling::HarnessError) {
        self.error = Some(e.to_string());
    }
}

pub fn color(c: Rgb) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

// ----- menu bar

pub fn menu_bar(ui: &mut Ui, app: &App, out: &mut PanelOutput) {
    let titles = match app.menu_items("") {
        Ok(t) => t,
        Err(e) => return out.fail(e),
    };
    egui::MenuBar::new().style(theme::menu_style).config(theme::menu_config()).ui(ui, |ui| {
        for t in titles.iter().filter(|t| !t.separator) {
            ui.add_enabled_ui(t.enabled, |ui| {
                ui.menu_button(t.label.as_str(), |ui| menu_items(ui, app, &t.label, out));
            });
        }
    });
}

fn menu_items(ui: &mut Ui, app: &App, path: &str, out: &mut PanelOutput) {
    let items = match app.menu_items(path) {
        Ok(items) => items,
        Err(e) => return out.fail(e),
    };
    // The drop-down's 3D edge, on the popup's rect as of last frame: egui
    // sizes a popup only after its content.
    if let Some(rect) = ui.ctx().memory(|m| m.area_rect(ui.layer_id().id)) {
        theme::frame_edges(ui.ctx(), ui.layer_id(), rect, Bevel::Window);
    }
    for it in items {
        if it.separator {
            theme::etched_separator(ui);
            continue;
        }
        let full = format!("{path} > {}", it.label);
        if it.submenu {
            let r = ui.add_enabled_ui(it.enabled, |ui| ui.menu_button(it.label.as_str(), |ui| menu_items(ui, app, &full, out)));
            if r.inner.response.contains_pointer() {
                out.hover = Some(Hover::MenuItem(full));
            }
            continue;
        }
        let label = match it.checked {
            Some(true) => format!("\u{2714} {}", it.label),
            Some(false) => format!("     {}", it.label),
            None => it.label.clone(),
        };
        let mut button = egui::Button::new(label);
        if let Some(s) = it.shortcut.as_deref() {
            button = button.shortcut_text(s);
        }
        let r = ui.add_enabled(it.enabled, button);
        if r.contains_pointer() {
            out.hover = Some(Hover::MenuItem(full.clone()));
        }
        if r.clicked() {
            out.actions.push(Action::Menu(full));
            ui.close();
        }
    }
}

// ----- tool box

/// The tool's short name: the button's accessible label, and its tooltip
/// when the app has none.
pub fn short_label(tool: Tool) -> &'static str {
    match tool {
        Tool::FreeFormSelect => "Free sel",
        Tool::Select => "Select",
        Tool::Eraser => "Eraser",
        Tool::Fill => "Fill",
        Tool::PickColor => "Pick",
        Tool::Magnifier => "Zoom",
        Tool::Pencil => "Pencil",
        Tool::Brush => "Brush",
        Tool::Airbrush => "Spray",
        Tool::Text => "Text",
        Tool::Line => "Line",
        Tool::Curve => "Curve",
        Tool::Rectangle => "Rect",
        Tool::Polygon => "Polygon",
        Tool::Ellipse => "Ellipse",
        Tool::RoundedRectangle => "Rounded",
    }
}

/// The options box for `tool`: (label, option, shown selected).
pub fn tool_option_set(tool: Tool, o: &ToolOptions) -> Vec<(String, ToolOption, bool)> {
    match tool {
        Tool::FreeFormSelect | Tool::Select | Tool::Text => vec![
            ("Opaque".into(), ToolOption::Opaque(true), o.opaque),
            ("Transparent".into(), ToolOption::Opaque(false), !o.opaque),
        ],
        Tool::Eraser => [4, 6, 8, 10]
            .map(|n| (format!("{n} px"), ToolOption::EraserSize(n), o.eraser_size == n))
            .to_vec(),
        Tool::Magnifier => [1, 2, 6, 8]
            .map(|n| (format!("{n}x"), ToolOption::MagnifierLevel(n), o.magnifier_highlight == Some(n)))
            .to_vec(),
        Tool::Brush => (1..=12)
            .map(|n| (format!("{n}"), ToolOption::BrushShape(n), o.brush_shape == n))
            .collect(),
        Tool::Airbrush => [(4, "Small"), (8, "Medium"), (12, "Large")]
            .map(|(n, l)| (l.to_owned(), ToolOption::AirbrushRadius(n), o.airbrush_radius == n))
            .to_vec(),
        Tool::Line | Tool::Curve => (1..=5)
            .map(|n| (format!("{n} px"), ToolOption::LineWidth(n), o.line_width == n))
            .collect(),
        Tool::Rectangle | Tool::Polygon | Tool::Ellipse | Tool::RoundedRectangle => {
            [(1, "Outline"), (2, "Outline + fill"), (3, "Fill")]
                .map(|(n, l)| (l.to_owned(), ToolOption::FillStyle(n), o.fill_style == n))
                .to_vec()
        }
        Tool::Fill | Tool::PickColor | Tool::Pencil => Vec::new(),
    }
}

/// Tool button side, in units.
pub const TOOL_BUTTON: f32 = 25.0;
/// Tool box columns.
pub const TOOL_COLUMNS: usize = 2;

/// Width of the docked tool box panel in points, margins included.
pub fn tool_box_width(ppp: f32) -> f32 {
    TOOL_BUTTON * TOOL_COLUMNS as f32 * theme::unit(ppp) + 2.0 * TOOL_BOX_MARGIN as f32
}

/// The tool box panel's inner margin, in points.
pub const TOOL_BOX_MARGIN: i8 = 3;

/// Cell `(column, row)` of the `i`th tool button.
pub fn tool_cell(i: usize) -> (usize, usize) {
    (i % TOOL_COLUMNS, i / TOOL_COLUMNS)
}

/// One square tool button: raised, or sunken on the dither when active or
/// held, with the tool's icon.
fn tool_button(ui: &Ui, rect: Rect, tool: Tool, active: bool, enabled: bool, r: &egui::Response) {
    let (u, ppp) = theme::metrics(ui);
    let rect = theme::snap(rect, ppp);
    let p = ui.painter();
    let down = active || (enabled && r.is_pointer_button_down_on() && r.hovered());
    let inner = if down {
        theme::bevel(p, rect, Bevel::Pressed, u);
        let inner = rect.shrink(2.0 * u);
        if active {
            theme::dither_fill(ui, inner);
        } else {
            theme::solid(p, inner, theme::FACE);
        }
        inner
    } else {
        theme::panel(p, rect, Bevel::Raised, theme::FACE, u)
    };
    let side = icons::SIZE as f32 * u;
    let off = (((inner.width() - side) / (2.0 * u)).floor() + if down { 1.0 } else { 0.0 }) * u;
    let at = theme::snap_pos(inner.min + Vec2::new(off, (((inner.height() - side) / (2.0 * u)).floor()) * u + if down { u } else { 0.0 }), ppp);
    let icon = Rect::from_min_size(at, Vec2::splat(side));
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let tex = icons::texture(ui.ctx(), tool);
    if enabled {
        p.image(tex, icon, uv, Color32::WHITE);
    } else {
        // Greyed out and engraved, like disabled text.
        p.image(tex, icon.translate(Vec2::splat(u)), uv, theme::WHITE);
        p.image(tex, icon, uv, theme::SHADOW);
    }
}

pub fn tool_box(ui: &mut Ui, app: &App, out: &mut PanelOutput) {
    let buttons = match app.tool_buttons() {
        Ok(b) => b,
        Err(e) => return out.fail(e),
    };
    let (u, ppp) = theme::metrics(ui);
    let tooltip = app.tooltip().ok().flatten();
    let rows = buttons.len().div_ceil(TOOL_COLUMNS);
    let side = TOOL_BUTTON * u;
    let (grid, _) = ui.allocate_exact_size(Vec2::new(side * TOOL_COLUMNS as f32, side * rows as f32), Sense::hover());
    let origin = theme::snap_pos(grid.min, ppp);
    for (i, b) in buttons.iter().enumerate() {
        let (c, r) = tool_cell(i);
        let rect = Rect::from_min_size(origin + Vec2::new(c as f32, r as f32) * side, Vec2::splat(side));
        let sense = if b.enabled { Sense::click() } else { Sense::hover() };
        let mut resp = ui.interact(rect, ui.id().with(("brushling-tool", i)), sense);
        let label = short_label(b.tool);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, b.enabled, b.active, label));
        tool_button(ui, rect, b.tool, b.active, b.enabled, &resp);
        if resp.contains_pointer() {
            out.hover = Some(Hover::ToolButton(b.tool));
            resp = resp.on_hover_text(tooltip.as_deref().unwrap_or(label));
        }
        if resp.clicked() {
            out.actions.push(Action::Tool(b.tool));
        }
    }
    ui.add_space(3.0 * u);
    let (area, _) = ui.allocate_exact_size(Vec2::new(grid.width(), options::BOX.1 as f32 * u), Sense::hover());
    let (Ok(tool), Ok(opts)) = (app.active_tool(), app.tool_options()) else { return };
    let set = tool_option_set(tool, &opts);
    let columns = if tool == Tool::Brush { 3 } else { 1 };
    if let Some(o) = options::options_box(ui, area, &set, columns) {
        out.actions.push(Action::Option(o));
    }
}

// ----- colour box

/// Swatch side in units: a 2-unit sunken edge round the colour.
pub const SWATCH: f32 = 16.0;
/// Swatches per row.
pub const SWATCH_COLUMNS: usize = 14;

/// Cell `(column, row)` of swatch `i`.
pub fn swatch_cell(i: usize) -> (usize, usize) {
    (i % SWATCH_COLUMNS, i / SWATCH_COLUMNS)
}

fn swatch(ui: &Ui, rect: Rect, c: Rgb) {
    let (u, ppp) = theme::metrics(ui);
    theme::panel(ui.painter(), theme::snap(rect, ppp), Bevel::Sunken, color(c), u);
}

/// The current-colours well: the background square behind and below-right
/// of the foreground one, on the dither.
fn current_colors(ui: &Ui, rect: Rect, fg: Rgb, bg: Rgb) {
    let (u, ppp) = theme::metrics(ui);
    let p = ui.painter();
    let well = theme::snap(rect, ppp);
    theme::bevel(p, well, Bevel::Sunken, u);
    let inner = well.shrink(2.0 * u);
    theme::dither_fill(ui, inner);
    let px = Pixels { painter: p, origin: inner.min, u };
    let n = (inner.width() / u) as i32;
    // Two overlapping squares of side `s`, 3 units in from opposite corners.
    let s = n / 2 - 1;
    for (x, c) in [(n - 3 - s, bg), (3, fg)] {
        px.fill(x, x, s, s, theme::WHITE);
        px.fill(x, x, s - 1, s - 1, theme::SHADOW);
        px.fill(x + 1, x + 1, s - 2, s - 2, color(c));
    }
}

pub fn color_box(ui: &mut Ui, app: &App, out: &mut PanelOutput) {
    let (swatches, (fg, bg)) = match (app.swatches(), app.colors()) {
        (Ok(s), Ok(c)) => (s, c),
        (Err(e), _) | (_, Err(e)) => return out.fail(e),
    };
    let (u, ppp) = theme::metrics(ui);
    let side = SWATCH * u;
    let rows = swatches.len().div_ceil(SWATCH_COLUMNS).max(1);
    let well = side * rows as f32;
    let width = well + 4.0 * u + side * SWATCH_COLUMNS as f32;
    let (area, _) = ui.allocate_exact_size(Vec2::new(width, well), Sense::hover());
    let origin = theme::snap_pos(area.min, ppp);
    current_colors(ui, Rect::from_min_size(origin, Vec2::splat(well)), fg, bg);
    let grid = origin + Vec2::new(well + 4.0 * u, 0.0);
    for (i, c) in swatches.iter().enumerate() {
        let n = i as u32 + 1;
        let (col, row) = swatch_cell(i);
        let rect = Rect::from_min_size(grid + Vec2::new(col as f32, row as f32) * side, Vec2::splat(side));
        let r = ui.interact(rect, ui.id().with(("brushling-swatch", i)), Sense::click());
        swatch(ui, rect, *c);
        if r.double_clicked() {
            out.actions.push(Action::Swatch(n, Button::Left, true));
        } else if r.clicked() {
            out.actions.push(Action::Swatch(n, Button::Left, false));
        } else if r.secondary_clicked() {
            out.actions.push(Action::Swatch(n, Button::Right, false));
        }
    }
}

// ----- status bar and Print Preview toolbar

/// Width of the position and size panes, in units.
pub const STATUS_PANE: f32 = 100.0;

/// Pane rects `[message, position, size]` across `row`, `gap` apart; the
/// message pane takes what the two fixed panes leave.
pub fn status_panes(row: Rect, pane: f32, gap: f32) -> [Rect; 3] {
    let size = Rect::from_min_max(Pos2::new((row.max.x - pane).max(row.min.x), row.min.y), row.max);
    let pos = Rect::from_min_max(Pos2::new((size.min.x - gap - pane).max(row.min.x), row.min.y), Pos2::new((size.min.x - gap).max(row.min.x), row.max.y));
    let msg = Rect::from_min_max(row.min, Pos2::new((pos.min.x - gap).max(row.min.x), row.max.y));
    [msg, pos, size]
}

/// The status line as three sunken panes; a harness error, when there is
/// one, leads the message pane.
pub fn status_bar(ui: &mut Ui, app: &App, error: Option<&str>) {
    let status = app.status_bar().ok().filter(|s| s.visible);
    let (u, ppp) = theme::metrics(ui);
    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0 * u), Sense::hover());
    let row = theme::snap(row, ppp);
    let panes = status_panes(row, STATUS_PANE * u, 2.0 * u);
    let p = ui.painter();
    let mut texts: [Vec<(String, Color32)>; 3] = Default::default();
    if let Some(e) = error {
        texts[0].push((e.to_owned(), ui.visuals().error_fg_color));
    }
    if let Some(s) = &status {
        texts[0].push((s.message.clone(), theme::BLACK));
        texts[1].push((s.position.clone(), theme::BLACK));
        texts[2].push((s.size.clone(), theme::BLACK));
    }
    for (pane, parts) in panes.iter().zip(texts) {
        if pane.width() < 4.0 * u {
            continue;
        }
        theme::bevel(p, theme::snap(*pane, ppp), Bevel::ThinSunken, u);
        let clip = p.with_clip_rect(pane.shrink(u).intersect(p.clip_rect()));
        let mut x = pane.min.x + 4.0 * u;
        for (text, c) in parts {
            let r = clip.text(theme::snap_pos(Pos2::new(x, pane.center().y), ppp), Align2::LEFT_CENTER, &text, theme::font(), c);
            x = r.max.x + 12.0 * u;
        }
    }
}

pub fn preview_toolbar(ui: &mut Ui, app: &App, mode: ViewMode, out: &mut PanelOutput) {
    let buttons = match app.preview_buttons() {
        Ok(b) => b,
        Err(e) => return out.fail(e),
    };
    ui.horizontal(|ui| {
        for b in &buttons {
            if theme::button(ui, &b.label, b.enabled, Vec2::new(60.0, 21.0)).clicked() {
                out.actions.push(Action::Preview(b.label.clone()));
            }
        }
        if let ViewMode::PrintPreview { page, pages } = mode {
            ui.add_space(8.0);
            ui.label(format!("Page {page} of {pages}"));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ToolOptions {
        ToolOptions {
            opaque: true,
            eraser_size: 6,
            magnifier_highlight: Some(2),
            brush_shape: 5,
            airbrush_radius: 8,
            line_width: 1,
            fill_style: 3,
        }
    }

    fn selected(tool: Tool) -> Vec<ToolOption> {
        tool_option_set(tool, &opts()).into_iter().filter(|o| o.2).map(|o| o.1).collect()
    }

    #[test]
    fn exactly_the_current_option_is_selected() {
        assert_eq!(selected(Tool::Eraser), vec![ToolOption::EraserSize(6)]);
        assert_eq!(selected(Tool::Magnifier), vec![ToolOption::MagnifierLevel(2)]);
        assert_eq!(selected(Tool::Select), vec![ToolOption::Opaque(true)]);
        assert_eq!(selected(Tool::Ellipse), vec![ToolOption::FillStyle(3)]);
        assert_eq!(selected(Tool::Curve), vec![ToolOption::LineWidth(1)]);
        assert_eq!(selected(Tool::Airbrush), vec![ToolOption::AirbrushRadius(8)]);
        assert_eq!(tool_option_set(Tool::Brush, &opts()).len(), 12);
        assert!(tool_option_set(Tool::Pencil, &opts()).is_empty());
    }

    #[test]
    fn every_tool_has_a_distinct_label() {
        let mut labels: Vec<_> = Tool::ALL.iter().map(|t| short_label(*t)).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 16);
    }

    #[test]
    fn tools_fill_a_two_column_grid_in_order() {
        let cells: Vec<_> = (0..Tool::ALL.len()).map(tool_cell).collect();
        assert_eq!(cells[0], (0, 0));
        assert_eq!(cells[1], (1, 0));
        assert_eq!(cells[15], (1, 7));
        let mut sorted = cells.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), 16);
        // The grid plus margins is the docked tool box's width at 1:1.
        assert_eq!(tool_box_width(1.0), 56.0);
    }

    #[test]
    fn swatches_wrap_after_fourteen() {
        assert_eq!(swatch_cell(13), (13, 0));
        assert_eq!(swatch_cell(14), (0, 1));
        assert_eq!(swatch_cell(27), (13, 1));
    }

    #[test]
    fn status_panes_keep_their_order_and_never_invert() {
        let row = Rect::from_min_size(Pos2::new(10.0, 0.0), Vec2::new(500.0, 18.0));
        let [msg, pos, size] = status_panes(row, 100.0, 2.0);
        assert_eq!(size.max.x, 510.0);
        assert_eq!(size.width(), 100.0);
        assert_eq!(pos.max.x, size.min.x - 2.0);
        assert_eq!(msg.min.x, 10.0);
        assert_eq!(msg.max.x, pos.min.x - 2.0);
        // Too narrow: panes collapse at the left edge instead of turning inside out.
        for r in status_panes(Rect::from_min_size(Pos2::ZERO, Vec2::new(50.0, 18.0)), 100.0, 2.0) {
            assert!(r.width() >= 0.0);
        }
    }
}
