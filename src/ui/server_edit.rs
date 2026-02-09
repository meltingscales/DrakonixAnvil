use crate::server::{
    Difficulty, GameMode, ModLoader, ModpackInfo, ModpackSource, ServerConfig, ServerProperties,
};
use crate::templates::ModpackTemplate;
use crate::ui::cf_browse::{CfBrowseWidget, CfCallbacks};
use crate::ui::mr_browse::{MrBrowseWidget, MrCallbacks};
use eframe::egui;

pub struct ServerEditResult {
    pub port: u16,
    pub memory_mb: u64,
    pub java_args: Vec<String>,
    pub server_properties: ServerProperties,
    pub modpack: ModpackInfo,
    pub java_version: u8,
    pub extra_env: Vec<String>,
}

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
    // Modpack info
    pub modpack_name: String,
    pub modpack_version: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub source: ModpackSource,
    // Java version & extra env
    pub java_version: String,
    pub extra_env: String,
    // Template picker
    pub selected_template_idx: Option<usize>,
    // CurseForge browse
    pub cf: CfBrowseWidget,
    // Modrinth browse
    pub mr: MrBrowseWidget,
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
            modpack_name: String::new(),
            modpack_version: String::new(),
            minecraft_version: String::new(),
            loader: ModLoader::Vanilla,
            source: ModpackSource::Local {
                path: ".".to_string(),
            },
            java_version: "21".to_string(),
            extra_env: String::new(),
            selected_template_idx: None,
            cf: CfBrowseWidget::default(),
            mr: MrBrowseWidget::default(),
            dirty: false,
        }
    }
}

const JAVA_VERSIONS: &[&str] = &["8", "11", "17", "21"];

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
        // Modpack
        self.modpack_name = config.modpack.name.clone();
        self.modpack_version = config.modpack.version.clone();
        self.minecraft_version = config.modpack.minecraft_version.clone();
        self.loader = config.modpack.loader.clone();
        self.source = config.modpack.source.clone();
        // Java version & extra env
        self.java_version = config.java_version.to_string();
        self.extra_env = config.extra_env.join("\n");
        self.selected_template_idx = None;
        self.cf.reset();
        self.mr.reset();
        self.dirty = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[ModpackTemplate],
        cf_callbacks: &mut CfCallbacks<'_>,
        mr_callbacks: &mut MrCallbacks<'_>,
        on_save: &mut impl FnMut(ServerEditResult),
        on_cancel: &mut impl FnMut(),
    ) {
        ui.heading(format!("Edit Server: {}", self.server_name));
        ui.add_space(20.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {

        // ── Modpack section ──────────────────────────────────────
        egui::CollapsingHeader::new("Modpack")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!(
                    "Current: {} v{} (MC {}, {:?})",
                    self.modpack_name, self.modpack_version, self.minecraft_version, self.loader
                ));
                ui.add_space(5.0);
                ui.label(format!("Source: {}", format_source(&self.source)));
                ui.add_space(10.0);

                // Template picker
                ui.horizontal(|ui| {
                    ui.label("Apply builtin template:");
                    let selected_label = match self.selected_template_idx {
                        Some(idx) => templates.get(idx).map_or("—", |t| t.name.as_str()),
                        None => "— select —",
                    };
                    egui::ComboBox::from_id_salt("template_picker")
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for (i, t) in templates.iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut self.selected_template_idx,
                                        Some(i),
                                        &t.name,
                                    )
                                    .on_hover_text(&t.description)
                                    .changed()
                                {
                                    // selection changed — don't apply yet
                                }
                            }
                        });

                    if self.selected_template_idx.is_some()
                        && ui.button("Apply Template").clicked()
                    {
                        if let Some(t) = self
                            .selected_template_idx
                            .and_then(|i| templates.get(i))
                        {
                            self.apply_template(t);
                        }
                    }
                });

                ui.add_space(10.0);

                // ── CurseForge search section ────────────────────
                egui::CollapsingHeader::new("Search CurseForge")
                    .default_open(false)
                    .show(ui, |ui| {
                        self.cf.show(ui, "edit_cf", cf_callbacks);

                        ui.add_space(8.0);
                        let has_cf_template = self.cf.template.is_some();
                        if ui
                            .add_enabled(
                                has_cf_template,
                                egui::Button::new("Apply CurseForge Pack"),
                            )
                            .clicked()
                        {
                            if let Some(t) = &self.cf.template.clone() {
                                self.apply_template(t);
                            }
                        }
                    });

                // ── Modrinth search section ──────────────────────
                egui::CollapsingHeader::new("Search Modrinth")
                    .default_open(false)
                    .show(ui, |ui| {
                        self.mr.show(ui, "edit_mr", mr_callbacks);

                        ui.add_space(8.0);
                        let has_mr_template = self.mr.template.is_some();
                        if ui
                            .add_enabled(
                                has_mr_template,
                                egui::Button::new("Apply Modrinth Pack"),
                            )
                            .clicked()
                        {
                            if let Some(t) = &self.mr.template.clone() {
                                self.apply_template(t);
                            }
                        }
                    });
            });

        ui.add_space(10.0);

        // ── Port / Memory grid ───────────────────────────────────
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

        // ── Java Version & Extra Env ─────────────────────────────
        egui::Grid::new("java_env_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Java Version:");
                egui::ComboBox::from_id_salt("java_version_combo")
                    .selected_text(&self.java_version)
                    .show_ui(ui, |ui| {
                        for &v in JAVA_VERSIONS {
                            if ui
                                .selectable_value(&mut self.java_version, v.to_string(), v)
                                .changed()
                            {
                                self.dirty = true;
                            }
                        }
                    });
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.label("Extra Environment Variables (one per line, KEY=VALUE):");
        ui.add_space(5.0);

        let env_edit = egui::TextEdit::multiline(&mut self.extra_env)
            .desired_width(f32::INFINITY)
            .desired_rows(4)
            .font(egui::TextStyle::Monospace);

        if ui.add(env_edit).changed() {
            self.dirty = true;
        }

        ui.add_space(10.0);
        ui.small("e.g. CF_EXCLUDE_MODS=optifine, CF_FORCE_SYNCHRONIZE=true");

        ui.add_space(20.0);

        // ── Server Properties section ────────────────────────────
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
            let java_version_valid = self.java_version.parse::<u8>().is_ok();
            let can_save =
                port_valid && memory_valid && max_players_valid && java_version_valid && self.dirty;

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
                let modpack = ModpackInfo {
                    name: self.modpack_name.clone(),
                    version: self.modpack_version.clone(),
                    minecraft_version: self.minecraft_version.clone(),
                    loader: self.loader.clone(),
                    source: self.source.clone(),
                };
                let java_version = self.java_version.parse().unwrap_or(21);
                let extra_env: Vec<String> = self
                    .extra_env
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                on_save(ServerEditResult {
                    port,
                    memory_mb,
                    java_args,
                    server_properties,
                    modpack,
                    java_version,
                    extra_env,
                });
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

            }); // end ScrollArea
    }

    /// Apply a modpack template (builtin or CurseForge) to this edit view.
    fn apply_template(&mut self, t: &ModpackTemplate) {
        self.modpack_name = t.name.clone();
        self.modpack_version = t.version.clone();
        self.minecraft_version = t.minecraft_version.clone();
        self.loader = t.loader.clone();
        self.source = t.source.clone();
        self.memory_mb = t.recommended_memory_mb.to_string();
        self.java_version = t.java_version.to_string();
        self.java_args = t.default_java_args.join("\n");
        self.extra_env = t.default_extra_env.join("\n");
        self.dirty = true;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn format_source(source: &ModpackSource) -> String {
    match source {
        ModpackSource::CurseForge { slug, file_id } => {
            if *file_id == 0 {
                format!("CurseForge: {} (latest)", slug)
            } else {
                format!("CurseForge: {} (file {})", slug, file_id)
            }
        }
        ModpackSource::ForgeWithPack {
            forge_version,
            pack_url,
        } => format!("Forge {} + pack ({})", forge_version, pack_url),
        ModpackSource::Ftb {
            pack_id,
            version_id,
        } => format!("FTB (pack {}, version {})", pack_id, version_id),
        ModpackSource::Modrinth {
            project_id,
            version_id,
        } => format!("Modrinth: {} v{}", project_id, version_id),
        ModpackSource::DirectDownload { url } => format!("Direct: {}", url),
        ModpackSource::Local { path } => format!("Local: {}", path),
    }
}
