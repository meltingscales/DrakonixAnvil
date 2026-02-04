use eframe::egui;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::runtime::Runtime;
use rust_mc_status::{McClient, ServerEdition, models::ServerData};

use crate::config::{get_server_data_path, get_container_name, load_servers, save_servers, load_settings, save_settings, AppSettings, MINECRAFT_IMAGE};
use crate::docker::DockerManager;
use crate::server::{ServerInstance, ServerConfig, ModpackInfo, ServerStatus};
use crate::templates::ModpackTemplate;
use crate::ui::{View, DashboardView, ServerCreateView, ServerEditView};

const MAX_LOG_LINES: usize = 500;

/// Messages sent from background tasks to the UI
enum TaskMessage {
    Log(String),
    ServerStatus { name: String, status: ServerStatus, container_id: Option<String> },
}

pub struct DrakonixApp {
    runtime: Runtime,
    docker: Option<Arc<DockerManager>>,
    docker_connected: bool,
    docker_version: String,

    servers: Vec<ServerInstance>,
    templates: Vec<ModpackTemplate>,
    settings: AppSettings,

    current_view: View,
    create_view: ServerCreateView,
    edit_view: ServerEditView,

    /// Container logs cache for the logs viewer
    container_logs: String,

    /// Temp buffer for settings UI
    settings_cf_key_input: String,

    status_message: Option<(String, std::time::Instant)>,
    log_buffer: Vec<String>,

    /// Channel receiver for background task messages
    task_rx: mpsc::Receiver<TaskMessage>,
    /// Channel sender (cloned for each background task)
    task_tx: mpsc::Sender<TaskMessage>,
}

impl DrakonixApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up custom fonts/style if needed
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::dark());

        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let (task_tx, task_rx) = mpsc::channel();

        let mut log_buffer = Vec::new();
        log_buffer.push(format!("[{}] DrakonixAnvil starting...", Self::timestamp()));

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
                log_buffer.push(format!("[{}] Docker connected (v{})", Self::timestamp(), version));
                (Some(Arc::new(dm)), connected, version)
            }
            Err(e) => {
                log_buffer.push(format!("[{}] ERROR: Failed to connect to Docker: {}", Self::timestamp(), e));
                (None, false, "N/A".to_string())
            }
        };

        // Load saved servers
        let servers = match load_servers() {
            Ok(mut servers) => {
                log_buffer.push(format!("[{}] Loaded {} server(s) from disk", Self::timestamp(), servers.len()));
                // Reset any transient states to Stopped
                for server in &mut servers {
                    match &server.status {
                        ServerStatus::Starting | ServerStatus::Stopping | ServerStatus::Pulling | ServerStatus::Initializing => {
                            server.status = ServerStatus::Stopped;
                        }
                        _ => {}
                    }
                }
                servers
            }
            Err(e) => {
                log_buffer.push(format!("[{}] ERROR: Failed to load servers: {}", Self::timestamp(), e));
                Vec::new()
            }
        };

        // Load global settings
        let settings = load_settings();
        let settings_cf_key_input = settings.curseforge_api_key.clone().unwrap_or_default();

        Self {
            runtime,
            docker,
            docker_connected,
            docker_version,
            servers,
            templates: ModpackTemplate::builtin_templates(),
            settings,
            current_view: View::Dashboard,
            create_view: ServerCreateView::default(),
            edit_view: ServerEditView::default(),
            container_logs: String::new(),
            settings_cf_key_input,
            status_message: None,
            log_buffer,
            task_rx,
            task_tx,
        }
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }

    fn log(&mut self, msg: String) {
        let line = format!("[{}] {}", Self::timestamp(), msg);
        tracing::info!("{}", msg);
        self.log_buffer.push(line);
        if self.log_buffer.len() > MAX_LOG_LINES {
            self.log_buffer.remove(0);
        }
    }

    fn show_status_message(&mut self, msg: String) {
        self.status_message = Some((msg.clone(), std::time::Instant::now()));
        self.log(msg);
    }

    fn save_servers(&mut self) {
        if let Err(e) = save_servers(&self.servers) {
            self.log(format!("ERROR: Failed to save servers: {}", e));
        }
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
        self.save_servers();
        self.show_status_message(format!("Server '{}' created successfully!", name));
        self.current_view = View::Dashboard;
        self.create_view.reset();
    }

    fn start_edit_server(&mut self, name: &str) {
        if let Some(server) = self.servers.iter().find(|s| s.config.name == name) {
            self.edit_view.load_from_config(&server.config);
            self.current_view = View::EditServer(name.to_string());
        }
    }

    fn save_server_edit(&mut self, name: &str, port: u16, java_args: Vec<String>) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            let port_changed = server.config.port != port;
            let args_changed = server.config.java_args != java_args;

            server.config.port = port;
            server.config.java_args = java_args;

            // If port or java args changed, we need to recreate the container
            if port_changed || args_changed {
                // Clear container_id to force recreation on next start
                server.container_id = None;
            }

            self.save_servers();
            self.show_status_message(format!("Server '{}' settings updated!", name));
        }
        self.current_view = View::Dashboard;
        self.edit_view.reset();
    }

    fn start_server(&mut self, name: &str) {
        let Some(docker) = self.docker.clone() else {
            self.show_status_message("Docker not connected".to_string());
            return;
        };

        // Find server index
        let server_idx = self.servers.iter().position(|s| s.config.name == name);
        let Some(idx) = server_idx else {
            self.show_status_message(format!("Server '{}' not found", name));
            return;
        };

        // Create data directory if needed
        let data_path = get_server_data_path(name);
        if let Err(e) = std::fs::create_dir_all(&data_path) {
            self.servers[idx].status = ServerStatus::Error(format!("Failed to create data dir: {}", e));
            self.show_status_message(format!("Failed to create data directory: {}", e));
            return;
        }

        // Determine if we need to pull/create or just start
        let needs_container = self.servers[idx].container_id.is_none();
        let container_id = self.servers[idx].container_id.clone();
        let container_name = get_container_name(name);
        let mut env_vars = self.servers[idx].config.build_docker_env();

        // Add CurseForge API key if configured
        if let Some(cf_key) = &self.settings.curseforge_api_key {
            if !cf_key.is_empty() {
                env_vars.push(format!("CF_API_KEY={}", cf_key));
            }
        }

        let port = self.servers[idx].config.port;
        let memory_mb = self.servers[idx].config.memory_mb;
        let server_name = name.to_string();
        let tx = self.task_tx.clone();

        // Set initial status
        if needs_container {
            self.servers[idx].status = ServerStatus::Pulling;
            self.log(format!("Pulling image for server '{}'...", name));
        } else {
            self.servers[idx].status = ServerStatus::Starting;
            self.log(format!("Starting server '{}'...", name));
        }

        // Spawn background task
        self.runtime.spawn(async move {
            let name = server_name.clone();

            // Pull image if needed
            if needs_container {
                tx.send(TaskMessage::Log(format!("Checking Docker image {}...", MINECRAFT_IMAGE))).ok();

                if let Err(e) = docker.ensure_image(MINECRAFT_IMAGE).await {
                    let err = format!("Failed to pull image: {}", e);
                    tx.send(TaskMessage::Log(err.clone())).ok();
                    tx.send(TaskMessage::ServerStatus {
                        name,
                        status: ServerStatus::Error(err),
                        container_id: None,
                    }).ok();
                    return;
                }
                tx.send(TaskMessage::Log(format!("Docker image {} ready", MINECRAFT_IMAGE))).ok();

                // Update status to Starting
                tx.send(TaskMessage::ServerStatus {
                    name: name.clone(),
                    status: ServerStatus::Starting,
                    container_id: None,
                }).ok();

                // Create container
                tx.send(TaskMessage::Log(format!("Creating container {}...", container_name))).ok();
                match docker.create_minecraft_container(
                    &container_name,
                    &name,
                    MINECRAFT_IMAGE,
                    port,
                    memory_mb,
                    env_vars,
                    &data_path,
                ).await {
                    Ok(new_container_id) => {
                        tx.send(TaskMessage::Log(format!("Created container {}", new_container_id))).ok();

                        // Start the new container
                        if let Err(e) = docker.start_container(&new_container_id).await {
                            let err = format!("Failed to start container: {}", e);
                            tx.send(TaskMessage::Log(err.clone())).ok();
                            tx.send(TaskMessage::ServerStatus {
                                name,
                                status: ServerStatus::Error(err),
                                container_id: Some(new_container_id),
                            }).ok();
                            return;
                        }

                        tx.send(TaskMessage::Log(format!("Container started, waiting for MC server to initialize..."))).ok();
                        tx.send(TaskMessage::ServerStatus {
                            name: name.clone(),
                            status: ServerStatus::Initializing,
                            container_id: Some(new_container_id.clone()),
                        }).ok();

                        // Poll MC server until it accepts connections
                        Self::poll_mc_server_ready(tx.clone(), name, port, new_container_id, docker).await;
                    }
                    Err(e) => {
                        let err = format!("Failed to create container: {}", e);
                        tx.send(TaskMessage::Log(err.clone())).ok();
                        tx.send(TaskMessage::ServerStatus {
                            name,
                            status: ServerStatus::Error(err),
                            container_id: None,
                        }).ok();
                    }
                }
            } else {
                // Just start existing container
                let cid = container_id.unwrap();
                if let Err(e) = docker.start_container(&cid).await {
                    let err = format!("Failed to start container: {}", e);
                    tx.send(TaskMessage::Log(err.clone())).ok();
                    tx.send(TaskMessage::ServerStatus {
                        name,
                        status: ServerStatus::Error(err),
                        container_id: Some(cid),
                    }).ok();
                    return;
                }

                tx.send(TaskMessage::Log(format!("Container started, waiting for MC server to initialize..."))).ok();
                tx.send(TaskMessage::ServerStatus {
                    name: name.clone(),
                    status: ServerStatus::Initializing,
                    container_id: Some(cid.clone()),
                }).ok();

                // Poll MC server until it accepts connections
                Self::poll_mc_server_ready(tx.clone(), name, port, cid, docker).await;
            }
        });
    }

    fn stop_server(&mut self, name: &str) {
        let Some(docker) = self.docker.clone() else {
            self.show_status_message("Docker not connected".to_string());
            return;
        };

        // Find server index
        let server_idx = self.servers.iter().position(|s| s.config.name == name);
        let Some(idx) = server_idx else {
            self.show_status_message(format!("Server '{}' not found", name));
            return;
        };

        // Check if we have a container_id
        let Some(container_id) = self.servers[idx].container_id.clone() else {
            self.show_status_message(format!("Server '{}' has no container", name));
            return;
        };

        // Set status to Stopping
        self.servers[idx].status = ServerStatus::Stopping;
        self.log(format!("Stopping server '{}'...", name));

        let server_name = name.to_string();
        let tx = self.task_tx.clone();

        // Spawn background task
        self.runtime.spawn(async move {
            match docker.stop_container(&container_id).await {
                Ok(()) => {
                    tx.send(TaskMessage::Log(format!("Server '{}' stopped successfully!", server_name))).ok();
                    tx.send(TaskMessage::ServerStatus {
                        name: server_name,
                        status: ServerStatus::Stopped,
                        container_id: Some(container_id),
                    }).ok();
                }
                Err(e) => {
                    let err = format!("Failed to stop: {}", e);
                    tx.send(TaskMessage::Log(err.clone())).ok();
                    tx.send(TaskMessage::ServerStatus {
                        name: server_name,
                        status: ServerStatus::Error(err),
                        container_id: Some(container_id),
                    }).ok();
                }
            }
        });
    }

    fn view_container_logs(&mut self, name: &str) {
        let Some(docker) = self.docker.clone() else {
            self.show_status_message("Docker not connected".to_string());
            return;
        };

        let Some(server) = self.servers.iter().find(|s| s.config.name == name) else {
            self.show_status_message(format!("Server '{}' not found", name));
            return;
        };

        let Some(container_id) = server.container_id.clone() else {
            self.container_logs = "No container found. Start the server first to see logs.".to_string();
            self.current_view = View::ContainerLogs(name.to_string());
            return;
        };

        // Fetch logs synchronously (blocking) for simplicity
        let logs = self.runtime.block_on(async {
            docker.get_container_logs(&container_id, 500).await.unwrap_or_else(|e| format!("Error fetching logs: {}", e))
        });

        self.container_logs = logs;
        self.current_view = View::ContainerLogs(name.to_string());
    }

    fn delete_server(&mut self, name: &str) {
        let Some(docker) = self.docker.clone() else {
            self.show_status_message("Docker not connected".to_string());
            return;
        };

        // Find and remove the server
        let server_idx = self.servers.iter().position(|s| s.config.name == name);
        let Some(idx) = server_idx else {
            self.show_status_message(format!("Server '{}' not found", name));
            return;
        };

        let server = self.servers.remove(idx);

        // Remove container if it exists
        if let Some(container_id) = server.container_id {
            let _ = self.runtime.block_on(async {
                // Try to stop first (ignore errors - might already be stopped)
                let _ = docker.stop_container(&container_id).await;
                docker.remove_container(&container_id).await
            });
        }

        self.save_servers();
        self.show_status_message(format!("Server '{}' deleted", name));
        self.current_view = View::Dashboard;
    }

    /// Process messages from background tasks
    fn process_task_messages(&mut self) {
        while let Ok(msg) = self.task_rx.try_recv() {
            match msg {
                TaskMessage::Log(text) => {
                    self.log(text);
                }
                TaskMessage::ServerStatus { name, status, container_id } => {
                    if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
                        server.status = status.clone();
                        if let Some(cid) = container_id {
                            server.container_id = Some(cid);
                        }
                        // Show status message for terminal states
                        match &status {
                            ServerStatus::Running => {
                                self.status_message = Some((format!("Server '{}' started!", name), std::time::Instant::now()));
                            }
                            ServerStatus::Stopped => {
                                self.status_message = Some((format!("Server '{}' stopped", name), std::time::Instant::now()));
                            }
                            ServerStatus::Error(e) => {
                                self.status_message = Some((e.clone(), std::time::Instant::now()));
                            }
                            _ => {}
                        }
                    }
                    self.save_servers();
                }
            }
        }
    }

    /// Check if any servers are in a transient state (need UI refresh)
    fn has_active_tasks(&self) -> bool {
        self.servers.iter().any(|s| matches!(
            s.status,
            ServerStatus::Pulling | ServerStatus::Starting | ServerStatus::Initializing | ServerStatus::Stopping
        ))
    }

    /// Poll the Minecraft server until it accepts connections
    async fn poll_mc_server_ready(
        tx: mpsc::Sender<TaskMessage>,
        name: String,
        port: u16,
        container_id: String,
        docker: Arc<DockerManager>,
    ) {
        let client = McClient::new().with_timeout(Duration::from_secs(3));
        let address = format!("127.0.0.1:{}", port);
        let max_attempts = 120; // 10 minutes at 5 second intervals
        let poll_interval = Duration::from_secs(5);

        for attempt in 1..=max_attempts {
            // First check if container is still running
            match docker.is_container_running(&container_id).await {
                Ok(true) => {} // Container still running, continue
                Ok(false) => {
                    // Container stopped/crashed
                    tx.send(TaskMessage::Log(format!(
                        "Container for '{}' has stopped. Check container logs for errors.",
                        name
                    ))).ok();
                    tx.send(TaskMessage::ServerStatus {
                        name,
                        status: ServerStatus::Error("Container exited unexpectedly".to_string()),
                        container_id: Some(container_id),
                    }).ok();
                    return;
                }
                Err(e) => {
                    tx.send(TaskMessage::Log(format!(
                        "Failed to check container status: {}", e
                    ))).ok();
                    // Continue trying - might be transient
                }
            }

            match client.ping(&address, ServerEdition::Java).await {
                Ok(status) if status.online => {
                    // Log basic connection info
                    tx.send(TaskMessage::Log(format!(
                        "Server '{}' is now accepting connections! (latency: {:.0}ms)",
                        name, status.latency
                    ))).ok();

                    // Extract and log rich Java status info
                    if let ServerData::Java(java) = &status.data {
                        // Version info
                        tx.send(TaskMessage::Log(format!(
                            "  Version: {} (protocol {})",
                            java.version.name, java.version.protocol
                        ))).ok();

                        // MOTD/Description
                        if !java.description.is_empty() {
                            tx.send(TaskMessage::Log(format!(
                                "  MOTD: {}",
                                java.description.lines().next().unwrap_or(&java.description)
                            ))).ok();
                        }

                        // Player info
                        tx.send(TaskMessage::Log(format!(
                            "  Players: {}/{} online",
                            java.players.online, java.players.max
                        ))).ok();

                        // Server software if available
                        if let Some(software) = &java.software {
                            tx.send(TaskMessage::Log(format!(
                                "  Software: {}", software
                            ))).ok();
                        }

                        // Mod count if modded
                        if let Some(mods) = &java.mods {
                            if !mods.is_empty() {
                                tx.send(TaskMessage::Log(format!(
                                    "  Mods: {} loaded", mods.len()
                                ))).ok();
                            }
                        }

                        // Plugin count if available
                        if let Some(plugins) = &java.plugins {
                            if !plugins.is_empty() {
                                tx.send(TaskMessage::Log(format!(
                                    "  Plugins: {} loaded", plugins.len()
                                ))).ok();
                            }
                        }

                        // Map name if available
                        if let Some(map) = &java.map {
                            tx.send(TaskMessage::Log(format!(
                                "  Map: {}", map
                            ))).ok();
                        }
                    }

                    tx.send(TaskMessage::ServerStatus {
                        name,
                        status: ServerStatus::Running,
                        container_id: Some(container_id),
                    }).ok();
                    return;
                }
                Ok(_) => {
                    // Server responded but says offline - keep trying
                    if attempt % 6 == 0 { // Log every 30 seconds
                        tx.send(TaskMessage::Log(format!(
                            "Server '{}' not ready yet (attempt {}/{})",
                            name, attempt, max_attempts
                        ))).ok();
                    }
                }
                Err(_) => {
                    // Connection failed - server not ready
                    if attempt % 6 == 0 { // Log every 30 seconds
                        tx.send(TaskMessage::Log(format!(
                            "Waiting for '{}' to initialize (attempt {}/{})",
                            name, attempt, max_attempts
                        ))).ok();
                    }
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Timed out but don't error - modpacks can take a very long time
        tx.send(TaskMessage::Log(format!(
            "Server '{}' still initializing after 10 minutes. Check container logs for progress.",
            name
        ))).ok();
        // Keep status as Initializing - user can check logs
    }
}

impl eframe::App for DrakonixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending messages from background tasks
        self.process_task_messages();

        // Request repaint if there are active background tasks
        if self.has_active_tasks() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Top panel with app title and navigation
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("DrakonixAnvil");
                ui.separator();

                if ui.selectable_label(self.current_view == View::Dashboard, "Servers").clicked() {
                    self.current_view = View::Dashboard;
                }
                if ui.selectable_label(self.current_view == View::Logs, "Logs").clicked() {
                    self.current_view = View::Logs;
                }
                if ui.selectable_label(self.current_view == View::Settings, "Settings").clicked() {
                    self.current_view = View::Settings;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/HenryPost/DrakonixAnvil");
                });
            });
        });

        // Compact status bar at the bottom
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(20.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Docker status indicator
                    if self.docker_connected {
                        ui.colored_label(egui::Color32::GREEN, "●");
                        ui.small(format!("Docker v{}", self.docker_version));
                    } else {
                        ui.colored_label(egui::Color32::RED, "●");
                        ui.small("Docker disconnected");
                    }

                    // Status message
                    if let Some((msg, time)) = &self.status_message {
                        if time.elapsed().as_secs() < 5 {
                            ui.separator();
                            ui.small(msg);
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
                    let mut edit_name = None;
                    let mut delete_name = None;
                    let mut logs_name = None;

                    DashboardView::show(
                        ui,
                        &self.servers,
                        self.docker_connected,
                        &self.docker_version,
                        &mut || create_clicked = true,
                        &mut |name| start_name = Some(name.to_string()),
                        &mut |name| stop_name = Some(name.to_string()),
                        &mut |name| edit_name = Some(name.to_string()),
                        &mut |name| delete_name = Some(name.to_string()),
                        &mut |name| logs_name = Some(name.to_string()),
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
                    if let Some(name) = edit_name {
                        self.start_edit_server(&name);
                    }
                    if let Some(name) = delete_name {
                        self.current_view = View::ConfirmDelete(name);
                    }
                    if let Some(name) = logs_name {
                        self.view_container_logs(&name);
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
                View::EditServer(name) => {
                    let mut saved = None;
                    let mut cancelled = false;
                    let name = name.clone();

                    self.edit_view.show(
                        ui,
                        &mut |port, java_args| {
                            saved = Some((port, java_args));
                        },
                        &mut || cancelled = true,
                    );

                    if let Some((port, java_args)) = saved {
                        self.save_server_edit(&name, port, java_args);
                    }
                    if cancelled {
                        self.current_view = View::Dashboard;
                        self.edit_view.reset();
                    }
                }
                View::ServerDetails(name) => {
                    ui.heading(format!("Server: {}", name));
                    ui.label("Server details view - Coming soon!");
                    if ui.button("Back to Dashboard").clicked() {
                        self.current_view = View::Dashboard;
                    }
                }
                View::ContainerLogs(name) => {
                    let name = name.clone();
                    ui.horizontal(|ui| {
                        ui.heading(format!("Container Logs: {}", name));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Refresh").clicked() {
                                self.view_container_logs(&name);
                            }
                            if ui.button("Back").clicked() {
                                self.current_view = View::Dashboard;
                            }
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.container_logs.as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                            );
                        });
                }
                View::ConfirmDelete(name) => {
                    let name = name.clone();
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("Delete Server?");
                        ui.add_space(20.0);
                        ui.label(format!("Are you sure you want to delete '{}'?", name));
                        ui.add_space(10.0);
                        ui.label("This will remove the Docker container.");
                        ui.colored_label(egui::Color32::YELLOW, "Server data in DrakonixAnvilData/servers/ will NOT be deleted.");
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 80.0);
                            if ui.button("Cancel").clicked() {
                                self.current_view = View::Dashboard;
                            }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(150, 40, 40))).clicked() {
                                self.delete_server(&name);
                            }
                        });
                    });
                }
                View::Logs => {
                    ui.horizontal(|ui| {
                        ui.heading("Logs");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                self.log_buffer.clear();
                            }
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.log_buffer {
                                ui.monospace(line);
                            }
                        });
                }
                View::Settings => {
                    ui.heading("Settings");
                    ui.add_space(10.0);

                    // CurseForge API Key
                    ui.group(|ui| {
                        ui.strong("CurseForge API Key");
                        ui.label("Required for downloading CurseForge modpacks.");
                        ui.horizontal(|ui| {
                            ui.label("Get your key:");
                            ui.hyperlink("https://console.curseforge.com/");
                        });
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            ui.label("API Key:");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.settings_cf_key_input)
                                    .password(true)
                                    .desired_width(300.0)
                                    .hint_text("Paste your CurseForge API key here")
                            );

                            // Show/hide toggle
                            if ui.button("👁").on_hover_text("Show/hide key").clicked() {
                                // Toggle would require state, for now just show the length
                            }

                            if response.changed() {
                                // Update settings when text changes
                                let key = self.settings_cf_key_input.trim().to_string();
                                self.settings.curseforge_api_key = if key.is_empty() {
                                    None
                                } else {
                                    Some(key)
                                };
                            }
                        });

                        // Status indicator
                        ui.horizontal(|ui| {
                            if self.settings.curseforge_api_key.is_some() {
                                ui.colored_label(egui::Color32::GREEN, "✓ API key configured");
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "○ No API key set");
                            }
                        });

                        ui.add_space(5.0);
                        if ui.button("Save Settings").clicked() {
                            if let Err(e) = save_settings(&self.settings) {
                                self.show_status_message(format!("Failed to save settings: {}", e));
                            } else {
                                self.show_status_message("Settings saved!".to_string());
                            }
                        }
                    });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Info section
                    ui.label("Note: After setting the API key, you'll need to recreate any CurseForge servers for the key to take effect.");
                }
            }
        });
    }
}
