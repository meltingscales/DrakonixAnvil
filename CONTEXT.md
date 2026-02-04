# Development Context

Last updated: 2026-02-03

## Recent Session Summary

### What We Built

1. **Server Edit View** (`src/ui/server_edit.rs`)
   - Edit server port and Java options after creation
   - Multiline text editor for JVM args (one per line)
   - Help tooltip encouraging users to edit `servers.json` for advanced options
   - Changes trigger container recreation on next start

2. **Delete Server**
   - Red "Delete" button on stopped servers
   - Confirmation dialog before deletion
   - Removes Docker container, preserves data folder

3. **Container Logs Viewer**
   - "Logs" button on all server cards (any state)
   - Shows last 500 lines from Docker container
   - Refresh button to reload

4. **Agrarian Skies 2 Template**
   - Added to templates (MC 1.7.10, Forge, Java 8)
   - FTB pack ID 17
   - Optimized G1GC Java args for older modpacks

5. **JVM_OPTS Passthrough**
   - `java_args` from config now passed to Docker via `JVM_OPTS` env var

### Files Modified
- `src/app.rs` - Main app logic, new views
- `src/docker/mod.rs` - Added `get_container_logs()`
- `src/server/mod.rs` - Added JVM_OPTS to Docker env
- `src/templates/mod.rs` - Added Agrarian Skies 2
- `src/ui/mod.rs` - New view variants
- `src/ui/dashboard.rs` - Edit, Delete, Logs buttons
- `src/ui/server_edit.rs` - New file

### Current State
- Basic CRUD complete (Create, Read, Update, Delete)
- Container logs viewing works
- 4 modpack templates (Agrarian Skies 2, FTB StoneBlock 4, ATM9, Vanilla)

## Next Up (Suggested Priority)

### High Priority
1. **Backup/restore** (v0.2 roadmap)
   - Zip `DrakonixAnvilData/servers/<name>/data/`
   - Store in `DrakonixAnvilData/backups/<name>/<timestamp>.zip`
   - Add Backup button and Restore option

2. **RCON console**
   - Send commands to running server
   - From PROMPT.md: "hook into stdin for java process"

3. **Port conflict detection**
   - Check if port already in use before starting

### Medium Priority
- Memory editing in edit view (currently only at creation)
- More templates (SkyFactory 4, Project Ozone, etc.)
- Port check wizard (external service to test if port reachable)

### From PROMPT.md Ideas
- Google Drive backup to `~/DrakonixAnvilMinecraftBackup/`
- Remote `nc` check for port forwarding verification

## Architecture Notes

```
DrakonixAnvil
├── src/
│   ├── main.rs          - Entry point, window setup
│   ├── app.rs           - Main app state, view routing, server lifecycle
│   ├── config.rs        - Paths, Docker constants
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
    └── servers/<name>/data/  - Container volume mounts
```

## Key Patterns

- **Async via channels**: Background tasks send `TaskMessage` to UI thread
- **View enum**: `View::Dashboard`, `View::EditServer(name)`, etc.
- **Callbacks**: Dashboard uses `FnMut` callbacks for actions
- **Docker**: itzg/minecraft-server image, Bollard client
