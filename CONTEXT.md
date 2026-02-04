# Development Context

Last updated: 2026-02-04

## Recent Session Summary

### What We Built (This Session)

1. **MC Server Health Polling** (`rust-mc-status` integration)
   - Polls server using actual Minecraft protocol (not just TCP)
   - New `Initializing` status: container running, MC server not ready yet
   - Logs rich info when ready: version, MOTD, players, mods count, software
   - Detects container crashes during polling and reports errors
   - 10 minute timeout for slow modpacks

2. **CurseForge API Key Settings**
   - Global `AppSettings` struct with `curseforge_api_key`
   - Settings UI in new Settings tab
   - Password-masked input field with status indicator
   - Auto-passed to Docker containers as `CF_API_KEY`

3. **Docker Logs View**
   - New "Docker Logs" tab in navigation
   - Shows combined logs from ALL managed containers
   - Header per container with name and state
   - Refresh button to reload

### Files Modified (This Session)
- `Cargo.toml` - Added `rust-mc-status = "2.0"`
- `src/app.rs` - Health polling, settings, Docker Logs view
- `src/config.rs` - Added `AppSettings`, load/save settings
- `src/docker/mod.rs` - `is_container_running()`, `get_all_managed_logs()`
- `src/server/mod.rs` - Added `ServerStatus::Initializing`
- `src/ui/mod.rs` - Added `View::DockerLogs`
- `src/ui/dashboard.rs` - Handle Initializing status

### Current State
- Basic CRUD complete (Create, Read, Update, Delete)
- Server health verification via MC protocol
- Container logs viewing (per-server and all-containers)
- Global settings with CurseForge API key
- 4 modpack templates (Agrarian Skies 2, FTB StoneBlock 4, ATM9, Vanilla)

## Data Storage

```
./DrakonixAnvilData/
├── servers.json          # Server configs (name, port, modpack, etc.)
├── settings.json         # Global settings (CF API key)
└── servers/
    └── <server-name>/
        └── data/         # Bind-mounted to /data in container
            ├── world/
            ├── mods/
            ├── server.properties
            └── ...
```

- **Bind mounts** (not Docker volumes) - data persists on host
- Stopping container preserves data
- Deleting server preserves data folder (only removes container)

## Next Up (Suggested Priority)

### High Priority
1. **Backup/restore** (v0.2 roadmap)
   - Zip `DrakonixAnvilData/servers/<name>/data/`
   - Store in `DrakonixAnvilData/backups/<name>/<timestamp>.zip`
   - Add Backup button and Restore dropdown

2. **Port conflict detection**
   - Check if port already in use before starting
   - Warn user and suggest available port

3. **RCON console**
   - Send commands to running server
   - Requires RCON password setup in container env

### Medium Priority
- Memory editing in edit view (currently only at creation)
- More templates (SkyFactory 4, Project Ozone, etc.)
- Delete server data option (separate from container delete)
- Show disk usage per server on dashboard

### Lower Priority
- Port check wizard (external service to test if port reachable)
- Google Drive backup integration
- Custom data root path in Settings

## Architecture Notes

```
DrakonixAnvil
├── src/
│   ├── main.rs          - Entry point, window setup
│   ├── app.rs           - Main app state, view routing, server lifecycle
│   ├── config.rs        - Paths, Docker constants, AppSettings
│   ├── server/mod.rs    - Data models, Docker env builder
│   ├── docker/mod.rs    - Bollard wrapper for Docker API
│   ├── templates/mod.rs - Modpack templates
│   └── ui/
│       ├── mod.rs           - View enum
│       ├── dashboard.rs     - Server list
│       ├── server_create.rs - Creation wizard
│       └── server_edit.rs   - Edit form
└── DrakonixAnvilData/
    ├── servers.json     - All server configs
    ├── settings.json    - Global app settings
    └── servers/<name>/data/  - Container volume mounts
```

## Key Patterns

- **Async via channels**: Background tasks send `TaskMessage` to UI thread
- **View enum**: `View::Dashboard`, `View::DockerLogs`, `View::Settings`, etc.
- **Callbacks**: Dashboard uses `FnMut` callbacks for actions
- **Docker**: itzg/minecraft-server image, Bollard client
- **Health polling**: `rust-mc-status` queries MC protocol after container starts
