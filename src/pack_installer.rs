use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

/// Download a modpack zip from a URL and extract it into the server's data directory.
/// Skips extraction if a marker file exists (pack already installed).
pub async fn install_forge_pack(data_path: &Path, pack_url: &str) -> Result<()> {
    let marker = data_path.join(".pack_installed");
    if marker.exists() {
        tracing::info!("Pack already installed (marker exists), skipping download");
        return Ok(());
    }

    tracing::info!("Downloading server pack from {}...", pack_url);

    let response = reqwest::get(pack_url)
        .await
        .context("Failed to download server pack")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download server pack: HTTP {}",
            response.status()
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read server pack response body")?;

    tracing::info!(
        "Downloaded {} bytes, extracting to {}...",
        bytes.len(),
        data_path.display()
    );

    // Extract zip to data directory
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to open server pack as zip")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(enclosed_name) = file.enclosed_name() else {
            tracing::warn!("Skipping zip entry with unsafe path: {:?}", file.name());
            continue;
        };

        let out_path = data_path.join(enclosed_name);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)
                .with_context(|| format!("Failed to create directory {}", out_path.display()))?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = std::fs::File::create(&out_path)
                .with_context(|| format!("Failed to create file {}", out_path.display()))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            std::io::Write::write_all(&mut outfile, &buf)?;
        }
    }

    // Write marker so we don't re-download on next start
    std::fs::write(&marker, pack_url).ok();

    tracing::info!(
        "Server pack extracted successfully ({} entries)",
        archive.len()
    );
    Ok(())
}
