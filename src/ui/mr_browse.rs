use crate::curseforge;
use crate::modrinth::{self, MrProject, MrSortIndex, MrVersion};
use crate::server::{ModLoader, ModpackSource};
use crate::templates::ModpackTemplate;
use eframe::egui;

// ── Types ──────────────────────────────────────────────────────────────────

/// Search filters the user can tweak before hitting Search.
#[derive(Debug, Clone)]
pub struct MrSearchState {
    pub query: String,
    pub mc_version_filter: String,
    pub loader_filter_idx: usize, // 0 = Any, 1 = Forge, 2 = Fabric, 3 = NeoForge
    pub sort_index: MrSortIndex,
    pub page_offset: u64,
}

impl Default for MrSearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            mc_version_filter: String::new(),
            loader_filter_idx: 0,
            sort_index: MrSortIndex::Downloads,
            page_offset: 0,
        }
    }
}

impl MrSearchState {
    pub fn selected_loader_str(&self) -> &str {
        match self.loader_filter_idx {
            1 => "forge",
            2 => "fabric",
            3 => "neoforge",
            _ => "",
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

/// All Modrinth browse state lives here.
#[derive(Debug, Clone, Default)]
pub struct MrBrowseState {
    pub search: MrSearchState,
    pub results: Vec<MrProject>,
    pub total_count: u64,
    pub loading_search: bool,
    pub search_error: Option<String>,
    pub selected_project: Option<MrProject>,
    pub versions: Vec<MrVersion>,
    pub loading_versions: bool,
    pub versions_error: Option<String>,
    /// Unique MC versions extracted from `versions`, sorted descending
    pub mc_versions: Vec<String>,
    /// Currently selected MC version in the dropdown
    pub selected_mc_version: Option<String>,
    /// Index into `self.versions` (original index, stable across filter changes)
    pub selected_version_idx: Option<usize>,
    /// Full description text (fetched from Modrinth project body, markdown)
    pub description: Option<String>,
    /// Whether we're currently fetching the description
    pub loading_description: bool,
}

/// Callbacks for triggering async Modrinth work from the widget.
pub struct MrCallbacks<'a> {
    pub on_search: &'a mut dyn FnMut(MrSearchState),
    pub on_fetch_versions: &'a mut dyn FnMut(String),
    pub on_fetch_description: &'a mut dyn FnMut(String),
}

// ── MrBrowseWidget ─────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MrBrowseWidget {
    pub state: MrBrowseState,
    pub template: Option<ModpackTemplate>,
}

impl MrBrowseWidget {
    /// Show the full Modrinth browse UI.
    ///
    /// `id_salt` prevents egui ID collisions when multiple instances exist.
    /// Returns `true` when `self.template` was just built this frame (user picked a version).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: &str,
        callbacks: &mut MrCallbacks<'_>,
    ) -> bool {
        let mut template_built = false;

        ui.push_id(id_salt, |ui| {
            // ── Search bar ────────────────────────────────────────────────
            let mut trigger_search = false;

            ui.horizontal(|ui| {
                ui.label("Search:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.state.search.query)
                        .desired_width(200.0)
                        .hint_text("e.g. Cobblemon"),
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
                egui::ComboBox::from_id_salt(format!("{}_mr_loader_filter", id_salt))
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
                egui::ComboBox::from_id_salt(format!("{}_mr_sort_field", id_salt))
                    .selected_text(self.state.search.sort_index.label())
                    .show_ui(ui, |ui| {
                        for si in MrSortIndex::ALL {
                            ui.selectable_value(
                                &mut self.state.search.sort_index,
                                si,
                                si.label(),
                            );
                        }
                    });
            });

            if trigger_search {
                self.state.search.page_offset = 0;
                self.state.loading_search = true;
                self.state.search_error = None;
                self.state.selected_project = None;
                self.state.versions.clear();
                self.state.mc_versions.clear();
                self.state.selected_mc_version = None;
                self.state.selected_version_idx = None;
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
                    ui.label("Searching Modrinth...");
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

            // Collect which project to fetch (versions + description) if clicked
            let mut fetch_project_id: Option<String> = None;

            let has_preview = self.state.selected_project.is_some();

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
                            .id_salt(format!("{}_mr_results_scroll", id_salt))
                            .auto_shrink([false, false])
                            .max_height(available)
                            .show(ui, |ui| {
                                for project in &self.state.results.clone() {
                                    let is_selected = self
                                        .state
                                        .selected_project
                                        .as_ref()
                                        .is_some_and(|p| {
                                            p.project_id == project.project_id
                                        });

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
                                                // Modpack icon thumbnail (64px)
                                                if let Some(icon_url) = &project.icon_url {
                                                    ui.add(
                                                        egui::Image::new(icon_url)
                                                            .max_width(64.0)
                                                            .max_height(64.0)
                                                            .rounding(4.0),
                                                    );
                                                } else {
                                                    ui.allocate_space(egui::vec2(
                                                        64.0, 64.0,
                                                    ));
                                                }

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.strong(&project.title);
                                                        ui.small(format!(
                                                            "({} downloads)",
                                                            curseforge::format_downloads(
                                                                project.downloads,
                                                            )
                                                        ));
                                                    });
                                                    ui.label(&project.description);
                                                    if !project.categories.is_empty() {
                                                        let cats: Vec<&str> = project
                                                            .categories
                                                            .iter()
                                                            .take(5)
                                                            .map(|s| s.as_str())
                                                            .collect();
                                                        ui.small(cats.join(", "));
                                                    }
                                                });
                                            });
                                        })
                                        .response;

                                    if resp.interact(egui::Sense::click()).clicked() {
                                        self.state.selected_project =
                                            Some(project.clone());
                                        self.state.versions.clear();
                                        self.state.mc_versions.clear();
                                        self.state.selected_mc_version = None;
                                        self.state.selected_version_idx = None;
                                        self.state.loading_versions = true;
                                        self.state.versions_error = None;
                                        self.state.description = None;
                                        self.state.loading_description = true;
                                        self.template = None;
                                        fetch_project_id =
                                            Some(project.slug.clone());
                                    }

                                    ui.add_space(3.0);
                                }

                                // ── Pagination ────────────────────────────
                                if self.state.total_count > 0 {
                                    ui.add_space(8.0);
                                    ui.separator();
                                    let page =
                                        (self.state.search.page_offset / 20) + 1;
                                    let total_pages =
                                        self.state.total_count.div_ceil(20);

                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                page > 1,
                                                egui::Button::new("< Prev"),
                                            )
                                            .clicked()
                                        {
                                            self.state.search.page_offset = self
                                                .state
                                                .search
                                                .page_offset
                                                .saturating_sub(20);
                                            self.state.loading_search = true;
                                            self.state.search_error = None;
                                            (callbacks.on_search)(
                                                self.state.search.clone(),
                                            );
                                        }

                                        ui.label(format!(
                                            "Page {} / {}",
                                            page, total_pages
                                        ));

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
                                            (callbacks.on_search)(
                                                self.state.search.clone(),
                                            );
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
                            if self.show_preview_panel(ui, id_salt, available) {
                                template_built = true;
                            }
                        },
                    );
                }
            });

            if let Some(project_id) = fetch_project_id {
                (callbacks.on_fetch_versions)(project_id.clone());
                (callbacks.on_fetch_description)(project_id);
            }
        });

        template_built
    }

    // ── Preview panel (right side) ──────────────────────────────────
    // Returns true if a template was built this frame.

    fn show_preview_panel(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: &str,
        available_height: f32,
    ) -> bool {
        let selected = match self.state.selected_project.clone() {
            Some(p) => p,
            None => return false,
        };

        let mut built = false;

        egui::ScrollArea::vertical()
            .id_salt(format!("{}_mr_preview_scroll", id_salt))
            .auto_shrink([false, false])
            .max_height(available_height)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // ── Large icon ──
                    if let Some(icon_url) = &selected.icon_url {
                        ui.add(
                            egui::Image::new(icon_url)
                                .max_width(128.0)
                                .max_height(128.0)
                                .rounding(8.0),
                        );
                        ui.add_space(8.0);
                    }

                    // ── Title + stats ──
                    ui.heading(&selected.title);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} downloads",
                            curseforge::format_downloads(selected.downloads)
                        ));
                    });
                    ui.add_space(4.0);

                    // ── Categories ──
                    if !selected.categories.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("Categories: ");
                            ui.label(selected.categories.join(", "));
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
                        // Modrinth descriptions are markdown; show as plain text
                        // (truncated to avoid massive renders)
                        let truncated = if desc.len() > 2000 {
                            format!("{}...", &desc[..2000])
                        } else {
                            desc.clone()
                        };
                        ui.label(truncated);
                    } else {
                        ui.label(&selected.description);
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
                            egui::ComboBox::from_id_salt(format!(
                                "{}_mr_mc_version_picker",
                                id_salt
                            ))
                            .selected_text(mc_label)
                            .show_ui(ui, |ui| {
                                for ver in &self.state.mc_versions.clone() {
                                    let is_sel = self
                                        .state
                                        .selected_mc_version
                                        .as_deref()
                                        == Some(ver.as_str());
                                    if ui.selectable_label(is_sel, ver).clicked() {
                                        self.state.selected_mc_version =
                                            Some(ver.clone());
                                        self.state.selected_version_idx = None;
                                        self.template = None;
                                    }
                                }
                            });
                        });

                        // ── Filter versions by selected MC version ──
                        let filtered_versions: Vec<(usize, String)> = self
                            .state
                            .versions
                            .iter()
                            .enumerate()
                            .filter(|(_i, v)| match &self.state.selected_mc_version {
                                Some(mc) => v.game_versions.iter().any(|gv| gv == mc),
                                None => true,
                            })
                            .map(|(i, v)| {
                                let date_short = v
                                    .date_published
                                    .split('T')
                                    .next()
                                    .unwrap_or(&v.date_published);
                                let label = if v.name.is_empty() {
                                    format!("{} ({})", v.version_number, date_short)
                                } else {
                                    format!("{} ({})", v.name, date_short)
                                };
                                (i, label)
                            })
                            .collect();

                        // ── Pack Version dropdown ──
                        let version_label = self
                            .state
                            .selected_version_idx
                            .and_then(|idx| self.state.versions.get(idx))
                            .map(|v| {
                                if v.name.is_empty() {
                                    v.version_number.as_str()
                                } else {
                                    v.name.as_str()
                                }
                            })
                            .unwrap_or("Select...");

                        let mut clicked_version_idx: Option<usize> = None;

                        ui.horizontal(|ui| {
                            ui.label("Pack Version:");
                            egui::ComboBox::from_id_salt(format!(
                                "{}_mr_pack_version_picker",
                                id_salt
                            ))
                            .selected_text(version_label)
                            .width(300.0)
                            .show_ui(ui, |ui| {
                                for (orig_idx, label) in &filtered_versions {
                                    let is_sel =
                                        self.state.selected_version_idx == Some(*orig_idx);
                                    if ui.selectable_label(is_sel, label).clicked() {
                                        clicked_version_idx = Some(*orig_idx);
                                    }
                                }
                            });
                        });

                        if let Some(orig_idx) = clicked_version_idx {
                            self.state.selected_version_idx = Some(orig_idx);
                            let version = self.state.versions[orig_idx].clone();
                            self.build_mr_template(&selected, &version);
                            built = true;
                        }

                        if filtered_versions.is_empty() {
                            ui.small("No versions for this Minecraft version.");
                        }
                    }
                });
            });

        built
    }

    // ── Build template from Modrinth data ────────────────────────────────

    pub fn build_mr_template(&mut self, project: &MrProject, version: &MrVersion) {
        // Detect MC version: first game_version that starts with a digit
        let mc_version = version
            .game_versions
            .iter()
            .find(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            .cloned()
            .unwrap_or_default();

        // Detect loader from the version's loaders array
        let loader_str = modrinth::detect_loader(&version.loaders);
        let loader = match loader_str {
            "neoforge" => ModLoader::NeoForge,
            "fabric" => ModLoader::Fabric,
            "forge" => ModLoader::Forge,
            _ => ModLoader::Forge,
        };

        let java_version = curseforge::infer_java_version(&mc_version);
        let memory = curseforge::default_memory_mb(&mc_version);

        let template = ModpackTemplate {
            name: project.title.clone(),
            description: project.description.clone(),
            version: version.version_number.clone(),
            minecraft_version: mc_version,
            loader,
            source: ModpackSource::Modrinth {
                project_id: project.slug.clone(),
                version_id: version.id.clone(),
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
