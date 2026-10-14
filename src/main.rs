//! Brushling entry point: opens the eframe window (desktop) or canvas
//! (browser, wasm32) over the library's headless [`brushling::App`]. All
//! behaviour lives in the library; see `shell/` for what the window draws
//! and forwards.

// No console window behind the release build on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod shell;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::path::PathBuf;

    use eframe::egui;

    use crate::shell;

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

    pub fn run() -> eframe::Result {
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
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    desktop::run()
}

/// The page (`web/index.html`) provides `<canvas id="brushling">`; start-up
/// errors replace the loading text in `#status`. Settings and files live in
/// memory for the life of the page.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().and_then(|w| w.document()).expect("Brushling needs a browser document");
        let status = document.get_element_by_id("status");
        let canvas = document.get_element_by_id("brushling").and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        let result = match canvas {
            Some(canvas) => {
                let launch = shell::Launch { config_dir: "/config".into(), args: Vec::new() };
                eframe::WebRunner::new()
                    .start(canvas, eframe::WebOptions::default(), Box::new(|_cc| Ok(Box::new(shell::Shell::new(launch)))))
                    .await
                    .map_err(|e| format!("{e:?}"))
            }
            None => Err("the page has no <canvas id=\"brushling\">".to_owned()),
        };
        if let Some(status) = status {
            match result {
                Ok(()) => status.remove(),
                Err(e) => status.set_text_content(Some(&format!("Brushling could not start: {e}"))),
            }
        }
    });
}
