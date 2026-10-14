//! Opening image files. Format decoders are contributed through the registry
//! (`Registry::decoder`); this module only runs them and installs the result
//! as the document, or reports why the file could not be opened.

use crate::doc::Document;
use crate::registry::registry;
use crate::state::State;
use crate::types::*;
use std::path::Path;

/// A decoded file.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub image: Image,
    /// Source bit depth: 1, 4, 8 or 24.
    pub depth: u32,
    pub palette: Option<Vec<Rgb>>,
}

/// A recognised file that cannot be opened; `message` is shown after the path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub message: String,
}

/// Decoder entry point: `None` when the bytes are not this decoder's format.
pub type DecodeFn = fn(&[u8]) -> Option<Result<Decoded, DecodeError>>;

pub const CANNOT_OPEN: &str =
    "This file cannot be opened: it is not a bitmap or uses a format Brushling does not support.";

/// Run the registered decoders in order; the first that recognises the bytes
/// decides.
pub fn decode(bytes: &[u8]) -> Result<Decoded, DecodeError> {
    for d in registry().decoders() {
        if let Some(r) = d(bytes) {
            return r;
        }
    }
    Err(DecodeError { message: CANNOT_OPEN.to_owned() })
}

impl State {
    /// Replace the document with the file at `path`. A file that cannot be
    /// read or decoded opens an error message box and leaves the current
    /// document untouched; the call itself still succeeds, like the user
    /// seeing the box.
    pub fn open_path(&mut self, path: &Path) -> HResult<()> {
        self.open_path_checked(path).map(|_| ())
    }

    /// Like [`State::open_path`], but reports whether the file was opened.
    pub(crate) fn open_path_checked(&mut self, path: &Path) -> HResult<bool> {
        let shown = path.display().to_string();
        let bytes = match crate::sysfs::read(path) {
            Ok(b) => b,
            Err(_) => {
                self.error_box(&format!("{shown}\n{CANNOT_OPEN}"));
                return Ok(false);
            }
        };
        match decode(&bytes) {
            Ok(d) => {
                let doc = Document::from_image(d.image, Some(path.to_path_buf()), d.depth, d.palette)?;
                self.replace_document(doc);
                Ok(true)
            }
            Err(e) => {
                self.error_box(&format!("{shown}\n{}", e.message));
                Ok(false)
            }
        }
    }
}
