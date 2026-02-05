╭─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Plan to implement                                                                                                                                                   │
│                                                                                                                                                                     │
│ Plan: Server Name Conflict Resolution                                                                                                                               │
│                                                                                                                                                                     │
│ Problem                                                                                                                                                             │
│                                                                                                                                                                     │
│ When creating a server, create_server() blindly pushes with no conflict checks. Three types of leftovers can exist:                                                 │
│ 1. Server entry in self.servers / servers.json                                                                                                                      │
│ 2. Docker container named drakonix-<name> (orphaned from unclean state)                                                                                             │
│ 3. Data directory at ./DrakonixAnvilData/servers/<name>/data/                                                                                                       │
│                                                                                                                                                                     │
│ Solution                                                                                                                                                            │
│                                                                                                                                                                     │
│ Intercept the creation flow to detect conflicts, show a dialog, and offer three resolutions: Rename, Replace, Reuse.                                                │
│                                                                                                                                                                     │
│ Files to Modify                                                                                                                                                     │
│                                                                                                                                                                     │
│ 1. src/ui/mod.rs — Add View variant                                                                                                                                 │
│                                                                                                                                                                     │
│ - Add ConfirmNameConflict (no payload — conflict data lives on DrakonixApp)                                                                                         │
│                                                                                                                                                                     │
│ 2. src/docker/mod.rs — Add container lookup by name                                                                                                                 │
│                                                                                                                                                                     │
│ - Add container_exists_by_name(&self, name: &str) -> Result<Option<String>>                                                                                         │
│ - Uses bollard's inspect_container() which accepts name or ID                                                                                                       │
│ - Returns Ok(Some(container_id)) if found, Ok(None) on 404, Err on other errors                                                                                     │
│                                                                                                                                                                     │
│ 3. src/app.rs — Core changes                                                                                                                                        │
│                                                                                                                                                                     │
│ New struct (private to module):                                                                                                                                     │
│ #[derive(Debug, Clone)]                                                                                                                                             │
│ struct NameConflict {                                                                                                                                               │
│     name: String,                                                                                                                                                   │
│     template: ModpackTemplate,                                                                                                                                      │
│     port: u16,                                                                                                                                                      │
│     memory_mb: u64,                                                                                                                                                 │
│     server_entry_exists: bool,                                                                                                                                      │
│     container_exists: bool,                                                                                                                                         │
│     container_id: Option<String>,                                                                                                                                   │
│     data_dir_exists: bool,                                                                                                                                          │
│ }                                                                                                                                                                   │
│                                                                                                                                                                     │
│ New field on DrakonixApp:                                                                                                                                           │
│ - pending_conflict: Option<NameConflict> (initialized to None)                                                                                                      │
│                                                                                                                                                                     │
│ New methods:                                                                                                                                                        │
│ - check_name_conflicts(name, template, port, memory) -> NameConflict — checks all three conflict types                                                              │
│ - resolve_conflict_replace() — removes server entry + stops/removes container + deletes data dir, then calls create_server()                                        │
│ - resolve_conflict_reuse() — removes server entry + stops/removes container (keeps data dir), then calls create_server()                                            │
│                                                                                                                                                                     │
│ Modified logic in View::CreateServer match arm:                                                                                                                     │
│ - After user clicks Create, call check_name_conflicts()                                                                                                             │
│ - If any conflict found → set pending_conflict, switch to View::ConfirmNameConflict                                                                                 │
│ - If no conflicts → call create_server() as before                                                                                                                  │
│                                                                                                                                                                     │
│ New match arm for View::ConfirmNameConflict:                                                                                                                        │
│ - Warning-styled dialog listing which conflicts were found                                                                                                          │
│ - Four buttons:                                                                                                                                                     │
│   - Rename — back to View::CreateServer (form preserved, user changes name)                                                                                         │
│   - Replace — wipe everything old, create fresh                                                                                                                     │
│   - Reuse — keep data directory, remove old entry/container, create new entry                                                                                       │
│   - Cancel — back to Dashboard, reset form                                                                                                                          │
│                                                                                                                                                                     │
│ Resolution Behavior                                                                                                                                                 │
│ ┌─────────┬──────────────┬───────────────────┬────────────────┐                                                                                                     │
│ │ Action  │ Server Entry │ Docker Container  │ Data Directory │                                                                                                     │
│ ├─────────┼──────────────┼───────────────────┼────────────────┤                                                                                                     │
│ │ Rename  │ untouched    │ untouched         │ untouched      │                                                                                                     │
│ ├─────────┼──────────────┼───────────────────┼────────────────┤                                                                                                     │
│ │ Replace │ removed      │ stopped + removed │ deleted        │                                                                                                     │
│ ├─────────┼──────────────┼───────────────────┼────────────────┤                                                                                                     │
│ │ Reuse   │ removed      │ stopped + removed │ kept           │                                                                                                     │
│ ├─────────┼──────────────┼───────────────────┼────────────────┤                                                                                                     │
│ │ Cancel  │ untouched    │ untouched         │ untouched      │                                                                                                     │
│ └─────────┴──────────────┴───────────────────┴────────────────┘                                                                                                     │
│ Both Replace and Reuse remove the old container because the new config may have different port/memory settings. The container gets recreated on next Start.         │
│                                                                                                                                                                     │
│ Edge Cases                                                                                                                                                          │
│                                                                                                                                                                     │
│ - Docker not connected: skip container check, container_exists: false                                                                                               │
│ - Rename: pending_conflict cleared, view set to CreateServer without resetting form                                                                                 │
│ - Only data dir exists (no entry, no container): Reuse and Replace both work, Reuse just skips the delete                                                           │
│                                                                                                                                                                     │
│ Verification                                                                                                                                                        │
│                                                                                                                                                                     │
│ 1. Create server "test1", then try creating "test1" again → conflict dialog shows server entry                                                                      │
│ 2. Delete "test1", try creating "test1" again → conflict dialog shows data directory                                                                                │
│ 3. Test Replace → data dir gone, fresh server                                                                                                                       │
│ 4. Test Reuse → data dir preserved, new server entry                                                                                                                │
│ 5. Test Rename → back to form with name editable                                                                                                                    │
│ 6. cargo check / cargo clippy pass                                                                                                                                  │
╰─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
