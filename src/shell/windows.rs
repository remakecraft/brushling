//! The modal dialog, the Help Topics window and the chrome the thumbnail
//! window shares with it. Each draws the app's model of that window and
//! reports clicks as [`Action`]s.

use std::collections::HashMap;

use brushling::{App, Dialog, DialogKind, HelpAction, HelpTab, Panel};
use eframe::egui::{self, Color32, Margin, Rect, Stroke, Ui, Vec2};

use super::theme::{self, Bevel, WindowChrome};
use super::Action;

/// The labels a choice field offers. The harness lists only the current
/// value, so the choices come from the dialog contracts in `harness.rs`.
pub fn choices(kind: DialogKind, field: &str) -> Option<&'static [&'static str]> {
    use DialogKind as K;
    let list: &'static [&'static str] = match (kind, field) {
        (K::FileOpen, "Files of type") => &["Bitmap files (*.bmp;*.dib)", "All files (*.*)"],
        (K::FileSave, "Save as type") => &[
            "Monochrome bitmap (1-bit)",
            "16-color bitmap (4-bit)",
            "256-color bitmap (8-bit)",
            "24-bit bitmap",
        ],
        (K::PageSetup, "Paper") => &["Letter", "Legal", "A4"],
        (K::PageSetup, "Orientation") => &["Portrait", "Landscape"],
        (K::Attributes, "Units") => &["Inches", "Cm", "Pixels"],
        (K::Attributes, "Colors") => &["Black and white", "Colors"],
        (K::FlipRotate, "Action") => &["Flip horizontal", "Flip vertical", "Rotate by angle"],
        (K::FlipRotate, "Angle") => &["90", "180", "270"],
        (K::CustomZoom, "Zoom") => &["100%", "200%", "400%", "600%", "800%"],
        _ => return None,
    };
    Some(list)
}

/// A window's face: grey, square, with room for the 3D edge painted on top.
fn window_frame() -> egui::Frame {
    egui::Frame::NONE.fill(theme::FACE).inner_margin(Margin::same(3))
}

/// The body under a title bar.
fn body_frame() -> egui::Frame {
    egui::Frame::NONE.inner_margin(Margin::symmetric(6, 4))
}

/// A movable window with a title bar and close button, drawn by the theme
/// (egui's own title bar cannot be coloured per state). Returns true when
/// the close button was clicked.
pub fn chrome_window(ctx: &egui::Context, id: egui::Id, title: &str, resizable: bool, default_size: Option<Vec2>, add: impl FnOnce(&mut Ui)) -> bool {
    let active = ctx.top_layer_id() == Some(egui::LayerId::new(egui::Order::Middle, id));
    let mut close = false;
    let mut window = egui::Window::new(title).id(id).title_bar(false).resizable(resizable).frame(window_frame());
    if let Some(s) = default_size {
        window = window.default_size(s);
    }
    let shown = window.show(ctx, |ui| {
        if resizable {
            ui.set_min_width(ui.available_width());
        }
        let chrome = WindowChrome::begin(ui);
        body_frame().show(ui, add);
        close = chrome.finish(ui, title, active, true, id);
    });
    if let Some(r) = shown {
        theme::frame_edges(ctx, r.response.layer_id, r.response.rect, Bevel::Window);
    }
    close
}

/// The sunken white edge of a text field or drop-down list, drawn over
/// egui's frame; a drop-down also gets its raised arrow button.
fn field_edges(ui: &Ui, rect: Rect, dropdown: bool) {
    let (u, ppp) = theme::metrics(ui);
    let rect = theme::snap(rect, ppp);
    let p = ui.painter();
    theme::bevel(p, rect, Bevel::Sunken, u);
    if !dropdown {
        return;
    }
    let inner = rect.shrink(2.0 * u);
    let button = Rect::from_min_max(egui::Pos2::new(inner.max.x - 16.0 * u, inner.min.y), inner.max);
    let face = theme::panel(p, button, Bevel::Raised, theme::FACE, u);
    let px = theme::Pixels { painter: p, origin: face.min, u };
    let (w, h) = ((face.width() / u) as i32, (face.height() / u) as i32);
    for i in 0..4 {
        px.fill((w - 7) / 2 + i, (h - 4) / 2 + i, 7 - 2 * i, 1, theme::BLACK);
    }
}

/// White, frameless widget backgrounds for a field's own scope.
fn white_fields(ui: &mut Ui) {
    let w = &mut ui.visuals_mut().widgets;
    for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        v.weak_bg_fill = Color32::WHITE;
        v.bg_fill = Color32::WHITE;
        v.bg_stroke = Stroke::NONE;
    }
}

/// Text being edited in the open dialog's fields. A field follows the app's
/// value except while it has keyboard focus.
#[derive(Default)]
pub struct DialogEdits {
    open: Option<(DialogKind, String)>,
    text: HashMap<String, String>,
    focus_first: bool,
}

pub fn dialog(ctx: &egui::Context, d: &Dialog, edits: &mut DialogEdits, out: &mut Vec<Action>) {
    let key = (d.kind, d.title.clone());
    if edits.open.as_ref() != Some(&key) {
        *edits = DialogEdits { open: Some(key), text: HashMap::new(), focus_first: true };
    }
    let id = egui::Id::new("brushling-dialog");
    // The modal draws no frame of its own; the window face inside it is
    // what gets the 3D edge.
    egui::Modal::new(id).frame(egui::Frame::NONE).backdrop_color(Color32::TRANSPARENT).show(ctx, |ui| {
        let face = window_frame().show(ui, |ui| {
            ui.set_min_width(340.0);
            let chrome = WindowChrome::begin(ui);
            body_frame().show(ui, |ui| dialog_body(ui, d, edits, out));
            chrome.finish(ui, &d.title, true, false, id);
        });
        theme::frame_edges(ui.ctx(), ui.layer_id(), face.response.rect, Bevel::Window);
    });
}

fn dialog_body(ui: &mut Ui, d: &Dialog, edits: &mut DialogEdits, out: &mut Vec<Action>) {
    if !d.text.is_empty() {
        ui.label(&d.text);
    }
    if !d.fields.is_empty() {
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            egui::Grid::new("brushling-dialog-fields").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for f in &d.fields {
                    ui.label(&f.name);
                    ui.add_enabled_ui(f.enabled, |ui| field(ui, d.kind, f, edits, out));
                    ui.end_row();
                }
            });
        });
    }
    // Web build: the file store starts empty, so Open dialogs can upload.
    #[cfg(target_arch = "wasm32")]
    if d.kind == DialogKind::FileOpen {
        ui.add_space(6.0);
        if theme::button(ui, "Upload from this device...", true, theme::DIALOG_BUTTON).clicked() {
            out.push(Action::Upload);
        }
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for b in &d.buttons {
            let enabled = !d.disabled_buttons.contains(b);
            if theme::button(ui, b, enabled, theme::DIALOG_BUTTON).clicked() {
                out.push(Action::DialogClick(b.clone()));
            }
        }
    });
}

fn field(ui: &mut Ui, kind: DialogKind, f: &brushling::DialogField, edits: &mut DialogEdits, out: &mut Vec<Action>) {
    if let Some(options) = choices(kind, &f.name) {
        let r = ui.scope(|ui| {
            white_fields(ui);
            egui::ComboBox::from_id_salt(("brushling-choice", &f.name))
                .selected_text(f.value.as_str())
                .width(200.0)
                .popup_style(theme::list_style.into())
                .show_ui(ui, |ui| {
                    for o in options {
                        if ui.selectable_label(f.value == *o, *o).clicked() && f.value != *o {
                            out.push(Action::DialogSet(f.name.clone(), (*o).to_owned()));
                        }
                    }
                })
                .response
        });
        field_edges(ui, r.inner.rect, true);
        return;
    }
    let buf = edits.text.entry(f.name.clone()).or_insert_with(|| f.value.clone());
    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(280.0).margin(Margin::symmetric(5, 3)));
    field_edges(ui, r.rect, false);
    if edits.focus_first && f.enabled {
        r.request_focus();
        edits.focus_first = false;
    }
    if r.changed() {
        out.push(Action::DialogSet(f.name.clone(), buf.clone()));
    } else if !r.has_focus() && *buf != f.value {
        buf.clone_from(&f.value);
    }
}

// ----- Help Topics

#[derive(Default)]
pub struct HelpEdits {
    keyword: String,
    search: String,
}

/// Size of the Help Topics toolbar buttons, in units.
const HELP_BUTTON: Vec2 = Vec2::new(60.0, 21.0);

pub fn help(ctx: &egui::Context, app: &App, edits: &mut HelpEdits, out: &mut Vec<Action>) {
    let Ok(Some(h)) = app.help_window() else { return };
    let id = egui::Id::new("brushling-help");
    let close = chrome_window(ctx, id, "Help Topics", true, Some(Vec2::new(620.0, 440.0)), |ui| {
        ui.horizontal(|ui| {
            for (label, enabled) in &h.toolbar {
                let r = theme::button(ui, label, *enabled, HELP_BUTTON);
                if label == "Options" {
                    let menu = egui::Popup::menu(&r).style(theme::menu_style).show(|ui| {
                        for o in &h.options {
                            if ui.button(o.as_str()).clicked() {
                                out.push(Action::Help(HelpAction::Options(o.clone())));
                                ui.close();
                            }
                        }
                    });
                    if let Some(m) = menu {
                        theme::frame_edges(ui.ctx(), m.response.layer_id, m.response.rect, Bevel::Window);
                    }
                    continue;
                }
                if r.clicked() {
                    let action = match label.as_str() {
                        "Back" => HelpAction::Back,
                        "Forward" => HelpAction::Forward,
                        _ => HelpAction::Hide,
                    };
                    out.push(Action::Help(action));
                }
            }
        });
        theme::etched_separator(ui);
        ui.horizontal_top(|ui| {
            if h.tabs_visible {
                ui.vertical(|ui| {
                    ui.set_width(230.0);
                    help_tabs(ui, &h, edits, out);
                });
                ui.add_space(4.0);
            }
            ui.vertical(|ui| {
                ui.heading(&h.topic_title);
                egui::ScrollArea::vertical().id_salt("brushling-help-topic").show(ui, |ui| ui.label(&h.topic_text));
            });
        });
    });
    if close {
        out.push(Action::ClosePanel(Panel::Help));
    }
}

/// A row of tabs, the current one raised to join the page below it.
fn tabs(ui: &mut Ui, current: HelpTab, out: &mut Vec<Action>) {
    let (u, ppp) = theme::metrics(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (tab, label) in [(HelpTab::Contents, "Contents"), (HelpTab::Index, "Index"), (HelpTab::Search, "Search")] {
            let on = current == tab;
            let galley = ui.painter().layout_no_wrap(label.to_owned(), theme::font(), theme::BLACK);
            let size = Vec2::new(galley.size().x + 14.0 * u, 20.0 * u);
            let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
            // The current tab stands two units taller.
            let rect = theme::snap(if on { rect } else { rect.with_min_y(rect.min.y + 2.0 * u) }, ppp);
            let p = ui.painter();
            theme::solid(p, rect, theme::FACE);
            let px = theme::Pixels { painter: p, origin: rect.min, u };
            let (w, h) = ((rect.width() / u) as i32, (rect.height() / u) as i32);
            px.fill(1, 0, w - 2, 1, theme::WHITE);
            px.fill(0, 1, 1, h - 1, theme::WHITE);
            px.fill(w - 1, 1, 1, h - 1, theme::BLACK);
            px.fill(w - 2, 1, 1, h - 1, theme::SHADOW);
            p.text(theme::snap_pos(rect.center(), ppp), egui::Align2::CENTER_CENTER, label, theme::font(), theme::BLACK);
            if r.clicked() && !on {
                out.push(Action::Help(HelpAction::SelectTab(tab)));
            }
        }
    });
}

fn help_tabs(ui: &mut Ui, h: &brushling::HelpView, edits: &mut HelpEdits, out: &mut Vec<Action>) {
    tabs(ui, h.tab, out);
    ui.add_space(4.0);
    let list = |ui: &mut Ui, id: &str, items: &[String], selected: Option<&String>, out: &mut Vec<Action>, pick: fn(String) -> HelpAction| {
        // A white, sunken list box.
        let r = egui::Frame::NONE.fill(Color32::WHITE).inner_margin(Margin::same(3)).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::vertical().id_salt(id).max_height(300.0).show(ui, |ui| {
                for it in items {
                    if ui.selectable_label(selected == Some(it), it.as_str()).clicked() {
                        out.push(Action::Help(pick(it.clone())));
                    }
                }
            });
        });
        field_edges(ui, r.response.rect, false);
    };
    match h.tab {
        HelpTab::Contents => list(ui, "brushling-help-contents", &h.contents, None, out, HelpAction::OpenTopic),
        HelpTab::Index => {
            sync_edit(ui, &mut edits.keyword, &h.keyword_field, out, HelpAction::TypeKeyword);
            list(ui, "brushling-help-index", &h.keywords, h.selected_keyword.as_ref(), out, HelpAction::SelectKeyword);
            if theme::button(ui, "Display", true, theme::DIALOG_BUTTON).clicked() {
                out.push(Action::Help(HelpAction::Display));
            }
        }
        HelpTab::Search => {
            sync_edit(ui, &mut edits.search, &h.search_field, out, HelpAction::TypeSearch);
            if theme::button(ui, "List Topics", true, theme::DIALOG_BUTTON).clicked() {
                out.push(Action::Help(HelpAction::ListTopics));
            }
            list(ui, "brushling-help-results", &h.results, h.selected_result.as_ref(), out, HelpAction::SelectResult);
            if theme::button(ui, "Display", true, theme::DIALOG_BUTTON).clicked() {
                out.push(Action::Help(HelpAction::Display));
            }
        }
    }
}

fn sync_edit(ui: &mut Ui, buf: &mut String, value: &str, out: &mut Vec<Action>, act: fn(String) -> HelpAction) {
    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(220.0).margin(Margin::symmetric(5, 3)));
    field_edges(ui, r.rect, false);
    if r.changed() {
        out.push(Action::Help(act(buf.clone())));
    } else if !r.has_focus() && buf != value {
        value.clone_into(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_fields_follow_the_dialog_contracts() {
        assert_eq!(choices(DialogKind::FlipRotate, "Angle"), Some(&["90", "180", "270"][..]));
        assert_eq!(choices(DialogKind::FileSave, "Save as type").map(<[_]>::len), Some(4));
        // Text fields have no choice list.
        assert_eq!(choices(DialogKind::FileSave, "File name"), None);
        assert_eq!(choices(DialogKind::Attributes, "Width"), None);
        assert_eq!(choices(DialogKind::EditColors, "Red"), None);
    }
}
