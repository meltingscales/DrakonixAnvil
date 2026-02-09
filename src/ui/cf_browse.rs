use crate::curseforge::{self, CfFile, CfMod, CfSortField};
use crate::server::{ModLoader, ModpackSource};
use crate::templates::ModpackTemplate;
use eframe::egui;

// ── Types ──────────────────────────────────────────────────────────────────

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
    /// Full description text (fetched from CurseForge API, HTML stripped)
    pub description: Option<String>,
    /// Whether we're currently fetching the description
    pub loading_description: bool,
}

/// Callbacks for triggering async CurseForge work from the widget.
pub struct CfCallbacks<'a> {
    pub on_search: &'a mut dyn FnMut(CfSearchState),
    pub on_fetch_versions: &'a mut dyn FnMut(u64),
    pub on_fetch_description: &'a mut dyn FnMut(u64),
    pub has_api_key: bool,
}

// ── CfBrowseWidget ─────────────────────────────────────────────────────────

#[derive(Default)]
pub struct CfBrowseWidget {
    pub state: CfBrowseState,
    pub template: Option<ModpackTemplate>,
}

impl CfBrowseWidget {
    /// Show the full CurseForge browse UI.
    ///
    /// `id_salt` prevents egui ID collisions when multiple instances exist.
    /// Returns `true` when `self.template` was just built this frame (user picked a version).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: &str,
        callbacks: &mut CfCallbacks<'_>,
    ) -> bool {
        let mut template_built = false;

        ui.push_id(id_salt, |ui| {
            if !callbacks.has_api_key {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "CurseForge API key required");
                    ui.add_space(8.0);
                    ui.label(
                        "Set your CurseForge API key in Settings to search for modpacks.",
                    );
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
                    egui::TextEdit::singleline(&mut self.state.search.query)
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
                    egui::TextEdit::singleline(&mut self.state.search.mc_version_filter)
                        .desired_width(60.0)
                        .hint_text("e.g. 1.20.1"),
                );

                ui.label("Loader:");
                egui::ComboBox::from_id_salt("cf_loader_filter")
                    .selected_text(self.state.search.loader_label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.state.search.loader_filter_idx,
                            0,
                            "Any",
                        );
                        ui.selectable_value(
                            &mut self.state.search.loader_filter_idx,
                            1,
                            "Forge",
                        );
                        ui.selectable_value(
                            &mut self.state.search.loader_filter_idx,
                            2,
                            "Fabric",
                        );
                        ui.selectable_value(
                            &mut self.state.search.loader_filter_idx,
                            3,
                            "NeoForge",
                        );
                    });

                ui.label("Sort:");
                egui::ComboBox::from_id_salt("cf_sort_field")
                    .selected_text(self.state.search.sort_field.label())
                    .show_ui(ui, |ui| {
                        for sf in CfSortField::ALL {
                            ui.selectable_value(
                                &mut self.state.search.sort_field,
                                sf,
                                sf.label(),
                            );
                        }
                    });
            });

            if trigger_search {
                self.state.search.page_offset = 0;
                self.state.loading_search = true;
                self.state.search_error = None;
                self.state.selected_mod = None;
                self.state.versions.clear();
                self.state.mc_versions.clear();
                self.state.selected_mc_version = None;
                self.state.selected_file_idx = None;
                self.state.description = None;
                self.state.loading_description = false;
                self.template = None;
                (callbacks.on_search)(self.state.search.clone());
            }

            ui.separator();

            // ── Split layout: results list (left) + preview panel (right) ──
            let available = ui.available_height();

            if self.state.loading_search {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Searching CurseForge...");
                });
                return;
            }

            if let Some(err) = &self.state.search_error.clone() {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                return;
            }

            if self.state.results.is_empty() && self.state.total_count == 0 {
                if self.state.search.query.is_empty() {
                    ui.label("Enter a search term and click Search to find modpacks.");
                } else {
                    ui.label("No results found.");
                }
                return;
            }

            // Collect which mod to fetch (versions + description) if clicked
            let mut fetch_mod_id: Option<u64> = None;

            let has_preview = self.state.selected_mod.is_some();

            let total_width = ui.available_width();
            let left_width = if has_preview {
                (total_width * 0.4).max(250.0)
            } else {
                total_width
            };

            ui.horizontal_top(|ui| {
                // ── Left column: result list ──────────────────────────
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, available),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("cf_results_scroll")
                            .auto_shrink([false, false])
                            .max_height(available)
                            .show(ui, |ui| {
                                for cf_mod in &self.state.results.clone() {
                                    let is_selected = self
                                        .state
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
                                                // Modpack logo thumbnail (64px)
                                                if let Some(logo) = &cf_mod.logo {
                                                    ui.add(
                                                        egui::Image::new(&logo.thumbnail_url)
                                                            .max_width(64.0)
                                                            .max_height(64.0)
                                                            .rounding(4.0),
                                                    );
                                                } else {
                                                    ui.allocate_space(egui::vec2(64.0, 64.0));
                                                }

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.strong(&cf_mod.name);
                                                        ui.small(format!(
                                                            "({} downloads)",
                                                            curseforge::format_downloads(
                                                                cf_mod.download_count,
                                                            )
                                                        ));
                                                    });
                                                    ui.label(&cf_mod.summary);
                                                    let mc_versions: Vec<&str> = cf_mod
                                                        .latest_files_indexes
                                                        .iter()
                                                        .map(|f| f.game_version.as_str())
                                                        .take(5)
                                                        .collect();
                                                    if !mc_versions.is_empty() {
                                                        ui.small(format!(
                                                            "MC: {}",
                                                            mc_versions.join(", ")
                                                        ));
                                                    }
                                                });
                                            });
                                        })
                                        .response;

                                    if resp.interact(egui::Sense::click()).clicked() {
                                        self.state.selected_mod = Some(cf_mod.clone());
                                        self.state.versions.clear();
                                        self.state.mc_versions.clear();
                                        self.state.selected_mc_version = None;
                                        self.state.selected_file_idx = None;
                                        self.state.loading_versions = true;
                                        self.state.versions_error = None;
                                        self.state.description = None;
                                        self.state.loading_description = true;
                                        self.template = None;
                                        fetch_mod_id = Some(cf_mod.id);
                                    }

                                    ui.add_space(3.0);
                                }

                                // ── Pagination ────────────────────────────
                                if self.state.total_count > 0 {
                                    ui.add_space(8.0);
                                    ui.separator();
                                    let page = (self.state.search.page_offset / 20) + 1;
                                    let total_pages = self.state.total_count.div_ceil(20);

                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                page > 1,
                                                egui::Button::new("< Prev"),
                                            )
                                            .clicked()
                                        {
                                            self.state.search.page_offset =
                                                self.state.search.page_offset.saturating_sub(20);
                                            self.state.loading_search = true;
                                            self.state.search_error = None;
                                            (callbacks.on_search)(self.state.search.clone());
                                        }

                                        ui.label(format!("Page {} / {}", page, total_pages));

                                        if ui
                                            .add_enabled(
                                                page < total_pages,
                                                egui::Button::new("Next >"),
                                            )
                                            .clicked()
                                        {
                                            self.state.search.page_offset += 20;
                                            self.state.loading_search = true;
                                            self.state.search_error = None;
                                            (callbacks.on_search)(self.state.search.clone());
                                        }
                                    });
                                }
                            });
                    },
                );

                // ── Right column: preview panel ──────────────────────
                if has_preview {
                    ui.separator();
                    let right_width = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_width, available),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            if self.show_preview_panel(ui, available) {
                                template_built = true;
                            }
                        },
                    );
                }
            });

            if let Some(mod_id) = fetch_mod_id {
                (callbacks.on_fetch_versions)(mod_id);
                (callbacks.on_fetch_description)(mod_id);
            }
        });

        template_built
    }

    // ── Preview panel (right side) ──────────────────────────────────
    // Returns true if a template was built this frame.

    fn show_preview_panel(&mut self, ui: &mut egui::Ui, available_height: f32) -> bool {
        let selected = match self.state.selected_mod.clone() {
            Some(m) => m,
            None => return false,
        };

        let mut built = false;

        egui::ScrollArea::vertical()
            .id_salt("cf_preview_scroll")
            .auto_shrink([false, false])
            .max_height(available_height)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // ── Large logo ──
                    if let Some(logo) = &selected.logo {
                        ui.add(
                            egui::Image::new(&logo.thumbnail_url)
                                .max_width(128.0)
                                .max_height(128.0)
                                .rounding(8.0),
                        );
                        ui.add_space(8.0);
                    }

                    // ── Title + stats ──
                    ui.heading(&selected.name);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} downloads",
                            curseforge::format_downloads(selected.download_count)
                        ));
                    });
                    ui.add_space(4.0);

                    // ── MC versions from latest_files_indexes ──
                    let mc_versions: Vec<&str> = selected
                        .latest_files_indexes
                        .iter()
                        .map(|f| f.game_version.as_str())
                        .collect();
                    if !mc_versions.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("MC Versions: ");
                            ui.label(mc_versions.join(", "));
                        });
                        ui.add_space(4.0);
                    }

                    // ── Description ──
                    ui.separator();
                    ui.add_space(4.0);
                    if self.state.loading_description {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading description...");
                        });
                    } else if let Some(desc) = &self.state.description {
                        ui.label(desc);
                    } else {
                        ui.label(&selected.summary);
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // ── Version picker ──
                    ui.strong("Version Selection");
                    ui.add_space(4.0);

                    if self.state.loading_versions {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading versions...");
                        });
                    } else if let Some(err) = &self.state.versions_error.clone() {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                    } else if self.state.versions.is_empty() {
                        ui.label("No versions found.");
                    } else {
                        // ── MC Version dropdown ──
                        ui.horizontal(|ui| {
                            ui.label("Minecraft Version:");
                            let mc_label = self
                                .state
                                .selected_mc_version
                                .as_deref()
                                .unwrap_or("Select...");
                            egui::ComboBox::from_id_salt("cf_mc_version_picker")
                                .selected_text(mc_label)
                                .show_ui(ui, |ui| {
                                    for ver in &self.state.mc_versions.clone() {
                                        let is_sel =
                                            self.state.selected_mc_version.as_deref()
                                                == Some(ver.as_str());
                                        if ui.selectable_label(is_sel, ver).clicked() {
                                            self.state.selected_mc_version = Some(ver.clone());
                                            self.state.selected_file_idx = None;
                                            self.template = None;
                                        }
                                    }
                                });
                        });

                        // ── Filter files by selected MC version ──
                        let filtered_files: Vec<(usize, String)> = self
                            .state
                            .versions
                            .iter()
                            .enumerate()
                            .filter(|(_i, f)| match &self.state.selected_mc_version {
                                Some(mc) => f.game_versions.iter().any(|v| v == mc),
                                None => true,
                            })
                            .map(|(i, f)| {
                                let date_short =
                                    f.file_date.split('T').next().unwrap_or(&f.file_date);
                                (i, format!("{} ({})", f.display_name, date_short))
                            })
                            .collect();

                        // ── Pack Version dropdown ──
                        let file_label = self
                            .state
                            .selected_file_idx
                            .and_then(|idx| self.state.versions.get(idx))
                            .map(|f| f.display_name.as_str())
                            .unwrap_or("Select...");

                        let mut clicked_file_idx: Option<usize> = None;

                        ui.horizontal(|ui| {
                            ui.label("Pack Version:");
                            egui::ComboBox::from_id_salt("cf_pack_version_picker")
                                .selected_text(file_label)
                                .width(300.0)
                                .show_ui(ui, |ui| {
                                    for (orig_idx, label) in &filtered_files {
                                        let is_sel =
                                            self.state.selected_file_idx == Some(*orig_idx);
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            clicked_file_idx = Some(*orig_idx);
                                        }
                                    }
                                });
                        });

                        if let Some(orig_idx) = clicked_file_idx {
                            self.state.selected_file_idx = Some(orig_idx);
                            let file = self.state.versions[orig_idx].clone();
                            self.build_cf_template(&selected, &file);
                            built = true;
                        }

                        if filtered_files.is_empty() {
                            ui.small("No files for this Minecraft version.");
                        }
                    }
                });
            });

        built
    }

    // ── Build template from CF data ────────────────────────────────────

    pub fn build_cf_template(&mut self, cf_mod: &CfMod, cf_file: &CfFile) {
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

        // AUTO_CURSEFORGE needs the client modpack file (which has the manifest),
        // not the server pack file. Always use the main file id.
        let file_id = cf_file.id;

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

        self.template = Some(template);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
