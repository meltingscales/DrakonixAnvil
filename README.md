# DrakonixAnvil

A cross-platform GUI for deploying, managing, and backing up Minecraft servers with Docker. Built with Rust and egui.

Downloads available here: <https://github.com/meltingscales/DrakonixAnvil/releases>

## Features

- **Point-and-click server management** — create, start, stop, edit, delete servers from a dashboard
- **CurseForge modpack browser** — search and install modpacks directly from CurseForge (requires free API key)
- **Modrinth modpack browser** — search and install modpacks from Modrinth (no API key needed)
- **9 built-in templates** — Agrarian Skies 2, ATM 9: To the Sky, FTB StoneBlock 4, All The Mods 9, Project Ozone Lite, Regrowth, Seaopolis: Submerged, SkyFactory 4, Vanilla
- **Backup and restore** — zip-based backups of the full server data directory, with progress bars
- **RCON console** — send commands to running servers from the GUI
- **Server health polling** — detects when a Minecraft server is actually ready (not just the container)
- **Container log viewer** — per-server and combined Docker log views with auto-refresh
- **Orphaned directory management** — detects leftover server folders from deleted servers, with adopt/delete options
- **Port conflict detection** — warns before starting if a port is already in use
- **Close confirmation** — warns when quitting with running servers
- **File logging** — timestamped logs in `DrakonixAnvilData/logs/`
- **CI/CD** — GitHub Actions builds Linux, Windows, and macOS binaries on tagged releases

## Requirements

- [Docker](https://www.docker.com/) (uses [itzg/minecraft-server](https://github.com/itzg/docker-minecraft-server))
- 4GB+ RAM per server instance

## Quick Start

```bash
# Download a release from GitHub Releases, or build from source:
git clone https://github.com/meltingscales/DrakonixAnvil
cd DrakonixAnvil
cargo build --release
./target/release/drakonix-anvil
```

## Data Layout

```
./DrakonixAnvilData/
  servers.json           # All server configs
  settings.json          # Global settings (CurseForge API key)
  logs/                  # Application log files
  servers/<name>/data/   # Bind-mounted as /data in Docker container
  backups/<name>/        # Backup zip files
```

Server data directories are preserved when a server is deleted. Orphaned directories appear on the dashboard with options to adopt or delete them.

## Architecture

```
src/
  main.rs              # Entry point, logging setup
  app.rs               # App state machine, view routing, server lifecycle
  backup.rs            # Backup/restore (zip-based, async with progress)
  config.rs            # Paths, settings, orphan detection
  curseforge.rs        # CurseForge API client
  modrinth.rs          # Modrinth API client
  pack_installer.rs    # Host-side modpack download + extraction
  rcon.rs              # RCON protocol implementation
  server/mod.rs        # Data models, Docker env builder
  docker/mod.rs        # Bollard wrapper for Docker API
  templates/mod.rs     # Built-in modpack templates
  ui/
    mod.rs             # View enum
    dashboard.rs       # Server list + orphaned dirs
    server_create.rs   # Creation wizard (templates + CurseForge/Modrinth browsers)
    server_edit.rs     # Edit form (with CurseForge/Modrinth pack search)
```

## Related Projects

- [itzg/docker-minecraft-server](https://github.com/itzg/docker-minecraft-server) — the Docker image that powers every server
- [CurseForge API](https://docs.curseforge.com/)
- [Modrinth API](https://docs.modrinth.com/)
- [Prism Launcher](https://prismlauncher.org/) — recommended client for playing
- [Original Ansible playbooks](https://github.com/meltingscales/VirtualMachineConfigs/blob/master/ansible/minecraft/vanilla/minecraft_vanilla.yaml) — what inspired this project

## Research Items

These were researched during the building of this project. Useful for understanding internals.

- [itzg/docker-minecraft-server](https://github.com/itzg/docker-minecraft-server)

- https://www.curseforge.com/minecraft/mc-mods/resource-loader                                                                                       
- https://docker-minecraft-server.readthedocs.io/en/latest/types-and-platforms/mod-platforms/auto-curseforge/                                        
- https://github.com/MineYourMind/Wiki                                                                                                               
- https://legacy.curseforge.com/minecraft/modpacks/agrarian-skies-2/pages/setting-up-an-agrarian-skies-2-server                                      
- https://mediafilez.forgecdn.net/files/3016/706/Agrarian%2BSkies%2B2%2B%282.0.6%29-Server.zip                                                       

## License

MIT
