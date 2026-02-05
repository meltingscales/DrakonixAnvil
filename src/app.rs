use eframe::egui;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::runtime::Runtime;
use rust_mc_status::{McClient, ServerEdition, models::ServerData};

use crate::backup::{self, BackupInfo};
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
    BackupProgress { server_name: String, current: usize, total: usize, current_file: String },
    BackupComplete { server_name: String, result: Result<std::path::PathBuf, String> },
    RestoreProgress { server_name: String, current: usize, total: usize, current_file: String },
    RestoreComplete { server_name: String, result: Result<(), String> },
    DockerLogs(String),
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

    /// Container logs cache for the per-server logs viewer
    container_logs: String,

    /// Combined Docker logs from all managed containers
    all_docker_logs: String,
    /// Last time Docker logs were refreshed (for auto-refresh)
    docker_logs_last_refresh: Option<std::time::Instant>,

    /// Cached backup list for the backups view
    backup_list: Vec<BackupInfo>,

    /// Backup in progress tracking (server_name -> (current, total, current_file))
    backup_progress: Option<(String, usize, usize, String)>,
    /// Restore in progress tracking (server_name -> (current, total, current_file))
    restore_progress: Option<(String, usize, usize, String)>,

    /// Console command input buffer
    console_input: String,
    /// Console output history
    console_output: Vec<String>,

    /// Temp buffer for settings UI
    settings_cf_key_input: String,

    status_message: Option<(String, std::time::Instant)>,
    log_buffer: Vec<String>,

    /// Show close confirmation dialog when servers are running
    show_close_confirmation: bool,

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
            all_docker_logs: String::new(),
            docker_logs_last_refresh: None,
            backup_list: Vec::new(),
            backup_progress: None,
            restore_progress: None,
            console_input: String::new(),
            console_output: Vec::new(),
            settings_cf_key_input,
            status_message: None,
            log_buffer,
            show_close_confirmation: false,
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

    /// Check if a port is already in use
    /// Returns Some(error_message) if there's a conflict, None if port is available
    fn check_port_conflict(&self, port: u16, server_name: &str) -> Option<String> {
        // First, check if another DrakonixAnvil server is configured with this port and running
        for server in &self.servers {
            if server.config.name != server_name && server.config.port == port {
                if matches!(server.status, ServerStatus::Running | ServerStatus::Starting | ServerStatus::Initializing) {
                    return Some(format!(
                        "Port {} is already used by running server '{}'",
                        port, server.config.name
                    ));
                }
            }
        }

        // Then, check if any process is listening on this port
        match std::net::TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(_listener) => {
                // Port is available (listener is dropped immediately)
                None
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::AddrInUse => {
                        // Find a suggested available port
                        let suggested = Self::find_available_port(port);
                        Some(format!(
                            "Port {} is already in use by another application. Try port {} instead.",
                            port,
                            suggested.unwrap_or(port + 1)
                        ))
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        Some(format!(
                            "Permission denied for port {}. Ports below 1024 require root privileges.",
                            port
                        ))
                    }
                    _ => {
                        Some(format!("Cannot bind to port {}: {}", port, e))
                    }
                }
            }
        }
    }

    /// Find an available port starting from the given port
    fn find_available_port(start_port: u16) -> Option<u16> {
        for port in start_port..=65535 {
            if std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
                return Some(port);
            }
        }
        None
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

        let port = self.servers[idx].config.port;
        let rcon_port = self.servers[idx].config.rcon_port();

        // Check for port conflicts
        if let Some(conflict) = self.check_port_conflict(port, name) {
            self.show_status_message(conflict);
            return;
        }

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
                    rcon_port,
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

    fn load_all_docker_logs(&mut self) {
        let Some(docker) = self.docker.clone() else {
            self.show_status_message("Docker not connected".to_string());
            return;
        };

        self.docker_logs_last_refresh = Some(std::time::Instant::now());
        self.current_view = View::DockerLogs;

        let tx = self.task_tx.clone();

        // Fetch logs in background to avoid UI freeze
        self.runtime.spawn(async move {
            let logs = docker.get_all_managed_logs(200).await
                .unwrap_or_else(|e| format!("Error fetching logs: {}", e));
            let _ = tx.send(TaskMessage::DockerLogs(logs));
        });
    }

    /// Refresh Docker logs without changing view (for auto-refresh)
    fn refresh_docker_logs(&mut self) {
        let Some(docker) = self.docker.clone() else {
            return;
        };

        self.docker_logs_last_refresh = Some(std::time::Instant::now());
        let tx = self.task_tx.clone();

        self.runtime.spawn(async move {
            let logs = docker.get_all_managed_logs(200).await
                .unwrap_or_else(|e| format!("Error fetching logs: {}", e));
            let _ = tx.send(TaskMessage::DockerLogs(logs));
        });
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

    fn create_backup(&mut self, name: &str) {
        // Check if a backup is already in progress
        if self.backup_progress.is_some() {
            self.show_status_message("A backup is already in progress".to_string());
            return;
        }

        self.log(format!("Creating backup for '{}'...", name));
        self.backup_progress = Some((name.to_string(), 0, 0, "Counting files...".to_string()));

        let server_name = name.to_string();
        let tx = self.task_tx.clone();

        // Run backup in background thread (not async, since it's CPU/IO bound)
        std::thread::spawn(move || {
            let (progress_tx, progress_rx) = std::sync::mpsc::channel::<backup::BackupProgress>();

            // Spawn a thread to forward progress updates
            let tx_progress = tx.clone();
            let name_for_progress = server_name.clone();
            std::thread::spawn(move || {
                while let Ok(progress) = progress_rx.recv() {
                    let _ = tx_progress.send(TaskMessage::BackupProgress {
                        server_name: name_for_progress.clone(),
                        current: progress.current,
                        total: progress.total,
                        current_file: progress.current_file,
                    });
                }
            });

            let result = backup::create_backup_with_progress(&server_name, Some(progress_tx));
            let _ = tx.send(TaskMessage::BackupComplete {
                server_name,
                result: result.map_err(|e| e.to_string()),
            });
        });
    }

    fn view_backups(&mut self, name: &str) {
        match backup::list_backups(name) {
            Ok(backups) => {
                self.backup_list = backups;
                self.current_view = View::Backups(name.to_string());
            }
            Err(e) => {
                self.show_status_message(format!("Failed to list backups: {}", e));
            }
        }
    }

    fn restore_backup(&mut self, name: &str, backup_path: &std::path::Path) {
        // Check if a restore is already in progress
        if self.restore_progress.is_some() {
            self.show_status_message("A restore is already in progress".to_string());
            return;
        }

        self.log(format!("Restoring backup for '{}'...", name));
        self.restore_progress = Some((name.to_string(), 0, 0, "Starting restore...".to_string()));
        self.current_view = View::Dashboard;

        let server_name = name.to_string();
        let backup_path = backup_path.to_path_buf();
        let tx = self.task_tx.clone();

        // Run restore in background thread
        std::thread::spawn(move || {
            let (progress_tx, progress_rx) = std::sync::mpsc::channel::<backup::BackupProgress>();

            // Spawn a thread to forward progress updates
            let tx_progress = tx.clone();
            let name_for_progress = server_name.clone();
            std::thread::spawn(move || {
                while let Ok(progress) = progress_rx.recv() {
                    let _ = tx_progress.send(TaskMessage::RestoreProgress {
                        server_name: name_for_progress.clone(),
                        current: progress.current,
                        total: progress.total,
                        current_file: progress.current_file,
                    });
                }
            });

            let result = backup::restore_backup_with_progress(&server_name, &backup_path, Some(progress_tx));
            let _ = tx.send(TaskMessage::RestoreComplete {
                server_name,
                result: result.map_err(|e| e.to_string()),
            });
        });
    }

    fn delete_backup(&mut self, name: &str, backup_path: &std::path::Path) {
        match backup::delete_backup(backup_path) {
            Ok(()) => {
                self.show_status_message("Backup deleted".to_string());
                // Refresh the backup list
                self.view_backups(name);
            }
            Err(e) => {
                self.show_status_message(format!("Failed to delete backup: {}", e));
            }
        }
    }

    fn open_console(&mut self, name: &str) {
        self.console_input.clear();
        self.console_output.clear();
        self.console_output.push(format!("Connected to RCON console for '{}'", name));
        self.console_output.push("Type commands and press Enter to send.".to_string());
        self.console_output.push("Common commands: list, say <msg>, op <player>, whitelist add <player>".to_string());
        self.console_output.push(String::new());
        self.current_view = View::Console(name.to_string());
    }

    fn send_rcon_command(&mut self, server_name: &str, command: &str) {
        // Find server config to get RCON password and port
        let Some(server) = self.servers.iter().find(|s| s.config.name == server_name) else {
            self.console_output.push(format!("Error: Server '{}' not found", server_name));
            return;
        };

        let rcon_port = server.config.rcon_port();
        let rcon_password = server.config.rcon_password.clone();

        // Connect and send command
        let address = format!("127.0.0.1:{}", rcon_port);

        self.console_output.push(format!("> {}", command));

        match std::net::TcpStream::connect(&address) {
            Ok(stream) => {
                match mcrcon::Connection::connect(stream, rcon_password) {
                    Ok(mut conn) => {
                        match conn.command(command.to_string()) {
                            Ok(response) => {
                                if response.payload.is_empty() {
                                    self.console_output.push("(no response)".to_string());
                                } else {
                                    // Split response into lines
                                    for line in response.payload.lines() {
                                        self.console_output.push(line.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                self.console_output.push(format!("Command error: {:?}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.console_output.push(format!("RCON auth failed: {:?}", e));
                        self.console_output.push("Make sure the server is fully started.".to_string());
                    }
                }
            }
            Err(e) => {
                self.console_output.push(format!("Connection failed: {}", e));
                self.console_output.push(format!("Is the server running on RCON port {}?", rcon_port));
            }
        }
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
                TaskMessage::BackupProgress { server_name, current, total, current_file } => {
                    self.backup_progress = Some((server_name, current, total, current_file));
                }
                TaskMessage::BackupComplete { server_name, result } => {
                    self.backup_progress = None;
                    match result {
                        Ok(path) => {
                            let filename = path.file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "backup".to_string());
                            self.show_status_message(format!("Backup created: {}", filename));
                            self.log(format!("Backup saved to {:?}", path));
                        }
                        Err(e) => {
                            self.show_status_message(format!("Backup failed: {}", e));
                            self.log(format!("ERROR: Backup failed: {}", e));
                        }
                    }
                    // If we're viewing backups for this server, refresh the list
                    if let View::Backups(name) = &self.current_view {
                        if name == &server_name {
                            if let Ok(backups) = backup::list_backups(&server_name) {
                                self.backup_list = backups;
                            }
                        }
                    }
                }
                TaskMessage::DockerLogs(logs) => {
                    self.all_docker_logs = logs;
                }
                TaskMessage::RestoreProgress { server_name, current, total, current_file } => {
                    self.restore_progress = Some((server_name, current, total, current_file));
                }
                TaskMessage::RestoreComplete { server_name, result } => {
                    self.restore_progress = None;
                    match result {
                        Ok(()) => {
                            self.show_status_message(format!("Backup restored for '{}'", server_name));
                            self.log(format!("Backup restored successfully for '{}'", server_name));
                        }
                        Err(e) => {
                            self.show_status_message(format!("Restore failed: {}", e));
                            self.log(format!("ERROR: Restore failed: {}", e));
                        }
                    }
                }
            }
        }
    }

    /// Check if any servers are in a transient state (need UI refresh)
    fn has_active_tasks(&self) -> bool {
        self.backup_progress.is_some() ||
        self.restore_progress.is_some() ||
        self.servers.iter().any(|s| matches!(
            s.status,
            ServerStatus::Pulling | ServerStatus::Starting | ServerStatus::Initializing | ServerStatus::Stopping
        ))
    }

    /// Get list of running server names
    fn running_servers(&self) -> Vec<&str> {
        self.servers.iter()
            .filter(|s| matches!(s.status, ServerStatus::Running | ServerStatus::Initializing))
            .map(|s| s.config.name.as_str())
            .collect()
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

        // Handle close request - warn if servers are running
        if ctx.input(|i| i.viewport().close_requested()) {
            let running = self.running_servers();
            if running.is_empty() {
                // No running servers, allow close
            } else {
                // Servers running, show confirmation
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_close_confirmation = true;
            }
        }

        // Show close confirmation dialog
        if self.show_close_confirmation {
            let running = self.running_servers();
            let running_names: Vec<String> = running.iter().map(|s| s.to_string()).collect();

            egui::Window::new("Servers Still Running")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::YELLOW,
                            format!("You have {} server(s) still running:", running_names.len()));
                        ui.add_space(5.0);

                        for name in &running_names {
                            ui.label(format!("  • {}", name));
                        }

                        ui.add_space(15.0);
                        ui.label("Closing will leave them running in Docker.");
                        ui.small("You can stop them later with 'docker stop'");
                        ui.add_space(15.0);

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_close_confirmation = false;
                            }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new("Close Anyway")
                                .fill(egui::Color32::from_rgb(150, 100, 40))).clicked()
                            {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                        ui.add_space(10.0);
                    });
                });
        }

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
                if ui.selectable_label(self.current_view == View::DockerLogs, "Docker Logs").clicked() {
                    self.load_all_docker_logs();
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
                    let mut backup_name = None;
                    let mut view_backups_name = None;
                    let mut console_name = None;

                    DashboardView::show(
                        ui,
                        &self.servers,
                        self.docker_connected,
                        &self.docker_version,
                        &self.backup_progress,
                        &self.restore_progress,
                        &mut || create_clicked = true,
                        &mut |name| start_name = Some(name.to_string()),
                        &mut |name| stop_name = Some(name.to_string()),
                        &mut |name| edit_name = Some(name.to_string()),
                        &mut |name| delete_name = Some(name.to_string()),
                        &mut |name| logs_name = Some(name.to_string()),
                        &mut |name| backup_name = Some(name.to_string()),
                        &mut |name| view_backups_name = Some(name.to_string()),
                        &mut |name| console_name = Some(name.to_string()),
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
                    if let Some(name) = backup_name {
                        self.create_backup(&name);
                    }
                    if let Some(name) = view_backups_name {
                        self.view_backups(&name);
                    }
                    if let Some(name) = console_name {
                        self.open_console(&name);
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

                    // Get server details for display (clone to avoid borrow issues)
                    let server_info = self.servers.iter().find(|s| s.config.name == name);
                    let container_name = crate::config::get_container_name(&name);
                    let modpack_name = server_info
                        .map(|s| s.config.modpack.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let port = server_info
                        .map(|s| s.config.port)
                        .unwrap_or(0);
                    let has_container = server_info
                        .and_then(|s| s.container_id.as_ref())
                        .is_some();

                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("Delete Server?");
                        ui.add_space(20.0);

                        // Resource indicator box
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(60, 30, 30))
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::RED, "🗑");
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.strong("Docker Container");
                                        ui.monospace(&container_name);
                                        ui.small(format!("Server: {}", name));
                                        ui.small(format!("Modpack: {}", modpack_name));
                                        ui.small(format!("Port: {}", port));
                                        if has_container {
                                            ui.colored_label(egui::Color32::YELLOW, "Container exists and will be removed");
                                        } else {
                                            ui.colored_label(egui::Color32::GRAY, "No container (config only)");
                                        }
                                    });
                                });
                            });

                        ui.add_space(20.0);
                        ui.colored_label(egui::Color32::GREEN, "Server data in DrakonixAnvilData/servers/ will NOT be deleted.");
                        ui.small("You can recreate the server later using the same data.");
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
                View::Backups(name) => {
                    let name = name.clone();
                    ui.horizontal(|ui| {
                        ui.heading(format!("Backups: {}", name));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Refresh").clicked() {
                                self.view_backups(&name);
                            }
                            if ui.button("Back").clicked() {
                                self.current_view = View::Dashboard;
                            }
                        });
                    });
                    ui.separator();

                    if self.backup_list.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            ui.label("No backups found for this server.");
                            ui.add_space(10.0);
                            ui.label("Use the 'Backup' button on the dashboard to create one.");
                        });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut restore_path = None;
                            let mut delete_path = None;

                            for backup in &self.backup_list {
                                egui::Frame::none()
                                    .fill(ui.style().visuals.extreme_bg_color)
                                    .rounding(8.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.strong(&backup.filename);
                                                ui.label(format!("Size: {}", backup::format_bytes(backup.size_bytes)));
                                                if let Ok(duration) = backup.created.elapsed() {
                                                    let hours = duration.as_secs() / 3600;
                                                    let days = hours / 24;
                                                    if days > 0 {
                                                        ui.small(format!("{} days ago", days));
                                                    } else if hours > 0 {
                                                        ui.small(format!("{} hours ago", hours));
                                                    } else {
                                                        ui.small("Just now");
                                                    }
                                                }
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(100, 30, 30))).clicked() {
                                                    delete_path = Some(backup.path.clone());
                                                }
                                                if ui.button("Restore").clicked() {
                                                    restore_path = Some(backup.path.clone());
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(8.0);
                            }

                            if let Some(path) = restore_path {
                                self.current_view = View::ConfirmRestore(name.clone(), path);
                            }
                            if let Some(path) = delete_path {
                                self.current_view = View::ConfirmDeleteBackup(name.clone(), path);
                            }
                        });
                    }
                }
                View::ConfirmRestore(name, path) => {
                    let name = name.clone();
                    let path = path.clone();
                    let filename = path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "backup".to_string());

                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("Restore Backup?");
                        ui.add_space(20.0);
                        ui.label(format!("Restore '{}' to server '{}'?", filename, name));
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::RED, "WARNING: This will overwrite all current server data!");
                        ui.label("Make sure the server is stopped before restoring.");
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 80.0);
                            if ui.button("Cancel").clicked() {
                                self.current_view = View::Backups(name.clone());
                            }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new("Restore").fill(egui::Color32::from_rgb(150, 100, 40))).clicked() {
                                self.restore_backup(&name, &path);
                            }
                        });
                    });
                }
                View::ConfirmDeleteBackup(name, path) => {
                    let name = name.clone();
                    let path = path.clone();
                    let filename = path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "backup".to_string());

                    // Get file size for display
                    let size_str = std::fs::metadata(&path)
                        .map(|m| backup::format_bytes(m.len()))
                        .unwrap_or_else(|_| "unknown size".to_string());

                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("Delete Backup?");
                        ui.add_space(20.0);

                        // Resource indicator box
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(60, 30, 30))
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::RED, "🗑");
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.strong("Backup File");
                                        ui.monospace(&filename);
                                        ui.small(format!("Size: {}", size_str));
                                        ui.small(format!("Server: {}", name));
                                    });
                                });
                            });

                        ui.add_space(20.0);
                        ui.label("This action cannot be undone.");
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 80.0);
                            if ui.button("Cancel").clicked() {
                                self.current_view = View::Backups(name.clone());
                            }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(150, 40, 40))).clicked() {
                                self.delete_backup(&name, &path);
                            }
                        });
                    });
                }
                View::Console(name) => {
                    let name = name.clone();
                    ui.horizontal(|ui| {
                        ui.heading(format!("Console: {}", name));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                self.console_output.clear();
                            }
                            if ui.button("Back").clicked() {
                                self.current_view = View::Dashboard;
                            }
                        });
                    });

                    // Show RCON password for reference
                    if let Some(server) = self.servers.iter().find(|s| s.config.name == name) {
                        ui.horizontal(|ui| {
                            ui.small(format!("RCON Port: {} | Password: {}",
                                server.config.rcon_port(),
                                server.config.rcon_password
                            ));
                        });
                    }
                    ui.separator();

                    // Console output (scrollable)
                    let available_height = ui.available_height() - 35.0; // Reserve space for input
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.console_output {
                                ui.monospace(line);
                            }
                        });

                    ui.separator();

                    // Command input
                    let mut send_command = false;
                    ui.horizontal(|ui| {
                        ui.label(">");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.console_input)
                                .desired_width(ui.available_width() - 70.0)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("Enter command...")
                        );

                        // Send on Enter key
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            send_command = true;
                        }

                        if ui.button("Send").clicked() {
                            send_command = true;
                        }
                    });

                    if send_command && !self.console_input.is_empty() {
                        let cmd = self.console_input.clone();
                        self.console_input.clear();
                        self.send_rcon_command(&name, &cmd);
                    }
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
                View::DockerLogs => {
                    // Auto-refresh every 5 seconds
                    let should_refresh = self.docker_logs_last_refresh
                        .map(|t| t.elapsed().as_secs() >= 5)
                        .unwrap_or(true);
                    if should_refresh {
                        self.refresh_docker_logs();
                    }
                    // Request repaint to keep auto-refresh going
                    ctx.request_repaint_after(std::time::Duration::from_secs(1));

                    ui.horizontal(|ui| {
                        ui.heading("Docker Logs");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Refresh").clicked() {
                                self.refresh_docker_logs();
                            }
                            // Show auto-refresh indicator
                            ui.small("(auto-refresh: 5s)");
                        });
                    });
                    ui.label("Combined logs from all DrakonixAnvil-managed containers");
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.all_docker_logs.as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                            );
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
