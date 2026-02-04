use eframe::egui;
use crate::server::{ServerInstance, ServerStatus};

/// Backup progress info: (current, total, current_file)
pub type BackupProgressInfo = Option<(String, usize, usize, String)>;

pub struct DashboardView;

impl DashboardView {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        servers: &[ServerInstance],
        _docker_connected: bool,
        _docker_version: &str,
        backup_progress: &BackupProgressInfo,
        on_create_server: &mut impl FnMut(),
        on_start_server: &mut impl FnMut(&str),
        on_stop_server: &mut impl FnMut(&str),
        on_edit_server: &mut impl FnMut(&str),
        on_delete_server: &mut impl FnMut(&str),
        on_view_logs: &mut impl FnMut(&str),
        on_backup_server: &mut impl FnMut(&str),
        on_view_backups: &mut impl FnMut(&str),
        on_open_console: &mut impl FnMut(&str),
    ) {
        ui.horizontal(|ui| {
            ui.heading("Servers");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New Server").clicked() {
                    on_create_server();
                }
            });
        });
        ui.separator();

        // Server list
        if servers.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No servers configured yet.");
                ui.label("Click 'Create New Server' to get started!");
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for server in servers {
                    Self::server_card(ui, server, backup_progress, on_start_server, on_stop_server, on_edit_server, on_delete_server, on_view_logs, on_backup_server, on_view_backups, on_open_console);
                    ui.add_space(10.0);
                }
            });
        }
    }

    fn server_card(
        ui: &mut egui::Ui,
        server: &ServerInstance,
        backup_progress: &BackupProgressInfo,
        on_start: &mut impl FnMut(&str),
        on_stop: &mut impl FnMut(&str),
        on_edit: &mut impl FnMut(&str),
        on_delete: &mut impl FnMut(&str),
        on_view_logs: &mut impl FnMut(&str),
        on_backup: &mut impl FnMut(&str),
        on_view_backups: &mut impl FnMut(&str),
        on_open_console: &mut impl FnMut(&str),
    ) {
        // Check if this server has an active backup
        let this_server_backup = backup_progress.as_ref()
            .filter(|(name, _, _, _)| name == &server.config.name);
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
            .rounding(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Status indicator
                    let (color, status_text) = match &server.status {
                        ServerStatus::Running => (egui::Color32::GREEN, "Running"),
                        ServerStatus::Pulling => (egui::Color32::YELLOW, "Pulling Image"),
                        ServerStatus::Starting => (egui::Color32::YELLOW, "Starting"),
                        ServerStatus::Initializing => (egui::Color32::from_rgb(255, 165, 0), "Initializing"), // Orange
                        ServerStatus::Stopping => (egui::Color32::YELLOW, "Stopping"),
                        ServerStatus::Stopped => (egui::Color32::GRAY, "Stopped"),
                        ServerStatus::Error(_) => (egui::Color32::RED, "Error"),
                    };

                    ui.colored_label(color, "●");
                    ui.add_space(8.0);

                    // Server info
                    ui.vertical(|ui| {
                        ui.strong(&server.config.name);
                        ui.label(format!(
                            "{} - Port {}",
                            server.config.modpack.name,
                            server.config.port
                        ));
                        ui.small(format!("Status: {}", status_text));
                        if let ServerStatus::Error(err) = &server.status {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &server.status {
                            ServerStatus::Running => {
                                if ui.button("Stop").clicked() {
                                    on_stop(&server.config.name);
                                }
                                if ui.button("Console").clicked() {
                                    on_open_console(&server.config.name);
                                }
                                if ui.button("Logs").clicked() {
                                    on_view_logs(&server.config.name);
                                }
                            }
                            ServerStatus::Stopped | ServerStatus::Error(_) => {
                                if ui.button("Start").clicked() {
                                    on_start(&server.config.name);
                                }
                                if ui.button("Edit").clicked() {
                                    on_edit(&server.config.name);
                                }
                                // Show progress bar if backup in progress, otherwise show Backup button
                                if let Some((_, current, total, _)) = this_server_backup {
                                    let progress = if *total > 0 {
                                        *current as f32 / *total as f32
                                    } else {
                                        0.0
                                    };
                                    ui.add(egui::ProgressBar::new(progress)
                                        .desired_width(100.0)
                                        .text(format!("{}/{}", current, total)));
                                } else {
                                    if ui.button("Backup").clicked() {
                                        on_backup(&server.config.name);
                                    }
                                }
                                if ui.button("Backups").clicked() {
                                    on_view_backups(&server.config.name);
                                }
                                if ui.button("Logs").clicked() {
                                    on_view_logs(&server.config.name);
                                }
                                if ui.add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(100, 30, 30))).clicked() {
                                    on_delete(&server.config.name);
                                }
                            }
                            ServerStatus::Pulling | ServerStatus::Starting | ServerStatus::Stopping | ServerStatus::Initializing => {
                                ui.spinner();
                                if ui.button("Logs").clicked() {
                                    on_view_logs(&server.config.name);
                                }
                            }
                        }
                    });
                });
            });
    }
}
