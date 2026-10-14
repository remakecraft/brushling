//! The eframe shell: a thin window over [`brushling::App`]. The app owns all
//! behaviour and renders the workspace itself; the shell draws the bars and
//! dialogs from the app's state, forwards input, and moves virtual time.
//! The same shell runs natively and in the browser; `web` holds what only
//! the browser needs (uploads and downloads for the in-memory file store).

mod icons;
mod input;
mod options;
mod panels;
mod scrollbar;
mod theme;
#[cfg(target_arch = "wasm32")]
mod web;
mod windows;

use std::path::PathBuf;
use std::time::Duration;

use brushling::{App, Button, HResult, HelpAction, Hover, Image, Key, Mods, Panel, PointerKind, Pos, StartOptions, Surface, Tool, ToolOption, ViewMode};
use eframe::egui::{self, Color32, Event, Margin, Pos2, Rect, Vec2};

use input::{Bars, Geometry};
use panels::PanelOutput;
use theme::Bevel;

/// A click in the shell, applied to the app after the frame's UI is drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Menu(String),
    Tool(Tool),
    Option(ToolOption),
    Swatch(u32, Button, bool),
    DialogSet(String, String),
    DialogClick(String),
    Help(HelpAction),
    ClosePanel(Panel),
    Preview(String),
    /// Web build: pick a file to upload into the open File Open dialog.
    #[cfg(target_arch = "wasm32")]
    Upload,
}

/// How to start the app once the screen size is known.
pub struct Launch {
    pub config_dir: PathBuf,
    pub args: Vec<String>,
}

/// A texture that is only re-uploaded when its image changes.
#[derive(Default)]
struct Texture {
    handle: Option<egui::TextureHandle>,
    shown: Option<Image>,
}

impl Texture {
    fn show(&mut self, ctx: &egui::Context, name: &str, img: Image) -> egui::TextureId {
        let options = egui::TextureOptions::NEAREST;
        if self.shown.as_ref() != Some(&img) || self.handle.is_none() {
            let rgb: Vec<u8> = img.pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            let color = egui::ColorImage::from_rgb([img.width as usize, img.height as usize], &rgb);
            match &mut self.handle {
                Some(h) => h.set(color, options),
                None => self.handle = Some(ctx.load_texture(name, color, options)),
            }
            self.shown = Some(img);
        }
        self.handle.as_ref().map_or(egui::TextureId::default(), egui::TextureHandle::id)
    }
}

pub struct Shell {
    launch: Launch,
    app: Option<App>,
    start_error: Option<String>,
    frames: u32,
    /// Last harness error, shown in the status line until the next one.
    error: Option<String>,
    /// egui's clock (seconds) at the last tick; `None` until the app starts.
    last_tick: Option<f64>,
    carry_ms: f64,
    workspace: Texture,
    thumbnail: Texture,
    /// The button whose press went to the workspace; it gets every move and
    /// its release wherever the pointer is.
    held: Option<Button>,
    pointer_outside: bool,
    requested_size: Option<(u32, u32)>,
    title: String,
    hover: Hover,
    fullscreen: bool,
    dialog_edits: windows::DialogEdits,
    help_edits: windows::HelpEdits,
    /// Where on the (horizontal, vertical) thumb a scroll bar drag started.
    scroll_grab: (Option<f32>, Option<f32>),
    /// Sub-pixel wheel scrolling not yet sent to the app.
    wheel_carry: (f32, f32),
    /// Whether the classic theme is installed yet (it needs the context).
    themed: bool,
    #[cfg(target_arch = "wasm32")]
    uploads: web::Uploads,
}

impl Shell {
    pub fn new(launch: Launch) -> Shell {
        Shell {
            launch,
            app: None,
            start_error: None,
            frames: 0,
            error: None,
            last_tick: None,
            carry_ms: 0.0,
            workspace: Texture::default(),
            thumbnail: Texture::default(),
            held: None,
            pointer_outside: true,
            requested_size: None,
            title: String::new(),
            hover: Hover::Nothing,
            fullscreen: false,
            dialog_edits: Default::default(),
            help_edits: Default::default(),
            scroll_grab: (None, None),
            wheel_carry: (0.0, 0.0),
            themed: false,
            #[cfg(target_arch = "wasm32")]
            uploads: web::Uploads::default(),
        }
    }

    fn note<T>(&mut self, r: HResult<T>) -> Option<T> {
        r.map_err(|e| self.error = Some(e.to_string())).ok()
    }

    /// Start the app on the first frame that knows the monitor size (or the
    /// harness default after a few frames). A file argument that fails to
    /// open falls back to an untitled image, with the error shown.
    fn start(&mut self, ctx: &egui::Context) {
        let screen = ctx.input(|i| {
            let vp = i.viewport();
            let ppp = vp.native_pixels_per_point.unwrap_or(1.0);
            vp.monitor_size.map(|s| ((s.x * ppp).round() as u32, (s.y * ppp).round() as u32))
        });
        self.frames += 1;
        if screen.is_none() && self.frames < 5 {
            return;
        }
        let mut opts = StartOptions::new(&self.launch.config_dir);
        opts.args = self.launch.args.clone();
        if let Some(s) = screen.filter(|s| s.0 > 0 && s.1 > 0) {
            opts.screen = s;
        }
        match App::start(opts.clone()) {
            Ok(app) => self.app = Some(app),
            Err(e) if !opts.args.is_empty() => {
                self.error = Some(format!("{}: {e}", opts.args[0]));
                opts.args.clear();
                match App::start(opts) {
                    Ok(app) => self.app = Some(app),
                    Err(e) => self.start_error = Some(e.to_string()),
                }
            }
            Err(e) => self.start_error = Some(e.to_string()),
        }
        self.last_tick = None;
    }

    /// Advance virtual time by the real time since the last frame (egui's
    /// clock, which also runs in the browser where `Instant` does not).
    fn tick(&mut self, app: &mut App, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let last = self.last_tick.replace(now).unwrap_or(now);
        self.carry_ms += (now - last).max(0.0) * 1000.0;
        let whole = self.carry_ms.floor();
        if whole >= 1.0 {
            self.carry_ms -= whole;
            let r = app.advance(whole as u64);
            self.note(r);
        }
    }

    /// Title, exit and the OS close button. Closing runs File > Exit so the
    /// app can ask about unsaved changes.
    fn sync_viewport(&mut self, app: &mut App, ctx: &egui::Context) {
        if let Ok(Some(_)) = app.exit_status() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if matches!(app.dialog(), Ok(None)) {
                let r = app.menu("File > Exit");
                self.note(r);
                if let Ok(Some(_)) = app.exit_status() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if let Some(t) = self.note(app.title()) {
            if t != self.title {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
                #[cfg(target_arch = "wasm32")]
                web::set_title(&t);
                self.title = t;
            }
        }
    }

    fn apply(&mut self, app: &mut App, actions: Vec<Action>) {
        for a in actions {
            let r = match a {
                Action::Menu(path) => app.menu(&path),
                Action::Tool(t) => app.select_tool(t),
                Action::Option(o) => app.set_tool_option(o),
                Action::Swatch(n, b, double) => app.click_swatch(n, b, double),
                Action::DialogSet(f, v) => app.dialog_set(&f, &v),
                Action::DialogClick(b) => app.dialog_click(&b),
                Action::Help(h) => app.help_action(h),
                Action::ClosePanel(p) => app.close_panel(p),
                Action::Preview(b) => app.preview_click(&b),
                #[cfg(target_arch = "wasm32")]
                Action::Upload => self.uploads.pick().map_err(brushling::HarnessError::Invalid),
            };
            if r.is_ok() {
                self.error = None;
            }
            self.note(r);
        }
    }

    fn send_pointer(&mut self, app: &mut App, kind: PointerKind, pos: Pos, mods: Mods) {
        let r = app.pointer(kind, pos, mods);
        self.note(r);
    }

    fn send_key(&mut self, app: &mut App, key: Key, mods: Mods) {
        let r = app.key(key, mods);
        self.note(r);
    }

    fn set_hover(&mut self, app: &mut App, hover: Hover) {
        if hover != self.hover {
            let r = app.hover(hover.clone());
            self.note(r);
            self.hover = hover;
        }
    }

    // ----- normal window

    fn normal_ui(&mut self, app: &mut App, ui: &mut egui::Ui, mode: ViewMode, popup_open: bool) {
        let preview = matches!(mode, ViewMode::PrintPreview { .. });
        let shown = |p: Panel| app.panel(p).map(|s| s.visible).unwrap_or(false);
        let (tool_box, color_box) = (!preview && shown(Panel::ToolBox), !preview && shown(Panel::ColorBox));
        let mut out = PanelOutput::default();

        // Grey bars without egui's separator lines; their 3D edges are drawn
        // by the panels themselves.
        let bar = |m: Margin| egui::Frame::NONE.fill(theme::FACE).inner_margin(m);
        egui::Panel::top("brushling-menu").frame(bar(Margin::symmetric(2, 1))).show_separator_line(false).show(ui, |ui| {
            if preview {
                panels::preview_toolbar(ui, app, mode, &mut out);
            } else {
                panels::menu_bar(ui, app, &mut out);
            }
        });
        if shown(Panel::StatusBar) || self.error.is_some() {
            let error = self.error.clone();
            egui::Panel::bottom("brushling-status")
                .frame(bar(Margin::symmetric(2, 2)))
                .show_separator_line(false)
                .show(ui, |ui| panels::status_bar(ui, app, error.as_deref()));
        }
        if color_box {
            egui::Panel::bottom("brushling-colors")
                .resizable(false)
                .frame(bar(Margin::symmetric(4, 3)))
                .show_separator_line(false)
                .show(ui, |ui| panels::color_box(ui, app, &mut out));
        }
        if tool_box {
            let width = panels::tool_box_width(ui.ctx().pixels_per_point());
            egui::Panel::left("brushling-tools")
                .resizable(false)
                .exact_size(width)
                .frame(bar(Margin::same(panels::TOOL_BOX_MARGIN)))
                .show_separator_line(false)
                .show(ui, |ui| panels::tool_box(ui, app, &mut out));
        }
        if let Some(e) = out.error.take() {
            self.error = Some(e);
        }
        let hover = out.hover.take().unwrap_or(Hover::Nothing);
        self.set_hover(app, hover);
        self.apply(app, out.actions);

        egui::CentralPanel::no_frame().frame(egui::Frame::NONE.fill(theme::FACE)).show(ui, |ui| {
            // The workspace sits in a sunken well; the app draws the grey
            // around the image itself.
            let (u, ppp) = theme::metrics(ui);
            let well = theme::snap(ui.available_rect_before_wrap(), ppp);
            theme::bevel(ui.painter(), well, Bevel::Sunken, u);
            let avail = well.shrink(Bevel::Sunken.width() * u);
            self.workspace_ui(app, ui, avail, !preview, popup_open);
        });
    }

    /// The workspace image 1:1 in physical pixels inside `avail`, its scroll
    /// bars, and the pointer and wheel input on it.
    fn workspace_ui(&mut self, app: &mut App, ui: &mut egui::Ui, avail: Rect, interactive: bool, popup_open: bool) {
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        // Snap the origin to a whole physical pixel so pixels stay crisp.
        let origin = Pos2::new((avail.min.x * ppp).ceil() / ppp, (avail.min.y * ppp).ceil() / ppp);
        let central = (
            ((avail.max.x - origin.x) * ppp).floor().max(1.0) as u32,
            ((avail.max.y - origin.y) * ppp).floor().max(1.0) as u32,
        );
        self.request_size(app, central);
        // The app's workspace is the central area minus its scroll bars.
        let Some(layout) = self.note(app.layout()) else { return };
        let (ww, wh) = (layout.workspace.2, layout.workspace.3);
        let area = Rect::from_min_size(origin, Vec2::new(ww as f32, wh as f32) / ppp);

        if interactive {
            let before = app.scroll().ok();
            self.scroll_bars_ui(app, ui, area, central, (layout.hscroll, layout.vscroll));
            self.wheel(app, &ctx, ui.layer_id(), Rect::from_min_size(origin, Vec2::new(central.0 as f32, central.1 as f32) / ppp), popup_open);
            let moved = app.scroll().ok() != before;
            self.workspace_input(app, &ctx, ui.layer_id(), origin, area, popup_open, moved);
        }
        let Some(img) = self.note(app.render(Surface::Workspace)) else { return };
        let size = Vec2::new(img.width as f32, img.height as f32) / ppp;
        let tex = self.workspace.show(&ctx, "brushling-workspace", img);
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        ui.painter().image(tex, Rect::from_min_size(origin, size), uv, Color32::WHITE);
    }

    /// The bars the app's layout asks for, in the strips between its
    /// workspace and the central area, and the gap where they meet.
    fn scroll_bars_ui(&mut self, app: &mut App, ui: &mut egui::Ui, area: Rect, central: (u32, u32), shown: (bool, bool)) {
        let (Some(offset), Some(range)) = (self.note(app.scroll()), self.note(app.scroll_range())) else { return };
        let ppp = ui.ctx().pixels_per_point();
        let right = area.min.x + central.0 as f32 / ppp;
        let bottom = area.min.y + central.1 as f32 / ppp;
        let (view_w, view_h) = ((area.width() * ppp).round() as i32, (area.height() * ppp).round() as i32);
        let mut want = offset;
        if shown.0 {
            let rect = Rect::from_min_max(Pos2::new(area.min.x, area.max.y), Pos2::new(area.max.x, bottom));
            let axis = scrollbar::Axis { view: view_w, range: range.0, offset: offset.0 };
            if let Some(x) = scrollbar::bar(ui, egui::Id::new("brushling-hscroll"), rect, true, axis, &mut self.scroll_grab.0) {
                want.0 = x;
            }
        } else {
            self.scroll_grab.0 = None;
        }
        if shown.1 {
            let rect = Rect::from_min_max(Pos2::new(area.max.x, area.min.y), Pos2::new(right, area.max.y));
            let axis = scrollbar::Axis { view: view_h, range: range.1, offset: offset.1 };
            if let Some(y) = scrollbar::bar(ui, egui::Id::new("brushling-vscroll"), rect, false, axis, &mut self.scroll_grab.1) {
                want.1 = y;
            }
        } else {
            self.scroll_grab.1 = None;
        }
        if shown.0 && shown.1 {
            let corner = Rect::from_min_max(area.max, Pos2::new(right, bottom));
            theme::solid(ui.painter(), corner, theme::FACE);
        }
        if want != offset {
            let r = app.set_scroll(want.0, want.1);
            self.note(r);
        }
    }

    /// Mouse wheel over the workspace or its bars scrolls (Shift+wheel
    /// sideways, via egui). Ctrl+wheel is egui's zoom gesture and is ignored.
    fn wheel(&mut self, app: &mut App, ctx: &egui::Context, layer: egui::LayerId, area: Rect, popup_open: bool) {
        let (delta, hover) = ctx.input(|i| (i.smooth_scroll_delta, i.pointer.hover_pos()));
        let over = hover.is_some_and(|p| area.contains(p) && ctx.layer_id_at(p).is_none_or(|l| l == layer));
        if delta == Vec2::ZERO || !over || popup_open {
            return;
        }
        let (dx, dy) = scrollbar::wheel_step(&mut self.wheel_carry, delta, ctx.pixels_per_point());
        if (dx, dy) == (0, 0) {
            return;
        }
        let Some((x, y)) = self.note(app.scroll()) else { return };
        let r = app.set_scroll(x.saturating_add(dx), y.saturating_add(dy));
        self.note(r);
    }

    fn request_size(&mut self, app: &mut App, central: (u32, u32)) {
        let docked = |p: Panel| app.panel(p).map(|s| s.visible && s.floating_at.is_none()).unwrap_or(false);
        let bars = Bars {
            tool_box_docked: docked(Panel::ToolBox),
            color_box_docked: docked(Panel::ColorBox),
            status_bar: docked(Panel::StatusBar),
        };
        let Some(layout) = self.note(app.layout()) else { return };
        let want = input::window_size_for(central, &layout, bars);
        if self.requested_size != Some(want) {
            let r = app.set_window_size(want.0, want.1);
            self.note(r);
            self.requested_size = Some(want);
        }
    }

    fn geometry(app: &App) -> Geometry {
        let scale = app.zoom().map_or(1, |z| (z / 100).max(1)) as i64;
        let doc = app.document().map_or((0, 0), |d| (d.width, d.height));
        Geometry {
            scale,
            scroll: app.scroll().unwrap_or((0, 0)),
            doc,
            selection: app.selection().ok().flatten().map(|s| s.bounds),
            text_frame: app.text_frame().ok().flatten().map(|t| t.rect),
        }
    }

    /// Pointer events over the workspace image. `scrolled` says the view
    /// moved this frame: the pointer then rests on a different image pixel,
    /// so the app gets a move even when the mouse stood still.
    #[allow(clippy::too_many_arguments)]
    fn workspace_input(&mut self, app: &mut App, ctx: &egui::Context, layer: egui::LayerId, origin: Pos2, area: Rect, popup_open: bool, scrolled: bool) {
        let ppp = ctx.pixels_per_point();
        let (events, mods, hover) = ctx.input(|i| (i.events.clone(), input::mods(i.modifiers), i.pointer.hover_pos()));
        let px = |p: Pos2| input::to_pixel(p.x - origin.x, p.y - origin.y, ppp);
        // Over the workspace and not under a menu, window or modal.
        let free = |p: Pos2| area.contains(p) && ctx.layer_id_at(p).is_none_or(|l| l == layer);
        for ev in events {
            match ev {
                Event::PointerMoved(p) => {
                    if self.held.is_some() || free(p) {
                        self.pointer_outside = false;
                        self.send_pointer(app, PointerKind::Move, input::canvas_pos(px(p)), mods);
                    } else if !self.pointer_outside {
                        self.pointer_outside = true;
                        self.send_pointer(app, PointerKind::Move, Pos::Outside, mods);
                    }
                }
                Event::PointerButton { pos, button, pressed, modifiers } => {
                    let Some(b) = input::button(button) else { continue };
                    let m = input::mods(modifiers);
                    if pressed {
                        if self.held.is_some_and(|h| h != b) {
                            // The other button during a drag: the app cancels the stroke.
                            self.send_pointer(app, PointerKind::Press(b), input::canvas_pos(px(pos)), m);
                        } else if self.held.is_none() && !popup_open && free(pos) {
                            let at = input::press_pos(px(pos), &Self::geometry(app));
                            self.held = Some(b);
                            self.send_pointer(app, PointerKind::Press(b), at, m);
                        }
                    } else if self.held == Some(b) {
                        self.held = None;
                        self.send_pointer(app, PointerKind::Release(b), input::canvas_pos(px(pos)), m);
                    }
                }
                Event::PointerGone if self.held.is_none() && !self.pointer_outside => {
                    self.pointer_outside = true;
                    self.send_pointer(app, PointerKind::Move, Pos::Outside, mods);
                }
                _ => {}
            }
        }
        if let Some(p) = hover.filter(|&p| scrolled && (self.held.is_some() || free(p))) {
            self.pointer_outside = false;
            self.send_pointer(app, PointerKind::Move, input::canvas_pos(px(p)), mods);
        }
    }

    // ----- View Bitmap

    /// The full-screen view: the window goes full screen and any key or
    /// button press is forwarded, which ends the view.
    fn view_bitmap_ui(&mut self, app: &mut App, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::no_frame().frame(egui::Frame::NONE.fill(Color32::BLACK)).show(ui, |ui| {
            let Some(img) = self.note(app.render(Surface::FullScreen)) else { return };
            let avail = ui.available_rect_before_wrap();
            let ppp = ctx.pixels_per_point();
            let size = Vec2::new(img.width as f32, img.height as f32) / ppp;
            let fit = (avail.width() / size.x).min(avail.height() / size.y).min(1.0);
            let rect = Rect::from_center_size(avail.center(), size * fit);
            let tex = self.workspace.show(&ctx, "brushling-workspace", img);
            ui.painter().image(tex, rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        });
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            if let Event::PointerButton { button, pressed: true, modifiers, .. } = ev {
                if let Some(b) = input::button(button) {
                    let m = input::mods(modifiers);
                    self.send_pointer(app, PointerKind::Press(b), Pos::Outside, m);
                    self.send_pointer(app, PointerKind::Release(b), Pos::Outside, m);
                    return;
                }
            }
        }
    }

    // ----- keyboard

    fn forward_keys(&mut self, app: &mut App, ctx: &egui::Context) {
        let dialog_open = matches!(app.dialog(), Ok(Some(_)));
        // A focused text field (dialog, help window) keeps its typing.
        let typing = ctx.egui_wants_keyboard_input();
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match &ev {
                Event::Key { key, pressed: true, modifiers, .. } => {
                    let Some((k, m)) = input::map_key(*key, *modifiers) else { continue };
                    let dialog_key = matches!(k, Key::Escape | Key::Enter);
                    if (dialog_open && dialog_key) || (!dialog_open && !typing) {
                        self.send_key(app, k, m);
                    }
                }
                Event::Text(t) if !dialog_open && !typing => {
                    for k in input::text_keys(t) {
                        self.send_key(app, k, Mods::NONE);
                    }
                }
                _ if !dialog_open && !typing => {
                    if let Some((k, m)) = input::clipboard_shortcut(&ev) {
                        self.send_key(app, k, m);
                    }
                }
                _ => {}
            }
        }
    }

    fn thumbnail_ui(&mut self, app: &mut App, ctx: &egui::Context, out: &mut Vec<Action>) {
        if !app.panel(Panel::Thumbnail).map(|s| s.visible).unwrap_or(false) {
            return;
        }
        let Some(img) = self.note(app.render(Surface::Thumbnail)) else { return };
        let size = Vec2::new(img.width as f32, img.height as f32) / ctx.pixels_per_point();
        let tex = self.thumbnail.show(ctx, "brushling-thumbnail", img);
        let close = windows::chrome_window(ctx, egui::Id::new("brushling-thumbnail"), "Thumbnail", false, None, |ui| {
            ui.add(egui::Image::new((tex, size)));
        });
        if close {
            out.push(Action::ClosePanel(Panel::Thumbnail));
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Shell {
    /// Browser-only work at the start of a frame: finished uploads go into
    /// the app's file store and through the open File Open dialog, finished
    /// saves become downloads, and File > Exit leaves a restart screen
    /// (a page cannot close itself). `None` while the app runs; after exit,
    /// `Some(restart)` with whether the user asked to start again.
    fn web_frame(&mut self, app: &mut App, ui: &mut egui::Ui) -> Option<bool> {
        for upload in self.uploads.take() {
            let r = upload.map_err(brushling::HarnessError::Invalid).and_then(|(name, bytes)| {
                let path = format!("{}/{}", web::UPLOAD_DIR, web::file_name(&name));
                app.put_file(&path, bytes)?;
                match app.dialog()? {
                    Some(d) if d.kind == brushling::DialogKind::FileOpen => {
                        app.dialog_set("File name", &path)?;
                        app.dialog_click("Open")
                    }
                    // The dialog was closed meanwhile: the file stays in the store.
                    _ => Ok(()),
                }
            });
            self.note(r);
        }
        if let Some(saved) = self.note(app.take_saved_files()) {
            for (path, bytes) in saved {
                if let Err(e) = web::download(web::file_name(&path), &bytes) {
                    self.error = Some(format!("Download of {path} failed: {e}"));
                }
            }
        }
        let exited = matches!(app.exit_status(), Ok(Some(_)));
        web::set_unsaved(!exited && app.document().is_ok_and(|d| d.dirty));
        if !exited {
            return None;
        }
        let mut restart = false;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading("Brushling has exited.");
                ui.add_space(8.0);
                restart = theme::button(ui, "Start again", true, theme::DIALOG_BUTTON).clicked();
            });
        });
        if restart {
            // The caller drops the exited app; the next frame starts a fresh
            // one (files in the store remain).
            self.frames = 0;
            self.requested_size = None;
            self.held = None;
            self.error = None;
        }
        Some(restart)
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.themed {
            theme::apply(&ctx);
            self.themed = true;
        }
        ctx.request_repaint_after(Duration::from_millis(16));
        if self.app.is_none() && self.start_error.is_none() {
            self.start(&ctx);
        }
        let Some(mut app) = self.app.take() else {
            let msg = self.start_error.clone().unwrap_or_else(|| "Starting...".to_owned());
            egui::CentralPanel::default().show(ui, |ui| ui.label(msg));
            return;
        };
        let popup_open = egui::Popup::is_any_open(&ctx);
        self.tick(&mut app, &ctx);
        #[cfg(target_arch = "wasm32")]
        if let Some(restart) = self.web_frame(&mut app, ui) {
            if !restart {
                self.app = Some(app);
            }
            return;
        }
        self.sync_viewport(&mut app, &ctx);

        let mode = app.view_mode().unwrap_or(ViewMode::Normal);
        let bitmap = mode == ViewMode::ViewBitmap;
        if bitmap != self.fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(bitmap));
            self.fullscreen = bitmap;
        }
        if bitmap {
            self.view_bitmap_ui(&mut app, ui);
            self.forward_keys(&mut app, &ctx);
        } else {
            self.normal_ui(&mut app, ui, mode, popup_open);
            let mut out = Vec::new();
            if let Ok(Some(d)) = app.dialog() {
                windows::dialog(&ctx, &d, &mut self.dialog_edits, &mut out);
            }
            windows::help(&ctx, &app, &mut self.help_edits, &mut out);
            self.thumbnail_ui(&mut app, &ctx, &mut out);
            self.apply(&mut app, out);
            self.forward_keys(&mut app, &ctx);
        }
        self.app = Some(app);
    }
}
