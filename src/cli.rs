//! Command-line interface for DrakonixAnvil.
//!
//! Reuses the same core modules the GUI drives (`docker`, `backup`, `rcon`,
//! `pack_installer`, `config`, `server`) so both frontends share behavior.
//! Running the binary with no subcommand launches the GUI instead (see `main.rs`).

use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

use crate::backup;
use crate::config;
use crate::docker::{CreateContainerParams, DockerManager};
use crate::pack_installer;
use crate::rcon::RconClient;
use crate::server::{
    ModLoader, ModpackInfo, ModpackSource, ServerConfig, ServerInstance, ServerStatus,
};

#[derive(clap::Parser)]
#[command(
    name = "drakonix-anvil",
    version,
    about = "Deploy and manage Minecraft servers with Docker.",
    long_about = "Deploy and manage Minecraft servers with Docker.\n\n\
                  Run without a subcommand to launch the graphical interface."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// List all configured servers and their container state
    List,
    /// Show detailed status for one server
    Status {
        /// Server name
        name: String,
    },
    /// Create (if missing) and start a server's container
    Start {
        /// Server name
        name: String,
    },
    /// Stop a running server
    Stop {
        /// Server name
        name: String,
    },
    /// Stop then start a server
    Restart {
        /// Server name
        name: String,
    },
    /// Create a new server configuration
    Create(CreateArgs),
    /// Delete a server (removes its container; optionally its data)
    Delete {
        /// Server name
        name: String,
        /// Also delete the server's data directory and backups
        #[arg(long)]
        purge: bool,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Create a zip backup of a server's data
    Backup {
        /// Server name
        name: String,
    },
    /// List backups for a server
    Backups {
        /// Server name
        name: String,
    },
    /// Restore a backup into a server (OVERWRITES current data!)
    Restore {
        /// Server name
        name: String,
        /// Backup filename (in the server's backup dir) or a path to a .zip
        file: String,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Export a server to a portable .drakonixanvil-server.zip bundle
    Export {
        /// Server name
        name: String,
        /// Output path for the bundle
        output: PathBuf,
    },
    /// Import a server from a .drakonixanvil-server.zip bundle
    Import {
        /// Path to the bundle
        file: PathBuf,
    },
    /// Send an RCON command to a running server
    Rcon {
        /// Server name
        name: String,
        /// The command and its arguments (e.g. `list`, or `say hello world`)
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
}

#[derive(clap::Args)]
pub struct CreateArgs {
    /// Unique server name (used for the container name and data directory)
    name: String,
    /// Host port to expose the Minecraft server on
    #[arg(short, long, default_value_t = 25565)]
    port: u16,
    /// Memory limit in MB
    #[arg(short, long, default_value_t = 4096)]
    memory: u64,
    /// Java version image tag (8, 11, 17, or 21)
    #[arg(short, long, default_value_t = 21)]
    java: u8,
    /// Minecraft version (e.g. 1.20.1). Optional; some sources infer it.
    #[arg(long)]
    mc_version: Option<String>,
    #[command(subcommand)]
    source: SourceCmd,
}

#[derive(clap::Subcommand)]
pub enum SourceCmd {
    /// CurseForge modpack (AUTO_CURSEFORGE) — needs a CF API key in settings
    Curseforge {
        /// CurseForge project slug (e.g. all-the-mods-9)
        slug: String,
        /// Specific file id; 0 lets the image pick the latest
        #[arg(long, default_value_t = 0)]
        file_id: u64,
    },
    /// Modrinth modpack
    Modrinth {
        /// Modrinth project id or slug
        project_id: String,
        /// Modrinth version id
        version_id: String,
    },
    /// Feed the Beast modpack
    Ftb {
        /// FTB modpack id
        pack_id: u64,
        /// Specific version id; 0 lets the image pick the latest
        #[arg(long, default_value_t = 0)]
        version_id: u64,
    },
    /// Install a Forge version, then overlay a server-pack zip URL
    Forge {
        /// Forge version (e.g. 47.2.0)
        forge_version: String,
        /// URL of a server-pack zip to overlay
        pack_url: String,
    },
    /// Direct download of a modpack/server zip URL (typed by mod loader)
    Direct {
        /// URL of the modpack/server zip
        url: String,
        /// Mod loader the pack uses
        #[arg(long, value_enum, default_value_t = Loader::Forge)]
        loader: Loader,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum Loader {
    Forge,
    Fabric,
    NeoForge,
    Vanilla,
}

impl Loader {
    fn to_modloader(&self) -> ModLoader {
        match self {
            Loader::Forge => ModLoader::Forge,
            Loader::Fabric => ModLoader::Fabric,
            Loader::NeoForge => ModLoader::NeoForge,
            Loader::Vanilla => ModLoader::Vanilla,
        }
    }
}

/// Entry point for CLI mode. Returns a process-level error on failure.
pub fn run(command: Command) -> Result<()> {
    init_logging();
    match command {
        Command::List => cmd_list(),
        Command::Status { name } => cmd_status(&name),
        Command::Start { name } => cmd_start(&name),
        Command::Stop { name } => cmd_stop(&name),
        Command::Restart { name } => cmd_restart(&name),
        Command::Create(args) => cmd_create(args),
        Command::Delete { name, purge, yes } => cmd_delete(&name, purge, yes),
        Command::Backup { name } => cmd_backup(&name),
        Command::Backups { name } => cmd_backups(&name),
        Command::Restore { name, file, yes } => cmd_restore(&name, &file, yes),
        Command::Export { name, output } => cmd_export(&name, &output),
        Command::Import { file } => cmd_import(&file),
        Command::Rcon { name, command } => cmd_rcon(&name, &command),
    }
}

/// Log to stderr so command output on stdout stays machine-friendly.
fn init_logging() {
    use tracing_subscriber::prelude::*;

    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into());
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true),
        )
        .init();
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build an async runtime and a Docker client, verifying the daemon responds.
fn docker_and_runtime() -> Result<(Runtime, DockerManager)> {
    let rt = Runtime::new().context("Failed to create async runtime")?;
    let docker =
        DockerManager::new().context("Failed to connect to Docker — is the daemon running?")?;
    if !rt.block_on(docker.check_connection())? {
        bail!("Docker is not responding. Is the Docker daemon running?");
    }
    Ok((rt, docker))
}

/// Human-readable container state for a server, using its stable container name.
fn container_state(rt: &Runtime, docker: &DockerManager, name: &str) -> String {
    let cname = config::get_container_name(name);
    rt.block_on(async {
        if !docker.container_exists(&cname).await.unwrap_or(false) {
            return "not-created".to_string();
        }
        match docker.is_container_running(&cname).await {
            Ok(true) => "running".to_string(),
            Ok(false) => "stopped".to_string(),
            Err(_) => "unknown".to_string(),
        }
    })
}

/// Find a server's index in the list or return a helpful error.
fn find_index(servers: &[ServerInstance], name: &str) -> Result<usize> {
    servers
        .iter()
        .position(|s| s.config.name == name)
        .with_context(|| format!("Server '{}' not found. Try `drakonix-anvil list`.", name))
}

/// Prompt for a yes/no confirmation on stdin. Returns true if the user confirmed.
fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N] ", prompt);
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("Failed to read confirmation")?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut)
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_list() -> Result<()> {
    let servers = config::load_servers()?;
    if servers.is_empty() {
        println!("No servers configured. Create one with `drakonix-anvil create`.");
        return Ok(());
    }

    // Live container state is best-effort — still list servers if Docker is down.
    let docker = docker_and_runtime().ok();

    println!(
        "{:<20} {:<7} {:<28} {:<12}",
        "NAME", "PORT", "MODPACK", "STATE"
    );
    for s in &servers {
        let state = match &docker {
            Some((rt, docker)) => container_state(rt, docker, &s.config.name),
            None => "?".to_string(),
        };
        println!(
            "{:<20} {:<7} {:<28} {:<12}",
            truncate(&s.config.name, 20),
            s.config.port,
            truncate(&s.config.modpack.name, 28),
            state
        );
    }
    Ok(())
}

fn cmd_status(name: &str) -> Result<()> {
    let servers = config::load_servers()?;
    let inst = &servers[find_index(&servers, name)?];
    let c = &inst.config;

    println!("Name:       {}", c.name);
    println!("Modpack:    {}", c.modpack.name);
    println!("Source:     {:?}", c.modpack.source);
    println!(
        "MC version: {}",
        if c.modpack.minecraft_version.is_empty() {
            "(auto)"
        } else {
            &c.modpack.minecraft_version
        }
    );
    println!("Port:       {}", c.port);
    println!("RCON:       127.0.0.1:{}", c.rcon_port());
    println!("Memory:     {} MB", c.memory_mb);
    println!("Java:       {}", c.java_version);

    match docker_and_runtime() {
        Ok((rt, docker)) => println!("State:      {}", container_state(&rt, &docker, name)),
        Err(_) => println!("State:      (docker unavailable)"),
    }
    Ok(())
}

fn cmd_start(name: &str) -> Result<()> {
    let (rt, docker) = docker_and_runtime()?;
    let mut servers = config::load_servers()?;
    let idx = find_index(&servers, name)?;
    start_instance(&rt, &docker, &mut servers, idx)?;
    config::save_servers(&servers)?;
    Ok(())
}

/// Start server `idx`, creating and configuring its container on first run.
/// Mirrors the GUI's `start_server` flow but runs synchronously.
fn start_instance(
    rt: &Runtime,
    docker: &DockerManager,
    servers: &mut [ServerInstance],
    idx: usize,
) -> Result<()> {
    let settings = config::load_settings();
    let name = servers[idx].config.name.clone();
    let container_name = config::get_container_name(&name);
    let port = servers[idx].config.port;
    let rcon_port = servers[idx].config.rcon_port();
    let memory_mb = servers[idx].config.memory_mb;
    let docker_image = servers[idx].config.docker_image();
    let modpack_source = servers[idx].config.modpack.source.clone();

    let data_path = config::get_server_data_path(&name);
    std::fs::create_dir_all(&data_path).context("Failed to create data directory")?;

    let mut env_vars = servers[idx].config.build_docker_env();
    if let Some(cf_key) = &settings.curseforge_api_key {
        if !cf_key.is_empty() {
            env_vars.push(format!("CF_API_KEY={}", cf_key));
        }
    }

    rt.block_on(async {
        if docker.container_exists(&container_name).await? {
            println!("Starting existing container '{}'...", container_name);
            docker.start_container(&container_name).await?;
        } else {
            println!("Pulling image {} (this can take a while)...", docker_image);
            docker.ensure_image(&docker_image).await?;

            // ForgeWithPack overlays a server pack on the host before the container starts.
            if let ModpackSource::ForgeWithPack { pack_url, .. } = &modpack_source {
                println!("Installing server pack on host...");
                pack_installer::install_forge_pack(&data_path, pack_url).await?;
            }

            println!("Creating container '{}'...", container_name);
            let id = docker
                .create_minecraft_container(CreateContainerParams {
                    container_name: &container_name,
                    server_name: &name,
                    image: &docker_image,
                    port,
                    rcon_port,
                    memory_mb,
                    env_vars,
                    data_path: &data_path,
                })
                .await?;
            docker.start_container(&id).await?;
            servers[idx].container_id = Some(id);
        }
        anyhow::Ok(())
    })?;

    servers[idx].status = ServerStatus::Running;
    println!(
        "Server '{}' started. Port {}, RCON on 127.0.0.1:{}.",
        name, port, rcon_port
    );
    println!(
        "The modpack may still be initializing — check `drakonix-anvil status {}` or `docker logs -f {}`.",
        name, container_name
    );
    Ok(())
}

fn cmd_stop(name: &str) -> Result<()> {
    let (rt, docker) = docker_and_runtime()?;
    let mut servers = config::load_servers()?;
    let idx = find_index(&servers, name)?;
    let container_name = config::get_container_name(name);

    rt.block_on(async {
        if !docker.container_exists(&container_name).await? {
            bail!("Server '{}' has no container to stop.", name);
        }
        docker.stop_container(&container_name).await?;
        anyhow::Ok(())
    })?;

    servers[idx].status = ServerStatus::Stopped;
    config::save_servers(&servers)?;
    println!("Server '{}' stopped.", name);
    Ok(())
}

fn cmd_restart(name: &str) -> Result<()> {
    // Ignore stop failures (e.g. container not yet created) — restart should still start.
    if let Err(e) = cmd_stop(name) {
        tracing::warn!("Stop during restart: {}", e);
    }
    cmd_start(name)
}

fn cmd_create(args: CreateArgs) -> Result<()> {
    let mut servers = config::load_servers()?;
    if servers.iter().any(|s| s.config.name == args.name) {
        bail!("A server named '{}' already exists.", args.name);
    }

    let (loader, source, modpack_name) = build_source(&args);
    let modpack = ModpackInfo {
        name: modpack_name,
        version: String::new(),
        minecraft_version: args.mc_version.clone().unwrap_or_default(),
        loader,
        source,
    };

    let mut cfg = ServerConfig::new(args.name.clone(), modpack);
    cfg.port = args.port;
    cfg.memory_mb = args.memory;
    cfg.java_version = args.java;

    servers.push(ServerInstance {
        config: cfg,
        container_id: None,
        status: ServerStatus::Stopped,
    });
    config::save_servers(&servers)?;

    println!(
        "Created server '{}'. Start it with `drakonix-anvil start {}`.",
        args.name, args.name
    );
    Ok(())
}

/// Map the source subcommand to a `(loader, source, display name)` triple.
/// Loader only affects env for `Direct`/`Local` sources; other sources ignore it.
fn build_source(args: &CreateArgs) -> (ModLoader, ModpackSource, String) {
    match &args.source {
        SourceCmd::Curseforge { slug, file_id } => (
            ModLoader::Forge,
            ModpackSource::CurseForge {
                slug: slug.clone(),
                file_id: *file_id,
            },
            slug.clone(),
        ),
        SourceCmd::Modrinth {
            project_id,
            version_id,
        } => (
            ModLoader::Fabric,
            ModpackSource::Modrinth {
                project_id: project_id.clone(),
                version_id: version_id.clone(),
            },
            project_id.clone(),
        ),
        SourceCmd::Ftb {
            pack_id,
            version_id,
        } => (
            ModLoader::Forge,
            ModpackSource::Ftb {
                pack_id: *pack_id,
                version_id: *version_id,
            },
            format!("FTB pack {}", pack_id),
        ),
        SourceCmd::Forge {
            forge_version,
            pack_url,
        } => (
            ModLoader::Forge,
            ModpackSource::ForgeWithPack {
                forge_version: forge_version.clone(),
                pack_url: pack_url.clone(),
            },
            format!("Forge {}", forge_version),
        ),
        SourceCmd::Direct { url, loader } => {
            let name = url.rsplit('/').next().unwrap_or(url).to_string();
            (
                loader.to_modloader(),
                ModpackSource::DirectDownload { url: url.clone() },
                name,
            )
        }
    }
}

fn cmd_delete(name: &str, purge: bool, yes: bool) -> Result<()> {
    let mut servers = config::load_servers()?;
    let idx = find_index(&servers, name)?;

    if !yes {
        let extra = if purge { " and its data/backups" } else { "" };
        if !confirm(&format!("Delete server '{}'{}?", name, extra))? {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Remove the container best-effort; a missing Docker daemon shouldn't block config cleanup.
    if let Ok((rt, docker)) = docker_and_runtime() {
        let cname = config::get_container_name(name);
        let _ = rt.block_on(async {
            let _ = docker.stop_container(&cname).await;
            docker.remove_container(&cname).await
        });
    }

    servers.remove(idx);
    config::save_servers(&servers)?;

    if purge {
        let server_path = config::get_server_path(name);
        if server_path.exists() {
            std::fs::remove_dir_all(&server_path).ok();
        }
        let backup_path = config::get_backup_path(name);
        if backup_path.exists() {
            std::fs::remove_dir_all(&backup_path).ok();
        }
    }

    println!("Server '{}' deleted.", name);
    Ok(())
}

fn cmd_backup(name: &str) -> Result<()> {
    let servers = config::load_servers()?;
    find_index(&servers, name)?;
    let path = backup::create_backup_with_progress(name, None)?;
    println!("Backup created: {}", path.display());
    Ok(())
}

fn cmd_backups(name: &str) -> Result<()> {
    let backups = backup::list_backups(name)?;
    if backups.is_empty() {
        println!("No backups for '{}'.", name);
        return Ok(());
    }
    println!("{:<24} {:>10}", "BACKUP", "SIZE");
    for b in backups {
        println!(
            "{:<24} {:>10}",
            b.filename,
            backup::format_bytes(b.size_bytes)
        );
    }
    Ok(())
}

fn cmd_restore(name: &str, file: &str, yes: bool) -> Result<()> {
    let servers = config::load_servers()?;
    find_index(&servers, name)?;

    // Accept either a direct path or a filename inside the server's backup dir.
    let candidate = PathBuf::from(file);
    let path = if candidate.exists() {
        candidate
    } else {
        config::get_backup_path(name).join(file)
    };
    if !path.exists() {
        bail!("Backup not found: {}", path.display());
    }

    if !yes
        && !confirm(&format!(
            "Restore '{}' into '{}'? This OVERWRITES current data.",
            path.display(),
            name
        ))?
    {
        println!("Aborted.");
        return Ok(());
    }

    backup::restore_backup_with_progress(name, &path, None)?;
    println!("Restored '{}' from {}", name, path.display());
    Ok(())
}

fn cmd_export(name: &str, output: &Path) -> Result<()> {
    let servers = config::load_servers()?;
    let inst = &servers[find_index(&servers, name)?];
    let data_path = config::get_server_data_path(name);
    let out = backup::export_server_with_progress(&inst.config, &data_path, output, None)?;
    println!("Exported '{}' to {}", name, out.display());
    Ok(())
}

fn cmd_import(file: &Path) -> Result<()> {
    let servers_dir = PathBuf::from(config::DATA_ROOT).join("servers");
    let imported = backup::import_server(file, &servers_dir, None)?;

    let mut servers = config::load_servers()?;
    if servers.iter().any(|s| s.config.name == imported.name) {
        bail!(
            "Data extracted, but a server named '{}' already exists — not registered.",
            imported.name
        );
    }

    let name = imported.name.clone();
    servers.push(ServerInstance {
        config: imported,
        container_id: None,
        status: ServerStatus::Stopped,
    });
    config::save_servers(&servers)?;
    println!("Imported server '{}'.", name);
    Ok(())
}

fn cmd_rcon(name: &str, command: &[String]) -> Result<()> {
    let servers = config::load_servers()?;
    let inst = &servers[find_index(&servers, name)?];

    let addr = format!("127.0.0.1:{}", inst.config.rcon_port());
    let mut client = RconClient::connect(&addr, &inst.config.rcon_password)
        .map_err(|e| anyhow::anyhow!("RCON connect to {} failed: {}", addr, e))?;

    let cmd = command.join(" ");
    let resp = client
        .command(&cmd)
        .map_err(|e| anyhow::anyhow!("RCON command failed: {}", e))?;

    if resp.trim().is_empty() {
        println!("(no output)");
    } else {
        println!("{}", resp);
    }
    Ok(())
}
