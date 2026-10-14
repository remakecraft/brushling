//! Brushling desktop entry point: opens the eframe window over the library's
//! headless [`brushling::App`]. All behaviour lives in the library; see
//! `shell/` for what the window draws and forwards.

// No console window behind the release build on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod shell;

use std::path::PathBuf;

use eframe::egui;

/// Per-user configuration folder: `%APPDATA%`, else the home folder, else
/// the current directory. The app keeps its settings in a `brushling`
/// subfolder of it.
fn config_dir() -> PathBuf {
    ["APPDATA", "USERPROFILE", "HOME"]
        .into_iter()
        .find_map(|var| std::env::var_os(var).filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() -> eframe::Result {
    let launch = shell::Launch { config_dir: config_dir(), args: std::env::args().skip(1).collect() };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Brushling")
            .with_app_id("brushling")
            .with_inner_size([1000.0, 780.0])
            .with_min_inner_size([480.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Brushling", options, Box::new(|_cc| Ok(Box::new(shell::Shell::new(launch)))))
}
