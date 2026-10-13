//! Feature file.recent: the recent-files list (most recent first, at most
//! four), kept in the settings as `recent.1` .. `recent.4`.

use crate::features::menu_bar::recent_labels;
use crate::registry::Registry;
use crate::state::State;
use crate::types::HResult;
use std::path::{Path, PathBuf};

pub const MAX_RECENT: usize = 4;

pub fn register(_r: &mut Registry) {}

fn key(n: usize) -> String {
    format!("recent.{n}")
}

fn list(st: &State) -> Vec<String> {
    (1..=MAX_RECENT)
        .filter_map(|n| st.settings.get(&key(n)).filter(|p| !p.is_empty()))
        .map(str::to_owned)
        .collect()
}

/// Store the list. Persisting is best effort: a settings file that cannot be
/// written must not fail the open or save that triggered the update (the
/// list is written again on exit).
fn store(st: &mut State, entries: &[String]) {
    for n in 1..=MAX_RECENT {
        let value = entries.get(n - 1).map_or("", String::as_str);
        let _ = st.settings.set(&key(n), value);
    }
    let _ = st.settings.save();
}

fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// A file was opened or saved: move it to position 1.
pub(crate) fn add(st: &mut State, path: &Path) {
    let text = path.display().to_string();
    if text.contains(['\n', '\r']) {
        return;
    }
    let mut entries: Vec<String> = list(st).into_iter().filter(|e| !same(e, &text)).collect();
    entries.insert(0, text);
    entries.truncate(MAX_RECENT);
    store(st, &entries);
}

fn remove(st: &mut State, path: &Path) {
    let text = path.display().to_string();
    let entries: Vec<String> = list(st).into_iter().filter(|e| !same(e, &text)).collect();
    store(st, &entries);
}

/// The file behind a File menu path such as `File > 2 C:\a.bmp`, if it names
/// a current recent entry.
pub(crate) fn entry_path(st: &State, menu_path: &str) -> Option<PathBuf> {
    let label = menu_path.strip_prefix("File > ")?;
    let n = recent_labels(st).into_iter().position(|l| l == label)?;
    list(st).get(n).map(PathBuf::from)
}

/// Open a recent entry (after the save-changes check). A file that cannot
/// be opened is reported and dropped from the list.
pub(crate) fn open_recent(st: &mut State, path: &Path) -> HResult<()> {
    if !crate::features::file_open::open_file(st, path)? {
        remove(st, path);
    }
    Ok(())
}
