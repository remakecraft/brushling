//! Plain data types of the harness surface. Everything here is complete;
//! builders do not need to change this file.

use std::fmt;
use std::path::PathBuf;

/// Error returned by every harness call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessError {
    /// The feature (spec feature id) that owns this call has not been built.
    Unbuilt(&'static str),
    /// The call is not valid in the current state (unknown menu path, no
    /// dialog open, unknown dialog field or button, coordinates for a surface
    /// that is not shown, ...).
    Invalid(String),
    /// A filesystem error, raised by the harness or by test helpers.
    Io(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::Unbuilt(feature) => write!(f, "feature {feature} is not built yet"),
            HarnessError::Invalid(msg) => write!(f, "invalid harness call: {msg}"),
            HarnessError::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<std::io::Error> for HarnessError {
    fn from(e: std::io::Error) -> Self {
        HarnessError::Io(e.to_string())
    }
}

pub type HResult<T> = Result<T, HarnessError>;

/// 8-bit-per-channel RGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const K: Rgb = Rgb::new(0, 0, 0);
    pub const W: Rgb = Rgb::new(255, 255, 255);
    pub const R: Rgb = Rgb::new(255, 0, 0);
    pub const G: Rgb = Rgb::new(0, 255, 0);
    pub const B: Rgb = Rgb::new(0, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }
}

/// Inclusive rectangle: covers columns x0..=x1 and rows y0..=y1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }
    pub const fn width(&self) -> i32 {
        self.x1 - self.x0 + 1
    }
    pub const fn height(&self) -> i32 {
        self.y1 - self.y0 + 1
    }
}

/// A row-major RGB raster: document pixels, clipboard bitmaps and rendered
/// surfaces all use this.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels, row by row from the top.
    pub pixels: Vec<Rgb>,
}

impl Image {
    /// Pixel at (x,y); `None` outside the raster.
    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        self.pixels.get(y as usize * self.width as usize + x as usize).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Right,
}

/// Modifier keys held during a pointer or key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };
    pub const SHIFT: Mods = Mods { shift: true, ctrl: false, alt: false };
    pub const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };
    pub const ALT: Mods = Mods { shift: false, ctrl: false, alt: true };
    pub const CTRL_SHIFT: Mods = Mods { shift: true, ctrl: true, alt: false };
}

/// A key press (press and release). `Char` is both typed text (inserted
/// as-is, e.g. `Char('A')`) and letter shortcuts (`Char('x')` + `Mods::CTRL`;
/// letters match shortcuts and access keys case-insensitively).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Escape,
    Enter,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F1,
    F4,
    /// Numeric keypad plus.
    NumPlus,
    /// Numeric keypad minus.
    NumMinus,
    /// The Alt key pressed and released on its own.
    Alt,
}

/// Canvas resize handles (canvas.resize_handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanvasHandle {
    /// Bottom-right corner: changes width and height.
    Corner,
    /// Right-edge midpoint: width only.
    Right,
    /// Bottom-edge midpoint: height only.
    Bottom,
}

/// The eight selection / text-frame handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Where a pointer event happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pos {
    /// Image pixel (x,y); may lie outside the canvas.
    Image(i32, i32),
    /// Screen pixel offset from the canvas's top-left screen pixel at the
    /// current zoom and scroll; converted with floor(offset / z).
    Canvas(i32, i32),
    /// Pointer over neither the canvas nor a handle (e.g. the grey workspace
    /// far from the canvas, or off the window).
    Outside,
    /// Press on a canvas resize handle. While it is held, a move or release
    /// at `Image(x,y)` sets the new width to x (if the handle changes width)
    /// and the new height to y (if it changes height), clamped to 1..=9999;
    /// i.e. the handle sits at image coordinate (W,H) / (W,H/2) / (W/2,H).
    CanvasHandle(CanvasHandle),
    /// Press on a handle of the active selection (or text frame). While it is
    /// held, a move or release at `Image(x,y)` moves the dragged edge(s) so
    /// the inclusive bound on that side becomes x (left/right edges) and/or y
    /// (top/bottom edges); the opposite side stays fixed.
    SelectionHandle(Handle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerKind {
    Press(Button),
    Move,
    Release(Button),
}

/// The 16 tools, declared in tool box row-major order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    FreeFormSelect,
    Select,
    Eraser,
    Fill,
    PickColor,
    Magnifier,
    Pencil,
    Brush,
    Airbrush,
    Text,
    Line,
    Curve,
    Rectangle,
    Polygon,
    Ellipse,
    RoundedRectangle,
}

impl Tool {
    pub const ALL: [Tool; 16] = [
        Tool::FreeFormSelect,
        Tool::Select,
        Tool::Eraser,
        Tool::Fill,
        Tool::PickColor,
        Tool::Magnifier,
        Tool::Pencil,
        Tool::Brush,
        Tool::Airbrush,
        Tool::Text,
        Tool::Line,
        Tool::Curve,
        Tool::Rectangle,
        Tool::Polygon,
        Tool::Ellipse,
        Tool::RoundedRectangle,
    ];
}

/// A click in the options box (or the equivalent setting). Values are the
/// spec's own numbers: eraser side 4/6/8/10, magnifier level 1/2/6/8,
/// brush shape 1..=12, airbrush radius 4/8/12, line width 1..=5,
/// fill style 1..=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolOption {
    /// true = opaque (upper option), false = transparent (lower option).
    Opaque(bool),
    EraserSize(u32),
    MagnifierLevel(u32),
    BrushShape(u32),
    AirbrushRadius(u32),
    LineWidth(u32),
    FillStyle(u32),
}

/// Current values of every tool option set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ToolOptions {
    pub opaque: bool,
    pub eraser_size: u32,
    /// The magnifier option shown highlighted (1, 2, 6 or 8), if any.
    pub magnifier_highlight: Option<u32>,
    pub brush_shape: u32,
    pub airbrush_radius: u32,
    pub line_width: u32,
    pub fill_style: u32,
}

/// One tool box button as the user sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolButton {
    pub tool: Tool,
    /// Tooltip name, e.g. "Eraser/Color Eraser", "Fill With Color".
    pub name: String,
    pub enabled: bool,
    /// Shown pressed (the active tool).
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorMode {
    #[default]
    Colors,
    BlackAndWhite,
}

/// Document state (core.document behaviour 2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocInfo {
    pub width: u32,
    pub height: u32,
    pub path: Option<PathBuf>,
    /// 1, 4, 8 or 24.
    pub source_depth: u32,
    pub palette: Option<Vec<Rgb>>,
    pub color_mode: ColorMode,
    pub dirty: bool,
}

/// The active selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub bounds: Rect,
    /// Row-major over `bounds` (width x height); true = selected.
    pub mask: Vec<bool>,
    /// True once the content has been lifted (moved, resized, transformed,
    /// pasted) and floats above the image.
    pub floating: bool,
}

impl Selection {
    /// Whether image pixel (x,y) is in the mask.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let b = self.bounds;
        if x < b.x0 || x > b.x1 || y < b.y0 || y > b.y1 {
            return false;
        }
        let i = (y - b.y0) as usize * b.width() as usize + (x - b.x0) as usize;
        self.mask.get(i).copied().unwrap_or(false)
    }
}

/// The active (uncommitted) text frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextFrame {
    pub rect: Rect,
    pub text: String,
}

/// One menu entry (or a menu title when listing the menu bar).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuItem {
    /// Exact label without access-key markers or shortcut, e.g. "Open...",
    /// "Set As Wallpaper (Tiled)", "1 C:\t\a.bmp". Empty for separators.
    pub label: String,
    pub separator: bool,
    pub enabled: bool,
    /// `Some(state)` for check items, `None` otherwise.
    pub checked: Option<bool>,
    /// Shortcut text shown right-aligned, e.g. "Ctrl+Shift+N", "Del", "F4".
    pub shortcut: Option<String>,
    /// The underlined access letter, as written in the label.
    pub access: Option<char>,
    /// True when the item opens a submenu (View > Zoom).
    pub submenu: bool,
}

/// What kind of modal dialog is open. See `harness` docs for each kind's
/// field names and button labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialogKind {
    /// Any message, warning or question box (errors, save-changes prompt,
    /// colour-reduction warning, enlarge-canvas question, validation
    /// messages).
    MessageBox,
    /// Platform open-file dialog (replaced by a recorder in the harness).
    FileOpen,
    /// Platform save-file dialog (replaced by a recorder in the harness).
    FileSave,
    /// Platform print dialog.
    Print,
    PageSetup,
    Attributes,
    FlipRotate,
    StretchSkew,
    CustomZoom,
    EditColors,
    About,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DialogField {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

/// The topmost open modal dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    pub kind: DialogKind,
    pub title: String,
    /// Body text (message boxes, About).
    pub text: String,
    /// Button labels in display order.
    pub buttons: Vec<String>,
    pub fields: Vec<DialogField>,
}

impl Dialog {
    /// Value of the named field, if the dialog has it.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.value.as_str())
    }
}

/// Renderable surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    /// The workspace client area (excluding scroll bars): grey background,
    /// canvas at (4,4) minus scroll, zoom, grid, marquee and handles.
    Workspace,
    /// The thumbnail window's client area (200 x 150).
    Thumbnail,
    /// The View Bitmap full-screen view (screen-sized).
    FullScreen,
    /// Image-sized, 1:1: the document as currently displayed, including a
    /// floating selection, an active text frame and live tool previews, but
    /// no marquee, handles or grid.
    Composite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    Normal,
    ViewBitmap,
    /// Print Preview; `page` is 1-based.
    PrintPreview { page: u32, pages: u32 },
}

/// Panels and secondary windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    ToolBox,
    ColorBox,
    StatusBar,
    TextToolbar,
    Thumbnail,
    /// The Help Topics window.
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PanelState {
    pub visible: bool,
    /// Screen position of a floating panel; `None` when docked (or for
    /// panels that cannot float).
    pub floating_at: Option<(i32, i32)>,
}

/// Main window geometry in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Layout {
    pub client_width: u32,
    pub client_height: u32,
    pub menu_bar_height: u32,
    pub status_bar_height: u32,
    /// Size (w,h) the tool box has when docked.
    pub tool_box_docked: (u32, u32),
    /// Size (w,h) the colour box has when docked.
    pub color_box_docked: (u32, u32),
    /// Workspace rectangle (x, y, w, h) in client coordinates.
    pub workspace: (i32, i32, u32, u32),
    pub hscroll: bool,
    pub vscroll: bool,
}

/// What the mouse pointer rests on (tooltips and status bar messages).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Hover {
    Nothing,
    ToolButton(Tool),
    /// A highlighted menu item, by menu path, e.g. "File > New".
    MenuItem(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBar {
    pub visible: bool,
    pub message: String,
    pub position: String,
    pub size: String,
}

/// System clipboard content (the clipboard service recorder).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ClipboardContent {
    #[default]
    Empty,
    Image(Image),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallpaperStyle {
    Tiled,
    Centered,
}

/// Calls recorded by the replaceable platform services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformCall {
    Print { pages: u32 },
    Mail { attachment: PathBuf },
    Wallpaper { path: PathBuf, style: WallpaperStyle },
}

/// Which optional platform services exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Services {
    pub printer: bool,
    pub mail: bool,
    pub wallpaper: bool,
}

impl Default for Services {
    fn default() -> Self {
        Services { printer: true, mail: true, wallpaper: true }
    }
}

/// How the harness starts the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartOptions {
    /// Per-user configuration folder; the app keeps its settings in the
    /// subfolder `brushling` (ui.layout_persistence).
    pub config_dir: PathBuf,
    /// Command-line arguments after the program name.
    pub args: Vec<String>,
    /// Primary screen size; the primary screen's top-left is (0,0).
    pub screen: (u32, u32),
    pub services: Services,
}

impl StartOptions {
    /// Defaults: no arguments, a 1920x1080 primary screen, all services.
    pub fn new(config_dir: impl Into<PathBuf>) -> StartOptions {
        StartOptions {
            config_dir: config_dir.into(),
            args: Vec::new(),
            screen: (1920, 1080),
            services: Services::default(),
        }
    }
}
