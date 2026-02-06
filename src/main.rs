#![deny(warnings)]

mod app;
mod backup;
mod config;
mod docker;
mod pack_installer;
mod rcon;
mod server;
mod templates;
mod ui;

use app::DrakonixApp;
use tracing_subscriber::prelude::*;

fn main() -> eframe::Result<()> {
    // Create logs directory
    let log_dir = std::path::Path::new("./DrakonixAnvilData/logs");
    std::fs::create_dir_all(log_dir).ok();

    // Generate timestamped log filename
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("drakonixanvil_{}.log", timestamp);

    // Set up file appender
    let file_appender = tracing_appender::rolling::never(log_dir, &log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Create filter
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into());

    // Set up dual logging: stdout + file
    let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false); // No ANSI colors in file

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    // Log header with version and issues link
    tracing::info!("═══════════════════════════════════════════════════════════════");
    tracing::info!("DrakonixAnvil v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Report issues: https://github.com/meltingscales/DrakonixAnvil/issues");
    tracing::info!("Log file: {}", log_dir.join(&log_filename).display());
    tracing::info!("═══════════════════════════════════════════════════════════════");

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
