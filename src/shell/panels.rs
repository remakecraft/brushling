//! The bars around the workspace, drawn from the app's state each frame:
//! menu bar, tool box with its options, colour box, status bar and the Print
//! Preview toolbar. Clicks become [`Action`]s the shell applies afterwards,
//! so drawing only ever needs a shared borrow of the app.

use brushling::{App, Button, Hover, Rgb, Tool, ToolOption, ToolOptions, ViewMode};
use eframe::egui::{self, Color32, Sense, Ui, Vec2};

use super::Action;

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
    egui::MenuBar::new().ui(ui, |ui| {
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
    for it in items {
        if it.separator {
            ui.separator();
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

/// Text for a tool button; the app's own name shows as its tooltip.
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

pub fn tool_box(ui: &mut Ui, app: &App, out: &mut PanelOutput) {
    let buttons = match app.tool_buttons() {
        Ok(b) => b,
        Err(e) => return out.fail(e),
    };
    let tooltip = app.tooltip().ok().flatten();
    egui::Grid::new("brushling-tools").num_columns(2).spacing([2.0, 2.0]).show(ui, |ui| {
        for (i, b) in buttons.iter().enumerate() {
            let button = egui::Button::selectable(b.active, short_label(b.tool)).min_size(Vec2::new(60.0, 24.0));
            let mut r = ui.add_enabled(b.enabled, button);
            if r.contains_pointer() {
                out.hover = Some(Hover::ToolButton(b.tool));
                if let Some(t) = &tooltip {
                    r = r.on_hover_text(t.as_str());
                }
            }
            if r.clicked() {
                out.actions.push(Action::Tool(b.tool));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.separator();
    let (Ok(tool), Ok(opts)) = (app.active_tool(), app.tool_options()) else { return };
    let set = tool_option_set(tool, &opts);
    let columns = if tool == Tool::Brush { 3 } else { 1 };
    egui::Grid::new("brushling-tool-options").num_columns(columns).spacing([2.0, 2.0]).show(ui, |ui| {
        for (i, (label, option, selected)) in set.into_iter().enumerate() {
            if ui.selectable_label(selected, label).clicked() {
                out.actions.push(Action::Option(option));
            }
            if (i + 1).is_multiple_of(columns) {
                ui.end_row();
            }
        }
    });
}

// ----- colour box

fn swatch(ui: &mut Ui, c: Rgb, size: f32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, color(c));
    let stroke = if r.hovered() { ui.visuals().widgets.hovered.fg_stroke } else { ui.visuals().widgets.noninteractive.bg_stroke };
    p.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
    r
}

pub fn color_box(ui: &mut Ui, app: &App, out: &mut PanelOutput) {
    let (swatches, (fg, bg)) = match (app.swatches(), app.colors()) {
        (Ok(s), Ok(c)) => (s, c),
        (Err(e), _) | (_, Err(e)) => return out.fail(e),
    };
    ui.horizontal(|ui| {
        // Background square behind and below-right of the foreground one.
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
        let p = ui.painter();
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let back = egui::Rect::from_min_size(rect.min + Vec2::splat(12.0), Vec2::splat(22.0));
        let front = egui::Rect::from_min_size(rect.min + Vec2::splat(2.0), Vec2::splat(22.0));
        p.rect_filled(back, 0.0, color(bg));
        p.rect_stroke(back, 0.0, stroke, egui::StrokeKind::Inside);
        p.rect_filled(front, 0.0, color(fg));
        p.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
        ui.add_space(6.0);
        egui::Grid::new("brushling-swatches").spacing([2.0, 2.0]).show(ui, |ui| {
            for (i, c) in swatches.iter().enumerate() {
                let n = i as u32 + 1;
                let r = swatch(ui, *c, 16.0);
                if r.double_clicked() {
                    out.actions.push(Action::Swatch(n, Button::Left, true));
                } else if r.clicked() {
                    out.actions.push(Action::Swatch(n, Button::Left, false));
                } else if r.secondary_clicked() {
                    out.actions.push(Action::Swatch(n, Button::Right, false));
                }
                if n.is_multiple_of(14) {
                    ui.end_row();
                }
            }
        });
    });
}

// ----- status bar and Print Preview toolbar

/// The status line; a harness error, when there is one, comes first.
pub fn status_bar(ui: &mut Ui, app: &App, error: Option<&str>) {
    let status = app.status_bar().ok().filter(|s| s.visible);
    ui.horizontal(|ui| {
        if let Some(e) = error {
            ui.colored_label(ui.visuals().error_fg_color, e);
            ui.separator();
        }
        if let Some(s) = &status {
            ui.label(&s.message);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_sized([90.0, 16.0], egui::Label::new(&s.size));
                ui.separator();
                ui.add_sized([90.0, 16.0], egui::Label::new(&s.position));
                ui.separator();
            });
        }
    });
}

pub fn preview_toolbar(ui: &mut Ui, app: &App, mode: ViewMode, out: &mut PanelOutput) {
    let buttons = match app.preview_buttons() {
        Ok(b) => b,
        Err(e) => return out.fail(e),
    };
    ui.horizontal(|ui| {
        for b in &buttons {
            if ui.add_enabled(b.enabled, egui::Button::new(b.label.as_str())).clicked() {
                out.actions.push(Action::Preview(b.label.clone()));
            }
        }
        if let ViewMode::PrintPreview { page, pages } = mode {
            ui.separator();
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
}
