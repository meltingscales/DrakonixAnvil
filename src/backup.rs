use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::config::{get_backup_path, get_server_data_path};
use crate::server::ServerConfig;

/// Progress update for backup/restore operations
#[derive(Debug, Clone)]
pub struct BackupProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}

/// Information about a backup file
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: std::time::SystemTime,
}

// ---------------------------------------------------------------------------
// Internal helpers shared by backup, restore, export, and import
// ---------------------------------------------------------------------------

/// Walk `data_path` and add all files/dirs into the zip under `prefix`.
/// Backup calls with `prefix=""`, export calls with `prefix="data/"`.
fn zip_directory_with_progress(
    zip: &mut ZipWriter<File>,
    data_path: &Path,
    prefix: &str,
    progress_tx: Option<&Sender<BackupProgress>>,
) -> Result<()> {
    let entries: Vec<_> = WalkDir::new(data_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            !e.path()
                .strip_prefix(data_path)
                .map(|p| p.as_os_str().is_empty())
                .unwrap_or(true)
        })
        .collect();
    let total_files = entries.len();

    let file_options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let dir_options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);

    for (idx, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let relative_path = path
            .strip_prefix(data_path)
            .context("Failed to get relative path")?;

        let path_str = format!("{}{}", prefix, relative_path.to_string_lossy());

        if let Some(tx) = progress_tx {
            let _ = tx.send(BackupProgress {
                current: idx + 1,
                total: total_files,
                current_file: path_str.clone(),
            });
        }

        if path.is_dir() {
            zip.add_directory(&path_str, dir_options)
                .context("Failed to add directory to zip")?;
        } else {
            zip.start_file(&path_str, file_options)
                .context("Failed to start file in zip")?;

            let mut file = File::open(path).context("Failed to open file for backup")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .context("Failed to read file")?;
            zip.write_all(&buffer)
                .context("Failed to write file to zip")?;
        }
    }

    Ok(())
}

/// Extract entries from a zip into `dest_path`.
/// If `strip_prefix` is `Some("data/")`, only entries starting with that prefix are extracted
/// and the prefix is removed from their path.
fn extract_zip_with_progress(
    archive: &mut ZipArchive<File>,
    dest_path: &Path,
    strip_prefix: Option<&str>,
    progress_tx: Option<&Sender<BackupProgress>>,
) -> Result<()> {
    let total_entries = archive.len();

    for i in 0..total_entries {
        let mut file = archive.by_index(i).context("Failed to read zip entry")?;

        let enclosed = match file.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };

        // Apply strip_prefix filter
        let relative = if let Some(pfx) = strip_prefix {
            let s = enclosed.to_string_lossy();
            if !s.starts_with(pfx) {
                // Skip entries outside the prefix (e.g. server-config.json)
                if let Some(tx) = progress_tx {
                    let _ = tx.send(BackupProgress {
                        current: i + 1,
                        total: total_entries,
                        current_file: s.to_string(),
                    });
                }
                continue;
            }
            PathBuf::from(&s[pfx.len()..])
        } else {
            enclosed.clone()
        };

        // Skip empty relative paths
        if relative.as_os_str().is_empty() {
            continue;
        }

        let outpath = dest_path.join(&relative);

        let file_name = relative.to_string_lossy().to_string();

        if let Some(tx) = progress_tx {
            let _ = tx.send(BackupProgress {
                current: i + 1,
                total: total_entries,
                current_file: file_name,
            });
        }

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .with_context(|| format!("Failed to create directory: {:?}", outpath))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
            }

            let mut outfile = File::create(&outpath)
                .with_context(|| format!("Failed to create file: {:?}", outpath))?;
            std::io::copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to write file: {:?}", outpath))?;
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mut mode) = file.unix_mode() {
                if file.is_dir() {
                    mode |= 0o111;
                }
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Backup / Restore (existing API, now thin wrappers)
// ---------------------------------------------------------------------------

/// Create a backup of a server's data directory
/// Returns the path to the created backup file
#[allow(dead_code)]
pub fn create_backup(server_name: &str) -> Result<PathBuf> {
    create_backup_with_progress(server_name, None)
}

/// Create a backup with optional progress reporting
/// The progress sender receives updates as files are processed
pub fn create_backup_with_progress(
    server_name: &str,
    progress_tx: Option<Sender<BackupProgress>>,
) -> Result<PathBuf> {
    let data_path = get_server_data_path(server_name);
    let backup_dir = get_backup_path(server_name);

    if !data_path.exists() {
        anyhow::bail!("Server data directory does not exist: {:?}", data_path);
    }

    fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}.zip", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    let file = File::create(&backup_path).context("Failed to create backup file")?;
    let mut zip = ZipWriter::new(file);

    zip_directory_with_progress(&mut zip, &data_path, "", progress_tx.as_ref())?;

    zip.finish().context("Failed to finalize zip file")?;

    Ok(backup_path)
}

/// List all backups for a server
pub fn list_backups(server_name: &str) -> Result<Vec<BackupInfo>> {
    let backup_dir = get_backup_path(server_name);

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "zip").unwrap_or(false) {
            let metadata = fs::metadata(&path)?;
            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            backups.push(BackupInfo {
                filename,
                path,
                size_bytes: metadata.len(),
                created: metadata
                    .created()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            });
        }
    }

    // Sort by creation time, newest first
    backups.sort_by(|a, b| b.created.cmp(&a.created));

    Ok(backups)
}

/// Restore a backup to a server's data directory
/// WARNING: This will overwrite existing data!
#[allow(dead_code)]
pub fn restore_backup(server_name: &str, backup_path: &Path) -> Result<()> {
    restore_backup_with_progress(server_name, backup_path, None)
}

/// Restore a backup with optional progress reporting
pub fn restore_backup_with_progress(
    server_name: &str,
    backup_path: &Path,
    progress_tx: Option<Sender<BackupProgress>>,
) -> Result<()> {
    let data_path = get_server_data_path(server_name);

    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist: {:?}", backup_path);
    }

    if data_path.exists() {
        fs::remove_dir_all(&data_path).context("Failed to clear existing data directory")?;
    }
    fs::create_dir_all(&data_path).context("Failed to create data directory")?;

    let file = File::open(backup_path).context("Failed to open backup file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;

    extract_zip_with_progress(&mut archive, &data_path, None, progress_tx.as_ref())?;

    Ok(())
}

/// Delete a backup file
pub fn delete_backup(backup_path: &Path) -> Result<()> {
    fs::remove_file(backup_path).context("Failed to delete backup file")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Export / Import (server transit)
// ---------------------------------------------------------------------------

/// Export a server to a `.drakonixanvil-server.zip` bundle.
/// The zip contains `server-config.json` and a `data/` directory with the full server data.
pub fn export_server_with_progress(
    config: &ServerConfig,
    data_path: &Path,
    output_path: &Path,
    progress_tx: Option<Sender<BackupProgress>>,
) -> Result<PathBuf> {
    if !data_path.exists() {
        anyhow::bail!("Server data directory does not exist: {:?}", data_path);
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let file = File::create(output_path).context("Failed to create export file")?;
    let mut zip = ZipWriter::new(file);

    // Write server-config.json as the first entry
    let config_json =
        serde_json::to_string_pretty(config).context("Failed to serialize server config")?;
    let file_options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file("server-config.json", file_options)
        .context("Failed to write config entry")?;
    zip.write_all(config_json.as_bytes())
        .context("Failed to write config data")?;

    // Add all data files under the "data/" prefix
    zip_directory_with_progress(&mut zip, data_path, "data/", progress_tx.as_ref())?;

    zip.finish().context("Failed to finalize export zip")?;

    Ok(output_path.to_path_buf())
}

/// Read the `server-config.json` from an export zip without extracting data.
/// Useful for previewing before import.
pub fn read_export_config(zip_path: &Path) -> Result<ServerConfig> {
    let file = File::open(zip_path).context("Failed to open export file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;

    let mut config_file = archive
        .by_name("server-config.json")
        .context("Export bundle missing server-config.json")?;

    let mut config_json = String::new();
    config_file
        .read_to_string(&mut config_json)
        .context("Failed to read server-config.json")?;

    let config: ServerConfig =
        serde_json::from_str(&config_json).context("Failed to parse server-config.json")?;

    Ok(config)
}

/// Import a server from a `.drakonixanvil-server.zip` bundle.
/// Extracts the `data/` contents into `servers_dir/{name}/data/` and returns the config.
pub fn import_server(
    zip_path: &Path,
    servers_dir: &Path,
    progress_tx: Option<Sender<BackupProgress>>,
) -> Result<ServerConfig> {
    let config = read_export_config(zip_path)?;

    let data_path = servers_dir.join(&config.name).join("data");
    fs::create_dir_all(&data_path).context("Failed to create server data directory")?;

    let file = File::open(zip_path).context("Failed to open export file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;

    extract_zip_with_progress(&mut archive, &data_path, Some("data/"), progress_tx.as_ref())?;

    Ok(config)
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
