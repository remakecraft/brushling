//! The headless application handle used by the GUI shell and by the tests.
//!
//! Every method below starts as a stub returning
//! `Err(HarnessError::Unbuilt("<owning feature id>"))`. Builders replace the
//! body (and add private state to [`App`]); signatures are frozen.
//!
//! # Events
//! - `pointer(kind, pos, mods)` drives every canvas tool. "click P" is
//!   `Press(Left)` + `Release(Left)` at P; "drag A->B" is a press at A, one
//!   `Move` to B and a release at B. A `Move` with no button held only moves
//!   the pointer (status bar position, magnifier rectangle, hover).
//! - Two presses within 500 ms of virtual time and 4 screen pixels form a
//!   double-click. Virtual time only advances through `advance(ms)`.
//! - `key(key, mods)` is a full key press. While a modal dialog is open keys
//!   go to the dialog (Esc = Cancel) and shortcuts are ignored.
//!
//! # Menus
//! Menu paths join exact labels with `" > "`: `"File > Open..."`,
//! `"View > Zoom > Large Size"`, `"Image > Flip/Rotate..."`,
//! `"Colors > Edit Colors..."`, recent files `"File > 1 C:\t\a.bmp"`.
//! `menu_items("")` lists the menu bar titles. Invoking an item behaves like
//! clicking it (including opening its dialog).
//!
//! # Dialogs
//! `dialog()` returns the topmost modal dialog. `dialog_set(field, value)`
//! types into a text field or picks a choice by its label; `dialog_click`
//! presses a button by label. Contracts (field names / buttons):
//! - MessageBox: `title`, `text`, buttons e.g. `OK` | `Yes No Cancel` |
//!   `OK Cancel`. Validation messages open as a MessageBox above their
//!   dialog; after `OK` the dialog underneath is open again.
//! - FileOpen: fields `File name` (full path), `Files of type`
//!   (`Bitmap files (*.bmp;*.dib)` | `All files (*.*)`); buttons `Open`, `Cancel`.
//! - FileSave: fields `File name` (full path or bare name), `Save as type`
//!   (`Monochrome bitmap (1-bit)` | `16-color bitmap (4-bit)` |
//!   `256-color bitmap (8-bit)` | `24-bit bitmap`); buttons `Save`, `Cancel`.
//! - Print: buttons `OK`, `Cancel`.
//! - PageSetup: fields `Paper` (`Letter`), `Orientation` (`Portrait` |
//!   `Landscape`), `Left`, `Right`, `Top`, `Bottom` (inches, decimal text);
//!   buttons `OK`, `Cancel`.
//! - Attributes: fields `Width`, `Height` (integers in Pixels, two decimals
//!   in Inches/Cm), `Units` (`Inches` | `Cm` | `Pixels`), `Colors`
//!   (`Black and white` | `Colors`); buttons `OK`, `Cancel`, `Default`.
//! - FlipRotate: fields `Action` (`Flip horizontal` | `Flip vertical` |
//!   `Rotate by angle`), `Angle` (`90` | `180` | `270`); buttons `OK`, `Cancel`.
//! - StretchSkew: fields `Stretch Horizontal`, `Stretch Vertical`,
//!   `Skew Horizontal`, `Skew Vertical`; buttons `OK`, `Cancel`.
//! - CustomZoom: field `Zoom` (`100%` | `200%` | `400%` | `600%` | `800%`);
//!   buttons `OK`, `Cancel`.
//! - EditColors: fields `Red`, `Green`, `Blue` (0-255), `Hue`, `Sat`, `Lum`
//!   (0-240), `Custom 1` .. `Custom 16` (value `r,g,b`); buttons
//!   `Define Custom Colors >>`, `Add to Custom Colors`, `OK`, `Cancel`.
//! - About: `text` holds name, version and licence; button `OK`.
//!
//! # Direct state setters
//! `new_document`, `set_pixel` and `set_colors` are harness conveniences:
//! they create no undo step and do not set the dirty flag. `set_colors`
//! otherwise behaves like choosing the colours in the colour box (an active
//! text frame is recoloured).

use crate::doc::Document;
use crate::settings::Settings;
use crate::state::State;
use crate::types::*;

/// One running application instance with its replaceable platform services
/// (clipboard, file dialogs, printer, mail, wallpaper, settings store).
#[derive(Debug)]
pub struct App {
    pub(crate) st: State,
}

impl App {
    // ----- core.document: construction, pixels, events, menus, dialogs, clock

    /// Start the application (startup behaviour of core.document: untitled
    /// default-size image, or the file named by `args[0]`).
    pub fn start(opts: StartOptions) -> HResult<App> {
        let settings = Settings::load(&opts.config_dir);
        let mut st = State::new(settings, opts.services, opts.screen)?;
        if let Some(arg) = opts.args.first() {
            st.open_path(std::path::Path::new(arg))?;
        }
        Ok(App { st })
    }

    /// Replace the document with a clean, untitled `w` x `h` image filled
    /// with `fill` (no prompt, history cleared, colours/tools unchanged).
    pub fn new_document(&mut self, w: u32, h: u32, fill: Rgb) -> HResult<()> {
        let doc = Document::new(w, h, fill)?;
        self.st.replace_document(doc);
        Ok(())
    }

    /// Set one committed document pixel directly.
    pub fn set_pixel(&mut self, x: i32, y: i32, c: Rgb) -> HResult<()> {
        if self.st.doc.put(x, y, c) {
            Ok(())
        } else {
            Err(HarnessError::Invalid(format!("pixel ({x},{y}) is outside the document")))
        }
    }

    /// One committed document pixel (excludes floating content and previews).
    pub fn pixel(&self, x: i32, y: i32) -> HResult<Rgb> {
        self.st
            .doc
            .get(x, y)
            .ok_or_else(|| HarnessError::Invalid(format!("pixel ({x},{y}) is outside the document")))
    }

    /// All committed document pixels.
    pub fn pixels(&self) -> HResult<Image> {
        Ok(self.st.doc.image())
    }

    pub fn document(&self) -> HResult<DocInfo> {
        Ok(self.st.doc.info())
    }

    /// Set foreground and background colours.
    pub fn set_colors(&mut self, fg: Rgb, bg: Rgb) -> HResult<()> {
        self.st.fg = fg;
        self.st.bg = bg;
        Ok(())
    }

    /// (foreground, background).
    pub fn colors(&self) -> HResult<(Rgb, Rgb)> {
        Ok((self.st.fg, self.st.bg))
    }

    /// Click a tool box button.
    pub fn select_tool(&mut self, tool: Tool) -> HResult<()> {
        crate::features::tools_toolbox::click(&mut self.st, tool)
    }

    pub fn active_tool(&self) -> HResult<Tool> {
        Ok(self.st.tool)
    }

    /// Click an option in the options box.
    pub fn set_tool_option(&mut self, option: ToolOption) -> HResult<()> {
        self.st.set_tool_option(option)
    }

    pub fn tool_options(&self) -> HResult<ToolOptions> {
        Ok(self.st.tool_options())
    }

    /// A pointer event (see module docs).
    pub fn pointer(&mut self, kind: PointerKind, pos: Pos, mods: Mods) -> HResult<()> {
        self.st.pointer(kind, pos, mods)
    }

    /// A key press (see module docs).
    pub fn key(&mut self, key: Key, mods: Mods) -> HResult<()> {
        self.st.key(key, mods)
    }

    /// Advance the virtual clock (airbrush ticks, double-click and tooltip
    /// timing).
    pub fn advance(&mut self, ms: u64) -> HResult<()> {
        self.st.advance(ms)
    }

    /// Seed the random generator (airbrush).
    pub fn seed(&mut self, seed: u64) -> HResult<()> {
        self.st.rng.reseed(seed);
        Ok(())
    }

    /// Invoke a menu command by path (see module docs).
    pub fn menu(&mut self, path: &str) -> HResult<()> {
        self.st.menu(path)
    }

    /// The topmost modal dialog, if any.
    pub fn dialog(&self) -> HResult<Option<Dialog>> {
        Ok(self.st.top_dialog().cloned())
    }

    /// Set a dialog field (text) or choice (by label).
    pub fn dialog_set(&mut self, field: &str, value: &str) -> HResult<()> {
        self.st.dialog_set(field, value)
    }

    /// Press a dialog button by label.
    pub fn dialog_click(&mut self, button: &str) -> HResult<()> {
        self.st.dialog_click(button)
    }

    /// Calls recorded by the printer, mail and wallpaper services, in order.
    pub fn platform_calls(&self) -> HResult<Vec<PlatformCall>> {
        Ok(self.st.platform_calls.clone())
    }

    /// Render a surface to pixels.
    pub fn render(&self, surface: Surface) -> HResult<Image> {
        self.st.render(surface)
    }

    /// `Some(exit code)` once the application has exited.
    pub fn exit_status(&self) -> HResult<Option<i32>> {
        Ok(self.st.exit_code)
    }

    // ----- ui.main_window

    /// Window title text.
    pub fn title(&self) -> HResult<String> {
        Ok(crate::features::ui_main_window::title(&self.st))
    }

    /// Request a client-area size (subject to the minimum size).
    pub fn set_window_size(&mut self, w: u32, h: u32) -> HResult<()> {
        crate::features::ui_main_window::set_window_size(&mut self.st, w, h);
        Ok(())
    }

    pub fn layout(&self) -> HResult<Layout> {
        Ok(crate::features::ui_main_window::layout(&self.st))
    }

    // ----- ui.menu_bar

    /// Entries of the menu at `path` ("" = menu bar titles), separators
    /// included.
    pub fn menu_items(&self, path: &str) -> HResult<Vec<MenuItem>> {
        self.st.menu_listing(path)
    }

    /// One entry by full path.
    pub fn menu_item(&self, path: &str) -> HResult<MenuItem> {
        self.st.menu_entry(path)
    }

    // ----- view.*

    /// Current zoom in percent.
    pub fn zoom(&self) -> HResult<u32> {
        Ok(self.st.zoom)
    }

    pub fn view_mode(&self) -> HResult<ViewMode> {
        if let Some(mode) = crate::features::file_print::preview_mode(&self.st) {
            return Ok(mode);
        }
        Ok(crate::features::view_view_bitmap::mode(&self.st))
    }

    pub fn panel(&self, panel: Panel) -> HResult<PanelState> {
        crate::features::view_panels::panel_state(&self.st, panel)
    }

    /// Drag a panel to float at a screen position (`Some`) or back to its
    /// docked place (`None`).
    pub fn place_panel(&mut self, panel: Panel, at: Option<(i32, i32)>) -> HResult<()> {
        crate::features::view_panels::place(&mut self.st, panel, at)
    }

    /// Press a panel's or secondary window's close button.
    pub fn close_panel(&mut self, panel: Panel) -> HResult<()> {
        crate::features::view_panels::close(&mut self.st, panel)
    }

    // ----- selection / text / clipboard

    pub fn selection(&self) -> HResult<Option<Selection>> {
        Ok(self.st.sel.as_ref().map(|s| s.public()))
    }

    pub fn text_frame(&self) -> HResult<Option<TextFrame>> {
        Ok(crate::features::tool_text::frame(&self.st))
    }

    /// Current system clipboard content.
    pub fn clipboard(&self) -> HResult<ClipboardContent> {
        Ok(self.st.clipboard.clone())
    }

    /// Place content on the system clipboard as another program would.
    pub fn set_clipboard(&mut self, content: ClipboardContent) -> HResult<()> {
        self.st.clipboard = content;
        Ok(())
    }

    // ----- tools.toolbox

    /// The 16 buttons in row-major order.
    pub fn tool_buttons(&self) -> HResult<Vec<ToolButton>> {
        Ok(crate::features::tools_toolbox::buttons(&self.st))
    }

    /// Rest the pointer on a tool button or menu item (or nothing).
    pub fn hover(&mut self, target: Hover) -> HResult<()> {
        crate::features::tools_toolbox::hover(&mut self.st, target)
    }

    /// The tooltip currently shown, if any.
    pub fn tooltip(&self) -> HResult<Option<String>> {
        Ok(crate::features::tools_toolbox::tooltip(&self.st))
    }

    // ----- ui.status_bar

    pub fn status_bar(&self) -> HResult<StatusBar> {
        Ok(crate::features::ui_status_bar::status_bar(&self.st))
    }

    // ----- colors.color_box

    /// The 28 swatches, index 0 = swatch 1.
    pub fn swatches(&self) -> HResult<Vec<Rgb>> {
        Ok(crate::features::color_box::swatches(&self.st))
    }

    /// Click swatch `n` (1..=28); `double` makes it a double-click.
    pub fn click_swatch(&mut self, n: u32, button: Button, double: bool) -> HResult<()> {
        crate::features::color_box::click_swatch(&mut self.st, n, button, double)
    }
}
