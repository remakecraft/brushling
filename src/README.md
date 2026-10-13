# src/ layout and extension points

`harness.rs` is the frozen public `App` API; it delegates to `State`
(`state.rs`). Behaviour lives in private modules. A feature adds behaviour by
editing its own file under `features/` (pre-wired in `features/mod.rs`) and,
where needed, adding fields to `State`.

| Module | Purpose |
|---|---|
| `doc.rs` | `Document` (pixels, path, depth, palette, colour mode, dirty), clipped `put`, undo snapshots (`push_undo`) |
| `raster.rs` | pure helpers: `snap_line_end`, `rect_from_corners`, `normalize`, `clip_rect`, `in_bounds` |
| `state.rs` | all mutable state: colours, tool, options, zoom, clock, rng, dialogs, settings |
| `tools/mod.rs` | `ToolHandler` trait, pointer/key/clock routing, double-click detection, tool options |
| `registry.rs` | `Registry`: tools, menu commands, shortcuts, pointer hooks, decoders, render layers |
| `commands.rs` | `State::menu` dispatch by path; File > New / File > Exit |
| `dialogs.rs` | modal dialog stack, `OpenDialog`, message boxes |
| `formats.rs` | `State::open_path`, decoder list (`Registry::decoder`) |
| `settings.rs` | `key=value` settings file in `<config_dir>/brushling/settings.ini` |
| `render.rs` | `Composite` and `Workspace` surfaces, with layer hooks |
| `rng.rs` | seeded generator behind `App::seed` |

## Register a tool
In `features/<tool>.rs`, implement `ToolHandler` (all methods default to
no-ops; override `press`, `drag`, `release`, `hover`, `key`, `tick`,
`activate`, `deactivate`) and call `r.tool(Tool::X, &HANDLER)` from `register`.
An unregistered tool ignores input. Per-gesture data goes into new `State`
fields. Draw with `st.doc.put(x, y, colour)` (clips), call `st.doc.push_undo()`
once per user-visible edit before changing pixels, use `st.fg` / `st.bg` and
`st.options`. Placeholders for the colour-per-button rule belong in
`features/mouse_colors.rs`.

## Register a menu command
`r.command("Image > Invert Colors", run)` (or `r.command_if(path, run,
Some(enabled_fn))`) and, for a shortcut, `r.shortcut(Key::Char('i'),
Mods::CTRL_SHIFT, "Image > Invert Colors")`. Paths are the exact labels joined
with `" > "`. A disabled command is a silent no-op; an unregistered path is
`HarnessError::Invalid`.

## Add a dialog
Build an `OpenDialog::new(Dialog { kind, title, text, buttons, fields }, handler)`
and `st.open_dialog(od)` from a command. The handler
`fn(&mut State, &mut OpenDialog, DialogAction) -> HResult<Outcome>` gets
`Set { field, value }` (call `dialogs::store_field` first, then validate) and
`Click(button)` (the button is already known to exist); return `Outcome::Keep`
or `Outcome::Close`. Dialogs opened from inside a handler appear above it, so a
validation message box returns to the dialog beneath after `OK`.
`st.message_box(title, text, &["Yes","No","Cancel"], Some(continuation))` runs
`continuation(st, button)` after the box closed.

## Bitmap codec (format.bmp_*)
- Decode: `features::format_bmp_read::decode_bmp(&[u8]) -> Result<Decoded, DecodeError>`
  (registered as a decoder already, so `State::open_path` / `formats::decode`
  use it). `NOT_ENOUGH_MEMORY` is the message for unknown compression.
- Encode: `features::format_bmp_write::encode_bmp(pixels, w, h, depth, palette)
  -> HResult<Vec<u8>>` (depth 1/4/8/24, pixels map to exact-or-nearest palette
  entry, ties to lower index); `encode_bmp_indexed(indices, w, h, depth,
  palette)` when indices are already known; `keeps_source_palette(source_depth,
  &palette, depth)` is the same-depth-keeps-palette rule for file.save.

## Open and save (file.open, file.save, format.color_reduction)
- `file_open::open_file(st, path) -> HResult<bool>`: replaces the document (true) or shows the
  cannot-open / not-enough-memory box and keeps it (false). The recent-file list should be
  updated from its success branch; the save-changes check belongs in front of
  `file_open::show_open_dialog(st)` (the File > Open... command body).
- `file_save::save_document(st)` is File > Save; `file_save::show_save_as(st)` opens the dialog.
- `file_save::request_save(st, path, depth)` writes after the colour-loss warning (OK/Cancel;
  the pending target lives in `State::pending_save`). `file_save::save_to(st, path, depth) ->
  HResult<bool>` writes without asking, updates path/depth/palette/pixels, clears dirty, never
  touches undo, and shows "The file could not be saved: ..." (false) on I/O failure. The recent
  list should be updated from its success branch. A "continue after save" hook for the
  save-changes prompt has to be added there (also after the warning box and the Save As dialog).
- `format_color_reduction::{fixed_palette(depth), reduce, loses_colors, WARNING}`.

## Other hooks
- `r.pointer_hook(f)`: sees every pointer event before the tool; return `true`
  to consume (resize handles, selection handles).
- `r.decoder(f)`: file format; `None` = not this format.
- `r.composite_layer(f)` / `r.workspace_layer(f)`: draw floating content,
  previews, grid, marquee into `render`.
- `State::replace_document` resets per-document state; add your reset there.
- `State::remember_new_size(w, h)` stores the Attributes size for File > New.
- `State::record_call(PlatformCall)` for printer, mail and wallpaper.
- Settings: `st.settings.get/set`, then `save()`; written on File > Exit.

## Selection model (`selection.rs`)
One selection exists at a time: `State::sel: Option<Sel>`. `Sel { bounds,
mask, floating, origin }`: `bounds` is inclusive in image coordinates, `mask`
is row-major over `bounds` (rectangles are all true, free-form shapes are
not), `floating` is `None` while the pixels still live in the image and
`Some(pixels)` (same indexing as `mask`) once lifted; `origin` is the bounds
at creation/lift. `App::selection()` exposes it via `Sel::public()`.

Helpers on `State` (none pushes an undo step; the caller calls
`doc.push_undo()` first when the user-visible edit needs one):
- `set_selection(sel)` ends the old selection, installs an unlifted one.
- `lift_selection(clear_source)` floats the pixels (source filled with
  `bg` when asked); sets `origin`. Move `sel.bounds` afterwards to move it.
- `stamp_selection()` writes floating pixels where they lie (copy stamp);
  `end_selection()` stamps and drops it; `discard_selection()` just drops;
  `erase_selection()` fills an unlifted mask with `bg` and drops it.
- `Sel::{rect, new, index, contains, is_floating}`; `State::has_selection`.
- `selection::{screen_box, handle_centres}` give the marquee/handle geometry
  used by the workspace layer (5x5 handles).

Floating pixels are drawn by the composite layer in `selection.rs`; the
dashed box, handles and drag preview by its workspace layer. The Select and
Free-Form tools (`features/tool_select.rs`) end the previous selection on
press, on tool change and on Esc. Per-gesture drag data is `State::sel_drag`.
Add your own `Sel` fields in `selection.rs` and your gesture state to `State`.
