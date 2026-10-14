//! The late-90s desktop look: a light grey face, navy highlights and the
//! bevelled 3D edges that egui's frames cannot draw on their own. Everything
//! here only paints; sizes are in "units", one whole number of physical
//! pixels each (see [`unit`]), so edges and pixel art stay crisp at any
//! display scale.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Margin, Painter, Pos2, Rect, Sense, Shape, Stroke, Style, TextStyle, Ui, Vec2};

pub const FACE: Color32 = Color32::from_gray(192);
pub const LIGHT: Color32 = Color32::from_gray(223);
pub const WHITE: Color32 = Color32::WHITE;
pub const SHADOW: Color32 = Color32::from_gray(128);
pub const BLACK: Color32 = Color32::BLACK;
pub const NAVY: Color32 = Color32::from_rgb(0, 0, 128);
/// The light end of an active title bar's gradient.
const TITLE_END: Color32 = Color32::from_rgb(16, 132, 208);
const TITLE_INACTIVE: Color32 = Color32::from_gray(128);
const TITLE_INACTIVE_END: Color32 = Color32::from_gray(181);
const ERROR: Color32 = Color32::from_rgb(128, 0, 0);

/// Height of a window title bar, in units.
pub const TITLE_HEIGHT: f32 = 18.0;

/// Install the style for both egui themes and pin the light one, so the
/// system's dark mode never shows through.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    let style = style();
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn widget(bg: Color32, stroke: Color32, fg: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius: CornerRadius::ZERO,
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    }
}

pub fn style() -> Style {
    let mut s = Style { visuals: egui::Visuals::light(), ..Style::default() };
    s.text_styles = [
        (TextStyle::Small, FontId::proportional(10.0)),
        (TextStyle::Body, FontId::proportional(12.0)),
        (TextStyle::Button, FontId::proportional(12.0)),
        (TextStyle::Heading, FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(11.0)),
    ]
    .into();

    let sp = &mut s.spacing;
    sp.item_spacing = Vec2::new(4.0, 3.0);
    sp.button_padding = Vec2::new(6.0, 2.0);
    sp.interact_size = Vec2::new(20.0, 18.0);
    sp.menu_margin = Margin::same(3);
    sp.window_margin = Margin::same(4);
    sp.indent = 12.0;
    sp.scroll = egui::style::ScrollStyle::solid();

    let v = &mut s.visuals;
    v.dark_mode = false;
    v.text_options.subpixel_binning = false;
    v.widgets.noninteractive = widget(FACE, SHADOW, BLACK);
    v.widgets.inactive = widget(FACE, SHADOW, BLACK);
    v.widgets.hovered = widget(FACE, BLACK, BLACK);
    v.widgets.active = widget(LIGHT, BLACK, BLACK);
    v.widgets.open = widget(FACE, SHADOW, BLACK);
    v.selection = egui::style::Selection { bg_fill: NAVY, stroke: Stroke::new(1.0, WHITE) };
    v.hyperlink_color = NAVY;
    v.faint_bg_color = LIGHT;
    v.extreme_bg_color = WHITE;
    v.text_edit_bg_color = Some(WHITE);
    v.code_bg_color = WHITE;
    v.error_fg_color = ERROR;
    v.warn_fg_color = ERROR;
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.window_fill = FACE;
    v.panel_fill = FACE;
    v.window_stroke = Stroke::new(1.0, BLACK);
    v.window_highlight_topmost = false;
    v.text_cursor.stroke = Stroke::new(1.0, BLACK);
    v.text_cursor.blink = true;
    s
}

/// Menus: the entry under the pointer, or the open one, turns navy with
/// white text.
pub fn menu_style(s: &mut Style) {
    egui::containers::menu::menu_style(s);
    s.spacing.button_padding = Vec2::new(8.0, 2.0);
    for w in [&mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active, &mut s.visuals.widgets.open] {
        w.weak_bg_fill = NAVY;
        w.bg_fill = NAVY;
        w.fg_stroke = Stroke::new(1.0, WHITE);
    }
}

pub fn menu_config() -> egui::containers::menu::MenuConfig {
    egui::containers::menu::MenuConfig::new().style(menu_style)
}

/// A list where the entry under the pointer turns navy (combo box lists).
pub fn list_style(s: &mut Style) {
    s.visuals.widgets.hovered.weak_bg_fill = NAVY;
    s.visuals.widgets.hovered.bg_fill = NAVY;
    s.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    s.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, WHITE);
}

// ----- units and snapping

/// Points per unit: one unit is the whole number of physical pixels
/// nearest to one point.
pub fn unit(ppp: f32) -> f32 {
    ppp.round().max(1.0) / ppp
}

/// `rect` with its edges moved to the nearest physical pixel boundaries.
pub fn snap(rect: Rect, ppp: f32) -> Rect {
    let r = |v: f32| (v * ppp).round() / ppp;
    Rect::from_min_max(Pos2::new(r(rect.min.x), r(rect.min.y)), Pos2::new(r(rect.max.x), r(rect.max.y)))
}

pub fn snap_pos(p: Pos2, ppp: f32) -> Pos2 {
    Pos2::new((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp)
}

/// Unit size and pixel ratio of the painter's context.
pub fn metrics(ui: &Ui) -> (f32, f32) {
    let ppp = ui.ctx().pixels_per_point();
    (unit(ppp), ppp)
}

// ----- bevels

/// The edge styles of the classic 3D look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bevel {
    /// A button at rest.
    Raised,
    /// A button held down, or a toggle that is on.
    Pressed,
    /// A text field, colour well or the workspace edge.
    Sunken,
    /// The outer edge of a window, dialog or menu.
    Window,
    /// One-pixel sunken: status bar panes, the tool options box.
    ThinSunken,
}

impl Bevel {
    /// (top-left, bottom-right) colour of each ring, outermost first.
    pub fn rings(self) -> &'static [(Color32, Color32)] {
        match self {
            Bevel::Raised => &[(WHITE, BLACK), (LIGHT, SHADOW)],
            Bevel::Pressed => &[(BLACK, WHITE), (SHADOW, LIGHT)],
            Bevel::Sunken => &[(SHADOW, WHITE), (BLACK, LIGHT)],
            Bevel::Window => &[(LIGHT, BLACK), (WHITE, SHADOW)],
            Bevel::ThinSunken => &[(SHADOW, WHITE)],
        }
    }

    /// Width of the edge in units.
    pub fn width(self) -> f32 {
        self.rings().len() as f32
    }
}

/// A one-unit ring just inside `r`. The bottom-right colour owns the
/// top-right and bottom-left corner pixels, as on the real thing.
/// Fill `rect` exactly, as a plain mesh. egui's own rect fill feathers
/// edges and turns thin rects into lines, which loses single-pixel dots.
pub fn solid(p: &Painter, rect: Rect, c: Color32) {
    if rect.is_positive() {
        let mut mesh = egui::Mesh::default();
        mesh.add_colored_rect(rect, c);
        p.add(mesh);
    }
}

fn ring(p: &Painter, r: Rect, tl: Color32, br: Color32, u: f32) {
    let fill = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        solid(p, Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)), c);
    };
    fill(r.min.x, r.min.y, r.max.x - u, r.min.y + u, tl);
    fill(r.min.x, r.min.y + u, r.min.x + u, r.max.y - u, tl);
    fill(r.min.x, r.max.y - u, r.max.x, r.max.y, br);
    fill(r.max.x - u, r.min.y, r.max.x, r.max.y - u, br);
}

/// Paint the edges of `rect` (already snapped) in style `b`.
pub fn bevel(p: &Painter, rect: Rect, b: Bevel, u: f32) {
    let mut r = rect;
    for &(tl, br) in b.rings() {
        if r.width() < 2.0 * u || r.height() < 2.0 * u {
            return;
        }
        ring(p, r, tl, br, u);
        r = r.shrink(u);
    }
}

/// `rect` filled with `fill` and bevelled; returns the area inside the edge.
pub fn panel(p: &Painter, rect: Rect, b: Bevel, fill: Color32, u: f32) -> Rect {
    solid(p, rect, fill);
    bevel(p, rect, b, u);
    rect.shrink(b.width() * u)
}

/// The engraved horizontal line between menu groups.
pub fn etched_hline(p: &Painter, x0: f32, x1: f32, y: f32, u: f32) {
    solid(p, Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + u)), SHADOW);
    solid(p, Rect::from_min_max(Pos2::new(x0, y + u), Pos2::new(x1, y + 2.0 * u)), WHITE);
}

pub fn etched_separator(ui: &mut Ui) {
    let (u, ppp) = metrics(ui);
    // Zero wide while egui measures a new menu, or the line would claim all
    // the room it is offered and the menu would grow to fit it.
    let width = if ui.is_sizing_pass() { 0.0 } else { ui.available_width() };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0 * u), Sense::hover());
    let y = snap_pos(Pos2::new(0.0, rect.center().y - u), ppp).y;
    etched_hline(ui.painter(), rect.min.x, rect.max.x, y, u);
}

// ----- pixel art

/// Paints in whole units from a pixel-aligned origin.
pub struct Pixels<'a> {
    pub painter: &'a Painter,
    pub origin: Pos2,
    pub u: f32,
}

impl Pixels<'_> {
    pub fn fill(&self, x: i32, y: i32, w: i32, h: i32, c: Color32) {
        let min = self.origin + Vec2::new(x as f32, y as f32) * self.u;
        solid(self.painter, Rect::from_min_size(min, Vec2::new(w as f32, h as f32) * self.u), c);
    }

    pub fn dot(&self, x: i32, y: i32, c: Color32) {
        self.fill(x, y, 1, 1, c);
    }
}

/// The 50% white-and-face checkerboard of scroll tracks and pressed
/// toggles, as a checkered texture shared through egui's memory. It is
/// tiled by hand rather than with a repeating sampler, which not every
/// renderer honours.
pub fn dither(ctx: &egui::Context) -> egui::TextureId {
    let id = egui::Id::new("brushling-dither");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t.id();
    }
    let n = DITHER_TILE as usize;
    let pixels = (0..n * n).map(|i| if (i % n + i / n).is_multiple_of(2) { WHITE } else { FACE }).collect();
    let img = egui::ColorImage::new([n, n], pixels);
    let t = ctx.load_texture("brushling-dither", img, egui::TextureOptions::NEAREST);
    let tid = t.id();
    ctx.data_mut(|d| d.insert_temp(id, t));
    tid
}

/// Side of the dither texture, in texels (units).
const DITHER_TILE: f32 = 64.0;

/// The tiles covering `size` units: (offset, size) pairs along one axis.
fn dither_tiles(len: f32) -> impl Iterator<Item = (f32, f32)> {
    let whole = (len / DITHER_TILE).ceil().max(0.0) as usize;
    (0..whole).map(move |i| {
        let at = i as f32 * DITHER_TILE;
        (at, (len - at).min(DITHER_TILE))
    })
}

/// Fill a snapped `rect` with the dither, one texel per unit.
pub fn dither_fill(ui: &Ui, rect: Rect) {
    let (u, _) = metrics(ui);
    let tex = dither(ui.ctx());
    let size = rect.size() / u;
    for (x, w) in dither_tiles(size.x) {
        for (y, h) in dither_tiles(size.y) {
            let tile = Rect::from_min_size(rect.min + Vec2::new(x, y) * u, Vec2::new(w, h) * u);
            let uv = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h) / DITHER_TILE);
            ui.painter().image(tex, tile, uv, Color32::WHITE);
        }
    }
}

// ----- text

pub fn font() -> FontId {
    FontId::proportional(12.0)
}

/// Text at `pos`; disabled text is engraved (white under grey) as on the
/// real thing.
pub fn text(p: &Painter, pos: Pos2, align: Align2, s: &str, color: Color32, enabled: bool, u: f32) -> Rect {
    if enabled {
        return p.text(pos, align, s, font(), color);
    }
    p.text(pos + Vec2::splat(u), align, s, font(), WHITE);
    p.text(pos, align, s, font(), SHADOW)
}

/// Bold-looking text from the regular face: drawn twice, one unit apart.
pub fn bold_text(p: &Painter, pos: Pos2, align: Align2, s: &str, color: Color32, u: f32) {
    p.text(pos, align, s, font(), color);
    p.text(pos + Vec2::new(u, 0.0), align, s, font(), color);
}

/// The dotted focus rectangle inside a focused button.
pub fn focus_rect(p: &Painter, r: Rect, u: f32) {
    let (nx, ny) = ((r.width() / u) as i32, (r.height() / u) as i32);
    let px = Pixels { painter: p, origin: r.min, u };
    for i in (0..nx).step_by(2) {
        px.dot(i, 0, BLACK);
        px.dot(i, ny - 1, BLACK);
    }
    for j in (0..ny).step_by(2) {
        px.dot(0, j, BLACK);
        px.dot(nx - 1, j, BLACK);
    }
}

// ----- widgets

/// Minimum size of a dialog push button, in units.
pub const DIALOG_BUTTON: Vec2 = Vec2::new(75.0, 23.0);

/// A bevelled push button at least `min` units big.
pub fn button(ui: &mut Ui, label: &str, enabled: bool, min: Vec2) -> egui::Response {
    let (u, ppp) = metrics(ui);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font(), BLACK);
    let size = Vec2::new((galley.size().x + 16.0 * u).max(min.x * u), (galley.size().y + 6.0 * u).max(min.y * u));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, r) = ui.allocate_exact_size(size, sense);
    let r = r.on_hover_cursor(egui::CursorIcon::Default);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let rect = snap(rect, ppp);
        let down = enabled && r.is_pointer_button_down_on() && r.hovered();
        let p = ui.painter();
        let inner = panel(p, rect, if down { Bevel::Pressed } else { Bevel::Raised }, FACE, u);
        let shift = if down { Vec2::splat(u) } else { Vec2::ZERO };
        text(p, snap_pos(inner.center() + shift, ppp), Align2::CENTER_CENTER, label, BLACK, enabled, u);
        if r.has_focus() {
            focus_rect(p, inner.shrink(u), u);
        }
    }
    r
}

/// A square title-bar button (close) with a pixel-art cross.
pub fn close_button(ui: &mut Ui, rect: Rect, id: egui::Id) -> egui::Response {
    let (u, ppp) = metrics(ui);
    let r = ui.interact(rect, id, Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close"));
    let rect = snap(rect, ppp);
    let down = r.is_pointer_button_down_on() && r.hovered();
    let p = ui.painter();
    let inner = panel(p, rect, if down { Bevel::Pressed } else { Bevel::Raised }, FACE, u);
    let off = if down { 1 } else { 0 };
    let size = (inner.width() / u) as i32;
    let x0 = (size - 8) / 2 + off;
    let y0 = ((inner.height() / u) as i32 - 7) / 2 + off;
    let px = Pixels { painter: p, origin: inner.min, u };
    for i in 0..7 {
        // Two pixels wide, like the bold cross of the era.
        px.fill(x0 + i, y0 + i, 2, 1, BLACK);
        px.fill(x0 + 6 - i, y0 + i, 2, 1, BLACK);
    }
    r
}

/// Paint a title bar across `rect` (Win98's navy-to-blue gradient when
/// `active`) with the title in bold white.
pub fn title_bar(p: &Painter, rect: Rect, title: &str, active: bool, u: f32) {
    let (a, b) = if active { (NAVY, TITLE_END) } else { (TITLE_INACTIVE, TITLE_INACTIVE_END) };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), b);
    mesh.colored_vertex(rect.left_bottom(), a);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
    let color = if active { WHITE } else { FACE };
    let clip = p.with_clip_rect(rect.intersect(p.clip_rect()));
    bold_text(&clip, Pos2::new(rect.min.x + 3.0 * u, rect.center().y), Align2::LEFT_CENTER, title, color, u);
}

/// A window's chrome inside its content `ui`: reserve the title bar at the
/// top now, and call [`WindowChrome::finish`] after the content to paint it
/// across the content's width and handle the close button.
pub struct WindowChrome {
    top: Pos2,
}

impl WindowChrome {
    pub fn begin(ui: &mut Ui) -> WindowChrome {
        let (u, ppp) = metrics(ui);
        let top = snap_pos(ui.cursor().min, ppp);
        ui.add_space(TITLE_HEIGHT * u + 2.0 * u);
        WindowChrome { top }
    }

    /// Paints the bar; returns true when its close button (shown when
    /// `closable`) was clicked.
    pub fn finish(self, ui: &mut Ui, title: &str, active: bool, closable: bool, id: egui::Id) -> bool {
        let (u, ppp) = metrics(ui);
        let right = snap_pos(Pos2::new(ui.min_rect().max.x, 0.0), ppp).x;
        let rect = Rect::from_min_max(self.top, Pos2::new(right, self.top.y + TITLE_HEIGHT * u));
        title_bar(ui.painter(), rect, title, active, u);
        if !closable {
            return false;
        }
        // 16x14 units, inset two units from the bar's top-right corner.
        let close = Rect::from_min_size(
            Pos2::new(rect.max.x - 18.0 * u, rect.min.y + 2.0 * u),
            Vec2::new(16.0 * u, TITLE_HEIGHT * u - 4.0 * u),
        );
        close_button(ui, close, id.with("close")).clicked()
    }
}

/// Paint `b`-style edges on `rect` in the topmost painter of `layer`.
pub fn frame_edges(ctx: &egui::Context, layer: egui::LayerId, rect: Rect, b: Bevel) {
    let ppp = ctx.pixels_per_point();
    bevel(&ctx.layer_painter(layer), snap(rect, ppp), b, unit(ppp));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dither_tiles_cover_the_length_exactly() {
        let tiles: Vec<_> = dither_tiles(150.0).collect();
        assert_eq!(tiles, vec![(0.0, 64.0), (64.0, 64.0), (128.0, 22.0)]);
        assert_eq!(dither_tiles(64.0).count(), 1);
        assert_eq!(dither_tiles(0.0).count(), 0);
    }

    #[test]
    fn units_are_whole_physical_pixels() {
        for ppp in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
            let px = unit(ppp) * ppp;
            assert_eq!(px, px.round(), "ppp {ppp}");
            assert!(px >= 1.0);
        }
        assert_eq!(unit(1.0), 1.0);
        assert_eq!(unit(2.0), 1.0);
        assert_eq!(unit(0.5), 2.0);
    }

    #[test]
    fn snapped_rects_sit_on_pixel_boundaries() {
        let r = snap(Rect::from_min_max(Pos2::new(10.3, 4.9), Pos2::new(30.62, 9.1)), 1.5);
        for v in [r.min.x, r.min.y, r.max.x, r.max.y] {
            assert!(((v * 1.5) - (v * 1.5).round()).abs() < 1e-4);
        }
    }

    #[test]
    fn bevels_light_from_the_top_left() {
        // Raised: the outer top-left edge is the lightest, bottom-right the darkest.
        assert_eq!(Bevel::Raised.rings()[0], (WHITE, BLACK));
        // Pressed and sunken invert it.
        assert_eq!(Bevel::Pressed.rings()[0].0, BLACK);
        assert_eq!(Bevel::Sunken.rings()[0], (SHADOW, WHITE));
        assert_eq!(Bevel::ThinSunken.width(), 1.0);
        assert_eq!(Bevel::Window.width(), 2.0);
    }

    #[test]
    fn style_is_square_and_light() {
        let s = style();
        assert!(!s.visuals.dark_mode);
        assert_eq!(s.visuals.window_corner_radius, CornerRadius::ZERO);
        assert_eq!(s.visuals.menu_corner_radius, CornerRadius::ZERO);
        let w = &s.visuals.widgets;
        for v in [&w.noninteractive, &w.inactive, &w.hovered, &w.active, &w.open] {
            assert_eq!(v.corner_radius, CornerRadius::ZERO);
        }
        assert_eq!(s.visuals.panel_fill, FACE);
        assert_eq!(s.visuals.selection.bg_fill, NAVY);
    }
}
