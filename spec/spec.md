# Brushling specification (version 1)

Rendered from `spec.json`; on any disagreement `spec.json` wins. 60 features. Uncovered inventory ids: none.

---

## 1. core.document: Document model, conventions and test harness

Priority: must | Layer: behaviour | Inventory: behaviour.document.single, behaviour.canvas.default_size

One raster image is open at a time; this feature fixes the pixel model, coordinate conventions, default new-image size and the harness every acceptance line relies on.

**Behaviour**
1. Exactly one image (the document) is open at a time. Opening or creating an image replaces the current one, after the save-changes check (feature file.save_changes_prompt).
2. The document is a W x H grid of pixels, 1 <= W <= 9999 and 1 <= H <= 9999, addressed (x,y) with (0,0) the top-left pixel, x growing right and y growing down. Every pixel holds an RGB colour with 8 bits per channel.
3. The document also records: file path (none for a new image), source bit depth (1, 4, 8 or 24), an optional palette (present when the source was 1/4/8-bit or after a reduced-colour save), colour mode (Colors or Black and white) and a dirty flag.
4. Rectangles in this spec include both corner pixels: a drag from (x0,y0) to (x1,y1) covers columns min(x0,x1)..max(x0,x1) and rows min(y0,y1)..max(y0,y1).
5. Pixel results are exact (tolerance 0) unless a line states otherwise. Drawing never anti-aliases. Pixels a tool would place outside the canvas are discarded; drawing never grows the canvas.
6. The application logic is drivable without a window. A test harness can: create a document of a given size and fill colour; send pointer press/move/release in image coordinates with a button (left or right) and modifiers (Shift, Ctrl); send key events; invoke any menu command by its menu path; advance a virtual clock; read any pixel, the foreground and background colours, active tool, tool options, zoom, selection bounds and mask, menu enabled/checked states, status bar text and the bytes of files written. Platform services (clipboard, file dialogs, printer, mail, wallpaper) are behind interfaces the harness can replace with recorders.
7. Acceptance shorthand used throughout: 'fresh state' = a new 32x32 image filled with (255,255,255), foreground (0,0,0), background (255,255,255), every tool option at its default, opaque selection mode, zoom 100%, empty undo history. 'click P' = press and release at P without moving. 'drag A->B' = press at A, a single move event to B, release at B. Colours: K=(0,0,0), W=(255,255,255), R=(255,0,0), G=(0,255,0), B=(0,0,255).
8. ASSUMPTION: the first new image (startup, or File > New when no size has ever been confirmed in Attributes) is 512 x 384 pixels. This is a guess; no source states the Win98 value (only a later version is documented with this size).
9. Confirming Image > Attributes stores its width and height as the size used by later File > New commands; the stored size persists across sessions.
10. At startup the app shows a new image named untitled, filled with (255,255,255). If a file path is given as the first command-line argument, that file is opened instead.

**Inputs:** command-line path (optional); harness events
**Outputs:** document state; pixel buffer

**Edge cases**
- A command-line path that cannot be read shows the cannot-read message and leaves the untitled image open.
- Settings file missing or unreadable: defaults are used and no error is shown.

**Acceptance**
- Start with no settings file: the image is 512x384, every pixel equals (255,255,255), and the window title is 'untitled - Brushling'.
- Open file A, then open file B: the document's pixels equal B's pixels and its path is B's path.
- Set Attributes to 100x50 and confirm, then File > New: the new image is 100x50.
- Restart after the previous check, then File > New: the new image is 100x50.
- Fresh state, Pencil drag (-5,3)->(40,3): pixels (0..31,3) are K, the image is still 32x32 and no other pixel changed.

---

## 2. ui.main_window: Main window layout and minimum size

Priority: must | Layer: ui | Inventory: ui.window, behaviour.window.min_size

A single top-level window with title, menu bar, docked tool box, scrolling workspace, docked colour box and status bar, which cannot be made smaller than its docked bars.

**Behaviour**
1. The window title is '<file name> - Brushling', where <file name> is the file's name without folder (with extension) or 'untitled' for a never-saved image.
2. Layout from top to bottom: title bar, menu bar, a middle band with the tool box docked at the left and the workspace filling the rest, the colour box docked along the bottom, then the status bar. There is no button toolbar.
3. The workspace has a grey (128,128,128) background; the canvas is drawn with its top-left corner 4 screen pixels right and below the workspace's top-left corner. Vertical and horizontal scroll bars appear when the zoomed canvas does not fit.
4. The window's minimum client width is the width of the docked colour box and its minimum height is the sum of menu bar, docked tool box and colour box and status bar heights. The minimum applies to every resize, including OS tiling and snapping.
5. The title updates immediately after New, Open and Save As.

**Inputs:** window resize
**Outputs:** window geometry; title text

**Edge cases**
- Hidden tool box or colour box (View menu) still defines the minimum size as if docked.

**Acceptance**
- After saving to C:/t/cat.bmp the title is exactly 'cat.bmp - Brushling'.
- Requesting a window size of 50x50 screen pixels results in a client width >= docked colour box width and a client height >= the docked tool box height.
- With a 2000x2000 image at 100% zoom in a 800x600 window, both scroll bars are present; with a 10x10 image neither is present.

---

## 3. ui.layout_persistence: Remembered window layout

Priority: should | Layer: behaviour | Inventory: behaviour.window.persist

Window size, visibility toggles and floating panel positions survive a restart.

**Behaviour**
1. On exit the app writes a settings file in the per-user configuration folder (subfolder 'brushling'). It stores: window size and position, the View toggles (Tool Box, Color Box, Status Bar, Text Toolbar), whether tool box, colour box and text toolbar are docked or floating and their floating positions, thumbnail window position, the default new-image size, and the recent-file list.
2. On start these values are restored. A floating panel whose stored position would place it fully outside every connected screen is placed at the top-left of the primary screen instead.

**Inputs:** exit; start
**Outputs:** settings file

**Edge cases**
- A corrupt settings file is ignored and replaced on next exit.

**Acceptance**
- Uncheck View > Status Bar, exit, start: View > Status Bar is unchecked and no status bar is shown.
- Float the tool box at screen position (300,200), exit, start: the tool box is floating at (300,200).
- Store a floating colour box position of (-5000,-5000) in the settings file, start: the colour box is visible at the primary screen's top-left.

---

## 4. ui.menu_bar: Menu bar, menus and access letters

Priority: must | Layer: ui | Inventory: menu.bar, menu.file, menu.edit, menu.view, menu.view.zoom, menu.image, menu.colors, menu.help, behaviour.menu.mnemonics, shortcut.alt_mnemonic

Six menus with fixed item order, separators and underlined access letters reachable with Alt.

**Behaviour**
1. Menus in order: File, Edit, View, Image, Colors, Help.
2. File: New, Open..., Save, Save As..., separator, Print Preview, Page Setup..., Print..., separator, Send..., separator, Set As Wallpaper (Tiled), Set As Wallpaper (Centered), separator, recent files (1 to 4 numbered entries; the group and its separator are absent when the list is empty), separator, Exit.
3. Edit: Undo, Repeat, separator, Cut, Copy, Paste, Clear Selection, Select All, separator, Copy To..., Paste From....
4. View: Tool Box, Color Box, Status Bar, Text Toolbar (these four are check items), separator, Zoom (submenu), View Bitmap.
5. ASSUMPTION: the Zoom submenu order is Normal Size, Large Size, Custom..., separator, Show Grid, Show Thumbnail. Guess; no capture of the open submenu exists.
6. Image: Flip/Rotate..., Stretch/Skew..., Invert Colors, Attributes..., Clear Image, Draw Opaque (check item).
7. ASSUMPTION: the Colors menu holds only Edit Colors...; there are no palette save/load commands. Follows the Win98-era screenshot over older-version sources that list palette file commands.
8. Help: Help Topics, separator, About Brushling.
9. Menu items show their keyboard shortcut right-aligned using the text forms Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+P, Alt+F4, Ctrl+Z, F4, Ctrl+X, Ctrl+C, Ctrl+V, Del, Ctrl+A, Ctrl+T, Ctrl+L, Ctrl+F, Ctrl+R, Ctrl+W, Ctrl+I, Ctrl+E, Ctrl+Shift+N, plus the zoom keys of feature ui.keyboard_shortcuts.
10. Each menu title and item has one underlined access letter. Alt+letter opens a menu; with a menu open, the letter runs the item.
11. ASSUMPTION: access letters (underlined character in brackets) are: [F]ile, [E]dit, [V]iew, [I]mage, [C]olors, [H]elp; File: [N]ew, [O]pen, [S]ave, Save [A]s, Print Pre[v]iew, Page Se[t]up, [P]rint, Sen[d], Set As [W]allpaper (Tiled), Set As Wallpaper (C[e]ntered), recent entries [1]-[4], E[x]it; Edit: [U]ndo, [R]epeat, Cu[t], [C]opy, [P]aste, C[l]ear Selection, Select [A]ll, C[o]py To, Paste [F]rom; View: [T]ool Box, [C]olor Box, [S]tatus Bar, T[e]xt Toolbar, [Z]oom, [V]iew Bitmap; Zoom: [N]ormal Size, [L]arge Size, C[u]stom, Show [G]rid, Show T[h]umbnail; Image: [F]lip/Rotate, [S]tretch/Skew, [I]nvert Colors, [A]ttributes, [C]lear Image, [D]raw Opaque; Colors: [E]dit Colors; Help: [H]elp Topics, [A]bout Brushling. Only Print Pre[v]iew is confirmed by a source; the rest are guesses.

**Inputs:** mouse on menus; Alt+letter
**Outputs:** menu commands

**Edge cases**
- Alt pressed and released alone highlights the File menu title; Esc then removes the highlight.

**Acceptance**
- The harness lists menu titles in order: File, Edit, View, Image, Colors, Help.
- The harness lists Edit items, ignoring separators: Undo, Repeat, Cut, Copy, Paste, Clear Selection, Select All, Copy To..., Paste From....
- Key sequence Alt+F then V opens Print Preview.
- Key sequence Alt+I then I inverts every pixel of a fresh-state image to (0,0,0).
- Colors menu has exactly one item.

---

## 5. ui.menu_enable_state: Greyed-out menu items

Priority: must | Layer: ui | Inventory: behaviour.menu.disabled_items

Items that cannot act stay listed but are disabled and do nothing when invoked by menu or shortcut.

**Behaviour**
1. Cut, Copy, Clear Selection and Copy To... are disabled when there is no selection.
2. Undo is disabled when the undo history is empty; Repeat is disabled when nothing is available to repeat.
3. Both Set As Wallpaper items are disabled while the document has no file path.
4. View > Text Toolbar is disabled unless the Text tool is active.
5. ASSUMPTION: Paste is disabled when the clipboard holds no image, except while a text frame is active, when it is enabled if the clipboard holds text. Guess.
6. ASSUMPTION: View > Zoom > Show Thumbnail is disabled at 100% zoom. Guess based on the thumbnail only being meaningful when zoomed.
7. A disabled command invoked through its keyboard shortcut does nothing and changes no state.

**Inputs:** document/selection/tool state
**Outputs:** menu enabled flags

**Edge cases**
- Shortcut of a disabled item while a drag is in progress is ignored.

**Acceptance**
- Fresh state: Cut, Copy, Clear Selection, Copy To..., Undo, Repeat, both Set As Wallpaper items and Text Toolbar report disabled.
- Fresh state, Pencil click (1,1), Undo: Repeat reports enabled.
- Fresh state, press Ctrl+X: no pixel changes and clipboard is not written.
- Save the image to a file: both Set As Wallpaper items report enabled.
- Select the Text tool: View > Text Toolbar reports enabled.

---

## 6. ui.keyboard_shortcuts: Global keyboard shortcuts

Priority: must | Layer: behaviour | Inventory: shortcut.ctrl_n, shortcut.ctrl_o, shortcut.ctrl_s, shortcut.ctrl_p, shortcut.alt_f4, shortcut.ctrl_z, shortcut.alt_backspace, shortcut.f4, shortcut.ctrl_x, shortcut.ctrl_c, shortcut.ctrl_v, shortcut.del, shortcut.ctrl_a, shortcut.ctrl_t, shortcut.ctrl_l, shortcut.ctrl_f, shortcut.ctrl_r, shortcut.ctrl_w, shortcut.ctrl_i, shortcut.ctrl_e, shortcut.ctrl_shift_n, shortcut.zoom_keys

Each shortcut runs exactly the same command as its menu item.

**Behaviour**
1. Ctrl+N New; Ctrl+O Open; Ctrl+S Save; Ctrl+P Print; Alt+F4 Exit.
2. Ctrl+Z Undo; Alt+Backspace also Undo (not shown on the menu); F4 Repeat.
3. Ctrl+X Cut; Ctrl+C Copy; Ctrl+V Paste; Del Clear Selection; Ctrl+A Select All.
4. Ctrl+T toggles Tool Box; Ctrl+L toggles Color Box; Ctrl+F View Bitmap.
5. Ctrl+R Flip/Rotate; Ctrl+W Stretch/Skew; Ctrl+I Invert Colors; Ctrl+E Attributes; Ctrl+Shift+N Clear Image.
6. ASSUMPTION: zoom keys are Ctrl+PageUp = Large Size, Ctrl+PageDown = Normal Size, Ctrl+G = toggle Show Grid. Guess; no source shows the Zoom submenu or its keys.
7. While a text frame is active, Ctrl+X, Ctrl+C, Ctrl+V, Del, Ctrl+A and Ctrl+Z act on the frame's text instead of the image.
8. Shortcuts are ignored while a modal dialog is open (the dialog gets the keys).

**Inputs:** key presses
**Outputs:** commands

**Edge cases**
- Ctrl+Shift+N must not also trigger Ctrl+N.

**Acceptance**
- Fresh state, Pencil click (5,5), Alt+Backspace: pixel (5,5) is W.
- Fresh state, set background R, Ctrl+Shift+N: all pixels R and no save prompt was shown.
- Fresh state, Ctrl+I: all pixels K.
- Ctrl+PageUp: zoom is 400%. Ctrl+PageDown: zoom is 100%.
- For every shortcut listed, invoking the shortcut and invoking the menu item from identical start states produce identical document state.

---

## 7. file.new: File > New and colour reset

Priority: must | Layer: behaviour | Inventory: menu.file.new, behaviour.palette.reset_on_new

Creates a blank image of the default size, filled with the current background colour, then resets the colour indicator to black on white.

**Behaviour**
1. If the document is dirty, the save-changes prompt runs first; Cancel aborts New.
2. ASSUMPTION: the new image is filled with the background colour that is current when New is invoked, and afterwards the foreground becomes (0,0,0) and the background (255,255,255). Follows vendor documentation over a tutorial saying the new image is always white.
3. The new image has the default size (feature core.document), colour mode Colors, source depth 24, no palette, no file path, name untitled, dirty flag clear, no selection, empty undo and repeat history.
4. The active tool and all tool options are unchanged.
5. ASSUMPTION: colour box swatches that were redefined with Edit Colors keep their redefined colours after New. Guess; no source says.

**Inputs:** File > New; Ctrl+N
**Outputs:** new document

**Edge cases**
- A floating selection is discarded, not committed, by New.

**Acceptance**
- Fresh state, set background (0,0,255), File > New: every pixel is (0,0,255), foreground reads (0,0,0), background reads (255,255,255).
- Fresh state, File > New: no prompt is shown (not dirty).
- Fresh state, Pencil click (1,1), File > New, answer Cancel: pixel (1,1) is still K.
- Redefine swatch 3 to (1,2,3), File > New: swatch 3 reads (1,2,3).
- After New, Undo is disabled.

---

## 8. file.save_changes_prompt: Save-changes prompt

Priority: must | Layer: ui | Inventory: dialog.save_changes

Before discarding a dirty document the app asks whether to save, with Yes, No and Cancel.

**Behaviour**
1. ASSUMPTION: the prompt appears before New, Open, opening a recent file, and Exit whenever the dirty flag is set. Only the New case is documented; the others are a guess.
2. ASSUMPTION: the prompt text is 'Save changes to <file name>?' with buttons Yes, No, Cancel (own wording; the original wording is unknown).
3. Yes runs Save (Save As when the document has no path); if that save is cancelled or fails, the pending action is aborted. No proceeds without saving. Cancel aborts the pending action.
4. The dirty flag is set by every change that creates an undo step and cleared by New, Open and a successful Save/Save As.

**Inputs:** New/Open/recent/Exit while dirty
**Outputs:** prompt result

**Edge cases**
- Undoing back to the saved state still counts as dirty.

**Acceptance**
- Fresh state, Pencil click (1,1), File > New: a prompt with exactly three buttons Yes, No, Cancel is shown.
- Same, answer No: the document is a new image and no file was written.
- Same, answer Yes, then cancel the Save As dialog: the old image with (1,1)=K is still open.
- Fresh state, File > Exit: the app exits without a prompt.

---

## 9. file.open: File > Open and open errors

Priority: must | Layer: behaviour | Inventory: menu.file.open, dialog.file_open, dialog.error.cannot_read, dialog.error.memory

Picks a file with the platform open dialog and replaces the document with it, reporting unreadable files.

**Behaviour**
1. After the save-changes check, the platform open-file dialog is shown.
2. ASSUMPTION: the dialog's type filter offers 'Bitmap files (*.bmp;*.dib)' (default) and 'All files (*.*)'. Guess; the original list is unknown.
3. A readable file (feature format.bmp_read) replaces the document; path, source depth and palette are taken from the file; dirty flag clear; undo history cleared; selection none; colours unchanged; zoom unchanged.
4. A file that is not a decodable bitmap shows a message box titled 'Brushling' with the file path and the text 'This file cannot be opened: it is not a bitmap or uses a format Brushling does not support.' (own wording), button OK; the current document is kept.
5. A bitmap whose compression type is not supported, or whose size cannot be allocated, shows a message box with the text 'There is not enough memory or resources to open this image.' (own wording), button OK; the current document is kept.
6. Every successfully opened file is added to the recent-file list.

**Inputs:** File > Open; Ctrl+O; path
**Outputs:** document; error message

**Edge cases**
- Opening the file that is already open re-reads it from disk.
- A zero-byte file gives the cannot-open message.

**Acceptance**
- Open a valid 24-bit 3x2 BMP: the document is 3x2, pixels equal the file's, title shows its name, Undo disabled.
- Open a text file renamed x.bmp: a message titled 'Brushling' containing the full path is shown and the previous document is unchanged.
- Open a BMP with compression field 4 (embedded JPEG): the not-enough-memory message is shown and the previous document is unchanged.

---

## 10. file.save: File > Save and Save As

Priority: must | Layer: behaviour | Inventory: menu.file.save, menu.file.save_as, dialog.file_save_as

Writes the document as a BMP to its current path or to a newly chosen path and depth.

**Behaviour**
1. ASSUMPTION: Save on a document without a path behaves exactly like Save As. Not shown in sources; standard behaviour assumed.
2. Save on a document with a path writes to that path at the document's source depth (with its palette if it has one), without a dialog.
3. Save As shows the platform save dialog with a file name field (default: current name, or 'untitled') and a type list.
4. ASSUMPTION: the type list reads, in order: 'Monochrome bitmap (1-bit)', '16-color bitmap (4-bit)', '256-color bitmap (8-bit)', '24-bit bitmap'; the preselected type is the document's source depth (24-bit for a new image). Own labels; the original labels are unknown.
5. If the chosen name does not end in '.bmp' (case-insensitive), '.bmp' is appended.
6. Saving to fewer colours than the image uses runs feature format.color_reduction first.
7. After a successful save: path and title are updated, source depth becomes the saved depth, dirty flag is cleared, the file is added to the recent list. Save never changes the undo history.
8. After any save the target folder contains no new file other than the saved file. If the write fails, a message box shows 'The file could not be saved: <reason>' and the previous file on disk is unchanged.

**Inputs:** File > Save; File > Save As; Ctrl+S; name; type
**Outputs:** BMP file

**Edge cases**
- Name 'pic.BMP' stays 'pic.BMP'.
- Name 'pic.png' becomes 'pic.png.bmp'.
- Saving over a read-only file shows the failure message.

**Acceptance**
- New image, Ctrl+S: the Save As dialog is shown.
- Save As 'cat' in an empty folder with type 24-bit: the folder then contains exactly one file, 'cat.bmp'.
- After saving, Undo is still enabled if it was before and the title is 'cat.bmp - Brushling'.
- Open a 4-bit BMP, Pencil click (0,0) with a palette colour, Save: the written file's bits-per-pixel field (offset 28) is 4.
- Make the target file read-only, Save: an error message is shown and the file's bytes are unchanged.

---

## 11. format.bmp_read: Reading bitmap files

Priority: must | Layer: format | Inventory: format.bmp.read, format.pcx.unsupported, format.jpeg_gif.filters

Decodes Windows bitmap files; every other format is rejected with the cannot-open message.

**Behaviour**
1. The decoder recognises a bitmap by its first two bytes 'BM', not by file extension.
2. ASSUMPTION: supported variants are: info headers of 12, 40, 108 or 124 bytes; 1, 4, 8, 16, 24 and 32 bits per pixel; compression 0 (none), 1 (RLE8), 2 (RLE4) and 3 (bit fields, 16/32-bit only); bottom-up (positive height) and top-down (negative height) rows. Guess; the exact original set is unknown.
3. An unknown or unsupported compression value gives the not-enough-memory message (follows vendor documentation that such files fail with a memory message). Any other decoding failure gives the cannot-open message.
4. 1, 4 and 8-bit files are loaded as indexed: the document keeps the file's palette and its source depth. 16, 24 and 32-bit files load as source depth 24 without a palette. Alpha or unused bytes are ignored.
5. ASSUMPTION: PCX, JPEG, GIF, PNG and all non-bitmap formats are not supported and give the cannot-open message. Follows the vendor statement that only bitmaps open unless third-party filters are installed; the remake ships no filters.

**Inputs:** file bytes
**Outputs:** pixels; palette; source depth

**Edge cases**
- Palette indices beyond the palette size read as (0,0,0).
- Truncated pixel data gives the cannot-open message.

**Acceptance**
- A 2x2 24-bit file with pixels top-left R, top-right G, bottom-left B, bottom-right W loads with exactly those pixels at (0,0),(1,0),(0,1),(1,1).
- The same pixels stored top-down (height -2) load identically.
- An 8-bit RLE8 file loads with the same pixels as its uncompressed equivalent.
- A valid PNG file renamed to x.bmp gives the cannot-open message.
- A .pcx file gives the cannot-open message.

---

## 12. format.bmp_write: Writing bitmap files

Priority: must | Layer: format | Inventory: format.bmp.write, format.bmp.indexed_preserve

Writes uncompressed Windows bitmaps at 1, 4, 8 or 24 bits per pixel with a fixed, testable header layout.

**Behaviour**
1. File layout: 14-byte file header, 40-byte info header, palette (indexed depths only), pixel rows. All integers little-endian.
2. File header: bytes 0-1 'BM'; bytes 2-5 total file size; bytes 6-9 zero; bytes 10-13 offset of pixel data = 54 + 4 x palette entries.
3. Info header: size 40; width; height positive (rows stored bottom-up); planes 1; bits per pixel 1, 4, 8 or 24; compression 0; image size = padded row bytes x height; horizontal and vertical resolution 3780 pixels per metre; colours used = palette entries (2, 16 or 256) for indexed depths and 0 for 24-bit; important colours 0.
4. Palette entries are 4 bytes each: blue, green, red, 0.
5. Rows are padded with zero bytes to a multiple of 4 bytes. 24-bit pixels are stored blue, green, red. 1 and 4-bit pixels are packed most significant bits first.
6. ASSUMPTION: an image whose source depth is indexed and which has a palette is saved at that same depth with its own palette unchanged, provided the chosen depth equals the source depth; otherwise the fixed palettes of feature format.color_reduction are used. Follows the documented behaviour that an opened 8-bit image keeps its palette.

**Inputs:** document; depth
**Outputs:** file bytes

**Edge cases**
- 1x1 image at 1-bit: row is 4 bytes.

**Acceptance**
- Save a 2x2 24-bit image: file is 70 bytes; bytes 0-1 = 0x42 0x4D; bytes 2-5 = 70; bytes 10-13 = 54; bytes 18-21 = 2; bytes 22-25 = 2; bytes 28-29 = 24; bytes 30-33 = 0; bytes 38-41 = 3780.
- In that file, bytes 54-56 hold the pixel (0,1) as blue, green, red and bytes 60-61 are zero padding.
- Save a 9x1 image at 1-bit: file is 66 bytes, bytes 10-13 = 62, palette bytes 54-61 = 00 00 00 00 FF FF FF 00.
- Open an 8-bit file with a 5-entry palette, Save: the written palette's first 5 entries equal the original's.

---

## 13. format.color_reduction: Saving with fewer colours

Priority: must | Layer: format | Inventory: format.bmp.color_reduction, dialog.save_color_loss

Saving at 1, 4 or 8 bits maps every pixel to the nearest colour of a fixed palette, without dithering, after a warning.

**Behaviour**
1. ASSUMPTION: the 16-colour palette, index 0 to 15, is: (0,0,0), (128,0,0), (0,128,0), (128,128,0), (0,0,128), (128,0,128), (0,128,128), (192,192,192), (128,128,128), (255,0,0), (0,255,0), (255,255,0), (0,0,255), (255,0,255), (0,255,255), (255,255,255). Guess (the classic 16-colour set); the original's palette is unknown.
2. ASSUMPTION: the 256-colour palette is: indices 0-15 the 16-colour palette above; indices 16-231 a colour cube where index = 16 + 36r + 6g + b for r,g,b in 0..5 and channel value = 51 x level; indices 232-255 greys with value 8 + 10k for k = 0..23. Guess.
3. ASSUMPTION: the monochrome palette is index 0 = (0,0,0), index 1 = (255,255,255); a pixel maps to white when 299R + 587G + 114B >= 128000, else black. Guess.
4. ASSUMPTION: for 16 and 256 colours each pixel maps to the palette entry with the smallest squared RGB distance (dR^2 + dG^2 + dB^2); ties go to the lowest index. No dithering and no palette optimisation. Guess consistent with the documented scrambling of colours.
5. ASSUMPTION: before writing at 1, 4 or 8 bits, if at least one pixel's colour is not in the target palette, a warning is shown: 'Saving at this colour depth will permanently change some colours. Continue?' (own wording; guess, the original text is unknown) with buttons OK and Cancel. Cancel aborts the save without writing.
6. ASSUMPTION: after a reduced save the in-memory image is replaced by the mapped colours and becomes indexed with the target palette, so the screen matches the file. Guess; sources only say the loss is irreversible.

**Inputs:** image pixels; target depth
**Outputs:** mapped pixels; warning

**Edge cases**
- An image whose colours are all already in the target palette saves without a warning.

**Acceptance**
- Pixel (200,100,50) saved at 1-bit becomes index 0 (black); pixel (128,128,128) becomes index 1 (white).
- Pixel (250,10,10) saved at 4-bit is written as index 9 and reads back as (255,0,0).
- Pixel (100,100,100) saved at 8-bit is written as index 102 ((102,102,102)); grey index 241 ((98,98,98)) is equally near and loses the tie to the lower index.
- Pixel (0,0,0) saved at 8-bit is written as index 0 (tie with index 16 goes to the lower index).
- Fresh-state image (all W) saved at 4-bit: no warning is shown.
- Image with one pixel (1,2,3) saved at 4-bit: the warning is shown; Cancel leaves the target folder without the file.

---

## 14. file.recent: Recent files list

Priority: should | Layer: behaviour | Inventory: menu.file.recent

The File menu lists recently opened or saved files, most recent first, numbered from 1.

**Behaviour**
1. ASSUMPTION: the list holds at most 4 entries. Follows the source stating four; a screenshot shows three, which may simply be all that existed.
2. Entries read '<n> <full path>' with n = 1..4, most recent first. A file opened or saved moves to position 1; a path already in the list is not duplicated (case-insensitive comparison).
3. Choosing an entry runs the save-changes check and opens that file. If it cannot be opened, the cannot-open message is shown and the entry is removed.
4. The list persists across sessions.

**Inputs:** open; save
**Outputs:** menu entries

**Edge cases**
- Opening a fifth distinct file drops the oldest entry.

**Acceptance**
- Open a.bmp, b.bmp, c.bmp, d.bmp, e.bmp in order: entries read 1 e, 2 d, 3 c, 4 b.
- Then open c.bmp again: entries read 1 c, 2 e, 3 d, 4 b.
- Delete d.bmp from disk and choose its entry: an error is shown and the list no longer contains d.bmp.

---

## 15. file.exit: File > Exit

Priority: must | Layer: behaviour | Inventory: menu.file.exit

Closes the application after the save-changes check.

**Behaviour**
1. ASSUMPTION: Exit (menu, Alt+F4 or window close button) runs the save-changes check when dirty; Cancel keeps the app open. Not shown in sources; assumed like New.
2. On exit the settings file is written (feature ui.layout_persistence).

**Inputs:** File > Exit; Alt+F4; close button
**Outputs:** process exit

**Edge cases**
- Exit while a text frame is active commits the text first, which makes the document dirty.

**Acceptance**
- Fresh state, Alt+F4: process exits with code 0 and no prompt.
- Fresh state, Pencil click (1,1), File > Exit, Cancel: the app is still running and (1,1) is K.

---

## 16. file.print: Printing, Print Preview and Page Setup

Priority: should | Layer: behaviour | Inventory: menu.file.print, menu.file.print_preview, menu.file.page_setup, dialog.print, dialog.page_setup, dialog.print_preview

Prints the image at a fixed physical scale across as many pages as needed, with a page preview and page settings.

**Behaviour**
1. ASSUMPTION: Page Setup offers paper size (default Letter, 8.5 x 11 in), orientation (default Portrait) and margins in inches (default left 0.75, right 0.75, top 1.0, bottom 1.0), with OK and Cancel. Guess; the dialog's controls are unknown. Settings last for the session and are never stored in the image file.
2. ASSUMPTION: the image prints at 96 image pixels per inch, starting at the top-left of the printable area; an image larger than the printable area is split into pages, left to right then top to bottom. Guess.
3. Page count = ceil(W / printable width in px) x ceil(H / printable height in px), with printable size = (paper size minus margins) x 96.
4. Print shows the platform print dialog; OK sends the pages to the chosen printer.
5. ASSUMPTION: Print Preview replaces the workspace with a view of the current page and buttons Print..., Next Page, Prev Page, Zoom In, Zoom Out, Close. Only Zoom In, Zoom Out and Close are documented; the rest are guesses. Close returns to the normal view; the image is unchanged.

**Inputs:** File > Print; Ctrl+P; File > Print Preview; File > Page Setup
**Outputs:** print job; preview pages

**Edge cases**
- Next Page is disabled on the last page; Prev Page on the first.

**Acceptance**
- Default page setup, 512x384 image: the preview reports 1 page.
- Default page setup, 2000x100 image: printable width 672 px, the preview reports 3 pages.
- Landscape, default margins, 2000x100 image: printable width (11 - 1.5) x 96 = 912 px, the preview reports 3 pages.
- With a recording print service, Print then OK records one job whose page count equals the preview's.

---

## 17. file.send: File > Send

Priority: could | Layer: behaviour | Inventory: menu.file.send

Hands the saved image file to the system mail client as an attachment.

**Behaviour**
1. ASSUMPTION: if the document has no path or is dirty, the save-changes prompt runs first (Cancel aborts); the saved file is then passed to the platform mail service as an attachment. If no mail service exists, a message says that no e-mail program is available. Guess; only a later version documents this command.

**Inputs:** File > Send
**Outputs:** mail service call

**Edge cases**
- Answering No to the save prompt on a never-saved image aborts Send.

**Acceptance**
- With a recording mail service and a saved, clean document at C:/t/a.bmp, Send records exactly one call with attachment path C:/t/a.bmp.

---

## 18. file.wallpaper: Set As Wallpaper (Tiled / Centered)

Priority: could | Layer: behaviour | Inventory: menu.file.wallpaper_tiled, menu.file.wallpaper_centered, format.wallpaper

Makes the saved image file the desktop wallpaper, either repeated or once in the middle.

**Behaviour**
1. ASSUMPTION: the menu labels are 'Set As Wallpaper (Tiled)' and 'Set As Wallpaper (Centered)'. Follows the Win98-era screenshot over a later version's labels.
2. Both items are enabled only when the document has a file path. They pass that path and the style (tiled or centered) to the platform wallpaper service.
3. ASSUMPTION: unsaved changes are not saved first; the file as it is on disk is used. Guess.

**Inputs:** menu choice
**Outputs:** wallpaper service call

**Edge cases**
- On platforms without a wallpaper service both items are disabled.

**Acceptance**
- With a recording wallpaper service and document path C:/t/a.bmp, Tiled records (C:/t/a.bmp, tiled) and Centered records (C:/t/a.bmp, centered).
- Dirty document with a path: choosing Tiled shows no prompt and the file bytes on disk are unchanged.

---

## 19. integration.embedding: Embedding in other documents

Priority: could | Layer: format | Inventory: format.ole_embed

In-place embedding in word processors is out of scope; images move between applications via files and the clipboard.

**Behaviour**
1. ASSUMPTION: version 1 provides no in-place embedding or editing inside other applications. Images reach other programs through saved files and the clipboard (feature edit.clipboard). Scope decision, not a guess about the original.

**Inputs:** none
**Outputs:** none

**Edge cases**
- none

**Acceptance**
- Copy a 4x4 selection, then read the system clipboard from another process: it holds a 4x4 bitmap with the same pixels.

---

## 20. edit.undo_repeat: Undo and Repeat

Priority: must | Layer: behaviour | Inventory: menu.edit.undo, menu.edit.repeat, behaviour.undo.depth, behaviour.undo.grouping, behaviour.undo.mid_operation, behaviour.undo.scope

Up to three picture changes can be undone; Repeat re-applies what was undone.

**Behaviour**
1. ASSUMPTION: the undo history holds the 3 most recent steps; older steps are dropped. Follows vendor documentation and a contemporaneous book over a tutorial claiming one step.
2. One step is: one pointer press-to-release of a painting tool (Pencil, Brush, Airbrush, Eraser, Fill, Line, Rectangle, Ellipse, Rounded Rectangle), one finished Curve or Polygon, one committed text frame, one selection drag (move, stamp, trail or handle resize), and each command that changes pixels or size (Cut, Paste commit, Clear Selection, Flip/Rotate, Stretch/Skew, Invert Colors, Attributes, Clear Image, canvas resize).
3. Changing tool or colour never merges two strokes; every stroke is its own step.
4. Not undoable and not recorded: colour changes, tool and option changes, zoom and view changes, Save, Save As, Copy, Copy To, printing.
5. New and Open clear both histories.
6. Repeat re-applies the most recently undone step; up to 3 undone steps can be repeated in reverse order of undoing.
7. ASSUMPTION: any new step clears the repeat history. Guess; sources do not say.
8. ASSUMPTION: Undo while a Curve or Polygon is unfinished discards the unfinished shape and also undoes the previous completed step; one Repeat restores that previous step only, the discarded shape is not restored. Partly follows a vendor bug report (which says both reappear); the restoration of the unfinished shape is deliberately not reproduced.
9. Undo while a selection is floating reverts the most recent selection drag; if the selection has not been dragged, Undo first commits nothing and undoes the previous step, leaving the selection unchanged.

**Inputs:** Edit > Undo; Ctrl+Z; Alt+Backspace; Edit > Repeat; F4
**Outputs:** restored pixels and size

**Edge cases**
- Undo of a canvas-size change restores the previous size and pixels.
- Undo with empty history does nothing.

**Acceptance**
- Fresh state, Pencil clicks at (1,1), (2,2), (3,3), (4,4); Undo four times: (4,4),(3,3),(2,2) are W and (1,1) is K.
- Fresh state, Pencil click (1,1), Undo, Repeat: (1,1) is K.
- Fresh state, Pencil click (1,1), Undo, Pencil click (2,2): Repeat is disabled and (1,1) is W.
- Fresh state, Rectangle drag (2,2)->(8,8), start a Curve with drag (10,10)->(20,10), Undo: no pixel is K. Repeat: the rectangle is back and pixels (10..20,10) are W.
- Fresh state, change foreground to R: Undo stays disabled.
- Attributes to 10x10, Undo: image is 32x32 again.

---

## 21. edit.clipboard: Cut, Copy, Paste and the clipboard

Priority: must | Layer: behaviour | Inventory: menu.edit.cut, menu.edit.copy, menu.edit.paste, format.clipboard, dialog.paste_enlarge

Exchanges the selection with the system clipboard as a bitmap and pastes clipboard images as a floating selection.

**Behaviour**
1. Copy puts the selection's bounding-box image on the system clipboard as a 24-bit bitmap. For a free-form selection, pixels inside the bounding box but outside the shape are written as the current background colour. A floating selection is copied as currently shown. The image is unchanged.
2. Cut does Copy, then fills the selected pixels with the background colour and removes the selection. One undo step.
3. Paste commits any current selection, switches the tool to Select, and inserts the clipboard image as a floating selection.
4. ASSUMPTION: the pasted selection's top-left is the top-left image pixel currently visible in the workspace ((0,0) when not scrolled). Guess; not stated.
5. Clipboard images from other programs are accepted; any alpha channel is ignored (colour channels used as-is).
6. ASSUMPTION: if the clipboard image is wider or taller than the canvas, a question box titled 'Brushling' asks 'The pasted image is larger than the canvas. Enlarge the canvas?' (own wording) with Yes, No, Cancel. Follows the Win98-era capture showing such a prompt rather than a source saying it arrived later.
7. ASSUMPTION: Yes grows the canvas to max(W, clip width) x max(H, clip height) with new area in the background colour, then pastes; No pastes without growing (parts beyond the canvas are dropped at commit); Cancel pastes nothing. The No and Cancel meanings are a guess.
8. While a text frame is active, Paste inserts clipboard text into the frame and never pastes an image.

**Inputs:** Edit > Cut/Copy/Paste; Ctrl+X/C/V; system clipboard
**Outputs:** clipboard bitmap; floating selection

**Edge cases**
- Paste with an empty clipboard does nothing.
- Paste on a scrolled view lands at the first visible pixel.

**Acceptance**
- Fresh state, set (5,5)=R, select (4,4)-(6,6), Copy: clipboard holds a 3x3 bitmap with centre R and the other 8 pixels W; the image is unchanged.
- Same with background set to G then Cut: pixels (4..6,4..6) are G.
- Fresh state, clipboard holds a 2x2 all-R bitmap, Paste: tool is Select, a floating 2x2 selection exists at (0,0)-(1,1), and after clicking outside it (0,0),(1,0),(0,1),(1,1) are R.
- Fresh state, clipboard holds a 40x10 bitmap, Paste: the enlarge prompt appears; Yes gives a 40x32 image whose pixels (32..39, 10..31) are the background colour.
- Same prompt, Cancel: image is 32x32 and no selection exists.

---

## 22. edit.clear_select_all: Clear Selection and Select All

Priority: must | Layer: behaviour | Inventory: menu.edit.clear_selection, menu.edit.select_all

Erases the selected pixels to the background colour, or selects the entire image.

**Behaviour**
1. Clear Selection sets every selected pixel (for free-form: inside the shape only) to the current background colour and removes the selection; a floating selection's content is discarded. One undo step.
2. Select All commits any current selection, switches the tool to Select and creates a rectangular selection (0,0)-(W-1,H-1).

**Inputs:** Edit > Clear Selection; Del; Edit > Select All; Ctrl+A
**Outputs:** pixels; selection

**Edge cases**
- Del with no selection does nothing.

**Acceptance**
- Fresh state, Brush paint over (10,10), select (8,8)-(12,12), set background B, Del: pixels (8..12,8..12) are B and no selection exists.
- Fresh state, Ctrl+A: selection bounds are (0,0)-(31,31) and the active tool is Select.
- Clear Selection then Undo: pixels are as before the clear.

---

## 23. edit.copy_to_paste_from: Copy To and Paste From files

Priority: should | Layer: behaviour | Inventory: menu.edit.copy_to, menu.edit.paste_from

Saves the selection as its own file, or loads a file as a floating selection.

**Behaviour**
1. Copy To shows the Save As dialog and writes the selection's bounding-box image (free-form outside pixels as background colour) as a BMP at the chosen depth, with the same extension and colour-reduction rules as Save As. The document's path, title and dirty flag do not change.
2. Paste From shows the Open dialog, reads the file with the bitmap reader and inserts it exactly like Paste of a clipboard image, including the enlarge prompt. Read errors show the same messages as Open.

**Inputs:** Edit > Copy To; Edit > Paste From
**Outputs:** BMP file; floating selection

**Edge cases**
- Copy To with a cancelled dialog writes nothing.

**Acceptance**
- Fresh state, select (0,0)-(3,1), Copy To 'part' 24-bit: part.bmp is a 4x2 image of W; the title still reads 'untitled - Brushling'.
- Fresh state, Paste From a 3x3 all-B file: a 3x3 floating selection exists and after commit (0..2,0..2) are B.

---

## 24. view.panels: Tool Box, Color Box and Status Bar toggles

Priority: must | Layer: behaviour | Inventory: menu.view.tool_box, menu.view.color_box, menu.view.status_bar

Check items that show or hide the tool box, colour box and status bar.

**Behaviour**
1. Each item toggles visibility of its panel and shows a check mark while the panel is visible. All three are visible on first start.
2. Hiding the tool box keeps the active tool; hiding the colour box keeps the colours.
3. Tool box and colour box can be dragged off their docked position to become floating panels and dragged back to dock; the toggles show and hide them in either state.

**Inputs:** View menu; Ctrl+T; Ctrl+L
**Outputs:** panel visibility

**Edge cases**
- Toggling while floating hides and re-shows at the same floating position.

**Acceptance**
- First start: Tool Box, Color Box and Status Bar report checked.
- Ctrl+T: Tool Box reports unchecked and the tool box is not visible; active tool unchanged. Ctrl+T again: checked and visible.
- View > Status Bar twice: status bar hidden then shown.

---

## 25. view.zoom: Zoom levels: Normal, Large, Custom

Priority: must | Layer: behaviour | Inventory: menu.view.zoom.normal, menu.view.zoom.large, menu.view.zoom.custom, dialog.custom_zoom

Changes only the view magnification; each image pixel is drawn as a z x z block of screen pixels.

**Behaviour**
1. At zoom z (in percent / 100) image pixel (x,y) occupies screen pixels canvas_left + x*z .. canvas_left + x*z + z - 1 horizontally (same vertically). No smoothing.
2. ASSUMPTION: the menu label is 'Normal Size' (capital S). Follows the majority usage.
3. Normal Size sets zoom to 100% and unchecks Show Grid.
4. ASSUMPTION: Large Size sets zoom to 400%. Guess; no source states the factor.
5. ASSUMPTION: Custom... opens a dialog with radio choices 100%, 200%, 400%, 600%, 800%, the current zoom preselected, and OK/Cancel; OK applies the chosen zoom. Guess; the list is unknown.
6. On a zoom change the image pixel at the top-left of the visible area stays at the top-left where possible (scroll clamped to the canvas).
7. Zoom never changes pixels and never creates an undo step. Pointer events are converted to image pixels with floor((screen - canvas_origin) / z).

**Inputs:** View > Zoom; Ctrl+PageUp; Ctrl+PageDown; custom choice
**Outputs:** zoom level; view

**Edge cases**
- Choosing the current zoom changes nothing.

**Acceptance**
- Large Size: harness reports zoom 400; screen pixels canvas_left+4..canvas_left+7 on the first canvas row show the colour of image pixel (1,0).
- Custom..., choose 600%, OK: zoom 600. Cancel instead: zoom unchanged.
- At 400% a screen press at canvas offset (9,13) is reported as image pixel (2,3).
- Zoom changes never set the dirty flag.

---

## 26. view.grid: Show Grid

Priority: should | Layer: behaviour | Inventory: menu.view.zoom.show_grid, behaviour.zoom.grid_threshold

A check item that overlays pixel grid lines when the zoom is above 200%.

**Behaviour**
1. Show Grid toggles a check mark. Grid lines are drawn only when Show Grid is checked and zoom > 200% (so at 400%, 600%, 800%, including Large Size).
2. ASSUMPTION: grid lines are 1 screen pixel wide, colour (128,128,128), drawn on the leftmost screen column and topmost screen row of every image pixel's block. Guess; colour and placement are not documented.
3. The grid is a view overlay only; it never appears in the image, saved files or the clipboard.

**Inputs:** View > Zoom > Show Grid; Ctrl+G
**Outputs:** overlay

**Edge cases**
- Show Grid checked at 200%: no lines drawn, check mark kept.

**Acceptance**
- Fresh state (all W), 400%, Show Grid on: in the rendered view, screen pixel at block offset (0,0) of image pixel (3,3) is (128,128,128) and at block offset (1,1) is (255,255,255).
- Same at 200%: no rendered canvas pixel is (128,128,128).
- With grid shown, Save: the file's pixels are all W.

---

## 27. view.thumbnail: Show Thumbnail

Priority: should | Layer: behaviour | Inventory: menu.view.zoom.show_thumbnail, behaviour.zoom.thumbnail_window

A small separate window that shows the zoomed area at 100% while the main view is magnified.

**Behaviour**
1. Show Thumbnail is a check item, enabled only when zoom > 100%. When checked, a small floating window appears.
2. ASSUMPTION: the thumbnail window's client area is 200 x 150 screen pixels and shows the image at 100%, its top-left being the top-left image pixel visible in the main view; areas beyond the image show (128,128,128). Guess; size and overflow behaviour are not documented.
3. The thumbnail updates live while drawing. Its position is remembered across sessions.
4. ASSUMPTION: returning to 100% hides the thumbnail and unchecks Show Thumbnail. Guess.

**Inputs:** View > Zoom > Show Thumbnail
**Outputs:** thumbnail window

**Edge cases**
- Closing the thumbnail window with its close button unchecks the item.

**Acceptance**
- Fresh state, 400%, Show Thumbnail: a window with a 200x150 client area exists; its client pixel (5,5) equals image pixel (5,5).
- Pencil click (5,5) while it is shown: thumbnail client pixel (5,5) becomes K.
- Normal Size: the thumbnail window is gone and Show Thumbnail reports unchecked.

---

## 28. view.view_bitmap: View Bitmap

Priority: should | Layer: behaviour | Inventory: menu.view.view_bitmap

Shows the image alone, full screen at 100%, until any click or key.

**Behaviour**
1. View Bitmap shows a full-screen view containing only the image at 100%, without menus, panels or the window frame.
2. ASSUMPTION: an image smaller than the screen is centred (offset floor((screen - image)/2) on each axis) on a (0,0,0) surround; a larger image shows its top-left part from screen (0,0). Guess.
3. Any mouse button press or any key press returns to the normal window; that input does nothing else.

**Inputs:** View > View Bitmap; Ctrl+F
**Outputs:** full-screen view

**Edge cases**
- A floating selection is shown as committed in the full-screen view but is not committed in the document.

**Acceptance**
- On a 1024x768 screen with a 100x100 R image: full-screen pixel (462,334) is R and (0,0) is (0,0,0).
- A key press returns: the main window is shown and the document is unchanged.

---

## 29. image.flip_rotate: Flip and Rotate

Priority: must | Layer: behaviour | Inventory: menu.image.flip_rotate, dialog.flip_rotate, dialog.flip_rotate.flip_h, dialog.flip_rotate.flip_v, dialog.flip_rotate.rotate

Mirrors the image or selection horizontally or vertically, or rotates it by 90, 180 or 270 degrees.

**Behaviour**
1. ASSUMPTION: the dialog has three radio choices 'Flip horizontal', 'Flip vertical', 'Rotate by angle' (default: Flip horizontal) and, under the last, angle choices 90, 180, 270 degrees (default 90) enabled only when Rotate is chosen; buttons OK and Cancel. Labels and defaults are a guess.
2. Flip horizontal: new(x,y) = old(W-1-x, y). Flip vertical: new(x,y) = old(x, H-1-y).
3. ASSUMPTION: rotation is clockwise. Guess; direction is not stated. Rotate 90: new size H x W, new(x,y) = old(y, H-1-x). Rotate 180: new(x,y) = old(W-1-x, H-1-y). Rotate 270: new size H x W, new(x,y) = old(W-1-y, x).
4. Without a selection the whole image is transformed and the canvas takes the new size. With a selection, see feature selection.image_ops_scope.
5. OK creates one undo step; Cancel changes nothing.

**Inputs:** Image > Flip/Rotate; Ctrl+R
**Outputs:** pixels; canvas size

**Edge cases**
- Rotating a 1xN image by 90 gives an Nx1 image.

**Acceptance**
- 2x1 image [R at (0,0), G at (1,0)], Flip horizontal: (0,0)=G, (1,0)=R.
- Same image, Rotate 90: image is 1x2 with (0,0)=R and (0,1)=G.
- Same image, Rotate 270: image is 1x2 with (0,0)=G and (0,1)=R.
- 3x2 image, Rotate 180 then Undo: pixels identical to the original.

---

## 30. image.stretch_skew: Stretch and Skew

Priority: must | Layer: behaviour | Inventory: menu.image.stretch_skew, dialog.stretch_skew, dialog.stretch_skew.stretch_h, dialog.stretch_skew.stretch_v, dialog.stretch_skew.skew_h, dialog.stretch_skew.skew_v, dialog.stretch_skew.stretch_error, dialog.stretch_skew.skew_error, dialog.stretch_skew.whats_this

Resizes by integer percentages and shears by integer degrees, with nearest-neighbour sampling.

**Behaviour**
1. The dialog has a Stretch group with Horizontal and Vertical percent fields and a Skew group with Horizontal and Vertical degree fields, plus OK and Cancel.
2. ASSUMPTION: fields open with Stretch 100 / 100 and Skew 0 / 0. Guess; only an unsaved how-to suggests 100.
3. Valid input: stretch fields are integers 1..500; skew fields are integers 0..89. On OK with an invalid stretch value a message 'Enter a whole number from 1 to 500.' (own wording) is shown and the dialog stays open with that field focused; for skew the message is 'Enter a whole number from 0 to 89.' Negative numbers and non-integers are invalid.
4. Operations run in this order: horizontal stretch, vertical stretch, horizontal skew, vertical skew.
5. ASSUMPTION: stretched size W' = max(1, floor(W x p / 100 + 0.5)) (same for H); sampling is nearest-neighbour with source x = floor((x' + 0.5) x W / W'). Guess; resampling and rounding are undocumented.
6. ASSUMPTION: horizontal skew by angle a shifts row y right by s(y) = floor((H-1-y) x tan(a) + 0.5), so the top row moves furthest right and the bottom row stays; the width grows to W + s(0). Guess; the sign convention follows the documented note that negative values would skew left, which implies positive skews right.
7. ASSUMPTION: vertical skew by angle a shifts column x down by t(x) = floor((W-1-x) x tan(a) + 0.5), so the right column stays at the top and the left column moves furthest down; the height grows to H + t(0). Guess; follows the documented note that negative values would skew toward the bottom, so positive raises the right side relative to the left.
8. ASSUMPTION: area exposed by a skew is filled with the current background colour. Guess.
9. Without a selection the whole image is transformed and the canvas takes the result size. With a selection, see feature selection.image_ops_scope.
10. OK with values 100/100/0/0 changes nothing and creates no undo step; any other valid OK creates one undo step.
11. Right-clicking any field label shows a one-line help popup in the remake's own words stating the field's meaning and valid range (stretch 1-500, skew 0-89; negatives not allowed).

**Inputs:** Image > Stretch/Skew; Ctrl+W; four integers
**Outputs:** pixels; size

**Edge cases**
- Stretch 1% of a 50 px wide image gives width 1.
- Skew 89 on a tall image may exceed 9999 px: the not-enough-memory message is shown and nothing changes.

**Acceptance**
- 4x1 image [R,G,B,W], Stretch horizontal 50: image is 2x1 [G,W].
- 2x1 image [R,G], Stretch horizontal 200: image is 4x1 [R,R,G,G].
- 3x1 image, Stretch horizontal 50: width is 2.
- 3x3 all-K image, background W, Skew horizontal 45: image is 5x3; row 0 is [W,W,K,K,K], row 1 [W,K,K,K,W], row 2 [K,K,K,W,W].
- 3x3 all-K image, background W, Skew vertical 45: image is 3x5; column 2 is [K,K,K,W,W] top to bottom and column 0 is [W,W,K,K,K].
- Enter -5 in Stretch horizontal, OK: a message containing '1' and '500' is shown and the image is unchanged.
- Enter 90 in Skew vertical, OK: a message containing '0' and '89' is shown.

---

## 31. image.invert: Invert Colors

Priority: must | Layer: behaviour | Inventory: menu.image.invert_colors

Replaces each pixel by its RGB complement in the selection or the whole image.

**Behaviour**
1. ASSUMPTION: each affected pixel (r,g,b) becomes (255-r, 255-g, 255-b). Guess; the math is undocumented.
2. ASSUMPTION: in an indexed or Black-and-white image the inverted colour is then snapped to the image's palette (feature colors.indexed_snap). Guess.
3. With a selection only selected pixels (inside the free-form shape) change; otherwise the whole image. One undo step.

**Inputs:** Image > Invert Colors; Ctrl+I
**Outputs:** pixels

**Edge cases**
- Invert twice restores the original exactly in a 24-bit image.

**Acceptance**
- Pixel (10,200,255) becomes (245,55,0).
- Fresh state, select (0,0)-(1,1), Ctrl+I, click outside: (0,0)..(1,1) are K and (2,2) is W.

---

## 32. image.attributes: Attributes: size, units, colour mode

Priority: must | Layer: behaviour | Inventory: menu.image.attributes, dialog.attributes, dialog.attributes.width, dialog.attributes.height, dialog.attributes.units, dialog.attributes.colors, dialog.attributes.default, behaviour.palette.monochrome_mode

Sets the canvas size in pixels, inches or centimetres and switches between colour and black-and-white mode.

**Behaviour**
1. The dialog shows Width and Height fields, a Units choice (Inches, Cm, Pixels), a Colors choice (Black and white, Colors), and buttons OK, Cancel, Default.
2. ASSUMPTION: there is no transparency option. Follows the sources describing only size, units and colours; the source mentioning transparency describes later versions.
3. ASSUMPTION: inches and centimetres convert at 96 pixels per inch (2.54 cm per inch); displayed values are rounded half-up to 2 decimals; on OK a value v converts to max(1, floor(v x 96 + 0.5)) pixels for inches and max(1, floor(v / 2.54 x 96 + 0.5)) for cm. Guess; DPI and rounding are undocumented.
4. Switching units only changes the displayed numbers. Pixels is the default unit.
5. Valid size is 1..9999 pixels per side; an invalid entry shows 'Enter a size from 1 to 9999 pixels.' (own wording) and keeps the dialog open.
6. ASSUMPTION: on OK the canvas is cropped at the right/bottom when smaller and extended at the right/bottom when larger, with new pixels in the current background colour. Guess.
7. ASSUMPTION: choosing Black and white converts every pixel with the threshold rule (white when 299R + 587G + 114B >= 128000, else black); later painting snaps to black or white; the colour box shows each swatch as its black/white mapping. Choosing Colors again changes no pixels. No confirmation prompt is shown. Guess; conversion and colour-box contents are undocumented.
8. ASSUMPTION: Default sets the fields to 512 x 384 pixels, Units Pixels, Colors. Guess, consistent with core.document.
9. OK stores the size as the default for later New commands and creates one undo step if size or mode changed. Cancel changes nothing.

**Inputs:** Image > Attributes; Ctrl+E
**Outputs:** canvas size; colour mode

**Edge cases**
- Changing units then pressing OK without editing keeps the exact pixel size.

**Acceptance**
- 512x384 image, switch Units to Inches: fields show 5.33 and 4.00; to Cm: 13.55 and 10.16.
- Enter 1 inch x 1 inch, OK: image is 96x96.
- Fresh state, background R, Attributes 40x20: pixels (32..39, 0..19) are R; image is 40x20.
- Pixel (200,100,50) becomes K and pixel (128,128,128) becomes W after choosing Black and white and OK.
- In Black-and-white mode with foreground (255,0,0), Pencil click (1,1): pixel (1,1) is K.
- Default then OK on a 32x32 image: image is 512x384.

---

## 33. image.clear_image: Clear Image

Priority: must | Layer: behaviour | Inventory: menu.image.clear_image

Fills the whole image with the background colour, keeping its size and the current colours.

**Behaviour**
1. Every pixel becomes the current background colour; size, colours, palette and path are unchanged. One undo step.
2. ASSUMPTION: an active selection is discarded first (a floating one is not committed) and the whole image is cleared, not just the selection. Guess.

**Inputs:** Image > Clear Image; Ctrl+Shift+N
**Outputs:** pixels

**Edge cases**
- none

**Acceptance**
- Fresh state, Pencil marks, background B, Clear Image: all pixels B; foreground still K; background still B.
- Clear Image then Undo: marks restored.

---

## 34. canvas.resize_handle: Canvas resize handles

Priority: must | Layer: behaviour | Inventory: behaviour.canvas.resize_handle

Dragging handles at the canvas edges enlarges or crops the canvas.

**Behaviour**
1. ASSUMPTION: there are three handles: bottom-right corner (changes width and height), right-edge midpoint (width only), bottom-edge midpoint (height only). Only the corner handle is documented; the edge handles are a guess.
2. Dragging a handle by (dx,dy) image pixels sets the new size to (W+dx, H+dy) on the affected axes, clamped to 1..9999. While dragging, a dashed outline shows the new size and the status bar size pane shows 'W,H'.
3. ASSUMPTION: on release, newly exposed pixels are the current background colour and pixels beyond the new size are cropped. Guess.
4. A resize is one undo step and sets the dirty flag.

**Inputs:** handle drag
**Outputs:** canvas size

**Edge cases**
- Release with no net change does nothing.

**Acceptance**
- Fresh state, background G, drag the corner handle by (+8,+4): image is 40x36; pixels (32..39, 0..35) and (0..39, 32..35) are G.
- Fresh state, drag the right-edge handle by (-30,+10): image is 2x32.
- Undo after a handle resize restores 32x32 with the original pixels.

---

## 35. tools.toolbox: Tool box, options box and startup tool

Priority: must | Layer: ui | Inventory: ui.toolbox, behaviour.toolbox.default_tool, behaviour.toolbox.tooltips

Sixteen tool buttons in a 2 x 8 grid with a tool-specific options box; Pencil is active at start.

**Behaviour**
1. Buttons, row by row (left, right): Free-Form Select, Select; Eraser/Color Eraser, Fill With Color; Pick Color, Magnifier; Pencil, Brush; Airbrush, Text; Line, Curve; Rectangle, Polygon; Ellipse, Rounded Rectangle. The active tool's button is shown pressed; exactly one is active. Button artwork is the remake's own.
2. Below the buttons an options box shows the active tool's options: selection tools and Text: opaque/transparent; Eraser: 4 sizes; Magnifier: 4 levels; Brush: 12 shapes; Airbrush: 3 sizes; Line and Curve: 5 widths; Rectangle, Polygon, Ellipse, Rounded Rectangle: 3 fill styles; Fill, Pick Color, Pencil: empty. Each option set keeps its value when switching tools.
3. Pencil is the active tool at startup.
4. Selecting a tool commits any floating selection, pending text frame, curve or polygon first.
5. The tool box can be dragged off to float and back to dock.
6. ASSUMPTION: hovering a tool button for 500 ms shows a tooltip with its name exactly as listed above (Eraser shown as 'Eraser/Color Eraser'), and the status bar's message pane shows a one-line description in the remake's own words. Guess; tooltips are undocumented.

**Inputs:** button click
**Outputs:** active tool; options

**Edge cases**
- The Text button is disabled when zoom > 100%.

**Acceptance**
- First start: active tool is Pencil.
- Harness lists the 16 tool names in row-major order exactly as above.
- Select Brush, choose shape 6, select Line, select Brush: brush shape is still 6.
- Hover Fill With Color for 600 ms: tooltip text is 'Fill With Color'.

---

## 36. tools.mouse_colors: Mouse buttons and colours

Priority: must | Layer: behaviour | Inventory: behaviour.mouse.buttons, behaviour.mouse.ctrl_third_color

Left button paints with the foreground colour, right button with the background colour.

**Behaviour**
1. Pencil, Brush, Airbrush, Line, Curve and Fill paint with the foreground colour when started with the left button and with the background colour when started with the right button. Shape tools follow tool.shapes; Eraser follows tool.eraser.
2. The button that starts a stroke decides its colour for the whole stroke; pressing the other button during a stroke cancels the stroke (pixels restored, no undo step).
3. ASSUMPTION: there is no third colour; holding Ctrl while painting gives the same result as without Ctrl (except where a feature gives Ctrl a meaning, e.g. selection stamping). Guess; the only hint of a Ctrl colour is an unverifiable snippet.

**Inputs:** left/right button; Ctrl
**Outputs:** paint colour

**Edge cases**
- Right-click with a selection tool does nothing.

**Acceptance**
- Fresh state, background R, Pencil right-drag (2,2)->(6,2): pixels (2..6,2) are R.
- Fresh state, Pencil Ctrl+left click (3,3): pixel (3,3) is K.
- Fresh state, Pencil left press (1,1), move (5,1), press right, release both: all pixels W and Undo disabled.

---

## 37. tool.select: Select and Free-Form Select

Priority: must | Layer: behaviour | Inventory: tool.free_form_select, tool.select, behaviour.selection.outline, behaviour.selection.deselect

Creates a rectangular or free-form selection shown with a dashed bounding box and eight handles; only one selection exists at a time.

**Behaviour**
1. Select: a drag from A to B selects the inclusive rectangle between them, clamped to the canvas. A click without movement selects nothing.
2. Free-Form Select: while dragging, the traced path is shown; on release the path is closed back to its start. The selection mask contains every pixel whose centre lies inside the closed path (even-odd rule) plus every pixel on the traced path. Its bounds are the mask's bounding box. A trace touching fewer than 2 distinct pixels selects nothing.
3. An active selection shows a dashed rectangle around its bounds and 8 square handles (4 corners, 4 edge midpoints) of 5 x 5 screen pixels; the handles are hit-tested within 3 screen pixels.
4. The selection ends (floating pixels merged into the image where they lie) when the user clicks outside it, starts a new selection, picks another tool, or presses Esc. Starting a new selection ends the previous one first.
5. Making a selection changes no pixels and creates no undo step.

**Inputs:** drag on canvas
**Outputs:** selection bounds and mask

**Edge cases**
- A drag starting outside the canvas is clamped to its edge.
- A free-form path that crosses itself uses the even-odd rule.

**Acceptance**
- Fresh state, Select drag (5,6)->(2,3): selection bounds (2,3)-(5,6), mask 4x4 all selected.
- Fresh state, Free-Form Select trace (2,2)->(12,2)->(12,12)->(2,12)->(2,2): bounds (2,2)-(12,12), mask contains (7,7) and (2,2), not (13,13).
- With a selection, click outside it: no selection exists and pixels unchanged.
- Making a selection leaves Undo disabled on a fresh state.

---

## 38. selection.transparency: Opaque and transparent selection mode, Draw Opaque

Priority: must | Layer: behaviour | Inventory: tool.selection.mode.opaque, tool.selection.mode.transparent, menu.image.draw_opaque

In transparent mode, selection and text-frame pixels equal to the background colour are not drawn.

**Behaviour**
1. One shared setting with two options (upper: opaque, lower: transparent) appears in the options box of Select, Free-Form Select and Text, and as the Image > Draw Opaque check item (checked = opaque). Changing any one changes all. Default: opaque.
2. Opaque: floating selection pixels are drawn and committed as they are.
3. Transparent: floating pixels exactly equal (tolerance 0) to the current background colour are neither shown nor committed; the image below shows through. The comparison uses the background colour at the time of display or commit.
4. ASSUMPTION: for Text, opaque fills the whole frame rectangle with the background colour behind the text; transparent draws only the glyph pixels. Guess; whether opaque fills the whole frame or only behind glyphs is undocumented.

**Inputs:** option click; Image > Draw Opaque
**Outputs:** mode

**Edge cases**
- Switching the mode while a selection floats updates its display immediately.

**Acceptance**
- Image > Draw Opaque reports checked at start.
- Click the transparent option: Draw Opaque reports unchecked.
- Fresh state: fill (10..19,0..9) with R; select (0,0)-(9,9) (all W) with Pencil dot K at (5,5) inside; transparent mode; drag the selection by (+10,0); click outside: pixel (15,5) is K and (16,6) is R.
- Same in opaque mode: (16,6) is W.

---

## 39. selection.move_copy: Moving, stamping and smearing a selection

Priority: must | Layer: behaviour | Inventory: behaviour.selection.move, behaviour.selection.stamp, behaviour.selection.trail

Dragging inside a selection moves it; Ctrl leaves copies; Shift leaves a trail.

**Behaviour**
1. Pressing inside the selection bounds (not on a handle) and dragging moves the selection by the pointer delta in image pixels. The content floats above the image until the selection ends.
2. ASSUMPTION: the first time a fresh (not yet floating) selection is moved, its source pixels (mask only) are set to the current background colour. Guess; sources do not say what is left behind.
3. Ctrl held at the press: before moving, a copy of the floating content is committed at its current position; for a fresh selection the source pixels are left unchanged instead of being cleared. Each further Ctrl-drag stamps another copy.
4. Shift held during the drag: at every pointer move event the content is committed at its current position, leaving a smeared trail.
5. Each such drag (press to release) is one undo step.

**Inputs:** drag inside selection; Ctrl; Shift
**Outputs:** pixels; selection position

**Edge cases**
- Moving content beyond the canvas keeps it floating; only in-canvas pixels are committed.

**Acceptance**
- Fresh state, Pencil dot K at (2,2), Select (1,1)-(3,3), drag from (2,2) to (12,2), click outside: (12,2) is K, (2,2) is W.
- Same but with background R before the move: pixels (1..3,1..3) are R after the move.
- Same with Ctrl held at the press: (2,2) and (12,2) are both K.
- Fresh state, dot K at (2,2), Select (2,2)-(2,2), Shift-drag (2,2)->(4,2)->(6,2) with move events at x=3,4,5,6, then click outside: (3,2),(4,2),(5,2),(6,2) are K.

---

## 40. selection.resize: Stretching a selection with its handles

Priority: must | Layer: behaviour | Inventory: behaviour.selection.resize

Dragging a handle scales the selection non-proportionally with nearest-neighbour sampling.

**Behaviour**
1. Dragging a corner handle moves that corner; an edge handle moves only that edge; the opposite corner or edge stays fixed. The new size is clamped to at least 1 x 1.
2. Content is resampled nearest-neighbour from the content as it was at the start of this handle drag, using the stretch sampling rule of image.stretch_skew.
3. A fresh selection becomes floating first (source cleared to background as in selection.move_copy).

**Inputs:** handle drag
**Outputs:** floating content size

**Edge cases**
- Dragging a handle past the opposite edge clamps at size 1.

**Acceptance**
- Fresh state, 2x1 content [R,G] at (0,0)-(1,0) selected; drag the right-edge handle so the bounds become (0,0)-(3,0); commit: pixels (0..3,0) are [R,R,G,G].

---

## 41. selection.image_ops_scope: Image commands on a selection

Priority: must | Layer: behaviour | Inventory: behaviour.selection.image_ops_scope

Flip/Rotate, Stretch/Skew and Invert Colors act on the selection when one exists, otherwise on the whole image.

**Behaviour**
1. With a selection, the selection becomes floating (source cleared to background if fresh), the transform is applied to its content and mask, and the result stays floating with its top-left at the selection's previous top-left. The canvas size does not change.
2. Pixels of the transformed content outside the mask, and skew-exposed pixels, are background colour and follow the opaque/transparent rule.
3. Invert Colors with a selection changes only masked pixels in place and does not float the selection.

**Inputs:** image command with selection
**Outputs:** floating content

**Edge cases**
- Rotating a non-square selection by 90 swaps its bounds' width and height around the same top-left.

**Acceptance**
- Fresh state, (0,0)=R, (1,0)=G, select (0,0)-(1,0), Flip horizontal, commit: (0,0)=G, (1,0)=R, image still 32x32.
- Fresh state, select (0,0)-(3,1), Rotate 90: selection bounds are (0,0)-(1,3).

---

## 42. tool.eraser: Eraser / Color Eraser

Priority: must | Layer: behaviour | Inventory: tool.eraser, tool.eraser.size, tool.eraser.color_replace, behaviour.eraser.color_eraser_24bit_quirk

Left-drag paints a square of background colour; right-drag replaces only foreground-coloured pixels with the background colour.

**Behaviour**
1. ASSUMPTION: the four eraser sizes are squares of side 4, 6, 8 and 10 pixels, in that order in the options box; default 8. Guess; pixel sizes are undocumented.
2. A square of side n covers offsets dx, dy in [-floor(n/2), ceil(n/2)-1] around the pointer pixel (n=8: -4..3).
3. Left drag: the square is stamped at every pixel of the 1-pixel path (line rule of tool.line) between consecutive pointer positions, setting covered pixels to the background colour.
4. Right drag (colour eraser): within the stamped squares, only pixels exactly equal to the foreground colour become the background colour; all others are untouched.
5. The colour eraser works on images of every depth, including those loaded from 24-bit files (a known flaw of the original where it did nothing on such images is not reproduced).
6. One press-to-release is one undo step.

**Inputs:** drag; size option
**Outputs:** pixels

**Edge cases**
- A click without movement stamps once.

**Acceptance**
- Fresh state with all pixels K, background W, default size, click (10,10): pixels (6..13,6..13) are W, (5,10) and (14,10) are K.
- Size 4, click (10,10) on all-K: exactly the 16 pixels (8..11,8..11) are W.
- Fresh state: (5,5)=K, (6,5)=R; foreground K, background W; right-drag size 10 from (5,5) to (6,5): (5,5)=W, (6,5)=R.

---

## 43. tool.fill: Fill With Color

Priority: must | Layer: behaviour | Inventory: tool.fill

Floods the contiguous area of the clicked colour with the foreground (left) or background (right) colour.

**Behaviour**
1. ASSUMPTION: the fill region is the set of pixels 4-connected (up, down, left, right) to the clicked pixel that have exactly the clicked pixel's RGB colour (tolerance 0). Guess; connectivity and tolerance are undocumented. Dithered areas are treated as separate colours, so a fill stops at every differing pixel.
2. Every pixel of the region becomes the paint colour. Enclosed areas of other colours are untouched; a gap in a boundary lets the fill through; an unbounded area fills to the canvas edges.
3. If the paint colour equals the clicked colour nothing changes and no undo step is created. Otherwise one undo step.
4. Clicks outside the canvas do nothing. Fill has no options.

**Inputs:** click; button
**Outputs:** pixels

**Edge cases**
- In an indexed image the paint colour is snapped first (colors.indexed_snap).

**Acceptance**
- 10x10 W image with K pixels at (i, 9-i) for i=0..9, foreground R, click (0,0): every pixel with x+y<9 is R and (9,9) is W.
- 10x10 W image with (5,5)=(254,255,255), foreground R, click (0,0): (5,5) is unchanged and all other pixels are R.
- Fresh state, background B, right-click (3,3): all pixels B.
- Fresh state, foreground W, click (3,3): Undo stays disabled.

---

## 44. tool.pick_color: Pick Color

Priority: must | Layer: behaviour | Inventory: tool.pick_color

Takes a pixel's colour as the foreground (left) or background (right) colour, then returns to the previous tool.

**Behaviour**
1. On release of a left click on the canvas the foreground colour becomes the clicked pixel's colour; a right click sets the background colour.
2. After the click the previously active tool becomes active again; if there was none (Pick Color chosen at start), Pencil.
3. Picking changes no pixels and creates no undo step. Clicks outside the canvas do nothing and keep Pick Color active.

**Inputs:** click
**Outputs:** foreground/background colour; active tool

**Edge cases**
- none

**Acceptance**
- Fresh state, (4,4)=(12,34,56), select Brush then Pick Color, click (4,4): foreground is (12,34,56) and active tool is Brush.
- Right-click instead: background is (12,34,56), foreground still K.

---

## 45. tool.magnifier: Magnifier

Priority: must | Layer: behaviour | Inventory: tool.magnifier, tool.magnifier.level

Zooms the view onto a chosen area; the options box offers four levels.

**Behaviour**
1. ASSUMPTION: the options box lists 1x, 2x, 6x, 8x; the option equal to the current zoom is highlighted (none at 400%); clicking an option sets that zoom immediately. Follows the tutorial's list over sources mentioning 4x or 'up to 8x'.
2. ASSUMPTION: at 100% the pointer over the canvas shows a rectangle of floor(view width / 4) x floor(view height / 4) image pixels (view size in screen pixels); clicking zooms to 400% with the rectangle's top-left at the view's top-left (clamped). Guess, combining the source saying a click usually zooms to 4x.
3. ASSUMPTION: after the zoom-in click the previously active tool is reselected; choosing Magnifier again and clicking the canvas while zoomed returns to 100%. Reconciles the two conflicting sources (one says the tool reverts, the other says clicking again returns to normal size).
4. The magnifier never changes pixels and never creates an undo step.

**Inputs:** click; level option
**Outputs:** zoom; scroll; active tool

**Edge cases**
- Right-click with the magnifier does the same as left-click.

**Acceptance**
- Fresh state, select Pencil, Magnifier, click the canvas: zoom is 400 and active tool is Pencil.
- Then select Magnifier and click the canvas: zoom is 100.
- Magnifier option 6x: zoom is 600 and the 6x option is highlighted.
- Pixels unchanged and Undo disabled after all of the above.

---

## 46. tool.pencil: Pencil

Priority: must | Layer: behaviour | Inventory: tool.pencil, behaviour.constrain.pencil

Draws a 1-pixel freehand line following the pointer.

**Behaviour**
1. On press the pointer pixel is painted; on each move the 1-pixel path (line rule of tool.line) from the previous to the new position is painted. Left = foreground, right = background.
2. Parts of the path outside the canvas are not drawn; the stroke continues when the pointer returns.
3. With Shift held at the press, the stroke is a straight line from the press point to the pointer snapped to 0/45/90 degrees (snap rule of tool.line), shown live and committed on release.
4. Pencil has no options. One press-to-release is one undo step.

**Inputs:** drag
**Outputs:** pixels

**Edge cases**
- A click paints exactly one pixel.

**Acceptance**
- Fresh state, click (7,7): exactly one pixel K.
- Fresh state, drag (0,0)->(6,2): K pixels are exactly (0,0),(1,0),(2,1),(3,1),(4,1),(5,2),(6,2).
- Fresh state, Shift-drag (10,10)->(30,13): K pixels are exactly (10..30,10).

---

## 47. tool.brush: Brush and its 12 shapes

Priority: must | Layer: behaviour | Inventory: tool.brush, tool.brush.shape, behaviour.constrain.brush

Freehand painting that stamps the selected brush footprint along the pointer path.

**Behaviour**
1. ASSUMPTION: the 12 shapes, in options-box order (4 rows of 3, read row by row): 1 round 7; 2 round 5; 3 round 1 (single-pixel pinpoint); 4 square 8; 5 square 5; 6 square 2; 7 ascending slant 8; 8 ascending slant 5; 9 ascending slant 2; 10 descending slant 8; 11 descending slant 5; 12 descending slant 2. Default shape 2. Guess; footprints and order are undocumented beyond 'round, square, slanted both ways, several sizes'.
2. ASSUMPTION: round d (d odd) covers offsets with dx^2 + dy^2 <= r^2 + r where r = (d-1)/2 (round 7 = 37 pixels, round 5 = 21 pixels, round 1 = 1 pixel). Square n covers dx, dy in [-floor(n/2), ceil(n/2)-1]. Ascending slant n covers (i - floor(n/2), floor(n/2) - i) for i = 0..n-1 (bottom-left to top-right). Descending slant n covers (i - floor(n/2), i - floor(n/2)) for i = 0..n-1 (top-left to bottom-right). Guess.
3. The footprint is stamped at every pixel of the 1-pixel path (line rule of tool.line) between consecutive pointer positions. Left = foreground, right = background.
4. With Shift held at the press, after the first move the axis with the larger absolute distance from the press point is locked: the stroke stays on the press row (horizontal) or column (vertical). There is no 45-degree constraint.
5. One press-to-release is one undo step.

**Inputs:** drag; shape option
**Outputs:** pixels

**Edge cases**
- A slant brush dragged parallel to its slant produces a 1-pixel-wide line.

**Acceptance**
- Fresh state, shape 1, click (10,10): exactly 37 pixels K, all within (7..13,7..13); (12,12) is K, (13,12) is W.
- Shape 2, click (10,10): exactly 21 pixels K; (12,11) is K, (12,12) is W.
- Shape 4, click (10,10): K pixels are exactly (6..13,6..13).
- Shape 7, click (10,10): K pixels are exactly (6,14),(7,13),(8,12),(9,11),(10,10),(11,9),(12,8),(13,7).
- Shape 3, Shift-drag (5,5)->(20,8): K pixels are exactly (5..20,5).

---

## 48. tool.airbrush: Airbrush

Priority: must | Layer: behaviour | Inventory: tool.airbrush, tool.airbrush.size

While the button is held, sprays random single pixels inside a disc around the pointer on a fixed timer.

**Behaviour**
1. ASSUMPTION: three sizes with disc radius 4, 8 and 12 pixels (disc = offsets with dx^2 + dy^2 <= r^2: 49, 197 and 441 pixels); default radius 4. Guess.
2. ASSUMPTION: spraying happens in ticks: one tick at the press, then one every 50 ms of (virtual) time while the button is held. Pointer moves do not cause extra ticks, so fast movement leaves a sparse streak and slow movement a dense one. Guess.
3. ASSUMPTION: each tick paints 2r dots (8, 16 or 24) at the current pointer position. Each dot picks dx, dy uniformly in [-r, r] and retries until dx^2 + dy^2 <= r^2 (uniform over the disc); dots may land on already painted pixels. Guess; the original pattern is undocumented.
4. The random generator is seedable by the harness; acceptance relies only on properties that hold for any seed.
5. Left = foreground, right = background. One press-to-release is one undo step.

**Inputs:** press/hold/move; size option; virtual clock
**Outputs:** pixels

**Edge cases**
- Dots outside the canvas are discarded but still count toward the tick's total.

**Acceptance**
- Fresh state, radius 4, press and release at (15,15) at the same instant: 1..8 pixels are K, all with (x-15)^2 + (y-15)^2 <= 16.
- Press at (15,15), hold 1000 ms without moving, release: 21 ticks happened; at most 168 pixels are K; none outside the radius-4 disc.
- Press at (15,15), hold 10 s: all 49 disc pixels are K (for every seed tested, 100 seeds).
- Radius 12, hold 500 ms at (15,15): no K pixel has (x-15)^2 + (y-15)^2 > 144.

---

## 49. tool.text: Text tool and text toolbar

Priority: must | Layer: behaviour | Inventory: tool.text, behaviour.text.zoom_disabled, behaviour.text.toolbar_context, behaviour.text.undo_paste_quirk, menu.view.text_toolbar, dialog.text_toolbar

Typed text in a frame, styled by a floating toolbar, becomes ordinary pixels once committed.

**Behaviour**
1. Dragging creates a text frame over the inclusive drag rectangle.
2. ASSUMPTION: a click without drag creates a frame 100 pixels wide and one text line high at the click point. Guess.
3. Typed text appears in the foreground colour. While the frame is active: its border can be dragged to move it, its handles resize it, changing the foreground colour recolours all its text, toolbar changes restyle all its text, and text editing keys (arrows, Backspace, Delete, Enter, Ctrl+X/C/V/A/Z) work on the text.
4. ASSUMPTION: text wraps at word boundaries to the frame width, and the frame grows downward to fit its lines (not beyond the canvas bottom). Guess.
5. Text renders without anti-aliasing: a glyph pixel is set when its coverage is at least 50%. Frame background follows selection.transparency (opaque fills the whole frame with the background colour).
6. The frame is committed (text becomes pixels, one undo step) when the user clicks outside it, selects another tool, changes zoom, or presses Esc. Committed text cannot be edited again. An empty frame commits nothing.
7. Only text can be pasted into a frame; Paste never inserts an image while the Text tool is active.
8. ASSUMPTION: the text toolbar is a floating panel with a font name list (the remake's bundled sans-serif font first and default, then installed fonts), a size list (8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72 points, default 10, converted at 96 DPI so 10 pt = 13 px), and Bold, Italic and Underline toggles (default off). Guess; layout and defaults are undocumented.
9. View > Text Toolbar is enabled only while the Text tool is active, checked by default, and toggles the toolbar. When a frame is created and the item is checked, the toolbar is shown.
10. Right-clicking an active frame shows a context menu with Cut, Copy, Paste and Text Toolbar; Text Toolbar there toggles the toolbar like the View item.
11. Pressing Esc while the toolbar's font size list has focus returns focus to the frame; the app keeps running (no crash).
12. When zoom is above 100% the Text tool is disabled. If it is active when zoom rises above 100%, any frame is committed and the previously active tool (or Pencil) becomes active.
13. Undo inside an active frame undoes the last text edit. Pasting text over selected text and then undoing restores the replaced text (a documented flaw of the original that lost it is not reproduced).

**Inputs:** drag; typing; toolbar
**Outputs:** text pixels

**Edge cases**
- Typing past the canvas bottom: lines beyond the canvas are clipped at commit.
- Help and dialogs do not discard or commit an active frame.

**Acceptance**
- Fresh state, background G, opaque, drag (10,10)->(109,49), type 'A', click (0,0): every pixel in (10..109,10..49) is K or G with at least one K; every pixel outside is W.
- Same in transparent mode: every pixel in the frame is K or W with at least one K; outside unchanged.
- While the frame is active set foreground R, then commit: no pixel is K and at least one is R.
- At 400% zoom the Text tool button reports disabled.
- Type 'abc', select 'b', paste 'XY', Ctrl+Z: frame text is 'abc'.
- After commit, Undo removes all text pixels in one step.

---

## 50. tool.line_width: Line width option and width keys

Priority: must | Layer: behaviour | Inventory: tool.line.width, shortcut.ctrl_numpad_plus, shortcut.ctrl_numpad_minus

Five widths shared by Line, Curve and all shape borders, with keys to step the width.

**Behaviour**
1. ASSUMPTION: the five widths are 1, 2, 3, 4 and 5 pixels, thin to thick; default 1. Guess; only 1 px and about 5 px are measurable in an example.
2. ASSUMPTION: a width-w stroke stamps a w x w square (offsets [-floor(w/2), ceil(w/2)-1]) at every pixel of the 1-pixel centre path. Ends are therefore square caps extending floor(w/2) pixels before the start and ceil(w/2)-1 after the end along each axis; corners of polygons and curves are joined by the union of squares (no rounding). Guess; caps and joins are undocumented.
3. The width is chosen in the Line or Curve options box and is used by Line, Curve, Rectangle, Polygon, Ellipse and Rounded Rectangle borders.
4. ASSUMPTION: Ctrl+NumPad+ selects the next thicker width and Ctrl+NumPad- the next thinner, one step at a time, clamped to 1..5; they act when the active tool is Line, Curve or a shape tool, also during a drag (the live preview redraws), and do nothing for other tools. Guess; step, limits and affected tools are undocumented.

**Inputs:** width option; Ctrl+NumPad+; Ctrl+NumPad-
**Outputs:** current width

**Edge cases**
- Ctrl+NumPad+ at width 5 keeps 5.

**Acceptance**
- Default width is 1.
- Line tool, Ctrl+NumPad+ twice: width 3. Ctrl+NumPad- five times: width 1.
- Brush active, Ctrl+NumPad+: line width unchanged.

---

## 51. tool.line: Line

Priority: must | Layer: behaviour | Inventory: tool.line, behaviour.constrain.line

Draws a straight line between press and release points in the current width.

**Behaviour**
1. During the drag a live preview is shown; on release the line is committed. Left = foreground, right = background. One undo step.
2. 1-pixel path rule (used by every tool that draws paths): both endpoints are included; along the major axis (larger of |dx|, |dy|) exactly one pixel per step, giving max(|dx|,|dy|)+1 pixels; the minor coordinate is the ideal line's value rounded to nearest, ties rounded toward the start point's coordinate.
3. Snap rule with Shift: let dx, dy be the offset from the start. If |dy| < 0.4142 |dx| the end is (x0+dx, y0); else if |dx| < 0.4142 |dy| the end is (x0, y0+dy); else the end is (x0 + sign(dx) m, y0 + sign(dy) m) with m = min(|dx|,|dy|).
4. Width follows tool.line_width.

**Inputs:** drag; Shift; width
**Outputs:** pixels

**Edge cases**
- Press and release on the same pixel draws a single w x w square.

**Acceptance**
- Fresh state, drag (0,0)->(6,2): K pixels exactly (0,0),(1,0),(2,1),(3,1),(4,1),(5,2),(6,2).
- Fresh state, width 3, drag (5,10)->(15,10): K pixels exactly (4..16, 9..11), 39 pixels.
- Fresh state, Shift-drag (10,10)->(30,25): K pixels exactly (10+k,10+k) for k=0..15.
- Fresh state, Shift-drag (10,10)->(12,30): K pixels exactly (10, 10..30).
- Fresh state, background R, right-drag (1,1)->(1,5): pixels (1,1..5) are R.

---

## 52. tool.curve: Curve

Priority: must | Layer: behaviour | Inventory: tool.curve, behaviour.constrain.curve

A straight line bent by up to two control points, drawn as a cubic curve in the current width.

**Behaviour**
1. Step 1: drag from P0 to P3 draws a straight preview line. The button used here sets the colour for the whole curve.
2. Step 2: the next press (and drag) sets both control points C1 = C2 = pointer, with a live preview; release fixes it.
3. Step 3: the next press (and drag) sets C2 = pointer, live; release commits the curve (one undo step). At most two bends exist.
4. ASSUMPTION: a curve with one bend is finished by a second click without moving at the same point as the first bend (a double-click), which leaves C2 = C1. Follows the reading that the curve is done after two bends, with a same-spot second click acting as the finishing double-click.
5. Curve shape: cubic Bezier B(t) with control points P0, C1, C2, P3, evaluated at t = i/64 for i = 0..64, each point rounded to the nearest pixel (halves rounded up), consecutive points joined by the 1-pixel path rule, then stamped with the width square.
6. A loop is made by pressing at P0 in step 2 and dragging away.
7. With Shift held during step 1 the line is horizontal or vertical only (the axis with the larger |delta|).
8. ASSUMPTION: selecting another tool during steps 2-3 commits the curve as currently shown; Esc discards it with no undo step. Guess; mid-curve tool change and Esc are undocumented.

**Inputs:** drags/clicks; Shift
**Outputs:** pixels

**Edge cases**
- Undo during steps 2-3 follows edit.undo_repeat.

**Acceptance**
- Fresh state 64x32, drag (10,20)->(50,20), click (30,0), click (30,0): K pixels include (10,20), (50,20) and (30,5); no K pixel has y < 5.
- Fresh state, drag (10,20)->(30,20), click (20,20), click (20,20): K pixels exactly (10..30,20).
- Fresh state, drag (10,20)->(30,20), then press Esc: all pixels W and Undo disabled.
- Shift-drag (5,5)->(25,9) in step 1: preview pixels are exactly (5..25,5).

---

## 53. tool.shapes: Rectangle, Ellipse, Rounded Rectangle and fill styles

Priority: must | Layer: behaviour | Inventory: tool.rectangle, tool.ellipse, tool.rounded_rectangle, tool.shape.fill_style, behaviour.shape.colors, behaviour.constrain.shapes

Drag-defined shapes inside the inclusive drag rectangle, drawn with the current border width and one of three fill styles.

**Behaviour**
1. The shape occupies the inclusive rectangle between press and release; nothing is drawn outside it. Live preview while dragging; one undo step on release.
2. ASSUMPTION: fill styles are 1 outline only (interior untouched, default), 2 outline plus interior filled, 3 interior only with no outline, where style 3 covers the whole shape area. Follows the contemporaneous book (also consistent with a tutorial) over a source saying styles 2 and 3 differ only in fill colour.
3. Left drag: outline in foreground, style-2 interior in background. Right drag: outline in background, interior in foreground.
4. ASSUMPTION: style 3 paints the whole shape area in the outline colour of the button used (foreground for left, background for right). Guess; sources disagree.
5. Rectangle: the outline is the outer w pixel rows and columns of the rectangle (w = current width); the interior is the rectangle inset by w on each side.
6. Ellipse: the 1-pixel centre outline is the ellipse inscribed in the drag rectangle inset by floor(w/2) at left/top and ceil(w/2)-1 at right/bottom, stamped with the w x w square; the interior is every pixel whose centre lies inside the ideal ellipse and is not an outline pixel. The result is mirror-symmetric about the rectangle's centre lines.
7. ASSUMPTION: Rounded Rectangle corners are quarter-ellipses with horizontal radius min(8, floor(width/2)) and vertical radius min(8, floor(height/2)) pixels, independent of border width; straight edges join them. Guess; the radius is undocumented.
8. With Shift held the shape is a square: side extent m = min(|dx|,|dy|) from the press point in the drag's direction.
9. Polygon uses the same fill styles and colours (tool.polygon).

**Inputs:** drag; fill style; width; button; Shift
**Outputs:** pixels

**Edge cases**
- A 1-pixel-wide drag draws a 1-pixel line for every shape.
- Border width larger than half the shape fills it entirely with the outline colour.

**Acceptance**
- Fresh state, Rectangle drag (2,2)->(7,5): K pixels exactly the 16 border pixels of (2,2)-(7,5); (4,3) is W.
- Fresh state, width 2, Rectangle drag (0,0)->(9,9): 64 pixels K; (2..7,2..7) W.
- Fresh state, background R, style 2, Rectangle drag (0,0)->(9,9): border K, (1..8,1..8) R.
- Fresh state, style 3, Rectangle drag (0,0)->(9,9): (0..9,0..9) all K.
- Fresh state, background R, style 2, right-drag (0,0)->(9,9): border R, interior K.
- Fresh state 32x32, Ellipse drag (0,0)->(20,10): (10,0),(10,10),(0,5),(20,5) are K; (10,5),(0,0),(20,0),(0,10),(20,10) are W; the set of K pixels is symmetric about x=10 and about y=5.
- Fresh state 64x64, Rounded Rectangle drag (0,0)->(39,29): (0,0) and (39,29) are W; (20,0) and (0,15) are K.
- Fresh state, Shift Rectangle drag (2,2)->(12,7): K pixels are exactly the border of (2,2)-(7,7).

---

## 54. tool.polygon: Polygon

Priority: must | Layer: behaviour | Inventory: tool.polygon, behaviour.constrain.polygon

A closed shape built edge by edge with clicks, closed by double-click or by clicking the first vertex.

**Behaviour**
1. The first edge is drawn by dragging from V0 to V1; the button used sets the colours for the whole polygon (as in tool.shapes).
2. Each further click (or press-drag, vertex placed at release) adds a vertex joined to the previous one; edges preview live.
3. ASSUMPTION: the polygon closes when the user double-clicks (second press within 500 ms and within 4 screen pixels of the first; the vertex is added once) or when a click releases exactly on V0's pixel. Follows both conflicting sources: double-click closes, and clicking precisely on the first vertex also closes.
4. On close an edge to V0 is added, the fill style is applied (interior: pixel centres inside the vertex path by the even-odd rule), the outline is stamped with the current width on top, and one undo step is created.
5. With Shift held, each edge is snapped from the previous vertex by the snap rule of tool.line.
6. ASSUMPTION: selecting another tool before closing closes and commits the polygon; Esc discards it with no undo step. Guess; undocumented.

**Inputs:** drag/clicks; Shift; fill style; width
**Outputs:** pixels

**Edge cases**
- A double-click right after the first edge makes a degenerate two-vertex polygon (a line).

**Acceptance**
- Fresh state, drag (2,2)->(12,2), click (12,12), double-click (2,12): K pixels are exactly the border of (2,2)-(12,12).
- Same with style 2 and background R: (7,7) is R.
- Fresh state, drag (2,2)->(12,2), click (12,12), click (2,2): the polygon is closed (the tool accepts a new first edge next) and (2,7) is K.
- Fresh state, drag (2,2)->(12,2), Shift click (20,9): edge from (12,2) ends at (19,9).
- Fresh state, drag (2,2)->(12,2), Esc: all pixels W.

---

## 55. colors.color_box: Colour box, default palette and swatch clicks

Priority: must | Layer: behaviour | Inventory: ui.color_box, format.palette.default, behaviour.palette.defaults, behaviour.palette.left_click, behaviour.palette.right_click, behaviour.palette.double_click

A strip with the current-colours indicator and 28 swatches; clicks set foreground or background.

**Behaviour**
1. Left part: an indicator of two overlapping squares, the upper-left showing the foreground and the lower-right the background colour. Right of it: 28 swatches in 2 rows of 14, each on a 16-screen-pixel pitch.
2. ASSUMPTION: default swatches, top row left to right: (0,0,0), (128,128,128), (128,0,0), (128,128,0), (0,128,0), (0,128,128), (0,0,128), (128,0,128), (128,128,64), (0,64,64), (0,128,255), (0,64,128), (64,0,255), (128,64,0); bottom row: (255,255,255), (192,192,192), (255,0,0), (255,255,0), (0,255,0), (0,255,255), (0,0,255), (255,0,255), (255,255,128), (0,255,128), (128,255,255), (128,128,255), (255,0,128), (255,128,64). Sampled from a lossless period screenshot; treated as exact but unconfirmed. Swatches are numbered 1-14 top row, 15-28 bottom row.
3. At start the foreground is (0,0,0) and the background (255,255,255).
4. Left-click on a swatch sets the foreground to its colour; right-click sets the background. Neither changes pixels or creates an undo step, except that an active text frame recolours (tool.text).
5. Double-click on a swatch opens Edit Colors for that swatch (colors.edit_colors).
6. The colour box can be dragged off to float and back to dock.

**Inputs:** swatch clicks
**Outputs:** foreground/background colour

**Edge cases**
- In an indexed image the indicator shows the chosen colour unsnapped; snapping happens when painting.

**Acceptance**
- Harness reads swatch 11 as (0,128,255) and swatch 28 as (255,128,64).
- Left-click swatch 17: foreground (255,0,0). Right-click swatch 21: background (0,0,255).
- First start: foreground (0,0,0), background (255,255,255).

---

## 56. colors.edit_colors: Edit Colors dialog

Priority: must | Layer: behaviour | Inventory: menu.colors.edit_colors, dialog.edit_colors, dialog.edit_colors.expanded

A colour chooser that redefines one swatch, with basic and custom colour grids and an expandable custom-colour panel.

**Behaviour**
1. ASSUMPTION: Colors > Edit Colors edits the swatch most recently clicked (left or right); if none was clicked since start, swatch 1. Guess.
2. Collapsed dialog: a grid of 48 basic colours (6 rows x 8), a grid of 16 custom colours (2 rows x 8, initially all (255,255,255)), a button to expand the custom-colour panel, OK and Cancel. The basic grid contains the 28 default swatch colours plus 20 further colours of the remake's choosing.
3. Expanding widens the dialog (the expand button becomes disabled) and shows a hue/saturation field with a cross-hair, a vertical luminosity bar with a marker, a preview of the chosen colour, numeric fields Hue, Sat, Lum (each 0-240, hue wrapping at 240) and Red, Green, Blue (0-255), and an 'Add to Custom Colors' button.
4. Editing any numeric field updates the others and the markers using the standard HSL-RGB conversion scaled to 0-240.
5. 'Add to Custom Colors' stores the current colour in the next custom slot, cycling 1..16.
6. ASSUMPTION: OK sets the edited swatch and the foreground colour to the chosen colour; Cancel changes nothing. Guess for the foreground part.
7. Custom colours persist for the session.

**Inputs:** Colors > Edit Colors; swatch double-click
**Outputs:** swatch colour; foreground colour

**Edge cases**
- Values typed out of range clamp to the range.

**Acceptance**
- Open Edit Colors, expand, enter Red 255, Green 0, Blue 0: Hue reads 0, Sat 240, Lum 120.
- Double-click swatch 5, choose (10,20,30), OK: swatch 5 and the foreground read (10,20,30).
- Same with Cancel: swatch 5 reads (0,128,0).
- Add to Custom Colors three times with different colours: custom slots 1-3 hold them in order.

---

## 57. colors.indexed_snap: Colours in indexed images

Priority: should | Layer: behaviour | Inventory: behaviour.palette.indexed_snap

In an image with its own palette, every painted colour is replaced by the nearest palette colour.

**Behaviour**
1. When the document has a palette (indexed source or after a reduced-colour save), every colour written by any tool or command is first mapped to the nearest palette entry (squared RGB distance, ties to the lowest index, as in format.color_reduction).
2. In Black-and-white mode the threshold rule of image.attributes is used instead.
3. The palette itself is not shown or editable.

**Inputs:** painting in indexed image
**Outputs:** snapped pixels

**Edge cases**
- none

**Acceptance**
- Open an 8-bit file whose palette is [(0,0,0),(255,255,255)], foreground (255,0,0), Pencil click (0,0): pixel (0,0) is (0,0,0).
- Same with foreground (200,200,200): pixel is (255,255,255).

---

## 58. ui.status_bar: Status bar

Priority: must | Layer: ui | Inventory: ui.status_bar, behaviour.status.idle_text, behaviour.status.coords, behaviour.status.size, behaviour.status.menu_hints

A message pane, a pointer-position pane and a size pane along the bottom edge.

**Behaviour**
1. Panes left to right: a wide message pane, a position pane, a size pane, then a window sizing grip.
2. The idle message is 'For help, open the Help menu.' (own wording).
3. The position pane shows 'x,y' (image pixel under the pointer, no spaces) while the pointer is over the canvas.
4. ASSUMPTION: the position pane is empty while the pointer is off the canvas. Guess.
5. While dragging a shape, a selection or the canvas handle, the size pane shows 'w,h' = (|dx|+1, |dy|+1) for shapes and selections (the inclusive extent) and the new canvas size for the handle; it is empty otherwise.
6. ASSUMPTION: while a menu item or tool button is highlighted, the message pane shows a one-line description in the remake's own words, unique per item; it returns to the idle message when the menu closes or the pointer leaves the button. Guess; descriptions are undocumented.

**Inputs:** pointer position; drags; menu highlight
**Outputs:** pane texts

**Edge cases**
- At zoom z the position is floor of screen offset / z.

**Acceptance**
- Pointer over image pixel (12,7): position pane reads '12,7'.
- Pointer outside the canvas: position pane is empty.
- On a 512x384 image, Rectangle press at (10,10) and move to (209,209), before release: size pane reads '200,200'.
- No menu open and no drag: message pane reads the idle message.
- Highlight File > New: message pane differs from the idle message and from the text shown for File > Open.

---

## 59. input.escape: Esc key

Priority: should | Layer: behaviour | Inventory: shortcut.esc

Esc cancels the operation in progress.

**Behaviour**
1. ASSUMPTION: Esc during a drag (button held) of Line, shapes, Select, Free-Form Select, selection move/resize or canvas handle cancels it: the image and selection return to their state at the press and no undo step is created. Guess; undocumented.
2. ASSUMPTION: Esc with an unfinished Curve or Polygon discards it; with a selection (not dragging) ends the selection, committing floating pixels; with an active text frame commits it. Guess.
3. Esc in a dialog equals Cancel; Esc in View Bitmap returns to the window; Esc in an open menu closes it.

**Inputs:** Esc
**Outputs:** cancelled operation

**Edge cases**
- Esc during a Pencil, Brush, Eraser or Airbrush stroke also cancels the stroke.

**Acceptance**
- Fresh state, Line press (2,2), move (20,20), Esc, release: all pixels W, Undo disabled.
- Fresh state, Pencil dot (2,2), select (1,1)-(3,3), drag by (+10,0) and release, Esc: no selection, (12,2) is K.

---

## 60. help.help_about: Help Topics and About

Priority: should | Layer: ui | Inventory: menu.help.topics, menu.help.about, dialog.help_viewer, dialog.about, behaviour.help.discards_pending

A separate help window with the remake's own help content, and an About box.

**Behaviour**
1. Help Topics opens a separate, non-modal help window with two panes: on the left tabs Contents, Index (keyword field, keyword list, Display button) and Search; on the right the selected topic. Its toolbar has Hide, Back, Forward and Options (including Print). All help text is the remake's own.
2. Opening help never changes the document: an active text frame, unfinished curve or polygon, and the selection all remain exactly as they were (a documented flaw of the original that dropped them is not reproduced).
3. ASSUMPTION: About Brushling shows a modal box with the application name 'Brushling', its version and its licence notice, and an OK button. Guess; the original's content is unknown and not reproduced.

**Inputs:** Help menu
**Outputs:** help window; about box

**Edge cases**
- Opening Help Topics twice focuses the existing window.

**Acceptance**
- Start a curve with drag (2,2)->(20,2), open Help Topics, close it, then click (10,10) and (10,10): the curve is committed with K pixels at (2,2) and (20,2).
- With a selection active, open Help Topics: selection bounds unchanged.
- About Brushling contains the text 'Brushling' and the build's version string.
