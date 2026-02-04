use anyhow::{Result, Context};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter, CompressionMethod};

use crate::config::{get_server_data_path, get_backup_path};

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
pub fn create_backup(server_name: &str) -> Result<PathBuf> {
    let data_path = get_server_data_path(server_name);
    let backup_dir = get_backup_path(server_name);

    // Ensure data directory exists
    if !data_path.exists() {
        anyhow::bail!("Server data directory does not exist: {:?}", data_path);
    }

    // Create backup directory
    fs::create_dir_all(&backup_dir)
        .context("Failed to create backup directory")?;

    // Generate backup filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}.zip", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // Create the zip file
    let file = File::create(&backup_path)
        .context("Failed to create backup file")?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Walk the data directory and add all files to the zip
    for entry in WalkDir::new(&data_path) {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let relative_path = path.strip_prefix(&data_path)
            .context("Failed to get relative path")?;

        // Skip the root directory itself
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let path_str = relative_path.to_string_lossy().to_string();

        if path.is_dir() {
            // Add directory entry
            zip.add_directory(path_str, options.clone())
                .context("Failed to add directory to zip")?;
        } else {
            // Add file
            zip.start_file(path_str, options.clone())
                .context("Failed to start file in zip")?;

            let mut file = File::open(path)
                .context("Failed to open file for backup")?;
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
            let filename = path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            backups.push(BackupInfo {
                filename,
                path,
                size_bytes: metadata.len(),
                created: metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            });
        }
    }

    // Sort by creation time, newest first
    backups.sort_by(|a, b| b.created.cmp(&a.created));

    Ok(backups)
}

/// Restore a backup to a server's data directory
/// WARNING: This will overwrite existing data!
pub fn restore_backup(server_name: &str, backup_path: &Path) -> Result<()> {
    let data_path = get_server_data_path(server_name);

    // Verify backup file exists
    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist: {:?}", backup_path);
    }

    // Clear existing data directory
    if data_path.exists() {
        fs::remove_dir_all(&data_path)
            .context("Failed to clear existing data directory")?;
    }
    fs::create_dir_all(&data_path)
        .context("Failed to create data directory")?;

    // Extract the zip file
    let file = File::open(backup_path)
        .context("Failed to open backup file")?;
    let mut archive = ZipArchive::new(file)
        .context("Failed to read zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .context("Failed to read zip entry")?;

        // Sanitize the path to prevent zip slip attacks
        let outpath = match file.enclosed_name() {
            Some(path) => data_path.join(path),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .context("Failed to create directory during restore")?;
        } else {
            // Create parent directories if needed
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create parent directory")?;
            }

            let mut outfile = File::create(&outpath)
                .context("Failed to create file during restore")?;
            std::io::copy(&mut file, &mut outfile)
                .context("Failed to write file during restore")?;
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    Ok(())
}

/// Delete a backup file
pub fn delete_backup(backup_path: &Path) -> Result<()> {
    fs::remove_file(backup_path)
        .context("Failed to delete backup file")?;
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
