use eframe::egui;
use tokio::runtime::Runtime;

use crate::docker::DockerManager;
use crate::server::{ServerInstance, ServerConfig, ModpackInfo, ServerStatus};
use crate::templates::ModpackTemplate;
use crate::ui::{View, DashboardView, ServerCreateView};

pub struct DrakonixApp {
    #[allow(dead_code)] // Will be used for async Docker operations
    runtime: Runtime,
    #[allow(dead_code)] // Will be used when container management is wired up
    docker: Option<DockerManager>,
    docker_connected: bool,
    docker_version: String,

    servers: Vec<ServerInstance>,
    templates: Vec<ModpackTemplate>,

    current_view: View,
    create_view: ServerCreateView,

    status_message: Option<(String, std::time::Instant)>,
}

impl DrakonixApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up custom fonts/style if needed
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::dark());

        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        // Try to connect to Docker
        let (docker, docker_connected, docker_version) = match DockerManager::new() {
            Ok(dm) => {
                let version = runtime.block_on(async {
                    match dm.get_version().await {
                        Ok(v) => v,
                        Err(_) => "unknown".to_string(),
                    }
                });
                let connected = runtime.block_on(async {
                    dm.check_connection().await.unwrap_or(false)
                });
                (Some(dm), connected, version)
            }
            Err(e) => {
                tracing::error!("Failed to connect to Docker: {}", e);
                (None, false, "N/A".to_string())
            }
        };

        Self {
            runtime,
            docker,
            docker_connected,
            docker_version,
            servers: Vec::new(),
            templates: ModpackTemplate::builtin_templates(),
            current_view: View::Dashboard,
            create_view: ServerCreateView::default(),
            status_message: None,
        }
    }

    fn show_status_message(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    fn create_server(&mut self, name: String, template: &ModpackTemplate, port: u16, memory_mb: u64) {
        let modpack_info = ModpackInfo {
            name: template.name.clone(),
            version: template.version.clone(),
            loader: template.loader.clone(),
            source: template.source.clone(),
        };

        let mut config = ServerConfig::new(name.clone(), modpack_info);
        config.port = port;
        config.memory_mb = memory_mb;
        config.java_args = template.default_java_args.clone();

        let instance = ServerInstance {
            config,
            container_id: None,
            status: ServerStatus::Stopped,
        };

        self.servers.push(instance);
        self.show_status_message(format!("Server '{}' created successfully!", name));
        self.current_view = View::Dashboard;
        self.create_view.reset();
    }

    fn start_server(&mut self, name: &str) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.status = ServerStatus::Starting;
            self.show_status_message(format!("Starting server '{}'...", name));
            // TODO: Actually start the Docker container
        }
    }

    fn stop_server(&mut self, name: &str) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.status = ServerStatus::Stopping;
            self.show_status_message(format!("Stopping server '{}'...", name));
            // TODO: Actually stop the Docker container
        }
    }
}

impl eframe::App for DrakonixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel with app title
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("DrakonixAnvil");
                ui.separator();

                if ui.selectable_label(self.current_view == View::Dashboard, "Dashboard").clicked() {
                    self.current_view = View::Dashboard;
                }
                if ui.selectable_label(self.current_view == View::Settings, "Settings").clicked() {
                    self.current_view = View::Settings;
                }
            });
        });

        // Bottom panel for status messages
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((msg, time)) = &self.status_message {
                    if time.elapsed().as_secs() < 5 {
                        ui.label(msg);
                    } else {
                        self.status_message = None;
                    }
                }
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.current_view {
                View::Dashboard => {
                    let mut create_clicked = false;
                    let mut start_name = None;
                    let mut stop_name = None;

                    DashboardView::show(
                        ui,
                        &self.servers,
                        self.docker_connected,
                        &self.docker_version,
                        &mut || create_clicked = true,
                        &mut |name| start_name = Some(name.to_string()),
                        &mut |name| stop_name = Some(name.to_string()),
                    );

                    if create_clicked {
                        self.current_view = View::CreateServer;
                    }
                    if let Some(name) = start_name {
                        self.start_server(&name);
                    }
                    if let Some(name) = stop_name {
                        self.stop_server(&name);
                    }
                }
                View::CreateServer => {
                    let mut created = None;
                    let mut cancelled = false;

                    self.create_view.show(
                        ui,
                        &self.templates,
                        &mut |name, template, port, memory| {
                            created = Some((name, template.clone(), port, memory));
                        },
                        &mut || cancelled = true,
                    );

                    if let Some((name, template, port, memory)) = created {
                        self.create_server(name, &template, port, memory);
                    }
                    if cancelled {
                        self.current_view = View::Dashboard;
                        self.create_view.reset();
                    }
                }
                View::ServerDetails(name) => {
                    ui.heading(format!("Server: {}", name));
                    ui.label("Server details view - Coming soon!");
                    if ui.button("Back to Dashboard").clicked() {
                        self.current_view = View::Dashboard;
                    }
                }
                View::Settings => {
                    ui.heading("Settings");
                    ui.label("Settings view - Coming soon!");
                }
            }
        });
    }
}
