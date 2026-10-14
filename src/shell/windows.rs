//! The modal dialog, the Help Topics window and the thumbnail window. Each
//! draws the app's model of that window and reports clicks as [`Action`]s.

use std::collections::HashMap;

use brushling::{App, Dialog, DialogKind, HelpAction, HelpTab, Panel};
use eframe::egui::{self, Ui};

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
    egui::Modal::new(egui::Id::new("brushling-dialog")).show(ctx, |ui| {
        ui.set_min_width(340.0);
        ui.heading(&d.title);
        if !d.text.is_empty() {
            ui.add_space(4.0);
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
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for b in &d.buttons {
                let enabled = !d.disabled_buttons.contains(b);
                if ui.add_enabled(enabled, egui::Button::new(b.as_str())).clicked() {
                    out.push(Action::DialogClick(b.clone()));
                }
            }
        });
    });
}

fn field(ui: &mut Ui, kind: DialogKind, f: &brushling::DialogField, edits: &mut DialogEdits, out: &mut Vec<Action>) {
    if let Some(options) = choices(kind, &f.name) {
        egui::ComboBox::from_id_salt(("brushling-choice", &f.name)).selected_text(f.value.as_str()).show_ui(ui, |ui| {
            for o in options {
                if ui.selectable_label(f.value == *o, *o).clicked() && f.value != *o {
                    out.push(Action::DialogSet(f.name.clone(), (*o).to_owned()));
                }
            }
        });
        return;
    }
    let buf = edits.text.entry(f.name.clone()).or_insert_with(|| f.value.clone());
    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(280.0));
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

pub fn help(ctx: &egui::Context, app: &App, edits: &mut HelpEdits, out: &mut Vec<Action>) {
    let Ok(Some(h)) = app.help_window() else { return };
    let mut open = true;
    egui::Window::new("Help Topics")
        .id(egui::Id::new("brushling-help"))
        .open(&mut open)
        .default_size([620.0, 440.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, enabled) in &h.toolbar {
                    if label == "Options" {
                        ui.add_enabled_ui(*enabled, |ui| {
                            ui.menu_button("Options", |ui| {
                                for o in &h.options {
                                    if ui.button(o.as_str()).clicked() {
                                        out.push(Action::Help(HelpAction::Options(o.clone())));
                                        ui.close();
                                    }
                                }
                            });
                        });
                        continue;
                    }
                    if ui.add_enabled(*enabled, egui::Button::new(label.as_str())).clicked() {
                        let action = match label.as_str() {
                            "Back" => HelpAction::Back,
                            "Forward" => HelpAction::Forward,
                            _ => HelpAction::Hide,
                        };
                        out.push(Action::Help(action));
                    }
                }
            });
            ui.separator();
            ui.horizontal_top(|ui| {
                if h.tabs_visible {
                    ui.vertical(|ui| {
                        ui.set_width(230.0);
                        help_tabs(ui, &h, edits, out);
                    });
                    ui.separator();
                }
                ui.vertical(|ui| {
                    ui.heading(&h.topic_title);
                    egui::ScrollArea::vertical().id_salt("brushling-help-topic").show(ui, |ui| ui.label(&h.topic_text));
                });
            });
        });
    if !open {
        out.push(Action::ClosePanel(Panel::Help));
    }
}

fn help_tabs(ui: &mut Ui, h: &brushling::HelpView, edits: &mut HelpEdits, out: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        for (tab, label) in [(HelpTab::Contents, "Contents"), (HelpTab::Index, "Index"), (HelpTab::Search, "Search")] {
            if ui.selectable_label(h.tab == tab, label).clicked() && h.tab != tab {
                out.push(Action::Help(HelpAction::SelectTab(tab)));
            }
        }
    });
    ui.separator();
    let list = |ui: &mut Ui, id: &str, items: &[String], selected: Option<&String>, out: &mut Vec<Action>, pick: fn(String) -> HelpAction| {
        egui::ScrollArea::vertical().id_salt(id).max_height(300.0).show(ui, |ui| {
            for it in items {
                if ui.selectable_label(selected == Some(it), it.as_str()).clicked() {
                    out.push(Action::Help(pick(it.clone())));
                }
            }
        });
    };
    match h.tab {
        HelpTab::Contents => list(ui, "brushling-help-contents", &h.contents, None, out, HelpAction::OpenTopic),
        HelpTab::Index => {
            sync_edit(ui, &mut edits.keyword, &h.keyword_field, out, HelpAction::TypeKeyword);
            list(ui, "brushling-help-index", &h.keywords, h.selected_keyword.as_ref(), out, HelpAction::SelectKeyword);
            if ui.button("Display").clicked() {
                out.push(Action::Help(HelpAction::Display));
            }
        }
        HelpTab::Search => {
            sync_edit(ui, &mut edits.search, &h.search_field, out, HelpAction::TypeSearch);
            if ui.button("List Topics").clicked() {
                out.push(Action::Help(HelpAction::ListTopics));
            }
            list(ui, "brushling-help-results", &h.results, h.selected_result.as_ref(), out, HelpAction::SelectResult);
            if ui.button("Display").clicked() {
                out.push(Action::Help(HelpAction::Display));
            }
        }
    }
}

fn sync_edit(ui: &mut Ui, buf: &mut String, value: &str, out: &mut Vec<Action>, act: fn(String) -> HelpAction) {
    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(220.0));
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
