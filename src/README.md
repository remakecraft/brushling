# src/ layout and extension points

`harness.rs` is the frozen public `App` API; it delegates to `State`
(`state.rs`). Behaviour lives in private modules. A feature adds behaviour by
editing its own file under `features/` (pre-wired in `features/mod.rs`) and,
where needed, adding fields to `State`.

| Module | Purpose |
|---|---|
| `doc.rs` | `Document` (pixels, path, depth, palette, colour mode, dirty), clipped `put`, undo snapshots (`push_undo`) |
| `raster.rs` | pure helpers: `line_points` (Bresenham), `rect_from_corners`, `normalize`, `clip_rect`, `in_bounds` |
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
