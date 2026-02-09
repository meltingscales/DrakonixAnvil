# Plan: Expose Server Properties in Edit View + Docker Env

## Context

`ServerProperties` (motd, max_players, difficulty, gamemode, pvp, online_mode, white_list) is defined in the data model and persisted in `servers.json`, but:
1. The edit UI only shows port, memory, and java_args — users must hand-edit JSON for server properties
2. `build_docker_env()` doesn't generate env vars for server properties — so even if set in JSON, they have no effect

This change adds UI controls for all server properties and wires them into the Docker environment.

## Files to Modify

| File | Change |
|------|--------|
| `src/server/mod.rs` | Add server property env vars to `build_docker_env()`, remove `#[allow(dead_code)]` from `to_properties_string` |
| `src/ui/server_edit.rs` | Add fields + UI controls for all 7 server properties |
| `src/app.rs` | Expand `save_server_edit()` and its call chain to include `ServerProperties` |

## Step 1: `src/server/mod.rs` — Add env vars to `build_docker_env()`

Add after the existing RCON block (before `extra_env`):

```
MOTD={motd}              (if non-empty)
DIFFICULTY={difficulty}  (peaceful/easy/normal/hard)
MODE={gamemode}          (survival/creative/adventure/spectator — itzg uses MODE not GAMEMODE)
MAX_PLAYERS={max_players}
PVP={pvp}
ONLINE_MODE={online_mode}
ENABLE_WHITELIST={white_list}
```

Note: itzg/minecraft-server uses `MODE` not `GAMEMODE`, and `ENABLE_WHITELIST` not `WHITE_LIST`.

Remove `#[allow(dead_code)]` from `to_properties_string()` — it's no longer dead since the difficulty/gamemode string conversions will be reused (or just inline the conversions in `build_docker_env`; decide during implementation which is cleaner).

## Step 2: `src/ui/server_edit.rs` — Add server property fields

**Add fields to `ServerEditView`:**
- `motd: String`
- `max_players: String` (string for text input, parsed to u32)
- `difficulty: Difficulty`
- `gamemode: GameMode`
- `pvp: bool`
- `online_mode: bool`
- `white_list: bool`

**Update `load_from_config()`:** populate from `config.server_properties`.

**Update `show()`:** after the Java Options section, add a "Server Properties" collapsible section (`CollapsingHeader`) containing:
- MOTD: text input
- Max Players: text input (validated as u32)
- Difficulty: `ComboBox` with Peaceful/Easy/Normal/Hard
- Game Mode: `ComboBox` with Survival/Creative/Adventure/Spectator
- PVP: checkbox
- Online Mode: checkbox
- Whitelist: checkbox

All changes set `dirty = true`.

**Update `on_save` callback signature:** add `ServerProperties` parameter. Build from the edit view fields when Save is clicked.

**Update `reset()`:** reset server property fields to defaults.

## Step 3: `src/app.rs` — Expand save chain

**Update `save_server_edit()`:**
- Add `server_properties: ServerProperties` parameter
- Save it to `server.config.server_properties`
- Include it in the "changed" check that clears `container_id`

**Update `View::EditServer` match arm:**
- The `on_save` closure now captures the `ServerProperties` too
- Pass it through to `save_server_edit()`

## Step 4: Remove "edit servers.json" hint

The footer in `server_edit.rs` (lines ~114-141) tells users to edit `servers.json` for server properties. Replace/update this now that the properties are editable in the UI. Keep a note about other advanced options (extra_env, modpack source) still requiring JSON editing.

## Verification

1. `cargo clippy` + `cargo build` — must pass with `#![deny(warnings)]`
2. Manual test: edit a server → change MOTD/difficulty/gamemode → save → start → verify Docker container has the correct env vars (check with `docker inspect`)
