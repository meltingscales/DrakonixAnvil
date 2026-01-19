# Plan: Implement Server Start/Stop + Define Standards

## Overview
Wire up `start_server()` and `stop_server()` to actually create and manage Docker containers, and establish filesystem/Docker standards for DrakonixAnvil.

## Standards Definition

### Filesystem Standard
```
./DrakonixAnvilData/
├── servers/
│   └── <SERVER_NAME>/
│       ├── data/           # Mounted as /data in container (world, mods, config)
│       ├── server.json     # DrakonixAnvil metadata (container_id, status, etc.)
│       └── logs/           # Server logs (bind mount or copy)
├── backups/
│   └── <SERVER_NAME>/
│       └── <TIMESTAMP>/    # Backup snapshots
└── config.json             # Global DrakonixAnvil settings
```

### Docker Standard
- **Image**: `itzg/minecraft-server` (industry standard Minecraft Docker image)
- **Container naming**: `drakonix-<server_name>`
- **Labels**:
  - `drakonix.managed=true`
  - `drakonix.server-name=<name>`
- **Volume mount**: `./DrakonixAnvilData/servers/<name>/data:/data`
- **Environment variables** (per modpack type):
  - `EULA=TRUE`
  - `TYPE=` (VANILLA, FORGE, FABRIC, NEOFORGE, CURSEFORGE, FTB)
  - `VERSION=<mc_version>`
  - `MEMORY=<memory>M`
  - For FTB: `FTB_MODPACK_ID`, `FTB_MODPACK_VERSION_ID`
  - For CurseForge: `CF_API_KEY`, `CF_PAGE_URL` or `CF_SLUG`

### FTB API Reference (from Prism Launcher)
- **Modern FTB API**: `https://api.modpacks.ch/public/modpack/<pack_id>`
- **Legacy FTB**: `https://dist.creeper.host/FTB2/static/modpacks.xml`
- Pack metadata includes: id, name, versions, files

## Implementation Changes

### 1. Add `data_dir` constant/config (`src/main.rs` or new `src/config.rs`)
- Define `DATA_ROOT = "./DrakonixAnvilData"`
- Function to get server data path: `get_server_path(name) -> PathBuf`

### 2. Update `DockerManager` (`src/docker/mod.rs`)
Add data directory parameter to `create_minecraft_container`:
```rust
pub async fn create_minecraft_container(
    &self,
    name: &str,
    server_config: &ServerConfig,
    data_path: &Path,
) -> Result<String>
```
- Add volume bind mount: `data_path:/data`
- Build env vars from `ServerConfig` and `ModpackSource`

### 3. Wire up `start_server()` (`src/app.rs`)
```rust
fn start_server(&mut self, name: &str) {
    // 1. Find server by name
    // 2. Create data directory if needed
    // 3. If no container_id: create container first
    // 4. Call docker.start_container(id)
    // 5. Update status to Running (on success) or Error (on failure)
}
```

### 4. Wire up `stop_server()` (`src/app.rs`)
```rust
fn stop_server(&mut self, name: &str) {
    // 1. Find server by name
    // 2. Get container_id (return if None)
    // 3. Call docker.stop_container(id)
    // 4. Update status to Stopped (on success) or Error (on failure)
}
```

### 5. Add helper to build env vars (`src/docker/mod.rs` or `src/server/mod.rs`)
```rust
fn build_docker_env(config: &ServerConfig) -> Vec<String> {
    // EULA=TRUE
    // TYPE= based on ModLoader + ModpackSource
    // VERSION= from config
    // MEMORY= from config
    // FTB_MODPACK_ID / CF_* based on source
}
```

### 6. Async handling pattern
The egui update loop is synchronous. Options:
- **Option A**: Use `runtime.block_on()` in callbacks (simple, blocks UI briefly)
- **Option B**: Use channels to communicate between UI and async tasks (better UX)

Recommend **Option A** for now (simpler), with TODO for Option B later.

## Files to Modify
1. `src/app.rs` - Wire up start/stop, add data path logic
2. `src/docker/mod.rs` - Update `create_minecraft_container` signature, add volume mount
3. `src/server/mod.rs` - Add `build_docker_env()` helper

## Files to Create
1. `src/config.rs` (optional) - Constants for paths, or just add to `main.rs`

## Verification
1. Run `cargo build` - should compile
2. Run the app, create a server
3. Click "Start" - verify:
   - Directory created at `./DrakonixAnvilData/servers/<name>/data/`
   - Docker container created with correct name, env vars, volume
   - Container starts running
4. Click "Stop" - verify container stops
5. Check `docker ps -a --filter label=drakonix.managed=true` to see managed containers
