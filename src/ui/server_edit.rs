use crate::server::{Difficulty, GameMode, ServerConfig, ServerProperties};
use eframe::egui;

pub struct ServerEditView {
    pub server_name: String,
    pub port: String,
    pub memory_mb: String,
    pub java_args: String,
    // Server properties
    pub motd: String,
    pub max_players: String,
    pub difficulty: Difficulty,
    pub gamemode: GameMode,
    pub pvp: bool,
    pub online_mode: bool,
    pub white_list: bool,
    pub dirty: bool,
}

impl Default for ServerEditView {
    fn default() -> Self {
        let defaults = ServerProperties::default();
        Self {
            server_name: String::new(),
            port: "25565".to_string(),
            memory_mb: "4096".to_string(),
            java_args: String::new(),
            motd: defaults.motd,
            max_players: defaults.max_players.to_string(),
            difficulty: defaults.difficulty,
            gamemode: defaults.gamemode,
            pvp: defaults.pvp,
            online_mode: defaults.online_mode,
            white_list: defaults.white_list,
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
        let sp = &config.server_properties;
        self.motd = sp.motd.clone();
        self.max_players = sp.max_players.to_string();
        self.difficulty = sp.difficulty.clone();
        self.gamemode = sp.gamemode.clone();
        self.pvp = sp.pvp;
        self.online_mode = sp.online_mode;
        self.white_list = sp.white_list;
        self.dirty = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        on_save: &mut impl FnMut(u16, u64, Vec<String>, ServerProperties),
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
            .desired_rows(6)
            .font(egui::TextStyle::Monospace);

        if ui.add(text_edit).changed() {
            self.dirty = true;
        }

        ui.add_space(10.0);
        ui.small("Common options: -XX:+UseG1GC, -XX:MaxGCPauseMillis=200, etc.");

        ui.add_space(20.0);

        // Server Properties section
        let max_players_valid = self.max_players.parse::<u32>().is_ok();
        egui::CollapsingHeader::new("Server Properties")
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("server_properties_grid")
                    .num_columns(2)
                    .spacing([20.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("MOTD:");
                        if ui.text_edit_singleline(&mut self.motd).changed() {
                            self.dirty = true;
                        }
                        ui.end_row();

                        ui.label("Max Players:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.max_players).desired_width(80.0),
                        );
                        if response.changed() {
                            self.dirty = true;
                        }
                        if !max_players_valid {
                            ui.colored_label(egui::Color32::RED, "Invalid");
                        }
                        ui.end_row();

                        ui.label("Difficulty:");
                        let current_label = format!("{:?}", self.difficulty);
                        egui::ComboBox::from_id_salt("difficulty_combo")
                            .selected_text(&current_label)
                            .show_ui(ui, |ui| {
                                for variant in &Difficulty::ALL {
                                    let label = format!("{:?}", variant);
                                    if ui
                                        .selectable_value(&mut self.difficulty, variant.clone(), &label)
                                        .changed()
                                    {
                                        self.dirty = true;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Game Mode:");
                        let current_label = format!("{:?}", self.gamemode);
                        egui::ComboBox::from_id_salt("gamemode_combo")
                            .selected_text(&current_label)
                            .show_ui(ui, |ui| {
                                for variant in &GameMode::ALL {
                                    let label = format!("{:?}", variant);
                                    if ui
                                        .selectable_value(&mut self.gamemode, variant.clone(), &label)
                                        .changed()
                                    {
                                        self.dirty = true;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("PVP:");
                        if ui.checkbox(&mut self.pvp, "").changed() {
                            self.dirty = true;
                        }
                        ui.end_row();

                        ui.label("Online Mode:");
                        if ui.checkbox(&mut self.online_mode, "").changed() {
                            self.dirty = true;
                        }
                        ui.end_row();

                        ui.label("Whitelist:");
                        if ui.checkbox(&mut self.white_list, "").changed() {
                            self.dirty = true;
                        }
                        ui.end_row();
                    });
            });

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                on_cancel();
            }

            ui.add_space(20.0);

            let port_valid = self.port.parse::<u16>().is_ok();
            let memory_valid = self.memory_mb.parse::<u64>().is_ok();
            let can_save = port_valid && memory_valid && max_players_valid && self.dirty;

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
                let server_properties = ServerProperties {
                    motd: self.motd.clone(),
                    max_players: self.max_players.parse().unwrap_or(20),
                    difficulty: self.difficulty.clone(),
                    gamemode: self.gamemode.clone(),
                    pvp: self.pvp,
                    online_mode: self.online_mode,
                    white_list: self.white_list,
                };
                on_save(port, memory_mb, java_args, server_properties);
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
            ui.small("For advanced options (extra env vars, modpack source), edit");
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
                - Extra environment variables (e.g. CF_EXCLUDE_MODS)\n\
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
