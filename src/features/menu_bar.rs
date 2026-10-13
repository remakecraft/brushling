//! Feature ui.menu_bar: the static menu tree (titles, item order, separators,
//! shortcut texts, access letters), the headless listing behind
//! `App::menu_items` / `App::menu_item`, and Alt+letter menu navigation.
//!
//! The enabled flag of an item comes from the command registry, so another
//! feature's `command_if` predicate shows up here; items whose command is not
//! registered are listed as enabled. Check state comes from [`check_state`],
//! the one place where check items read feature state.

use crate::registry::registry;
use crate::state::State;
use crate::types::*;

const SEP: &str = " > ";

enum Kind {
    Sep,
    Item,
    Check,
    Sub(&'static [Entry]),
    /// Placeholder where the recent-file group (with its separator) goes.
    Recent,
}

struct Entry {
    label: &'static str,
    access: Option<char>,
    shortcut: Option<&'static str>,
    kind: Kind,
}

const fn item(label: &'static str, access: char, shortcut: Option<&'static str>) -> Entry {
    Entry { label, access: Some(access), shortcut, kind: Kind::Item }
}
const fn check(label: &'static str, access: char, shortcut: Option<&'static str>) -> Entry {
    Entry { label, access: Some(access), shortcut, kind: Kind::Check }
}
const fn sub(label: &'static str, access: char, items: &'static [Entry]) -> Entry {
    Entry { label, access: Some(access), shortcut: None, kind: Kind::Sub(items) }
}
const SEPARATOR: Entry = Entry { label: "", access: None, shortcut: None, kind: Kind::Sep };
const RECENT: Entry = Entry { label: "", access: None, shortcut: None, kind: Kind::Recent };

static FILE: [Entry; 16] = [
    item("New", 'N', Some("Ctrl+N")),
    item("Open...", 'O', Some("Ctrl+O")),
    item("Save", 'S', Some("Ctrl+S")),
    item("Save As...", 'A', None),
    SEPARATOR,
    item("Print Preview", 'v', None),
    item("Page Setup...", 't', None),
    item("Print...", 'P', Some("Ctrl+P")),
    SEPARATOR,
    item("Send...", 'd', None),
    SEPARATOR,
    item("Set As Wallpaper (Tiled)", 'W', None),
    item("Set As Wallpaper (Centered)", 'e', None),
    SEPARATOR,
    RECENT,
    item("Exit", 'x', Some("Alt+F4")),
];

static EDIT: [Entry; 11] = [
    item("Undo", 'U', Some("Ctrl+Z")),
    item("Repeat", 'R', Some("F4")),
    SEPARATOR,
    item("Cut", 't', Some("Ctrl+X")),
    item("Copy", 'C', Some("Ctrl+C")),
    item("Paste", 'P', Some("Ctrl+V")),
    item("Clear Selection", 'l', Some("Del")),
    item("Select All", 'A', Some("Ctrl+A")),
    SEPARATOR,
    item("Copy To...", 'o', None),
    item("Paste From...", 'F', None),
];

static ZOOM: [Entry; 6] = [
    item("Normal Size", 'N', Some("Ctrl+PageDown")),
    item("Large Size", 'L', Some("Ctrl+PageUp")),
    item("Custom...", 'u', None),
    SEPARATOR,
    check("Show Grid", 'G', Some("Ctrl+G")),
    check("Show Thumbnail", 'h', None),
];

static VIEW: [Entry; 7] = [
    check("Tool Box", 'T', Some("Ctrl+T")),
    check("Color Box", 'C', Some("Ctrl+L")),
    check("Status Bar", 'S', None),
    check("Text Toolbar", 'e', None),
    SEPARATOR,
    sub("Zoom", 'Z', &ZOOM),
    item("View Bitmap", 'V', Some("Ctrl+F")),
];

static IMAGE: [Entry; 6] = [
    item("Flip/Rotate...", 'F', Some("Ctrl+R")),
    item("Stretch/Skew...", 'S', Some("Ctrl+W")),
    item("Invert Colors", 'I', Some("Ctrl+I")),
    item("Attributes...", 'A', Some("Ctrl+E")),
    item("Clear Image", 'C', Some("Ctrl+Shift+N")),
    check("Draw Opaque", 'D', None),
];

static COLORS: [Entry; 1] = [item("Edit Colors...", 'E', None)];

static HELP: [Entry; 3] = [item("Help Topics", 'H', None), SEPARATOR, item("About Brushling", 'A', None)];

static BAR: [Entry; 6] = [
    sub("File", 'F', &FILE),
    sub("Edit", 'E', &EDIT),
    sub("View", 'V', &VIEW),
    sub("Image", 'I', &IMAGE),
    sub("Colors", 'C', &COLORS),
    sub("Help", 'H', &HELP),
];

/// Recent files as stored in the settings (`recent.1` .. `recent.4`), as
/// `"<n> <path>"` labels. Empty when the list is empty. The feature that
/// maintains the list owns those keys.
pub fn recent_labels(st: &State) -> Vec<String> {
    (1..=4)
        .filter_map(|n| st.settings.get(&format!("recent.{n}")).filter(|p| !p.is_empty()))
        .enumerate()
        .map(|(i, p)| format!("{} {}", i + 1, p))
        .collect()
}

/// State of a check item. Features that own a toggle read their own state
/// here; the defaults are the start-up state.
fn check_state(_st: &State, path: &str) -> bool {
    matches!(path, "View > Tool Box" | "View > Color Box" | "View > Status Bar")
}

fn find_entries(path: &str) -> Option<&'static [Entry]> {
    let mut cur: &'static [Entry] = &BAR;
    if path.is_empty() {
        return Some(cur);
    }
    for part in path.split(SEP) {
        let e = cur.iter().find(|e| e.label == part)?;
        match e.kind {
            Kind::Sub(s) => cur = s,
            _ => return None,
        }
    }
    Some(cur)
}

fn join(parent: &str, label: &str) -> String {
    if parent.is_empty() {
        label.to_owned()
    } else {
        format!("{parent}{SEP}{label}")
    }
}

fn separator() -> MenuItem {
    MenuItem { separator: true, ..Default::default() }
}

fn command_enabled(st: &State, path: &str) -> bool {
    registry().command_for(path).and_then(|c| c.enabled).is_none_or(|f| f(st))
}

fn build(st: &State, parent: &str, e: &Entry) -> MenuItem {
    let path = join(parent, e.label);
    MenuItem {
        label: e.label.to_owned(),
        separator: false,
        enabled: matches!(e.kind, Kind::Sub(_)) || command_enabled(st, &path),
        checked: matches!(e.kind, Kind::Check).then(|| check_state(st, &path)),
        shortcut: e.shortcut.map(str::to_owned),
        access: e.access,
        submenu: matches!(e.kind, Kind::Sub(_)),
    }
}

/// The items of the menu at `path` ("" = menu bar), recent group expanded.
fn listing(st: &State, path: &str) -> Option<Vec<MenuItem>> {
    let entries = find_entries(path)?;
    let mut out = Vec::new();
    for e in entries {
        match e.kind {
            Kind::Sep => out.push(separator()),
            Kind::Recent => {
                let recent = recent_labels(st);
                if !recent.is_empty() {
                    for (i, label) in recent.into_iter().enumerate() {
                        out.push(MenuItem {
                            enabled: command_enabled(st, &join(path, &label)),
                            access: char::from_digit(i as u32 + 1, 10),
                            label,
                            ..Default::default()
                        });
                    }
                    out.push(separator());
                }
            }
            _ => out.push(build(st, path, e)),
        }
    }
    Some(out)
}

fn unknown(path: &str) -> HarnessError {
    HarnessError::Invalid(format!("unknown menu {path:?}"))
}

/// The entry of menu `path` whose access letter is `c` (case-insensitive).
fn pick(st: &State, path: &str, c: char) -> Option<MenuItem> {
    listing(st, path)?.into_iter().find(|i| i.access.is_some_and(|a| a.eq_ignore_ascii_case(&c)))
}

impl State {
    /// Entries of the menu at `path`, separators included.
    pub fn menu_listing(&self, path: &str) -> HResult<Vec<MenuItem>> {
        listing(self, path).ok_or_else(|| unknown(path))
    }

    /// One menu entry by full path.
    pub fn menu_entry(&self, path: &str) -> HResult<MenuItem> {
        let (parent, label) = path.rsplit_once(SEP).unwrap_or(("", path));
        listing(self, parent)
            .and_then(|l| l.into_iter().find(|i| !i.separator && i.label == label))
            .ok_or_else(|| unknown(path))
    }

    /// Alt+letter menu navigation. Returns true when the key was consumed.
    pub fn menu_key(&mut self, key: Key, mods: Mods) -> HResult<bool> {
        let Some(open) = self.menu_open.clone() else {
            if let (Key::Char(c), true, false) = (key, mods.alt, mods.ctrl) {
                if let Some(title) = pick(self, "", c) {
                    self.menu_open = Some(title.label);
                    return Ok(true);
                }
            }
            return Ok(false);
        };
        match key {
            Key::Escape => {
                self.menu_open = open.rsplit_once(SEP).map(|(p, _)| p.to_owned());
            }
            Key::Char(c) if !mods.ctrl => {
                // Alt+letter while a menu is open switches to another menu.
                let parent = if mods.alt { "" } else { open.as_str() };
                match pick(self, parent, c) {
                    Some(it) if it.submenu => self.menu_open = Some(join(parent, &it.label)),
                    Some(it) => {
                        self.menu_open = None;
                        let path = join(parent, &it.label);
                        if registry().command_for(&path).is_some() {
                            self.menu(&path)?;
                        }
                    }
                    None => {}
                }
            }
            _ => self.menu_open = None,
        }
        Ok(true)
    }
}
