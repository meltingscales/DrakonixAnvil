use crate::templates::ModpackTemplate;
use crate::ui::cf_browse::{CfBrowseWidget, CfCallbacks};
use eframe::egui;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CreateTab {
    #[default]
    Featured,
    SearchCurseForge,
}

/// Callbacks from the create view back to app.rs.
pub struct CreateViewCallbacks<'a> {
    pub on_create: &'a mut dyn FnMut(String, ModpackTemplate, u16, u64),
    pub on_cancel: &'a mut dyn FnMut(),
}

// ── ServerCreateView ───────────────────────────────────────────────────────

pub struct ServerCreateView {
    // Common fields
    pub server_name: String,
    pub port: String,
    pub memory_mb: String,
    // Tab
    pub active_tab: CreateTab,
    // Featured
    pub selected_template_idx: Option<usize>,
    // CurseForge
    pub cf: CfBrowseWidget,
}

impl Default for ServerCreateView {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            port: "25565".to_string(),
            memory_mb: "4096".to_string(),
            active_tab: CreateTab::Featured,
            selected_template_idx: None,
            cf: CfBrowseWidget::default(),
        }
    }
}

impl ServerCreateView {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[ModpackTemplate],
        cf_callbacks: &mut CfCallbacks<'_>,
        callbacks: &mut CreateViewCallbacks<'_>,
    ) {
        ui.heading("Create New Server");
        ui.add_space(10.0);

        // ── Common fields ──────────────────────────────────────────────
        egui::Grid::new("create_common_fields")
            .num_columns(6)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Server Name:");
                ui.add(egui::TextEdit::singleline(&mut self.server_name).desired_width(300.0));
                ui.label("Port:");
                ui.add(egui::TextEdit::singleline(&mut self.port).desired_width(60.0));
                ui.label("Memory (MB):");
                ui.add(egui::TextEdit::singleline(&mut self.memory_mb).desired_width(60.0));
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // ── Tabs ───────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.active_tab == CreateTab::Featured, "Featured")
                .clicked()
            {
                self.active_tab = CreateTab::Featured;
            }
            if ui
                .selectable_label(
                    self.active_tab == CreateTab::SearchCurseForge,
                    "Search CurseForge",
                )
                .clicked()
            {
                self.active_tab = CreateTab::SearchCurseForge;
            }
        });
        ui.separator();

        // ── Bottom bar: pinned at bottom ────────────────────────────
        let selected_template = self.resolve_selected_template(templates);
        let mut should_cancel = false;
        let mut should_create = false;
        let create_template = selected_template.clone();

        egui::TopBottomPanel::bottom("create_server_bottom_bar").show_inside(ui, |ui| {
            ui.add_space(4.0);

            if let Some(t) = &selected_template {
                ui.horizontal(|ui| {
                    ui.strong("Selected:");
                    ui.label(format!(
                        "{} (MC {}, {:?}, Java {})",
                        t.name, t.minecraft_version, t.loader, t.java_version
                    ));
                });
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    should_cancel = true;
                }

                ui.add_space(20.0);

                let can_create = !self.server_name.is_empty()
                    && self.port.parse::<u16>().is_ok()
                    && self.memory_mb.parse::<u64>().is_ok()
                    && selected_template.is_some();

                if ui
                    .add_enabled(can_create, egui::Button::new("Create Server"))
                    .clicked()
                {
                    should_create = true;
                }
            });
            ui.add_space(4.0);
        });

        // ── Tab content (fills remaining space) ─────────────────────
        match self.active_tab {
            CreateTab::Featured => {
                self.show_featured_tab(ui, templates);
            }
            CreateTab::SearchCurseForge => {
                if self.cf.show(ui, "create_cf", cf_callbacks) {
                    // Template was just built — update memory from it
                    if let Some(t) = &self.cf.template {
                        self.memory_mb = t.recommended_memory_mb.to_string();
                    }
                }
            }
        }

        // ── Act on bottom bar clicks ────────────────────────────────
        if should_cancel {
            (callbacks.on_cancel)();
        }
        if should_create {
            if let Some(template) = create_template {
                let port = self.port.parse().unwrap_or(25565);
                let memory = self.memory_mb.parse().unwrap_or(4096);
                (callbacks.on_create)(self.server_name.clone(), template, port, memory);
            }
        }
    }

    // ── Featured tab ───────────────────────────────────────────────────

    fn show_featured_tab(&mut self, ui: &mut egui::Ui, templates: &[ModpackTemplate]) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for (idx, template) in templates.iter().enumerate() {
                    let is_selected = self.selected_template_idx == Some(idx);
                    let frame_fill = if is_selected {
                        egui::Color32::from_rgb(40, 60, 80)
                    } else {
                        ui.style().visuals.extreme_bg_color
                    };

                    let resp = egui::Frame::none()
                        .fill(frame_fill)
                        .rounding(6.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(&template.name);
                                    ui.label(&template.description);
                                    ui.horizontal(|ui| {
                                        ui.small(format!("MC {}", template.minecraft_version));
                                        ui.small("|");
                                        ui.small(format!("{:?}", template.loader));
                                        ui.small("|");
                                        ui.small(format!("Java {}", template.java_version));
                                        ui.small("|");
                                        ui.small(format!("{} MB", template.recommended_memory_mb));
                                    });
                                });
                            });
                        })
                        .response;

                    if resp.interact(egui::Sense::click()).clicked() {
                        self.selected_template_idx = Some(idx);
                        self.cf.template = None; // Clear CF selection
                        self.memory_mb = template.recommended_memory_mb.to_string();
                    }

                    ui.add_space(4.0);
                }
            });
    }

    /// Determine the currently-selected template (Featured or CF).
    fn resolve_selected_template(&self, templates: &[ModpackTemplate]) -> Option<ModpackTemplate> {
        if self.active_tab == CreateTab::Featured {
            self.selected_template_idx
                .and_then(|idx| templates.get(idx))
                .cloned()
        } else {
            self.cf.template.clone()
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
