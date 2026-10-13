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
    /// Every connected screen, the primary one (at 0,0) first.
    pub screens: Vec<ScreenRect>,
    pub platform_calls: Vec<PlatformCall>,
    pub rng: Rng,
    /// Virtual clock in milliseconds; advances only through `advance`.
    pub now_ms: u64,
    pub held: Option<Held>,
    pub last_press: Option<LastPress>,
    /// Last position the pointer had *over the canvas*, in image coordinates;
    /// it keeps its value when the pointer leaves (tools that need "where it
    /// was"). Contrast `ptr_image`, which is `None` off the canvas.
    pub pointer_image: Option<(i32, i32)>,
    /// Open modal dialogs, bottom first; the last one is topmost.
    pub dialogs: Vec<OpenDialog>,
    pub exit_code: Option<i32>,
    /// ui.menu_bar: path of the menu opened with Alt+letter, if any.
    pub menu_open: Option<String>,
    /// ui.menu_bar: the menu title highlighted by a lone Alt press (menu closed).
    pub menu_highlight: Option<String>,
    // ----- tool.pencil (per-gesture data)
    pub pencil_last: Option<(i32, i32)>,
    pub pencil_base: Option<Vec<Rgb>>,
    // ----- tool.eraser / tool.brush (per-gesture data)
    pub eraser_last: Option<(i32, i32)>,
    pub brush_stroke: Option<crate::features::tool_brush::Stroke>,
    /// Modifiers at the latest event of a Line / shape drag, so the preview
    /// (which only sees `&State`) constrains the way the commit will.
    pub gesture_mods: Mods,
    // ----- tool.select (shared selection model, see selection.rs)
    pub sel: Option<crate::selection::Sel>,
    pub sel_drag: Option<crate::selection::SelDrag>,
    /// selection.move_copy / selection.resize: the drag in progress on the
    /// selection itself (body move or handle resize).
    pub sel_gesture: Option<crate::selection::SelGesture>,
    /// Kind of the pointer event being routed; lets pointer hooks tell a
    /// press from a drag from a release (set by `State::pointer`).
    pub ptr_kind: Option<PointerKind>,
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
    // ----- view.grid / tool.magnifier
    /// View > Zoom > Show Grid check mark (drawn only above 200%).
    pub show_grid: bool,
    /// Image pixel under the pointer while the magnifier hovers the canvas.
    pub mag_hover: Option<(i32, i32)>,
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
    // ----- tool.text
    pub text: Option<crate::features::tool_text::TextSession>,
    pub text_drag: Option<crate::features::tool_text::TextDrag>,
    /// Text toolbar choices (font, size, B/I/U); they restyle the active frame.
    pub text_style: crate::features::tool_text::TextStyle,
    /// View > Text Toolbar check state (shown by default).
    pub text_toolbar: bool,
    // ----- help.help_about / input.escape
    /// The Help Topics window is open.
    pub help_open: bool,
    /// What the Help Topics window shows; reset each time the window opens.
    pub help: crate::features::help_help_about::HelpModel,
    /// Document and selection as they were at the press of a selection gesture (Esc restores them).
    pub esc_base: Option<crate::features::input_escape::EscBase>,
    // ----- file.save_changes_prompt
    /// The action waiting for the save-changes question to be answered.
    pub pending_action: Option<crate::features::file_save_changes_prompt::Pending>,
    /// The action to continue once the save started by the prompt succeeds.
    pub after_save: Option<crate::features::file_save_changes_prompt::Pending>,
    // ----- image.attributes / canvas.resize_handle
    /// Pixel size behind the open Attributes dialog (units only change display).
    pub attr_draft: crate::features::image_attributes::AttrDraft,
    /// A canvas resize-handle drag (live: the document already shows the
    /// size a release would keep). Only active while `held` is set.
    pub canvas_drag: Option<crate::features::canvas_resize_handle::CanvasDrag>,
    // ----- colors.color_box / colors.edit_colors (session state, kept across File > New)
    /// The 28 swatches; Edit Colors redefines them.
    pub swatches: [Rgb; crate::features::color_box::SWATCH_COUNT],
    /// Index (0-based) of the swatch most recently clicked, the one Edit Colors edits.
    pub last_swatch: usize,
    /// The 16 custom colours of the Edit Colors dialog.
    pub custom_colors: [Rgb; 16],
    /// Slot (0-based) the next "Add to Custom Colors" fills.
    pub custom_next: usize,
    // ----- ui.main_window / ui.status_bar / tools.toolbox
    /// Requested client size of the main window (clamped to the minimum on read).
    pub window: (u32, u32),
    /// Top-left of the main window on the virtual desktop (remembered; the
    /// headless window is only moved through `set_window_pos`).
    pub window_pos: (i32, i32),
    /// Where the pointer rests: a tool button, a highlighted menu item or nothing.
    pub hover: Hover,
    /// Virtual time at which `hover` last changed (tooltip delay).
    pub hover_since: u64,
    /// Current pointer position in image coordinates, `None` when it is off
    /// the canvas (unlike `pointer_image`, which remembers the last on-canvas
    /// point), and its modifiers, for the status bar.
    pub ptr_image: Option<(i32, i32)>,
    pub ptr_mods: Mods,
    // ----- file.print
    /// Page Setup choices; they last for the session only.
    pub page_setup: crate::features::file_print::PageSetup,
    /// Print Preview: the 1-based current page while the preview is shown.
    pub print_preview: Option<u32>,
    /// Index into the preview's magnification steps.
    pub preview_zoom: usize,
    // ----- view.panels / view.thumbnail / view.view_bitmap
    /// View toggles and floating positions of the panels and the thumbnail window.
    pub panels: crate::features::view_panels::Panels,
    /// The full-screen View Bitmap view is up.
    pub view_bitmap: bool,
}

impl State {
    pub fn new(settings: Settings, services: Services, screen: (u32, u32)) -> HResult<State> {
        State::with_screens(settings, services, screen, &[])
    }

    /// Like [`State::new`] on a machine that also has the `extra` screens.
    pub fn with_screens(settings: Settings, services: Services, screen: (u32, u32), extra: &[ScreenRect]) -> HResult<State> {
        let (w, h) = settings.new_size().unwrap_or(DEFAULT_SIZE);
        let mut st = State {
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
            screens: std::iter::once(ScreenRect { x: 0, y: 0, width: screen.0, height: screen.1 })
                .chain(extra.iter().copied())
                .collect(),
            platform_calls: Vec::new(),
            rng: Rng::new(0x5EED),
            now_ms: 0,
            held: None,
            last_press: None,
            pointer_image: None,
            dialogs: Vec::new(),
            exit_code: None,
            menu_open: None,
            menu_highlight: None,
            pencil_last: None,
            pencil_base: None,
            eraser_last: None,
            brush_stroke: None,
            gesture_mods: Mods::NONE,
            sel: None,
            sel_drag: None,
            sel_gesture: None,
            ptr_kind: None,
            pencil_active: false,
            pencil_pushed: false,
            stroke_base: None,
            prev_tool: Tool::Pencil,
            show_grid: false,
            mag_hover: None,
            pending_save: None,
            clipboard: ClipboardContent::Empty,
            pending_paste: None,
            pending_copy_to: None,
            curve: Default::default(),
            poly: Default::default(),
            airbrush: Default::default(),
            text: None,
            text_drag: None,
            text_style: Default::default(),
            text_toolbar: true,
            help_open: false,
            help: Default::default(),
            esc_base: None,
            pending_action: None,
            after_save: None,
            attr_draft: Default::default(),
            canvas_drag: None,
            swatches: crate::features::color_box::DEFAULT_SWATCHES,
            last_swatch: 0,
            custom_colors: [Rgb::W; 16],
            custom_next: 0,
            window: crate::features::ui_main_window::default_window(),
            window_pos: (0, 0),
            hover: Hover::Nothing,
            hover_since: 0,
            ptr_image: None,
            ptr_mods: Mods::default(),
            page_setup: Default::default(),
            print_preview: None,
            preview_zoom: 0,
            panels: crate::features::view_panels::Panels::new(screen),
            view_bitmap: false,
        };
        crate::features::ui_layout_persistence::restore(&mut st);
        Ok(st)
    }

    /// Install a new document and drop every piece of per-document state.
    /// Features that keep per-document data (selection, text frame, repeat
    /// history, ...) reset it here.
    pub fn replace_document(&mut self, doc: Document) {
        self.doc = doc;
        self.reset_selection();
        self.text = None;
        self.text_drag = None;
        self.held = None;
        self.last_press = None;
        self.pointer_image = None;
        self.pencil_last = None;
        self.pencil_base = None;
        self.eraser_last = None;
        self.brush_stroke = None;
        self.gesture_mods = Mods::NONE;
        self.pencil_active = false;
        self.pencil_pushed = false;
        self.stroke_base = None;
        self.curve = Default::default();
        self.poly = Default::default();
        self.airbrush = Default::default();
        self.canvas_drag = None;
        self.mag_hover = None;
        self.print_preview = None;
        crate::features::ui_main_window::refresh_workspace(self);
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

    /// The background colour as a paint operation writes it: snapped to the
    /// black-and-white or palette rule of the document (colors.indexed_snap).
    /// Fills use this; moved or copied pixels stay raw.
    pub fn bg_snapped(&self) -> Rgb {
        self.doc.snap(self.bg)
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
