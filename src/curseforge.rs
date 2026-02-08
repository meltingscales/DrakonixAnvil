use crate::server::ModLoader;
use serde::Deserialize;

// ── CurseForge API response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfSearchResponse {
    pub data: Vec<CfMod>,
    pub pagination: CfPagination,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfPagination {
    pub total_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: u64,
    pub logo: Option<CfLogo>,
    pub latest_files_indexes: Vec<CfLatestFileIndex>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLatestFileIndex {
    pub game_version: String,
    #[allow(dead_code)] // Used by API, may be used for filtering later
    pub mod_loader: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFilesResponse {
    pub data: Vec<CfFile>,
}

#[derive(Debug, Deserialize)]
pub struct CfDescriptionResponse {
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u64,
    pub display_name: String,
    #[allow(dead_code)] // Available for future display
    pub file_name: String,
    pub game_versions: Vec<String>,
    pub file_date: String,
    #[allow(dead_code)] // Deserialized from API, may be useful for display
    pub server_pack_file_id: Option<u64>,
}

// ── Search parameters ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CfSortField {
    Popularity,
    LastUpdated,
    Name,
    TotalDownloads,
}

impl CfSortField {
    pub fn as_api_value(self) -> u32 {
        match self {
            Self::Popularity => 2,
            Self::LastUpdated => 3,
            Self::Name => 5,
            Self::TotalDownloads => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Popularity => "Popularity",
            Self::LastUpdated => "Last Updated",
            Self::Name => "Name",
            Self::TotalDownloads => "Total Downloads",
        }
    }

    pub const ALL: [CfSortField; 4] = [
        Self::Popularity,
        Self::LastUpdated,
        Self::Name,
        Self::TotalDownloads,
    ];
}

/// Map our ModLoader enum to CurseForge's modLoaderType query param.
pub fn mod_loader_api_value(loader: &ModLoader) -> Option<u32> {
    match loader {
        ModLoader::Forge => Some(1),
        ModLoader::Fabric => Some(4),
        ModLoader::NeoForge => Some(6),
        ModLoader::Vanilla => None,
    }
}

// ── Async API functions ────────────────────────────────────────────────────

const CF_BASE: &str = "https://api.curseforge.com/v1";
const GAME_ID: u32 = 432; // Minecraft
const CLASS_ID: u32 = 4471; // Modpacks

/// Search CurseForge for modpacks. Returns (results, total_count).
pub async fn search_modpacks(
    api_key: &str,
    query: &str,
    game_version: &str,
    mod_loader: Option<&ModLoader>,
    sort_field: CfSortField,
    page_offset: u64,
) -> anyhow::Result<(Vec<CfMod>, u64)> {
    let client = reqwest::Client::new();

    let mut req = client
        .get(format!("{}/mods/search", CF_BASE))
        .header("x-api-key", api_key)
        .query(&[
            ("gameId", GAME_ID.to_string()),
            ("classId", CLASS_ID.to_string()),
            ("pageSize", "20".to_string()),
            ("sortField", sort_field.as_api_value().to_string()),
            ("sortOrder", "desc".to_string()),
            ("index", page_offset.to_string()),
        ]);

    if !query.is_empty() {
        req = req.query(&[("searchFilter", query)]);
    }
    if !game_version.is_empty() {
        req = req.query(&[("gameVersion", game_version)]);
    }
    if let Some(loader) = mod_loader {
        if let Some(val) = mod_loader_api_value(loader) {
            req = req.query(&[("modLoaderType", val.to_string())]);
        }
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("CurseForge API error {}: {}", status, body);
    }

    let data: CfSearchResponse = resp.json().await?;
    let total = data.pagination.total_count;
    Ok((data.data, total))
}

/// Fetch available files for a specific mod/modpack.
pub async fn get_mod_files(api_key: &str, mod_id: u64) -> anyhow::Result<Vec<CfFile>> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/mods/{}/files", CF_BASE, mod_id))
        .header("x-api-key", api_key)
        .query(&[("pageSize", "50")])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("CurseForge API error {}: {}", status, body);
    }

    let data: CfFilesResponse = resp.json().await?;
    Ok(data.data)
}

/// Fetch the HTML description for a mod/modpack and return it as plain text.
pub async fn get_mod_description(api_key: &str, mod_id: u64) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/mods/{}/description", CF_BASE, mod_id))
        .header("x-api-key", api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("CurseForge API error {}: {}", status, body);
    }

    let data: CfDescriptionResponse = resp.json().await?;
    Ok(strip_html(&data.data))
}

/// Strip HTML tags and decode common entities to produce plain text.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                // Insert newline for block elements
                let lower = html.as_bytes();
                let _ = lower; // just to mark block boundary
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Decode common HTML entities
    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse multiple blank lines into at most two newlines
    let mut collapsed = String::with_capacity(result.len());
    let mut blank_count = 0u32;
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                collapsed.push('\n');
            }
        } else {
            blank_count = 0;
            if !collapsed.is_empty() && !collapsed.ends_with('\n') {
                collapsed.push('\n');
            }
            collapsed.push_str(trimmed);
        }
    }

    collapsed.trim().to_string()
}

// ── Helper functions ───────────────────────────────────────────────────────

/// Infer the required Java version from a Minecraft version string.
pub fn infer_java_version(mc_version: &str) -> u8 {
    let parts: Vec<u32> = mc_version
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    let (major, minor) = match (parts.first(), parts.get(1)) {
        (Some(&1), Some(&minor)) => (1, minor),
        (Some(&1), None) => (1, 0),
        _ => return 21, // Unknown or future versions default to 21
    };

    if major == 1 {
        if minor <= 16 {
            return 8;
        }
        if minor <= 20 {
            // 1.17-1.20.4 -> Java 17, 1.20.5+ -> Java 21
            let patch = parts.get(2).copied().unwrap_or(0);
            if minor == 20 && patch >= 5 {
                return 21;
            }
            if minor > 20 {
                return 21;
            }
            return 17;
        }
        return 21;
    }
    21
}

/// Infer our ModLoader from CurseForge's numeric modLoaderType.
#[allow(dead_code)] // Available for future use
pub fn infer_mod_loader(cf_loader: Option<u32>) -> ModLoader {
    match cf_loader {
        Some(1) => ModLoader::Forge,
        Some(4) => ModLoader::Fabric,
        Some(6) => ModLoader::NeoForge,
        _ => ModLoader::Forge, // Default to Forge for unknown
    }
}

/// Format a download count for display (e.g. 1234567 -> "1.2M").
pub fn format_downloads(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Default G1GC JVM arguments.
pub fn default_java_args() -> Vec<String> {
    vec![
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=200".to_string(),
        "-XX:+UnlockExperimentalVMOptions".to_string(),
        "-XX:+DisableExplicitGC".to_string(),
        "-XX:G1NewSizePercent=30".to_string(),
        "-XX:G1MaxNewSizePercent=40".to_string(),
        "-XX:G1HeapRegionSize=8M".to_string(),
        "-XX:G1ReservePercent=20".to_string(),
    ]
}

/// Default memory allocation based on Minecraft version.
/// Modern packs (1.16+) get 6144MB, older get 4096MB.
pub fn default_memory_mb(mc_version: &str) -> u64 {
    let parts: Vec<u32> = mc_version
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    match (parts.first(), parts.get(1)) {
        (Some(&1), Some(minor)) if *minor >= 16 => 6144,
        _ => 4096,
    }
}

/// Extract sorted unique Minecraft versions from a list of CfFiles.
/// Filters out non-MC strings (like "Forge", "NeoForge") and returns
/// versions sorted descending (newest first).
pub fn extract_mc_versions(files: &[CfFile]) -> Vec<String> {
    let mut versions: Vec<String> = files
        .iter()
        .flat_map(|f| f.game_versions.iter())
        .filter(|v| v.starts_with(|c: char| c.is_ascii_digit()))
        .cloned()
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();

    versions.sort_by(|a, b| {
        let parse = |s: &str| -> Vec<u32> {
            s.split('.').filter_map(|p| p.parse().ok()).collect()
        };
        parse(b).cmp(&parse(a))
    });

    versions
}
