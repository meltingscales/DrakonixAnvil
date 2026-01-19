use eframe::egui;
use crate::server::{ServerInstance, ServerStatus};

pub struct DashboardView;

impl DashboardView {
    pub fn show(
        ui: &mut egui::Ui,
        servers: &[ServerInstance],
        docker_connected: bool,
        docker_version: &str,
        on_create_server: &mut impl FnMut(),
        on_start_server: &mut impl FnMut(&str),
        on_stop_server: &mut impl FnMut(&str),
    ) {
        ui.heading("Server Dashboard");
        ui.add_space(10.0);

        // Docker status
        ui.horizontal(|ui| {
            if docker_connected {
                ui.colored_label(egui::Color32::GREEN, "● Docker Connected");
                ui.label(format!("(v{})", docker_version));
            } else {
                ui.colored_label(egui::Color32::RED, "● Docker Disconnected");
                ui.label("Please ensure Docker is running");
            }
        });

        ui.add_space(20.0);

        // Create server button
        if ui.button("+ Create New Server").clicked() {
            on_create_server();
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

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
                    Self::server_card(ui, server, on_start_server, on_stop_server);
                    ui.add_space(10.0);
                }
            });
        }
    }

    fn server_card(
        ui: &mut egui::Ui,
        server: &ServerInstance,
        on_start: &mut impl FnMut(&str),
        on_stop: &mut impl FnMut(&str),
    ) {
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
            .rounding(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Status indicator
                    let (color, status_text) = match &server.status {
                        ServerStatus::Running => (egui::Color32::GREEN, "Running"),
                        ServerStatus::Starting => (egui::Color32::YELLOW, "Starting"),
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
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &server.status {
                            ServerStatus::Running => {
                                if ui.button("Stop").clicked() {
                                    on_stop(&server.config.name);
                                }
                            }
                            ServerStatus::Stopped => {
                                if ui.button("Start").clicked() {
                                    on_start(&server.config.name);
                                }
                            }
                            _ => {
                                ui.add_enabled(false, egui::Button::new("..."));
                            }
                        }
                    });
                });
            });
    }
}
