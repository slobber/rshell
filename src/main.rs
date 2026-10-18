#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rshell::app::ShellCoreApp;
use rshell::config::AppConfig;

fn main() -> eframe::Result<()> {
    seed_hostname_environment();
    let config = AppConfig::load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RShell — SSH / SFTP / WSL / RDP Workbench")
        .with_inner_size([1580.0, 900.0])
        .with_min_inner_size([1120.0, 720.0])
        .with_position([36.0, 36.0])
        .with_decorations(false)
        .with_resizable(true);
    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        centered: false,
        persist_window: false,
        persistence_path: Some(config.root_dir.clone()),
        ..Default::default()
    };

    eframe::run_native(
        "RShell",
        native_options,
        Box::new(move |cc| Ok(Box::new(ShellCoreApp::new(cc, config)))),
    )
}

fn load_app_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../assets/app-icon.png")).ok()?;
    let image = image::imageops::resize(
        &image.into_rgba8(),
        256,
        256,
        image::imageops::FilterType::Lanczos3,
    );
    Some(egui::IconData {
        rgba: image.into_raw(),
        width: 256,
        height: 256,
    })
}

#[cfg(windows)]
fn seed_hostname_environment() {
    if std::env::var_os("HOSTNAME").is_none() {
        if let Some(computer_name) = std::env::var_os("COMPUTERNAME") {
            std::env::set_var("HOSTNAME", computer_name);
        }
    }
}

#[cfg(not(windows))]
fn seed_hostname_environment() {}
