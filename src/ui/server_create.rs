use crate::curseforge::{
    self, CfFile, CfMod, CfSortField,
};
use crate::server::{ModLoader, ModpackSource};
use crate::templates::ModpackTemplate;
use eframe::egui;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CreateTab {
    #[default]
    Featured,
    SearchCurseForge,
}

/// Search filters the user can tweak before hitting Search.
#[derive(Debug, Clone)]
pub struct CfSearchState {
    pub query: String,
    pub mc_version_filter: String,
    pub loader_filter_idx: usize, // 0 = Any, 1 = Forge, 2 = Fabric, 3 = NeoForge
    pub sort_field: CfSortField,
    pub page_offset: u64,
}

impl Default for CfSearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            mc_version_filter: String::new(),
            loader_filter_idx: 0,
            sort_field: CfSortField::Popularity,
            page_offset: 0,
        }
    }
}

impl CfSearchState {
    pub fn selected_loader(&self) -> Option<ModLoader> {
        match self.loader_filter_idx {
            1 => Some(ModLoader::Forge),
            2 => Some(ModLoader::Fabric),
            3 => Some(ModLoader::NeoForge),
            _ => None,
        }
    }

    fn loader_label(&self) -> &'static str {
        match self.loader_filter_idx {
            1 => "Forge",
            2 => "Fabric",
            3 => "NeoForge",
            _ => "Any",
        }
    }
}

/// All CurseForge browse state lives here.
#[derive(Debug, Clone, Default)]
pub struct CfBrowseState {
    pub search: CfSearchState,
    pub results: Vec<CfMod>,
    pub total_count: u64,
    pub loading_search: bool,
    pub search_error: Option<String>,
    pub selected_mod: Option<CfMod>,
    pub versions: Vec<CfFile>,
    pub loading_versions: bool,
    pub versions_error: Option<String>,
    /// Unique MC versions extracted from `versions`, sorted descending
    pub mc_versions: Vec<String>,
    /// Currently selected MC version in the dropdown
    pub selected_mc_version: Option<String>,
    /// Index into `self.versions` (original index, stable across filter changes)
    pub selected_file_idx: Option<usize>,
}

/// Callbacks from the create view back to app.rs.
pub struct CreateViewCallbacks<'a> {
    pub on_create: &'a mut dyn FnMut(String, ModpackTemplate, u16, u64),
    pub on_cancel: &'a mut dyn FnMut(),
    pub on_cf_search: &'a mut dyn FnMut(CfSearchState),
    pub on_cf_fetch_versions: &'a mut dyn FnMut(u64),
    pub has_cf_api_key: bool,
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
    pub cf: CfBrowseState,
    pub cf_template: Option<ModpackTemplate>,
}

impl Default for ServerCreateView {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            port: "25565".to_string(),
            memory_mb: "4096".to_string(),
            active_tab: CreateTab::Featured,
            selected_template_idx: None,
            cf: CfBrowseState::default(),
            cf_template: None,
        }
    }
}

impl ServerCreateView {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[ModpackTemplate],
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
                ui.add(egui::TextEdit::singleline(&mut self.server_name).desired_width(200.0));
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

        // ── Tab content ────────────────────────────────────────────────
        match self.active_tab {
            CreateTab::Featured => {
                self.show_featured_tab(ui, templates);
            }
            CreateTab::SearchCurseForge => {
                self.show_curseforge_tab(ui, callbacks);
            }
        }

        // ── Bottom bar: selection summary + buttons ────────────────────
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Show what's selected
        let selected_template = self.resolve_selected_template(templates);
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
                (callbacks.on_cancel)();
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
                if let Some(template) = selected_template {
                    let port = self.port.parse().unwrap_or(25565);
                    let memory = self.memory_mb.parse().unwrap_or(4096);
                    (callbacks.on_create)(self.server_name.clone(), template, port, memory);
                }
            }
        });
    }

    // ── Featured tab ───────────────────────────────────────────────────

    fn show_featured_tab(&mut self, ui: &mut egui::Ui, templates: &[ModpackTemplate]) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 80.0)
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
                        self.cf_template = None; // Clear CF selection
                        self.memory_mb = template.recommended_memory_mb.to_string();
                    }

                    ui.add_space(4.0);
                }
            });
    }

    // ── CurseForge tab ─────────────────────────────────────────────────

    fn show_curseforge_tab(
        &mut self,
        ui: &mut egui::Ui,
        callbacks: &mut CreateViewCallbacks<'_>,
    ) {
        if !callbacks.has_cf_api_key {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "CurseForge API key required",
                );
                ui.add_space(8.0);
                ui.label("Set your CurseForge API key in Settings to search for modpacks.");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Get a free key at");
                    ui.hyperlink("https://console.curseforge.com/");
                });
            });
            return;
        }

        // ── Search bar ────────────────────────────────────────────────
        let mut trigger_search = false;

        ui.horizontal(|ui| {
            ui.label("Search:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.cf.search.query)
                    .desired_width(200.0)
                    .hint_text("e.g. All The Mods"),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                trigger_search = true;
            }
            if ui.button("Search").clicked() {
                trigger_search = true;
            }
        });

        // ── Filters ───────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("MC Version:");
            ui.add(
                egui::TextEdit::singleline(&mut self.cf.search.mc_version_filter)
                    .desired_width(60.0)
                    .hint_text("e.g. 1.20.1"),
            );

            ui.label("Loader:");
            egui::ComboBox::from_id_salt("cf_loader_filter")
                .selected_text(self.cf.search.loader_label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.cf.search.loader_filter_idx, 0, "Any");
                    ui.selectable_value(&mut self.cf.search.loader_filter_idx, 1, "Forge");
                    ui.selectable_value(&mut self.cf.search.loader_filter_idx, 2, "Fabric");
                    ui.selectable_value(&mut self.cf.search.loader_filter_idx, 3, "NeoForge");
                });

            ui.label("Sort:");
            egui::ComboBox::from_id_salt("cf_sort_field")
                .selected_text(self.cf.search.sort_field.label())
                .show_ui(ui, |ui| {
                    for sf in CfSortField::ALL {
                        ui.selectable_value(&mut self.cf.search.sort_field, sf, sf.label());
                    }
                });
        });

        if trigger_search {
            self.cf.search.page_offset = 0;
            self.cf.loading_search = true;
            self.cf.search_error = None;
            self.cf.selected_mod = None;
            self.cf.versions.clear();
            self.cf.mc_versions.clear();
            self.cf.selected_mc_version = None;
            self.cf.selected_file_idx = None;
            self.cf_template = None;
            (callbacks.on_cf_search)(self.cf.search.clone());
        }

        ui.separator();

        // ── Results area ──────────────────────────────────────────────
        let available = ui.available_height() - 80.0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(available)
            .show(ui, |ui| {
                if self.cf.loading_search {
                    ui.spinner();
                    ui.label("Searching CurseForge...");
                    return;
                }

                if let Some(err) = &self.cf.search_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                    return;
                }

                if self.cf.results.is_empty() && self.cf.total_count == 0 {
                    if self.cf.search.query.is_empty() {
                        ui.label("Enter a search term and click Search to find modpacks.");
                    } else {
                        ui.label("No results found.");
                    }
                    return;
                }

                // ── Result cards ──────────────────────────────────────
                let mut fetch_versions_for: Option<u64> = None;

                for cf_mod in &self.cf.results {
                    let is_selected = self
                        .cf
                        .selected_mod
                        .as_ref()
                        .is_some_and(|m| m.id == cf_mod.id);

                    let frame_fill = if is_selected {
                        egui::Color32::from_rgb(40, 60, 80)
                    } else {
                        ui.style().visuals.extreme_bg_color
                    };

                    let resp = egui::Frame::none()
                        .fill(frame_fill)
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Modpack logo thumbnail
                                if let Some(logo) = &cf_mod.logo {
                                    ui.add(
                                        egui::Image::new(&logo.thumbnail_url)
                                            .max_width(48.0)
                                            .max_height(48.0)
                                            .rounding(4.0),
                                    );
                                } else {
                                    ui.allocate_space(egui::vec2(48.0, 48.0));
                                }

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.strong(&cf_mod.name);
                                        ui.small(format!(
                                            "({} downloads)",
                                            curseforge::format_downloads(cf_mod.download_count)
                                        ));
                                    });
                                    ui.label(&cf_mod.summary);
                                    // Show available MC versions
                                    let mc_versions: Vec<&str> = cf_mod
                                        .latest_files_indexes
                                        .iter()
                                        .map(|f| f.game_version.as_str())
                                        .take(5)
                                        .collect();
                                    if !mc_versions.is_empty() {
                                        ui.small(format!("MC: {}", mc_versions.join(", ")));
                                    }
                                });
                            });
                        })
                        .response;

                    if resp.interact(egui::Sense::click()).clicked() {
                        self.cf.selected_mod = Some(cf_mod.clone());
                        self.cf.versions.clear();
                        self.cf.mc_versions.clear();
                        self.cf.selected_mc_version = None;
                        self.cf.selected_file_idx = None;
                        self.cf.loading_versions = true;
                        self.cf.versions_error = None;
                        self.cf_template = None;
                        self.selected_template_idx = None; // Clear featured selection
                        fetch_versions_for = Some(cf_mod.id);
                    }

                    ui.add_space(3.0);
                }

                if let Some(mod_id) = fetch_versions_for {
                    (callbacks.on_cf_fetch_versions)(mod_id);
                }

                // ── Version picker (below results, if a mod is selected) ──
                if let Some(selected) = self.cf.selected_mod.clone() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.strong(format!("Versions for: {}", selected.name));

                    if self.cf.loading_versions {
                        ui.spinner();
                        ui.label("Loading versions...");
                    } else if let Some(err) = &self.cf.versions_error {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                    } else if self.cf.versions.is_empty() {
                        ui.label("No versions found.");
                    } else {
                        // ── MC Version dropdown ──
                        ui.horizontal(|ui| {
                            ui.label("Minecraft Version:");
                            let mc_label = self
                                .cf
                                .selected_mc_version
                                .as_deref()
                                .unwrap_or("Select...");
                            egui::ComboBox::from_id_salt("cf_mc_version_picker")
                                .selected_text(mc_label)
                                .show_ui(ui, |ui| {
                                    for ver in &self.cf.mc_versions.clone() {
                                        let is_sel = self
                                            .cf
                                            .selected_mc_version
                                            .as_deref()
                                            == Some(ver.as_str());
                                        if ui.selectable_label(is_sel, ver).clicked() {
                                            self.cf.selected_mc_version = Some(ver.clone());
                                            self.cf.selected_file_idx = None;
                                            self.cf_template = None;
                                        }
                                    }
                                });
                        });

                        // ── Filter files by selected MC version ──
                        // Collect (original_index, display_label) to avoid
                        // holding a borrow on self.cf.versions across the
                        // mutable build_cf_template call.
                        let filtered_files: Vec<(usize, String)> = self
                            .cf
                            .versions
                            .iter()
                            .enumerate()
                            .filter(|(_i, f)| match &self.cf.selected_mc_version {
                                Some(mc) => f.game_versions.iter().any(|v| v == mc),
                                None => true,
                            })
                            .map(|(i, f)| {
                                let date_short = f
                                    .file_date
                                    .split('T')
                                    .next()
                                    .unwrap_or(&f.file_date);
                                (i, format!("{} ({})", f.display_name, date_short))
                            })
                            .collect();

                        // ── Pack Version dropdown ──
                        let file_label = self
                            .cf
                            .selected_file_idx
                            .and_then(|idx| self.cf.versions.get(idx))
                            .map(|f| f.display_name.as_str())
                            .unwrap_or("Select...");

                        let mut clicked_file_idx: Option<usize> = None;

                        ui.horizontal(|ui| {
                            ui.label("Pack Version:");
                            egui::ComboBox::from_id_salt("cf_pack_version_picker")
                                .selected_text(file_label)
                                .width(400.0)
                                .show_ui(ui, |ui| {
                                    for (orig_idx, label) in &filtered_files {
                                        let is_sel =
                                            self.cf.selected_file_idx == Some(*orig_idx);
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            clicked_file_idx = Some(*orig_idx);
                                        }
                                    }
                                });
                        });

                        if let Some(orig_idx) = clicked_file_idx {
                            self.cf.selected_file_idx = Some(orig_idx);
                            let file = self.cf.versions[orig_idx].clone();
                            self.build_cf_template(&selected, &file);
                        }

                        if filtered_files.is_empty() {
                            ui.small("No files for this Minecraft version.");
                        }
                    }
                }

                // ── Pagination ────────────────────────────────────────
                if self.cf.total_count > 0 {
                    ui.add_space(8.0);
                    ui.separator();
                    let page = (self.cf.search.page_offset / 20) + 1;
                    let total_pages = self.cf.total_count.div_ceil(20);

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(page > 1, egui::Button::new("< Prev"))
                            .clicked()
                        {
                            self.cf.search.page_offset =
                                self.cf.search.page_offset.saturating_sub(20);
                            self.cf.loading_search = true;
                            self.cf.search_error = None;
                            (callbacks.on_cf_search)(self.cf.search.clone());
                        }

                        ui.label(format!("Page {} / {}", page, total_pages));

                        if ui
                            .add_enabled(page < total_pages, egui::Button::new("Next >"))
                            .clicked()
                        {
                            self.cf.search.page_offset += 20;
                            self.cf.loading_search = true;
                            self.cf.search_error = None;
                            (callbacks.on_cf_search)(self.cf.search.clone());
                        }
                    });
                }
            });
    }

    // ── Build template from CF data ────────────────────────────────────

    fn build_cf_template(&mut self, cf_mod: &CfMod, cf_file: &CfFile) {
        // Detect MC version: first game_version that starts with a digit
        let mc_version = cf_file
            .game_versions
            .iter()
            .find(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            .cloned()
            .unwrap_or_default();

        // Detect loader from game_versions strings
        let loader = if cf_file
            .game_versions
            .iter()
            .any(|v| v.eq_ignore_ascii_case("NeoForge"))
        {
            ModLoader::NeoForge
        } else if cf_file
            .game_versions
            .iter()
            .any(|v| v.eq_ignore_ascii_case("Fabric"))
        {
            ModLoader::Fabric
        } else {
            ModLoader::Forge
        };

        // Prefer server_pack_file_id, fall back to file id
        let file_id = cf_file.server_pack_file_id.unwrap_or(cf_file.id);

        let java_version = curseforge::infer_java_version(&mc_version);
        let memory = curseforge::default_memory_mb(&mc_version);

        let template = ModpackTemplate {
            name: cf_mod.name.clone(),
            description: cf_mod.summary.clone(),
            version: cf_file.display_name.clone(),
            minecraft_version: mc_version,
            loader,
            source: ModpackSource::CurseForge {
                slug: cf_mod.slug.clone(),
                file_id,
            },
            recommended_memory_mb: memory,
            java_version,
            default_java_args: curseforge::default_java_args(),
            default_extra_env: vec![],
        };

        self.memory_mb = template.recommended_memory_mb.to_string();
        self.cf_template = Some(template);
    }

    /// Determine the currently-selected template (Featured or CF).
    fn resolve_selected_template(&self, templates: &[ModpackTemplate]) -> Option<ModpackTemplate> {
        if self.active_tab == CreateTab::Featured {
            self.selected_template_idx
                .and_then(|idx| templates.get(idx))
                .cloned()
        } else {
            self.cf_template.clone()
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
