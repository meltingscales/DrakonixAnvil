use std::path::PathBuf;
use anyhow::Result;
use crate::server::ServerInstance;

/// Root directory for all DrakonixAnvil data
pub const DATA_ROOT: &str = "./DrakonixAnvilData";

/// Path to the servers index file
pub fn get_servers_index_path() -> PathBuf {
    PathBuf::from(DATA_ROOT).join("servers.json")
}

/// Save all servers to disk
pub fn save_servers(servers: &[ServerInstance]) -> Result<()> {
    let path = get_servers_index_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(servers)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load servers from disk
pub fn load_servers() -> Result<Vec<ServerInstance>> {
    let path = get_servers_index_path();

    if !path.exists() {
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string(&path)?;
    let servers: Vec<ServerInstance> = serde_json::from_str(&json)?;
    Ok(servers)
}

/// Get the path to a server's data directory
pub fn get_server_path(server_name: &str) -> PathBuf {
    PathBuf::from(DATA_ROOT).join("servers").join(server_name)
}

/// Get the path to a server's data volume (mounted as /data in container)
pub fn get_server_data_path(server_name: &str) -> PathBuf {
    get_server_path(server_name).join("data")
}

/// Get the path to a server's logs directory
#[allow(dead_code)]
pub fn get_server_logs_path(server_name: &str) -> PathBuf {
    get_server_path(server_name).join("logs")
}

/// Get the path to a server's metadata file
#[allow(dead_code)]
pub fn get_server_metadata_path(server_name: &str) -> PathBuf {
    get_server_path(server_name).join("server.json")
}

/// Get the path to backups for a server
#[allow(dead_code)]
pub fn get_backup_path(server_name: &str) -> PathBuf {
    PathBuf::from(DATA_ROOT).join("backups").join(server_name)
}

/// Docker container name prefix
pub const CONTAINER_PREFIX: &str = "drakonix";

/// Get the Docker container name for a server
pub fn get_container_name(server_name: &str) -> String {
    format!("{}-{}", CONTAINER_PREFIX, server_name)
}

/// The standard Minecraft server Docker image
pub const MINECRAFT_IMAGE: &str = "itzg/minecraft-server:latest";
