use crate::server::ServerConfig;
use eframe::egui;

pub struct ServerEditView {
    pub server_name: String,
    pub port: String,
    pub memory_mb: String,
    pub java_args: String,
    pub dirty: bool,
}

impl Default for ServerEditView {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            port: "25565".to_string(),
            memory_mb: "4096".to_string(),
            java_args: String::new(),
            dirty: false,
        }
    }
}

impl ServerEditView {
    pub fn load_from_config(&mut self, config: &ServerConfig) {
        self.server_name = config.name.clone();
        self.port = config.port.to_string();
        self.memory_mb = config.memory_mb.to_string();
        self.java_args = config.java_args.join("\n");
        self.dirty = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        on_save: &mut impl FnMut(u16, u64, Vec<String>),
        on_cancel: &mut impl FnMut(),
    ) {
        ui.heading(format!("Edit Server: {}", self.server_name));
        ui.add_space(20.0);

        egui::Grid::new("edit_server_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Port:");
                if ui.text_edit_singleline(&mut self.port).changed() {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Memory (MB):");
                if ui
                    .add(egui::TextEdit::singleline(&mut self.memory_mb).desired_width(80.0))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.label("Java Options (one per line):");
        ui.add_space(5.0);

        let text_edit = egui::TextEdit::multiline(&mut self.java_args)
            .desired_width(f32::INFINITY)
            .desired_rows(10)
            .font(egui::TextStyle::Monospace);

        if ui.add(text_edit).changed() {
            self.dirty = true;
        }

        ui.add_space(10.0);
        ui.small("Common options: -XX:+UseG1GC, -XX:MaxGCPauseMillis=200, etc.");

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                on_cancel();
            }

            ui.add_space(20.0);

            let port_valid = self.port.parse::<u16>().is_ok();
            let memory_valid = self.memory_mb.parse::<u64>().is_ok();
            let can_save = port_valid && memory_valid && self.dirty;

            if ui
                .add_enabled(can_save, egui::Button::new("Save Changes"))
                .clicked()
            {
                let port = self.port.parse().unwrap_or(25565);
                let memory_mb = self.memory_mb.parse().unwrap_or(4096);
                let java_args: Vec<String> = self
                    .java_args
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                on_save(port, memory_mb, java_args);
            }

            if !port_valid {
                ui.colored_label(egui::Color32::RED, "Invalid port number");
            }
            if !memory_valid {
                ui.colored_label(egui::Color32::RED, "Invalid memory value");
            }
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        ui.small("Note: Changes will take effect the next time the server starts.");
        ui.small("The container will be recreated with the new settings.");

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.small("For advanced options, edit");
            ui.small("DrakonixAnvilData/servers.json");
            ui.add(
                egui::Label::new(
                    egui::RichText::new("(?)")
                        .small()
                        .color(egui::Color32::LIGHT_BLUE),
                )
                .sense(egui::Sense::hover()),
            )
            .on_hover_text(
                "The servers.json file contains all server configurations.\n\n\
                You can edit it directly to configure:\n\
                - Server properties (MOTD, max players, difficulty, etc.)\n\
                - Modpack source settings\n\
                - Any other advanced options\n\n\
                Make sure the server is stopped before editing.\n\
                Changes are loaded when you restart DrakonixAnvil.",
            );
        });
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
