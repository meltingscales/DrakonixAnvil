use crate::server::{ServerInstance, ServerStatus};
use eframe::egui;

/// Progress info: (server_name, current, total, current_file)
pub type ProgressInfo = Option<(String, usize, usize, String)>;

/// Callbacks for server actions on the dashboard
pub struct DashboardCallbacks<'a> {
    pub on_create_server: &'a mut dyn FnMut(),
    pub on_start_server: &'a mut dyn FnMut(&str),
    pub on_stop_server: &'a mut dyn FnMut(&str),
    pub on_edit_server: &'a mut dyn FnMut(&str),
    pub on_delete_server: &'a mut dyn FnMut(&str),
    pub on_view_logs: &'a mut dyn FnMut(&str),
    pub on_backup_server: &'a mut dyn FnMut(&str),
    pub on_view_backups: &'a mut dyn FnMut(&str),
    pub on_open_console: &'a mut dyn FnMut(&str),
    pub on_adopt_server: &'a mut dyn FnMut(&str),
    pub on_delete_orphan: &'a mut dyn FnMut(&str),
    pub on_export_server: &'a mut dyn FnMut(&str),
    pub on_import_server: &'a mut dyn FnMut(),
    pub orphaned_dirs: &'a [String],
}

pub struct DashboardView;

impl DashboardView {
    pub fn show(
        ui: &mut egui::Ui,
        servers: &[ServerInstance],
        _docker_connected: bool,
        _docker_version: &str,
        backup_progress: &ProgressInfo,
        restore_progress: &ProgressInfo,
        cb: &mut DashboardCallbacks<'_>,
    ) {
        let orphaned_dirs = cb.orphaned_dirs;
        ui.horizontal(|ui| {
            ui.heading("Servers");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New Server").clicked() {
                    (cb.on_create_server)();
                }
                if ui.button("Import Server").clicked() {
                    (cb.on_import_server)();
                }
            });
        });
        ui.separator();

        // Server list
        if servers.is_empty() && orphaned_dirs.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No servers configured yet.");
                ui.label("Click 'Create New Server' to get started!");
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for server in servers {
                    Self::server_card(ui, server, backup_progress, restore_progress, cb);
                    ui.add_space(10.0);
                }

                // Orphaned server directories
                if !orphaned_dirs.is_empty() {
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("Orphaned Server Directories ({})", orphaned_dirs.len()),
                    );
                    ui.small(
                        "These folders exist in DrakonixAnvilData/servers/ but aren't tracked. \
                         They may be from deleted servers.",
                    );
                    ui.add_space(8.0);

                    for dir_name in orphaned_dirs {
                        Self::orphan_row(ui, dir_name, cb);
                        ui.add_space(4.0);
                    }
                }
            });
        }
    }

    fn orphan_row(ui: &mut egui::Ui, dir_name: &str, cb: &mut DashboardCallbacks<'_>) {
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
            .rounding(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "?");
                    ui.add_space(8.0);
                    ui.label(dir_name);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("Delete")
                                    .fill(egui::Color32::from_rgb(100, 30, 30)),
                            )
                            .clicked()
                        {
                            (cb.on_delete_orphan)(dir_name);
                        }
                        if ui.button("Adopt").clicked() {
                            (cb.on_adopt_server)(dir_name);
                        }
                    });
                });
            });
    }

    fn server_card(
        ui: &mut egui::Ui,
        server: &ServerInstance,
        backup_progress: &ProgressInfo,
        restore_progress: &ProgressInfo,
        cb: &mut DashboardCallbacks<'_>,
    ) {
        // Check if this server has an active backup or restore
        let this_server_backup = backup_progress
            .as_ref()
            .filter(|(name, _, _, _)| name == &server.config.name);
        let this_server_restore = restore_progress
            .as_ref()
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
                        ServerStatus::Initializing => {
                            (egui::Color32::from_rgb(255, 165, 0), "Initializing")
                        } // Orange
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
                            server.config.modpack.name, server.config.port
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
                                    (cb.on_stop_server)(&server.config.name);
                                }
                                if ui.button("Console").clicked() {
                                    (cb.on_open_console)(&server.config.name);
                                }
                                if ui.button("Logs").clicked() {
                                    (cb.on_view_logs)(&server.config.name);
                                }
                            }
                            ServerStatus::Stopped | ServerStatus::Error(_) => {
                                // Show restore progress if in progress
                                if let Some((_, current, total, _)) = this_server_restore {
                                    let progress = if *total > 0 {
                                        *current as f32 / *total as f32
                                    } else {
                                        0.0
                                    };
                                    ui.add(
                                        egui::ProgressBar::new(progress)
                                            .desired_width(120.0)
                                            .text(format!("Restoring {}/{}", current, total)),
                                    );
                                } else {
                                    if ui.button("Start").clicked() {
                                        (cb.on_start_server)(&server.config.name);
                                    }
                                    if ui.button("Edit").clicked() {
                                        (cb.on_edit_server)(&server.config.name);
                                    }
                                    // Show progress bar if backup in progress, otherwise show Backup button
                                    if let Some((_, current, total, _)) = this_server_backup {
                                        let progress = if *total > 0 {
                                            *current as f32 / *total as f32
                                        } else {
                                            0.0
                                        };
                                        ui.add(
                                            egui::ProgressBar::new(progress)
                                                .desired_width(100.0)
                                                .text(format!("{}/{}", current, total)),
                                        );
                                    } else if ui.button("Backup").clicked() {
                                        (cb.on_backup_server)(&server.config.name);
                                    }
                                    if ui.button("Backups").clicked() {
                                        (cb.on_view_backups)(&server.config.name);
                                    }
                                    if ui.button("Export").clicked() {
                                        (cb.on_export_server)(&server.config.name);
                                    }
                                    if ui.button("Logs").clicked() {
                                        (cb.on_view_logs)(&server.config.name);
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new("Delete")
                                                .fill(egui::Color32::from_rgb(100, 30, 30)),
                                        )
                                        .clicked()
                                    {
                                        (cb.on_delete_server)(&server.config.name);
                                    }
                                }
                            }
                            ServerStatus::Pulling
                            | ServerStatus::Starting
                            | ServerStatus::Stopping
                            | ServerStatus::Initializing => {
                                ui.spinner();
                                if ui.button("Logs").clicked() {
                                    (cb.on_view_logs)(&server.config.name);
                                }
                            }
                        }
                    });
                });
            });
    }
}
