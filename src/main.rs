#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod app;
mod core;
mod document;
pub mod i18n;
mod platform;
mod settings;
pub mod updater;

use app::QuietReaderApp;
use settings::AppSettings;

fn main() -> eframe::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let initial_file = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let Some(_instance_guard) = platform::acquire_single_instance(&initial_file) else {
        return Ok(());
    };

    let settings = AppSettings::load();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_min_inner_size([600.0, 400.0])
        .with_title("QuietReader");

    let (pos_x, pos_y, w, h) = platform::sanitize_window_geometry(
        settings.window.x,
        settings.window.y,
        settings.window.width,
        settings.window.height,
    );
    viewport = viewport.with_inner_size([w, h]);

    if let (Some(x), Some(y)) = (pos_x, pos_y) {
        viewport = viewport.with_position([x, y]);
    }

    if settings.window.maximized {
        viewport = viewport.with_maximized(true);
    }

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/app_64.png")) {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "QuietReader",
        native_options,
        Box::new(move |cc| Ok(Box::new(QuietReaderApp::new(cc, initial_file, settings)))),
    )
}
