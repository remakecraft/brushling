//! Brushling: headless application core and test harness surface.
//!
//! The GUI (eframe/egui) is a thin shell over [`App`]; every behaviour in the
//! spec is reachable through this API without a window. Both test suites
//! (`tests/visible`, `tests/sealed`) use only the items exported here, so the
//! public signatures in this crate are frozen: builders add behaviour behind
//! them and may add private modules, but must not rename, remove or change
//! any public item.
//!
//! Conventions shared by every method (see `harness` module docs for the
//! dialog, menu and pointer contracts):
//! - Image coordinates: (0,0) top-left, x right, y down, `i32` so tests can
//!   address pixels outside the canvas.
//! - Every method that performs or observes behaviour returns [`HResult`].
//!   An unknown menu path, an unknown dialog field or button, or a call that
//!   makes no sense in the current state returns `HarnessError::Invalid`.
//!   Invoking a *disabled* menu item is not an error: it returns `Ok(())` and
//!   changes nothing, exactly like clicking a greyed item.

// Shared helpers are consumed by features as they land.
#![allow(dead_code)]

pub mod harness;
pub mod types;

// Private modules behind the frozen API (see src/README.md). Shared helpers
// are consumed by features as they land, so unused items are expected.
mod commands;
mod dialogs;
mod doc;
mod features;
mod formats;
mod raster;
mod registry;
mod render;
mod rng;
mod selection;
mod settings;
mod state;
mod sysfs;
mod tools;

pub use harness::App;
pub use types::*;

/// The build's version string, shown in Help > About Brushling.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
