//! The mutable application state behind [`crate::App`]. Features add their
//! own fields here (one block per feature) and reset them in
//! [`State::replace_document`] when they hold per-document data.

use crate::dialogs::OpenDialog;
use crate::doc::{Document, DEFAULT_SIZE};
use crate::rng::Rng;
use crate::settings::Settings;
use crate::types::*;

/// Default value of every tool option set (spec: tools.toolbox and the
/// per-tool defaults).
pub fn default_tool_options() -> ToolOptions {
    ToolOptions {
        opaque: true,
        eraser_size: 8,
        magnifier_highlight: Some(1),
        brush_shape: 2,
        airbrush_radius: 4,
        line_width: 1,
        fill_style: 1,
    }
}

/// A pointer button currently held down, with where it went down.
#[derive(Debug, Clone, Copy)]
pub struct Held {
    pub button: Button,
    pub start: Pos,
    pub start_image: Option<(i32, i32)>,
    pub mods: Mods,
    /// The other button went down mid-stroke (tools.mouse_colors): the rest
    /// of the gesture, up to the release, is ignored by the tools.
    pub cancelled: bool,
}

/// The previous button press, for double-click detection.
#[derive(Debug, Clone, Copy)]
pub struct LastPress {
    pub at_ms: u64,
    pub screen: (i32, i32),
    pub button: Button,
}

#[derive(Debug)]
pub struct State {
    // ----- core.document
    pub doc: Document,
    pub fg: Rgb,
    pub bg: Rgb,
    pub tool: Tool,
    pub options: ToolOptions,
    /// Zoom in percent (owned by the view features; the magnifier option and
    /// the canvas coordinate conversion read it).
    pub zoom: u32,
    /// Canvas scroll offset in screen pixels.
    pub scroll: (i32, i32),
    /// Workspace client size in screen pixels used by `render(Workspace)`.
    pub workspace_size: (u32, u32),
    pub settings: Settings,
    pub services: Services,
    pub screen: (u32, u32),
    pub platform_calls: Vec<PlatformCall>,
    pub rng: Rng,
    /// Virtual clock in milliseconds; advances only through `advance`.
    pub now_ms: u64,
    pub held: Option<Held>,
    pub last_press: Option<LastPress>,
    /// Last pointer position over the canvas, in image coordinates.
    pub pointer_image: Option<(i32, i32)>,
    /// Open modal dialogs, bottom first; the last one is topmost.
    pub dialogs: Vec<OpenDialog>,
    pub exit_code: Option<i32>,
    /// ui.menu_bar: path of the menu opened with Alt+letter, if any.
    pub menu_open: Option<String>,
    // ----- tool.pencil (per-gesture data)
    pub pencil_last: Option<(i32, i32)>,
    pub pencil_base: Option<Vec<Rgb>>,
    // ----- tool.select (shared selection model, see selection.rs)
    pub sel: Option<crate::selection::Sel>,
    pub sel_drag: Option<crate::selection::SelDrag>,
    /// The stroke has started (set at a press on the canvas, so a pointer
    /// that left and re-enters the canvas continues it).
    pub pencil_active: bool,
    /// The stroke's undo step has been recorded (lazily, at the first pixel
    /// that lands on the canvas).
    pub pencil_pushed: bool,
    // ----- tools.mouse_colors (stroke cancel)
    pub stroke_base: Option<crate::features::mouse_colors::StrokeBase>,
    // ----- tool.pick_color
    /// The tool that was active before the current one.
    pub prev_tool: Tool,
    /// file.save: a save waiting for the colour-loss warning to be answered.
    pub pending_save: Option<(std::path::PathBuf, u32)>,
    // ----- edit.clipboard / edit.copy_to_paste_from
    /// The in-process model of the system clipboard.
    pub clipboard: ClipboardContent,
    /// A paste waiting for the "enlarge the canvas?" question to be answered.
    pub pending_paste: Option<Image>,
    /// A Copy To waiting for the colour-loss warning: target, depth, image.
    pub pending_copy_to: Option<(std::path::PathBuf, u32, Image)>,
    // ----- tool.curve / tool.polygon / tool.airbrush (per-gesture data)
    pub curve: crate::features::tool_curve::CurveState,
    pub poly: crate::features::tool_polygon::PolyState,
    pub airbrush: crate::features::tool_airbrush::AirbrushState,
}

impl State {
    pub fn new(settings: Settings, services: Services, screen: (u32, u32)) -> HResult<State> {
        let (w, h) = settings.new_size().unwrap_or(DEFAULT_SIZE);
        Ok(State {
            doc: Document::new(w, h, Rgb::W)?,
            fg: Rgb::K,
            bg: Rgb::W,
            tool: Tool::Pencil,
            options: default_tool_options(),
            zoom: 100,
            scroll: (0, 0),
            workspace_size: (800, 600),
            settings,
            services,
            screen,
            platform_calls: Vec::new(),
            rng: Rng::new(0x5EED),
            now_ms: 0,
            held: None,
            last_press: None,
            pointer_image: None,
            dialogs: Vec::new(),
            exit_code: None,
            menu_open: None,
            pencil_last: None,
            pencil_base: None,
            sel: None,
            sel_drag: None,
            pencil_active: false,
            pencil_pushed: false,
            stroke_base: None,
            prev_tool: Tool::Pencil,
            pending_save: None,
            clipboard: ClipboardContent::Empty,
            pending_paste: None,
            pending_copy_to: None,
            curve: Default::default(),
            poly: Default::default(),
            airbrush: Default::default(),
        })
    }

    /// Install a new document and drop every piece of per-document state.
    /// Features that keep per-document data (selection, text frame, repeat
    /// history, ...) reset it here.
    pub fn replace_document(&mut self, doc: Document) {
        self.doc = doc;
        self.reset_selection();
        self.held = None;
        self.last_press = None;
        self.pointer_image = None;
        self.pencil_last = None;
        self.pencil_base = None;
        self.pencil_active = false;
        self.pencil_pushed = false;
        self.stroke_base = None;
        self.curve = Default::default();
        self.poly = Default::default();
        self.airbrush = Default::default();
    }

    /// The size File > New creates: the confirmed Attributes size, else the
    /// built-in default.
    pub fn default_new_size(&self) -> (u32, u32) {
        self.settings.new_size().unwrap_or(DEFAULT_SIZE)
    }

    /// Replace the document with a clean untitled image of the default size.
    pub fn new_default_document(&mut self) -> HResult<()> {
        let (w, h) = self.default_new_size();
        let doc = Document::new(w, h, Rgb::W)?;
        self.replace_document(doc);
        Ok(())
    }

    /// Record the size later File > New commands use, and persist it.
    pub fn remember_new_size(&mut self, w: u32, h: u32) -> HResult<()> {
        self.settings.set_new_size(w, h)?;
        self.settings.save()
    }

    /// Record a call on one of the replaceable platform services.
    pub fn record_call(&mut self, call: PlatformCall) {
        self.platform_calls.push(call);
    }

    /// Option values as the user sees them; the magnifier highlight follows
    /// the current zoom.
    pub fn tool_options(&self) -> ToolOptions {
        let mut o = self.options;
        o.magnifier_highlight = match self.zoom {
            100 => Some(1),
            200 => Some(2),
            600 => Some(6),
            800 => Some(8),
            _ => None,
        };
        o
    }

    /// Screen pixels per image pixel (zoom 100% = 1, 400% = 4, ...).
    pub fn scale(&self) -> u32 {
        (self.zoom / 100).max(1)
    }

    /// Convert a pointer position to image coordinates, if it addresses the
    /// canvas.
    pub fn image_point(&self, pos: Pos) -> Option<(i32, i32)> {
        match pos {
            Pos::Image(x, y) => Some((x, y)),
            Pos::Canvas(x, y) => {
                let z = self.scale() as i32;
                Some(((x + self.scroll.0).div_euclid(z), (y + self.scroll.1).div_euclid(z)))
            }
            Pos::Outside | Pos::CanvasHandle(_) | Pos::SelectionHandle(_) => None,
        }
    }

    /// Approximate screen-pixel position of a pointer position, for the
    /// double-click distance test.
    pub fn screen_point(&self, pos: Pos) -> (i32, i32) {
        match pos {
            Pos::Image(x, y) => {
                let z = self.scale() as i64;
                ((x as i64 * z).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                 (y as i64 * z).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
            }
            Pos::Canvas(x, y) => (x, y),
            Pos::Outside | Pos::CanvasHandle(_) | Pos::SelectionHandle(_) => (i32::MIN, i32::MIN),
        }
    }
}
