# Brushling

A single-document raster paint program in Rust and egui, rewritten from scratch by [Remakecraft](https://github.com/remakecraft/remakecraft) from a 1998-era paint program's observed behaviour. No code from the original or from any known remake was used: the builders worked from a written spec and tests behind a logged wall, and known reimplementations were on a blocklist.

## Parity

Measured on 2026-10-13 against the conformance suite Remakecraft wrote before any code. About a third of the tests are sealed: the builders never had them, and only the harness ran them.

| Layer | Visible | Sealed | Overall |
|---|---|---|---|
| Behaviour | 181/182 | 84/84 | 99.6% |
| Pixel | 61/61 | 33/33 | 100% |
| File format | 36/36 | 18/18 | 100% |
| UI | 38/38 | 19/19 | 100% |
| **Total** | **316/317** | **154/154** | **99.7%** |

60 features (47 must, 10 should, 3 could), 2 h 56 min, $41.08 in tokens. The one failure, `tool.polygon`'s `click_first_vertex_closes`, is a spec defect: the test expects a pixel that is not on the triangle. Parity measures the spec, not the original, so a behaviour nobody wrote down is not tested.

## What is here

| Path | What |
|---|---|
| `src/` | The app as a library: `harness.rs` is the public `App` API, behaviour lives under `features/`. See [`src/README.md`](src/README.md) |
| `src/main.rs`, `src/shell/` | The window (eframe), a thin shell over `App`, for the desktop and the browser |
| `web/` | The browser page and `build.sh`, which builds it into `web/dist/` |
| `spec/` | The behaviour spec the builders worked from |
| `tests/visible/` | The visible conformance tests. The sealed tests and the evidence about the original are not published |
| `remake.json` | The run's settings, including the blocklist |

## Run it

```bash
cargo run --release
```

`src/main.rs` and `src/shell/` are a thin eframe window over the library's `App`: the app renders the workspace, and the shell draws the menu bar, tool box and options, colour box, status bar, dialogs, Help Topics, thumbnail, Print Preview toolbar, View Bitmap and the workspace scroll bars from the app's state, and forwards mouse, wheel, keyboard and elapsed time. A file path argument opens that file. Settings go to `%APPDATA%\brushling`.

The shell has a classic late-90s desktop look of its own (`src/shell/theme.rs`): grey bevelled 3D controls, navy title bars and highlights, a sunken workspace well, and original 16x16 pixel-art tool icons drawn in code (`src/shell/icons.rs`). It is light-only and not affiliated with Microsoft.

Not covered by the shell yet:

| Gap | Why |
|---|---|
| Text toolbar (font, size, bold/italic/underline) | No harness call exposes or sets it; typing into a text frame works |
| Floating tool box and colour box | Always drawn docked |
| Alt-key menu navigation | The app's keyboard menu state is not visible to the egui menu bar; menus work with the mouse |
| Ctrl+V when the system clipboard holds no text | egui swallows the key; Edit > Paste works |

### In the browser

The same shell builds for the web (eframe on WebGL, `wasm32-unknown-unknown`). One-time setup, then the build:

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

```bash
bash web/build.sh
```

`web/dist/` then holds `index.html`, `brushling.js` and `brushling_bg.wasm` (about 5 MB, 2 MB gzipped), a static folder that works from any path. Serve it with anything that sends `.wasm` as `application/wasm`:

```bash
python -m http.server -d web/dist 8000
```

The browser has no file system, so the app's files live in memory for the life of the page. Differences from the desktop:

| Area | In the browser |
|---|---|
| File > Open, Edit > Paste From | The Open dialog gets an **Upload from this device...** button; the picked file opens (or pastes) at `/uploads/<name>`. Typed paths reach only files uploaded or saved in this page |
| File > Save, Save As, Edit > Copy To | Each save also downloads the file; the browser picks the folder. The file on the user's disk is never overwritten |
| Settings, recent files | Kept in memory; gone when the page closes |
| Ctrl+N, Ctrl+Shift+N, Ctrl+T, Ctrl+W, Ctrl+PageUp/PageDown, Alt+F4 | The browser keeps them (new window, new tab, close tab, switch tab). Use the menus for New, Clear Image, Tool Box, Stretch/Skew and the zoom sizes. The other Ctrl shortcuts reach the app |
| Closing the tab with unsaved changes | The browser's own leave-page prompt, not the app's Save Changes dialog |
| File > Exit | Shows a "Start again" screen; a page cannot close itself |
| View Bitmap | Fills the page, not the screen; the app assumes a 1920x1080 screen because the browser does not report one |
| Touch screens | One finger is the left button; there is no right button, so background-colour drawing needs a mouse |

Print, Send and Set As Wallpaper behave as on the desktop.

Build and test everything with:

```bash
cargo build --all-targets
cargo test
```

## Not affiliated

Microsoft, Windows and Microsoft Paint are trademarks of the Microsoft group of companies. Brushling is an independent project, not affiliated with, endorsed by, or sponsored by Microsoft.

## Licence

[MIT](LICENSE).
