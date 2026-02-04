#![deny(warnings)]

mod app;
mod backup;
mod config;
mod docker;
mod server;
mod templates;
mod ui;

use app::DrakonixApp;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting DrakonixAnvil");

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("DrakonixAnvil - Minecraft Server Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "DrakonixAnvil",
        native_options,
        Box::new(|cc| Ok(Box::new(DrakonixApp::new(cc)))),
    )
}
