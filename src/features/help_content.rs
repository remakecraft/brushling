//! The text of the Help Topics window. All wording is the remake's own; it
//! describes the menus and tools as this application implements them.

/// One help topic: its title in the Contents list, the Index keywords that
/// lead to it, and its body.
pub struct Topic {
    pub title: &'static str,
    pub keywords: &'static [&'static str],
    pub body: &'static str,
}

/// All topics in Contents order. The first one is shown when the window opens.
pub static TOPICS: [Topic; 11] = [
    Topic {
        title: "Welcome to Brushling",
        keywords: &["overview", "getting started"],
        body: "Brushling is a small bitmap painting program. Pick a tool in the Tool Box, choose colours in the \
Color Box, and draw on the canvas. The left mouse button paints with the foreground colour and the right \
button with the background colour.\n\nUse the Contents tab to browse the topics, the Index tab to jump to a \
keyword, or the Search tab to find a word in the help text.",
    },
    Topic {
        title: "File menu",
        keywords: &["file", "new", "open", "save", "print", "exit", "wallpaper", "recent files"],
        body: "New starts an empty picture and Open loads a bitmap file. Save writes the picture to its file and \
Save As asks for a name and a colour depth. Print Preview shows the pages a print would use, Page Setup sets \
the paper, orientation and margins, and Print sends the pages to a printer. Send mails the picture as an \
attachment. Set As Wallpaper makes it the desktop background, tiled or centred. The four most recently used \
files are listed above Exit. Before a changed picture is replaced you are asked whether to save it.",
    },
    Topic {
        title: "Printing and Print Preview",
        keywords: &["print preview", "page setup", "margins", "paper", "pages"],
        body: "The picture prints at 96 pixels per inch from the top-left corner of the printable area, which is \
the paper minus the margins. A picture larger than that area is split into pages, left to right and then top \
to bottom. Print Preview shows one page at a time with the buttons Print, Next Page, Prev Page, Zoom In, Zoom \
Out and Close. Next Page is unavailable on the last page and Prev Page on the first. Close returns to the \
normal view and leaves the picture unchanged. Page Setup values last until you leave the program.",
    },
    Topic {
        title: "Edit menu",
        keywords: &["edit", "undo", "repeat", "cut", "copy", "paste", "clear selection", "select all", "clipboard"],
        body: "Undo takes back the last change and Repeat does the last undone change again. \
Cut, Copy and Paste move a selection through the clipboard. Clear Selection deletes the selection and Select \
All selects the whole picture. Copy To writes the selection to a file and Paste From inserts a bitmap file \
as a selection. While the Text tool has a text frame, Cut, Copy and Paste work on the text instead.",
    },
    Topic {
        title: "View menu",
        keywords: &["view", "zoom", "grid", "thumbnail", "tool box", "color box", "status bar", "view bitmap"],
        body: "The first four items show or hide the Tool Box, the Color Box, the Status Bar and the Text \
Toolbar. Zoom offers Normal Size, Large Size, a custom percentage, a pixel grid that appears at high zoom, and \
a thumbnail window that shows the whole picture at actual size. View Bitmap shows the picture alone on a \
black screen until you press a key or a mouse button.",
    },
    Topic {
        title: "Image menu",
        keywords: &["image", "flip", "rotate", "stretch", "skew", "invert", "attributes", "clear image", "opaque"],
        body: "Flip/Rotate mirrors the picture or turns it by 90, 180 or 270 degrees. Stretch/Skew scales it by \
a percentage or slants it by an angle. Invert Colors swaps every colour for its opposite. Attributes sets the \
width, height, units and whether the picture is black and white. Clear Image fills the picture with the \
background colour. Draw Opaque decides whether the background of a moved selection is painted over what is \
below it. These commands act on the selection when there is one.",
    },
    Topic {
        title: "Colors menu",
        keywords: &["colors", "edit colors", "custom colors", "foreground", "background", "palette"],
        body: "Click a swatch in the Color Box with the left button to set the foreground colour and with the \
right button to set the background colour. Edit Colors opens a dialog where you can pick a colour by its red, \
green and blue values or by hue, saturation and luminance, and store up to sixteen custom colours. In a black \
and white picture every colour is snapped to black or white.",
    },
    Topic {
        title: "Tool Box",
        keywords: &["tools", "tool box", "options"],
        body: "The Tool Box holds sixteen tools: Free-Form Select, Select, Eraser, Fill, Pick Color, Magnifier, \
Pencil, Brush, Airbrush, Text, Line, Curve, Rectangle, Polygon, Ellipse and Rounded Rectangle. Resting the \
pointer on a button shows its name. Some tools show extra choices in the box below the tools, such as a brush \
shape or a line width.",
    },
    Topic {
        title: "Selecting and moving",
        keywords: &["select", "free-form select", "selection", "move", "transparent", "handles"],
        body: "Select drags a rectangle and Free-Form Select follows a freehand outline. Drag inside the \
selection to move it, or hold Ctrl while dragging to leave a copy behind. The small handles on the border \
resize it. Press Esc to cancel a drag, or click outside the selection to put it down. The two options below \
the tools choose whether the selection is opaque or lets the background show through.",
    },
    Topic {
        title: "Drawing tools",
        keywords: &["pencil", "brush", "airbrush", "eraser", "fill", "pick color", "line", "curve", "polygon", "rectangle", "ellipse", "rounded rectangle", "magnifier"],
        body: "Pencil draws one pixel wide freehand lines, Brush and Airbrush paint with a chosen shape and \
size, and Eraser paints the background colour. Fill floods the area around the pointer. Pick Color copies the \
colour under the pointer into the foreground or background. Line, Curve, Rectangle, Polygon, Ellipse and \
Rounded Rectangle draw shapes; the choice below the tools makes them outlined, filled or both. Hold Shift to \
keep a line horizontal, vertical or diagonal and a box square. Magnifier zooms in on the clicked area and \
back out again.",
    },
    Topic {
        title: "Text tool and keyboard shortcuts",
        keywords: &["text", "font", "keyboard", "shortcuts", "ctrl", "text toolbar"],
        body: "Drag a frame with the Text tool and type into it. The Text Toolbar sets the font, size and \
bold, italic and underline styles. Text is part of the picture once you click outside the frame. The text \
tool is only available at 100% zoom.\n\nShortcuts: Ctrl+N New, Ctrl+O Open, Ctrl+S Save, Ctrl+P Print, Ctrl+Z \
Undo, F4 Repeat, Ctrl+X Cut, Ctrl+C Copy, Ctrl+V Paste, Del Clear Selection, Ctrl+A Select All, Ctrl+T Tool \
Box, Ctrl+L Color Box, Ctrl+F View Bitmap, Ctrl+R Flip/Rotate, Ctrl+W Stretch/Skew, Ctrl+I Invert Colors, \
Ctrl+E Attributes, Ctrl+Shift+N Clear Image, Ctrl+PageUp Large Size, Ctrl+PageDown Normal Size, Ctrl+G Show \
Grid.",
    },
];
