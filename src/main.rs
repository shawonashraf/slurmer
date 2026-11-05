// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fetch;
mod slurm;
mod ssh_config;
mod store;
mod ui;

use gpui::{
    actions, px, size, App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem,
    TitlebarOptions, WindowBounds, WindowOptions,
};
use std::path::PathBuf;
use ui::app_view::SlurmerApp;
use ui::assets::Assets;
use ui::theme::Theme;

actions!(slurmer, [Quit]);

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

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        ui::text_input::bind_keys(cx);
        ui::cluster_form::bind_keys(cx);
        let quit = if cfg!(target_os = "macos") {
            "cmd-q"
        } else {
            "ctrl-q"
        };
        cx.bind_keys([KeyBinding::new(quit, Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.set_menus(vec![Menu {
            name: "Slurmer".into(),
            items: vec![MenuItem::action("Quit Slurmer", Quit)],
        }]);
        cx.set_global(Theme::light());

        let bounds = Bounds::centered(None, size(px(1100.), px(680.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Slurmer".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(900.), px(500.))),
                app_id: Some(APP_ID.to_string()),
                ..Default::default()
            },
            |window, cx| {
                cx.set_global(Theme::for_appearance(window.appearance()));
                cx.new(|cx| SlurmerApp::new(config_path(), window, cx))
            },
        )
        .expect("open window");
        // Single-window app: closing the window quits, as the Tauri build did.
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
}
