// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Backend modules are wired to the UI in a later commit.
#![allow(dead_code)]

mod config;
mod fetch;
mod slurm;
mod ssh_config;
mod store;

use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, Window, WindowBounds,
    WindowOptions,
};
use std::path::PathBuf;

/// Bundle identifier; also the config directory name, matching where the
/// Tauri build kept `clusters.json` so existing profiles carry over.
pub const APP_ID: &str = "org.shawonashraf.slurmer";

/// `$SLURMER_CONFIG` if set (manual testing), else
/// `<platform config dir>/org.shawonashraf.slurmer/clusters.json`.
pub fn config_path() -> PathBuf {
    if let Some(p) = std::env::var_os("SLURMER_CONFIG") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
        .join("clusters.json")
}

struct Placeholder;

impl Render for Placeholder {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("slurmer")
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.), px(680.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Placeholder),
        )
        .expect("open window");
        cx.activate(true);
    });
}
