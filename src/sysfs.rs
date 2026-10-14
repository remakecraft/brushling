//! The file system as the app sees it. On the desktop these are the
//! `std::fs` calls; in the browser (wasm32) there is no file system, so files
//! live in an in-memory store for the life of the page. The web shell adds
//! uploads with [`put`] and turns finished saves ([`take_saved`]) into
//! downloads.

use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    pub fn read(path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
        std::fs::write(path, bytes)
    }

    pub fn create_dir_all(path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    /// Write to a sibling temporary file and rename it over the target, so a
    /// failure leaves the old file intact and no stray file behind.
    pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
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
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Default)]
    struct Store {
        files: HashMap<PathBuf, Vec<u8>>,
        saved: Vec<(PathBuf, Vec<u8>)>,
    }

    thread_local! {
        static STORE: RefCell<Store> = RefCell::default();
    }

    pub fn read(path: &Path) -> io::Result<Vec<u8>> {
        STORE.with_borrow(|s| s.files.get(path).cloned()).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
        STORE.with_borrow_mut(|s| s.files.insert(path.to_path_buf(), bytes.to_vec()));
        Ok(())
    }

    /// Folders are implicit in the in-memory store.
    pub fn create_dir_all(_path: &Path) -> io::Result<()> {
        Ok(())
    }

    /// A document save: stored, and queued for the shell to download.
    pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
        if path.file_name().is_none() {
            return Err("the file name is empty".into());
        }
        STORE.with_borrow_mut(|s| {
            s.files.insert(path.to_path_buf(), bytes.to_vec());
            s.saved.push((path.to_path_buf(), bytes.to_vec()));
        });
        Ok(())
    }

    pub fn put(path: &Path, bytes: Vec<u8>) {
        STORE.with_borrow_mut(|s| s.files.insert(path.to_path_buf(), bytes));
    }

    pub fn take_saved() -> Vec<(PathBuf, Vec<u8>)> {
        STORE.with_borrow_mut(|s| std::mem::take(&mut s.saved))
    }
}

pub(crate) use imp::{create_dir_all, read, save, write};
#[cfg(target_arch = "wasm32")]
pub(crate) use imp::{put, take_saved};
