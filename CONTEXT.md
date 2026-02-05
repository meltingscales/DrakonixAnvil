# Development Context

Last updated: 2026-02-05

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

4. **Backup & Restore**
   - New `src/backup.rs` module
   - "Backup" button on stopped servers - zips entire `data/` folder
   - "Backups" button - opens backup list view
   - Backups view shows all backups with filename, size, age
   - Restore with confirmation dialog (warns about overwrite)
   - Delete backup option
   - Covers everything: world, mods, configs, scripts, etc.

5. **Port Conflict Detection**
   - Checks before starting a server
   - Detects if another DrakonixAnvil server is using the port
   - Detects if any system process is binding to the port
   - Suggests next available port in error message
   - Handles permission denied for privileged ports (<1024)

6. **RCON Console**
   - `mcrcon` crate for RCON protocol
   - Auto-generated memorable 4-word passwords (Minecraft-themed)
   - RCON port = game port + 10 (to avoid conflicts)
   - "Console" button on running servers
   - Command input with Enter key support
   - Scrollable output history
   - Shows RCON port/password for external client use
   - Fixed: Docker `exposed_ports` required for port binding to work

7. **Backup Progress Bar**
   - Backups now run in background thread (UI doesn't freeze)
   - Progress bar shows file count (current/total) on dashboard
   - Counts files before zipping for accurate progress
   - Uses existing TaskMessage channel pattern

8. **Detailed Delete Confirmations**
   - Server delete: Shows container name, modpack, port, container status
   - Backup delete: Now requires confirmation (was immediate before)
   - Both show highlighted resource indicator box with details
   - Clarifies what will/won't be deleted (server data preserved)

9. **File Logging**
   - Logs to both stdout and timestamped file
   - Log files: `DrakonixAnvilData/logs/drakonixanvil_YYYYMMDD_HHMMSS.log`
   - Header shows version and GitHub issues link for bug reports
   - Uses `tracing-appender` for non-blocking file writes

10. **Docker Logs Auto-Refresh**
    - Auto-refreshes every 5 seconds when viewing Docker logs
    - Async fetching (doesn't freeze UI)
    - Shows "(auto-refresh: 5s)" indicator

11. **Backup Permission Fix**
    - Fixed: Directories were stored with 0o644 (missing execute bit)
    - Now uses 0o755 for directories, 0o644 for files
    - Restore also forces execute bit on directories (fixes old backups)

12. **Restore Progress Bar**
    - Restore now runs in background thread (UI doesn't freeze)
    - Progress bar shows "Restoring X/Y" on dashboard
    - Buttons hidden during restore to prevent conflicts

### Files Modified (This Session)
- `Cargo.toml` - Added `rust-mc-status`, `zip`, `walkdir`, `mcrcon`, `rand`, `tracing-appender`
- `src/main.rs` - Added backup module, file logging setup
- `src/backup.rs` - New file: backup/restore logic
- `src/app.rs` - Health polling, settings, Docker Logs, backup views, RCON console
- `src/config.rs` - Added `AppSettings`, load/save settings
- `src/docker/mod.rs` - `is_container_running()`, `get_all_managed_logs()`, RCON port
- `src/server/mod.rs` - `ServerStatus::Initializing`, `rcon_password`, `rcon_port()`
- `src/ui/mod.rs` - Added `View::DockerLogs`, `View::Backups`, `View::Console`, `View::ConfirmDeleteBackup`, etc.
- `src/ui/dashboard.rs` - Backup/Backups/Console buttons

### Current State
- Full CRUD with detailed delete confirmations
- Server health verification via MC protocol
- Container logs viewing (per-server and all-containers, auto-refresh)
- Global settings with CurseForge API key
- Backup & restore with progress bars (non-blocking)
- Port conflict detection with suggested alternatives
- RCON console for sending commands to running servers
- File logging with timestamped log files
- 4 modpack templates (Agrarian Skies 2, FTB StoneBlock 4, ATM9, Vanilla)

## Data Storage

```
./DrakonixAnvilData/
├── servers.json          # Server configs (name, port, rcon_password, etc.)
├── settings.json         # Global settings (CF API key)
├── logs/                 # Application logs
│   └── drakonixanvil_20260205_100037.log
├── servers/
│   └── <server-name>/
│       └── data/         # Bind-mounted to /data in container
│           ├── world/
│           ├── mods/
│           ├── config/
│           ├── server.properties
│           └── ...
└── backups/
    └── <server-name>/
        ├── 20260204_130512.zip
        └── 20260204_141023.zip
```

- **Bind mounts** (not Docker volumes) - data persists on host
- Stopping container preserves data
- Deleting server preserves data folder (only removes container)
- Backups include entire data/ folder (world, mods, configs, scripts, etc.)

## Next Up (Suggested Priority)

### High Priority
1. **CI/CD automated releases**
   - GitHub Actions for Win/Mac/Linux binaries
   - Trigger on version tags (v1.0, v2.0, etc.)

### Medium Priority
- Memory editing in edit view (currently only at creation)
- More templates (SkyFactory 4, Project Ozone, etc.)
- Delete server data option (separate from container delete)
- Show disk usage per server on dashboard
- Scheduled/automatic backups

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
│   ├── backup.rs        - Backup/restore operations
│   ├── config.rs        - Paths, Docker constants, AppSettings
│   ├── server/mod.rs    - Data models, Docker env builder, RCON config
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
    ├── servers/<name>/data/  - Container volume mounts
    └── backups/<name>/       - Backup zips
```

## Key Patterns

- **Async via channels**: Background tasks send `TaskMessage` to UI thread
- **View enum**: `View::Dashboard`, `View::Console`, `View::Backups`, etc.
- **Callbacks**: Dashboard uses `FnMut` callbacks for actions
- **Docker**: itzg/minecraft-server image, Bollard client
- **Health polling**: `rust-mc-status` queries MC protocol after container starts
- **Backups**: Deflate-compressed zips of entire data/ directory, runs in background thread with progress reporting
- **RCON**: `mcrcon` crate, memorable passwords, port = game_port + 10

## Technical: Docker Bind Mounts

Bind mounts are **not symlinks** and don't use OverlayFS. They use the kernel's mount namespace feature:

1. **Mount namespaces**: Linux namespaces isolate what mounts a process can see
2. **Bind mount syscall**: `mount --bind /host/path /container/path` makes the same inode accessible at two paths
3. **VFS layer**: The kernel's Virtual File System redirects file operations - when container writes to `/data/world/level.dat`, the VFS routes it directly to the host's `DrakonixAnvilData/servers/foo/data/world/level.dat`

OverlayFS is used for the **container's root filesystem** (layered images), but bind mounts bypass it entirely - they're a direct passthrough to host storage. This is why:
- Writes are instant (no copy-on-write)
- Files are real files on host (not in a Docker volume)
- Backups just read normal files from the host filesystem
