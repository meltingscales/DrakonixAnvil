use crate::templates::ModpackTemplate;
use eframe::egui;

pub struct ServerCreateView {
    pub server_name: String,
    pub selected_template_idx: usize,
    pub port: String,
    pub memory_mb: String,
}

impl Default for ServerCreateView {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            selected_template_idx: 0,
            port: "25565".to_string(),
            memory_mb: "4096".to_string(),
        }
    }
}

impl ServerCreateView {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[ModpackTemplate],
        on_create: &mut impl FnMut(String, &ModpackTemplate, u16, u64),
        on_cancel: &mut impl FnMut(),
    ) {
        ui.heading("Create New Server");
        ui.add_space(20.0);

        egui::Grid::new("create_server_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Server Name:");
                ui.text_edit_singleline(&mut self.server_name);
                ui.end_row();

                ui.label("Modpack:");
                egui::ComboBox::from_id_salt("modpack_select")
                    .selected_text(
                        templates
                            .get(self.selected_template_idx)
                            .map(|t| t.name.as_str())
                            .unwrap_or("Select..."),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, template) in templates.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_template_idx,
                                idx,
                                &template.name,
                            );
                        }
                    });
                ui.end_row();

                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port);
                ui.end_row();

                ui.label("Memory (MB):");
                ui.text_edit_singleline(&mut self.memory_mb);
                ui.end_row();
            });

        // Show template details
        if let Some(template) = templates.get(self.selected_template_idx) {
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(format!("Description: {}", template.description));
            ui.label(format!("Minecraft Version: {}", template.minecraft_version));
            ui.label(format!(
                "Recommended Memory: {} MB",
                template.recommended_memory_mb
            ));
            ui.label(format!("Java Version: {}", template.java_version));
        }

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                on_cancel();
            }

            ui.add_space(20.0);

            let can_create = !self.server_name.is_empty()
                && self.port.parse::<u16>().is_ok()
                && self.memory_mb.parse::<u64>().is_ok();

            if ui
                .add_enabled(can_create, egui::Button::new("Create Server"))
                .clicked()
            {
                if let Some(template) = templates.get(self.selected_template_idx) {
                    let port = self.port.parse().unwrap_or(25565);
                    let memory = self.memory_mb.parse().unwrap_or(4096);
                    on_create(self.server_name.clone(), template, port, memory);
                }
            }
        });
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
