use serde::Deserialize;

// ── Modrinth API response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MrSearchResponse {
    pub hits: Vec<MrProject>,
    pub total_hits: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MrProject {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    /// Game versions this project supports (from search hits)
    #[serde(default)]
    #[allow(dead_code)] // Deserialized from API, may be useful for display later
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MrVersion {
    pub id: String,
    pub version_number: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
    #[allow(dead_code)] // Deserialized from API, available for future use
    pub files: Vec<MrFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Deserialized from API, available for future use
pub struct MrFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

/// Full project detail (for fetching the body/description).
#[derive(Debug, Deserialize)]
pub struct MrProjectDetail {
    pub body: String,
}

// ── Sort enum ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MrSortIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl MrSortIndex {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Downloads => "downloads",
            Self::Follows => "follows",
            Self::Newest => "newest",
            Self::Updated => "updated",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Relevance => "Relevance",
            Self::Downloads => "Downloads",
            Self::Follows => "Follows",
            Self::Newest => "Newest",
            Self::Updated => "Updated",
        }
    }

    pub const ALL: [MrSortIndex; 5] = [
        Self::Relevance,
        Self::Downloads,
        Self::Follows,
        Self::Newest,
        Self::Updated,
    ];
}

// ── Async API functions ──────────────────────────────────────────────────

const MR_BASE: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "henrypost/DrakonixAnvil/0.4.0";

fn modrinth_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
}

/// Search Modrinth for modpacks. Returns (results, total_hits).
pub async fn search_modpacks(
    query: &str,
    game_version: &str,
    loader: &str,
    sort: MrSortIndex,
    offset: u64,
) -> anyhow::Result<(Vec<MrProject>, u64)> {
    let client = modrinth_client();

    // Build facets: always filter project_type:modpack
    let mut facets: Vec<String> = vec!["[\"project_type:modpack\"]".to_string()];
    if !game_version.is_empty() {
        facets.push(format!("[\"versions:{}\"]", game_version));
    }
    if !loader.is_empty() {
        facets.push(format!("[\"categories:{}\"]", loader));
    }
    let facets_str = format!("[{}]", facets.join(","));

    let mut req = client
        .get(format!("{}/search", MR_BASE))
        .query(&[
            ("facets", facets_str.as_str()),
            ("limit", "20"),
            ("index", sort.as_api_value()),
            ("offset", &offset.to_string()),
        ]);

    if !query.is_empty() {
        req = req.query(&[("query", query)]);
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Modrinth API error {}: {}", status, body);
    }

    let data: MrSearchResponse = resp.json().await?;
    let total = data.total_hits;
    Ok((data.hits, total))
}

/// Fetch all versions for a project (by slug or id).
pub async fn get_project_versions(id_or_slug: &str) -> anyhow::Result<Vec<MrVersion>> {
    let client = modrinth_client();

    let resp = client
        .get(format!("{}/project/{}/version", MR_BASE, id_or_slug))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Modrinth API error {}: {}", status, body);
    }

    let versions: Vec<MrVersion> = resp.json().await?;
    Ok(versions)
}

/// Fetch the full project description (body field, markdown).
pub async fn get_project_description(id_or_slug: &str) -> anyhow::Result<String> {
    let client = modrinth_client();

    let resp = client
        .get(format!("{}/project/{}", MR_BASE, id_or_slug))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Modrinth API error {}: {}", status, body);
    }

    let detail: MrProjectDetail = resp.json().await?;
    Ok(detail.body)
}

// ── Helper functions ─────────────────────────────────────────────────────

/// Extract sorted unique Minecraft versions from a list of MrVersions.
/// Filters to versions starting with a digit, sorted descending (newest first).
pub fn extract_mc_versions(versions: &[MrVersion]) -> Vec<String> {
    let mut mc_versions: Vec<String> = versions
        .iter()
        .flat_map(|v| v.game_versions.iter())
        .filter(|v| v.starts_with(|c: char| c.is_ascii_digit()))
        .cloned()
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();

    mc_versions.sort_by(|a, b| {
        let parse =
            |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
        parse(b).cmp(&parse(a))
    });

    mc_versions
}

/// Detect the mod loader from a Modrinth version's loaders array.
/// Returns a lowercase string like "forge", "fabric", "neoforge".
pub fn detect_loader(loaders: &[String]) -> &str {
    // Priority: neoforge > fabric > forge > first entry
    if loaders.iter().any(|l| l.eq_ignore_ascii_case("neoforge")) {
        return "neoforge";
    }
    if loaders.iter().any(|l| l.eq_ignore_ascii_case("fabric")) {
        return "fabric";
    }
    if loaders.iter().any(|l| l.eq_ignore_ascii_case("forge")) {
        return "forge";
    }
    loaders.first().map(|s| s.as_str()).unwrap_or("forge")
}
