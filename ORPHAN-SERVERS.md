# Orphaned Server Directories on Dashboard

## Problem

When a server is deleted, its data directory in `DrakonixAnvilData/servers/` is intentionally preserved (the delete confirmation says so). Over time this creates "orphaned" directories not tracked by `servers.json`. Users need visibility into these and the ability to either delete them or adopt them back as servers.

## Implementation

### Files Modified

| File | Change |
|------|--------|
| `src/config.rs` | Added `find_orphaned_server_dirs()` function |
| `src/ui/dashboard.rs` | Added `orphaned_dirs` field + `on_adopt_server`/`on_delete_orphan` callbacks to `DashboardCallbacks`, orphaned section rendering |
| `src/app.rs` | Added `orphaned_dirs: Vec<String>` field, wired callbacks, added `adopt_server()`/`delete_orphan()`/`refresh_orphaned_dirs()` methods |

### `src/config.rs` — Orphan detection

`find_orphaned_server_dirs(&[ServerInstance]) -> Vec<String>`:
- Reads entries from `DrakonixAnvilData/servers/` directory
- Filters to directories only
- Excludes any whose name matches a `ServerConfig.name` in the provided servers list
- Returns sorted `Vec<String>` of orphaned directory names
- Returns empty vec on IO errors (directory might not exist yet)

### `src/ui/dashboard.rs` — Orphaned section on dashboard

Added to `DashboardCallbacks`:
- `on_adopt_server: &'a mut dyn FnMut(&str)`
- `on_delete_orphan: &'a mut dyn FnMut(&str)`
- `orphaned_dirs: &'a [String]` (moved here to avoid exceeding clippy's max argument count)

Rendered after the server list (inside the `ScrollArea`), only when non-empty:
- Yellow header: "Orphaned Server Directories" with count
- Explanatory label about what they are
- For each orphaned dir: a card row with the directory name, an "Adopt" button, and a red "Delete" button

### `src/app.rs` — State and methods

**Field:** `orphaned_dirs: Vec<String>` — cached list, refreshed on startup and after mutations.

**`refresh_orphaned_dirs()`:** Calls `find_orphaned_server_dirs(&self.servers)`, updates `self.orphaned_dirs`.

**`adopt_server(name)`:**
- Creates a minimal `ServerInstance` with Vanilla loader, `ModpackSource::Local { path: "." }`, and placeholder modpack info
- Pushes to `self.servers`, saves, refreshes orphaned list, shows status message
- Navigates to `View::EditServer(name)` so the user can configure it immediately

**`delete_orphan(name)`:**
- `fs::remove_dir_all(get_server_path(name))`
- Also removes backup dir if it exists: `DrakonixAnvilData/backups/{name}/`
- Refreshes orphaned list, shows status message

**Refresh points:**
- `DrakonixApp::new()` — after loading servers
- `delete_server()` — after removing from list
- `adopt_server()` and `delete_orphan()` — after mutations

## Testing

1. `cargo clippy` + `cargo build` — passes with `#![deny(warnings)]`
2. Manual test: delete a server -> dashboard shows it under "Orphaned Server Directories" -> Adopt brings it back -> Delete removes folder from disk
