//! Feature file.save: File > Save (Ctrl+S) and File > Save As....
//!
//! Entry points for other features: [`save_to`] (write now), [`request_save`]
//! (write, but ask first when colours would change), [`save_document`] (the
//! File > Save command) and [`show_save_as`] (the dialog).

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::doc::Document;
use crate::features::format_bmp_write::{encode_bmp, encode_bmp_indexed, keeps_source_palette};
use crate::features::format_color_reduction::{fixed_palette, loses_colors, reduce, WARNING};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;
use std::path::{Path, PathBuf};

const TYPES: [(&str, u32); 4] = [
    ("Monochrome bitmap (1-bit)", 1),
    ("16-color bitmap (4-bit)", 4),
    ("256-color bitmap (8-bit)", 8),
    ("24-bit bitmap", 24),
];

pub fn register(r: &mut Registry) {
    r.command("File > Save", save_document);
    r.command("File > Save As...", show_save_as);
    r.shortcut(Key::Char('s'), Mods::CTRL, "File > Save");
}

pub(crate) fn type_label(depth: u32) -> &'static str {
    TYPES.iter().find(|(_, d)| *d == depth).map_or(TYPES[3].0, |(l, _)| l)
}

pub(crate) fn type_depth(label: &str) -> Option<u32> {
    TYPES.iter().find(|(l, _)| *l == label).map(|(_, d)| *d)
}

/// File > Save: write to the document's path at its source depth, or behave
/// like Save As when it has no path yet.
pub(crate) fn save_document(st: &mut State) -> HResult<()> {
    match st.doc.path.clone() {
        Some(path) => request_save(st, path, st.doc.source_depth),
        None => show_save_as(st),
    }
}

/// File > Save As...: the save dialog, preselecting the document's depth.
pub(crate) fn show_save_as(st: &mut State) -> HResult<()> {
    let name = st
        .doc
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(|| "untitled".to_owned(), |n| n.to_string_lossy().into_owned());
    let dialog = Dialog {
        kind: DialogKind::FileSave,
        title: "Save As".to_owned(),
        text: String::new(),
        buttons: vec!["Save".to_owned(), "Cancel".to_owned()],
        fields: vec![
            OpenDialog::field("File name", &name),
            OpenDialog::field("Save as type", type_label(st.doc.source_depth)),
        ],
    };
    st.open_dialog(OpenDialog::new(dialog, handler));
    Ok(())
}

/// What a save at some depth writes: the palette and indices for indexed
/// depths, plus whether the image holds colours the palette cannot represent.
pub(crate) struct Plan {
    pub palette: Vec<Rgb>,
    pub indices: Vec<u8>,
    pub lossy: bool,
}

/// `None` for 24-bit; otherwise the document's own palette when the depth
/// matches its source depth, else the fixed palette of the depth.
pub(crate) fn plan(doc: &Document, depth: u32) -> Option<Plan> {
    if depth == 24 {
        return None;
    }
    let (palette, mono) = if keeps_source_palette(doc.source_depth, &doc.palette, depth) {
        let own = doc.palette.as_deref().unwrap_or(&[]);
        (own[..own.len().min(1usize << depth)].to_vec(), false)
    } else {
        (fixed_palette(depth), depth == 1)
    };
    let indices = reduce(&doc.pixels, &palette, mono);
    let lossy = loses_colors(&doc.pixels, &palette);
    Some(Plan { palette, indices, lossy })
}

/// Save to `path` at `depth` (1, 4, 8 or 24), asking first when colours
/// would change. Cancelling the warning aborts without writing.
pub(crate) fn request_save(st: &mut State, path: PathBuf, depth: u32) -> HResult<()> {
    if !matches!(depth, 1 | 4 | 8 | 24) {
        return Err(HarnessError::Invalid(format!("bitmap depth {depth} is not supported")));
    }
    if plan(&st.doc, depth).is_some_and(|p| p.lossy) {
        st.pending_save = Some((path, depth));
        st.message_box("Brushling", WARNING, &["OK", "Cancel"], Some(warning_answered));
        return Ok(());
    }
    save_to(st, &path, depth).map(|_| ())
}

fn warning_answered(st: &mut State, button: &str) -> HResult<()> {
    let pending = st.pending_save.take();
    match (button, pending) {
        ("OK", Some((path, depth))) => save_to(st, &path, depth).map(|_| ()),
        _ => Ok(()),
    }
}

/// Write the document to `path` at `depth` without any question. On success
/// the document takes the path and depth, shows the mapped colours (indexed
/// depths), and is clean; undo history is untouched. A write failure opens
/// "The file could not be saved: <reason>" and leaves the file on disk as it
/// was. Returns whether the file was written. The recent-file list
/// (file.recent) is updated from here on success.
pub(crate) fn save_to(st: &mut State, path: &Path, depth: u32) -> HResult<bool> {
    let doc = &st.doc;
    let planned = plan(doc, depth);
    let bytes = match &planned {
        None => encode_bmp(&doc.pixels, doc.width, doc.height, 24, &[])?,
        Some(p) => encode_bmp_indexed(&p.indices, doc.width, doc.height, depth, &p.palette)?,
    };
    if let Err(e) = write_atomically(path, &bytes) {
        st.error_box(&format!("The file could not be saved: {e}"));
        return Ok(false);
    }
    let doc = &mut st.doc;
    match planned {
        Some(p) => {
            doc.pixels = p.indices.iter().map(|&i| p.palette[i as usize]).collect();
            doc.palette = Some(p.palette);
        }
        None => doc.palette = None,
    }
    doc.source_depth = depth;
    doc.path = Some(path.to_path_buf());
    doc.dirty = false;
    Ok(true)
}

/// Write to a sibling temporary file and rename it over the target, so a
/// failure leaves the old file intact and no stray file behind.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.is_dir() {
            return Err("the target is a folder".into());
        }
        if meta.permissions().readonly() {
            return Err("the file is read-only".into());
        }
    }
    let name = path.file_name().ok_or("the file name is empty")?.to_string_lossy().into_owned();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(())
}

/// Final path for a typed name: '.bmp' appended unless already there
/// (case-insensitive); a bare name lands next to the current file, if any.
pub(crate) fn resolve(st: &State, typed: &str) -> PathBuf {
    let typed = typed.trim().trim_matches('"').trim();
    let mut name = typed.to_owned();
    if !name.to_lowercase().ends_with(".bmp") {
        name.push_str(".bmp");
    }
    let path = PathBuf::from(name);
    let bare = path.components().count() == 1;
    match st.doc.path.as_ref().and_then(|p| p.parent()) {
        Some(dir) if bare && !dir.as_os_str().is_empty() => dir.join(path),
        _ => path,
    }
}

fn handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            if field == "Save as type" && type_depth(value).is_none() {
                return Err(HarnessError::Invalid(format!("unknown save type {value:?}")));
            }
            store_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click("Save") => {
            let typed = od.dialog.field("File name").unwrap_or("").trim().to_owned();
            if typed.trim_matches('"').trim().is_empty() {
                return Err(HarnessError::Invalid("no file name entered".into()));
            }
            let depth = od.dialog.field("Save as type").and_then(type_depth).unwrap_or(24);
            let path = resolve(st, &typed);
            request_save(st, path, depth)?;
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_rule() {
        let dir = std::env::temp_dir().join(format!("brushling-save-{}", std::process::id()));
        let st = State::new(crate::settings::Settings::load(&dir), Services::default(), (1920, 1080)).unwrap();
        assert_eq!(resolve(&st, "pic.BMP"), PathBuf::from("pic.BMP"));
        assert_eq!(resolve(&st, "pic.png"), PathBuf::from("pic.png.bmp"));
        assert_eq!(resolve(&st, "cat"), PathBuf::from("cat.bmp"));
    }
}
