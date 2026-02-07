# Plan: CurseForge Modpack Browser

## Context

Users currently pick from 9 hardcoded `ModpackTemplate` structs when creating a server. This means every new modpack requires a code change, hardcoded ZIP URLs go stale, and version mismatches are common. We'll add a CurseForge search/browse UI directly into the server creation flow, replacing the dropdown with a richer "Featured" + "Search CurseForge" tabbed experience. The itzg/minecraft-server Docker image already supports `TYPE=AUTO_CURSEFORGE` + `CF_SLUG` + `CF_FILE_ID`, and the existing `ModpackSource::CurseForge` variant already generates those env vars — so we just need the browse UI and API client.

## Files to Create/Modify

| File | Action | Summary |
|------|--------|---------|
| `src/curseforge.rs` | **Create** | CurseForge API client: data types, search/files functions, helpers |
| `src/main.rs` | Modify | Add `mod curseforge;` |
| `src/ui/server_create.rs` | **Rewrite** | Tabbed UI: Featured templates + CurseForge search/results/version picker |
| `src/app.rs` | Modify | New `TaskMessage` variants, async spawn for search/versions, rework `View::CreateServer` arm |
| `README.md` | Modify | Add CurseForge API and Modrinth API URLs to Related Projects |

No new Cargo dependencies needed (`reqwest`, `serde`, `tokio`, `anyhow` already present).

---

## Step 1: Create `src/curseforge.rs`

### API response types (serde `Deserialize`)

- `CfSearchResponse { data: Vec<CfMod>, pagination: CfPagination }`
- `CfPagination { total_count: u64 }`
- `CfMod { id: u64, name, slug, summary, download_count: u64, logo: Option<CfLogo>, latest_files_indexes: Vec<CfLatestFileIndex> }`
- `CfLogo { thumbnail_url: String }` (captured for future icon support)
- `CfLatestFileIndex { game_version: String, mod_loader: Option<u32> }`
- `CfFilesResponse { data: Vec<CfFile> }`
- `CfFile { id: u64, display_name, file_name, game_versions: Vec<String>, file_date: String, server_pack_file_id: Option<u64> }`

All use `#[serde(rename = "camelCase")]` for CurseForge's JSON field names.

### Search parameters

- `CfSortField` enum: `Popularity(2)`, `LastUpdated(3)`, `Name(5)`, `TotalDownloads(6)` with `as_api_value()` and `label()` methods, plus `ALL` const array
- `mod_loader_api_value(loader: &ModLoader) -> Option<u32>` — Forge=1, Fabric=4, NeoForge=6, Vanilla=None

### Async functions

```
search_modpacks(api_key, query, game_version, mod_loader, sort_field, page_offset)
  -> Result<(Vec<CfMod>, u64)>
```
- `GET https://api.curseforge.com/v1/mods/search`
- `x-api-key` header, `gameId=432`, `classId=4471` (modpacks), `pageSize=20`

```
get_mod_files(api_key, mod_id) -> Result<Vec<CfFile>>
```
- `GET https://api.curseforge.com/v1/mods/{modId}/files`
- `pageSize=50`

### Helper functions

- `infer_java_version(mc_version: &str) -> u8` — 1.0-1.16→8, 1.17-1.20.4→17, 1.20.5+→21
- `infer_mod_loader(cf_loader: Option<u32>) -> ModLoader`
- `format_downloads(count: u64) -> String` — 1234567→"1.2M", 1234→"1.2K"
- `default_java_args() -> Vec<String>` — G1GC defaults
- `default_memory_mb(mc_version: &str) -> u64` — modern packs (1.16+) get 6144, older get 4096

---

## Step 2: Add `mod curseforge;` to `src/main.rs`

One line after existing `mod` declarations.

---

## Step 3: Rewrite `src/ui/server_create.rs`

### New types

- `CreateTab` enum: `Featured` (default), `SearchCurseForge`
- `CfSearchState { query, mc_version_filter, loader_filter_idx, sort_field, page_offset }`
- `CfBrowseState { search, results, total_count, loading_search, search_error, selected_mod, versions, loading_versions, versions_error, selected_version_idx }`

### Reworked `ServerCreateView`

```rust
pub struct ServerCreateView {
    // Common
    pub server_name: String,
    pub port: String,
    pub memory_mb: String,
    // Tab
    pub active_tab: CreateTab,
    // Featured
    pub selected_template_idx: usize,
    // CurseForge
    pub cf: CfBrowseState,
    pub cf_template: Option<ModpackTemplate>,
}
```

### Callback struct (follows `DashboardCallbacks` pattern)

```rust
pub struct CreateViewCallbacks<'a> {
    pub on_create: &'a mut dyn FnMut(String, ModpackTemplate, u16, u64),
    pub on_cancel: &'a mut dyn FnMut(),
    pub on_cf_search: &'a mut dyn FnMut(&CfSearchState),
    pub on_cf_fetch_versions: &'a mut dyn FnMut(u64),
    pub has_cf_api_key: bool,
}
```

### UI layout in `show()`

```
Create New Server
─────────────────
Server Name: [____________]   Port: [25565]   Memory (MB): [4096]
─────────────────
[Featured]  [Search CurseForge]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Featured tab:
  Scrollable list of builtin templates as cards
  (name, description, MC version, loader, Java, RAM)
  Click to select → updates memory field

Search CurseForge tab:
  [Search: ___________] [Search]
  MC Version: [____]  Loader: [Any ▾]  Sort: [Popularity ▾]
  ─────────────────
  (Scrollable results list — name, summary, downloads, MC versions)
  Click result → fetches versions
  ─────────────────
  Versions for: <pack name>
  (Selectable list — display_name, game_versions, date)
  Click version → builds ModpackTemplate, updates memory
  ─────────────────
  [< Prev]  Page 1 / 5  [Next >]

  If no API key: "Set your CurseForge API key in Settings" message

Bottom:
  Selected: <pack name> (MC 1.20.1, Forge, Java 17)
  [Cancel]  [Create Server]
```

### Key method: `build_cf_template(&mut self, cf_mod, cf_file)`

Constructs a `ModpackTemplate` from CurseForge API data:
- MC version: first entry in `game_versions` that starts with a digit
- Loader: detect from `game_versions` strings ("NeoForge", "Fabric", default Forge)
- `file_id`: prefer `server_pack_file_id`, fall back to `cf_file.id`
- Java/memory: inferred via helper functions
- Source: `ModpackSource::CurseForge { slug, file_id }`

---

## Step 4: Modify `src/app.rs`

### New `TaskMessage` variants

```rust
CfSearchResults { results: Vec<CfMod>, total_count: u64 },
CfSearchError(String),
CfVersionResults { mod_id: u64, files: Vec<CfFile> },
CfVersionError { mod_id: u64, error: String },
```

### Handle in `process_task_messages()`

- `CfSearchResults` → set `create_view.cf.results`, clear loading
- `CfSearchError` → set error, clear loading
- `CfVersionResults` → check `mod_id` matches selected mod, set versions
- `CfVersionError` → check `mod_id`, set error

### Rework `View::CreateServer` arm (~line 1381)

Replace the current 4-arg closure calls with `CreateViewCallbacks`. Add two new action captures (`search_request`, `version_request`) and spawn async tasks after the `show()` call:

```rust
// Fire async CF search
if let Some(search_state) = search_request {
    let api_key = self.settings.curseforge_api_key.clone().unwrap_or_default();
    let tx = self.task_tx.clone();
    self.runtime.spawn(async move {
        match curseforge::search_modpacks(&api_key, ...).await {
            Ok((results, total)) => tx.send(CfSearchResults { ... }).ok(),
            Err(e) => tx.send(CfSearchError(e.to_string())).ok(),
        }
    });
}

// Fire async version fetch
if let Some(mod_id) = version_request {
    // similar pattern
}
```

---

## Step 5: Update `README.md`

Add to the Related Projects section:
- [CurseForge API Documentation](https://docs.curseforge.com/)
- [Modrinth API Documentation](https://docs.modrinth.com/)

---

## Step 6: Verify

1. `cargo clippy` — must pass (`#![deny(warnings)]`)
2. `cargo build` — must compile
3. Manual testing:
   - Open app → Create Server → Featured tab shows all 9 builtin templates as cards
   - Switch to Search CurseForge tab without API key → shows setup message
   - Set API key in Settings → Search tab works
   - Search "All The Mods" → results appear with names, summaries, download counts
   - Click a result → versions load
   - Pick a version → "Selected: ..." appears with correct MC version, loader, Java
   - Fill name/port → Create Server → server created with `ModpackSource::CurseForge`
   - Start server → Docker container gets `TYPE=AUTO_CURSEFORGE`, `CF_SLUG=...`, `CF_FILE_ID=...`

## Edge Cases Handled

- No API key → clear message directing to Settings
- API errors (401, 500, network) → red error text in UI
- 0 results → "No results found" message
- User switches mods while versions loading → `mod_id` check prevents stale data
- `server_pack_file_id` missing → falls back to main file ID
- MC version not parseable → defaults to Java 21, 6144MB
- Rapid re-searches → last result wins (acceptable for MVP)
