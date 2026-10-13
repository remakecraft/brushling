//! Feature integration.embedding: scope decision, nothing to build.
//!
//! Version 1 has no in-place embedding or editing inside other applications.
//! Images reach other programs through saved files and the clipboard, which
//! `edit.clipboard` provides (Copy puts the selection on the clipboard as a
//! bitmap with the same pixels).

use crate::registry::Registry;

pub fn register(_r: &mut Registry) {}
