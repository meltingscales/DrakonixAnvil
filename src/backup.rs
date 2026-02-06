use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::config::{get_backup_path, get_server_data_path};

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

    // Ensure data directory exists
    if !data_path.exists() {
        anyhow::bail!("Server data directory does not exist: {:?}", data_path);
    }

    // Create backup directory
    fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

    // Count total files first for progress reporting
    let entries: Vec<_> = WalkDir::new(&data_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            !e.path()
                .strip_prefix(&data_path)
                .map(|p| p.as_os_str().is_empty())
                .unwrap_or(true)
        })
        .collect();
    let total_files = entries.len();

    // Generate backup filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}.zip", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // Create the zip file
    let file = File::create(&backup_path).context("Failed to create backup file")?;
    let mut zip = ZipWriter::new(file);

    let file_options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let dir_options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755); // Directories need execute bit to be traversable

    // Process each entry
    for (idx, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let relative_path = path
            .strip_prefix(&data_path)
            .context("Failed to get relative path")?;

        let path_str = relative_path.to_string_lossy().to_string();

        // Send progress update
        if let Some(tx) = &progress_tx {
            let _ = tx.send(BackupProgress {
                current: idx + 1,
                total: total_files,
                current_file: path_str.clone(),
            });
        }

        if path.is_dir() {
            // Add directory entry
            zip.add_directory(path_str, dir_options)
                .context("Failed to add directory to zip")?;
        } else {
            // Add file
            zip.start_file(path_str, file_options)
                .context("Failed to start file in zip")?;

            let mut file = File::open(path).context("Failed to open file for backup")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .context("Failed to read file")?;
            zip.write_all(&buffer)
                .context("Failed to write file to zip")?;
        }
    }

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

    // Verify backup file exists
    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist: {:?}", backup_path);
    }

    // Clear existing data directory
    if data_path.exists() {
        fs::remove_dir_all(&data_path).context("Failed to clear existing data directory")?;
    }
    fs::create_dir_all(&data_path).context("Failed to create data directory")?;

    // Extract the zip file
    let file = File::open(backup_path).context("Failed to open backup file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;

    let total_entries = archive.len();

    for i in 0..total_entries {
        let mut file = archive.by_index(i).context("Failed to read zip entry")?;

        // Sanitize the path to prevent zip slip attacks
        let outpath = match file.enclosed_name() {
            Some(path) => data_path.join(path),
            None => continue,
        };

        let file_name = file
            .enclosed_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Send progress update
        if let Some(tx) = &progress_tx {
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
            // Create parent directories if needed
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
                // Ensure directories have execute bit (required for traversal)
                // This fixes backups created with incorrect directory permissions
                if file.is_dir() {
                    mode |= 0o111; // Add execute for user/group/other
                }
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    Ok(())
}

/// Delete a backup file
pub fn delete_backup(backup_path: &Path) -> Result<()> {
    fs::remove_file(backup_path).context("Failed to delete backup file")?;
    Ok(())
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
