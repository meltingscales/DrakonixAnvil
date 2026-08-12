# DrakonixAnvil

A cross-platform GUI for deploying, managing, and backing up Minecraft servers with Docker. Built with Rust and egui.

Downloads available here: <https://github.com/meltingscales/DrakonixAnvil/releases>

## Features

- **Point-and-click server management** — create, start, stop, edit, delete servers from a dashboard
- **CurseForge modpack browser** — search and install modpacks directly from CurseForge (requires free API key)
- **Modrinth modpack browser** — search and install modpacks from Modrinth (no API key needed)
- **9 built-in templates** — Agrarian Skies 2, ATM 9: To the Sky, FTB StoneBlock 4, All The Mods 9, Project Ozone Lite, Regrowth, Seaopolis: Submerged, SkyFactory 4, Vanilla
- **Backup and restore** — zip-based backups of the full server data directory, with progress bars
- **Server export/import** — bundle a server (config + world data) into a portable `.drakonixanvil-server.zip` and import it on another machine, with progress bar
- **RCON console** — send commands to running servers from the GUI
- **Server health polling** — detects when a Minecraft server is actually ready (not just the container)
- **Container log viewer** — per-server and combined Docker log views with auto-refresh
- **Orphaned directory management** — detects leftover server folders from deleted servers, with adopt/delete options
- **Open server folder** — open any server's data directory in your file manager
- **Port conflict detection** — warns before starting if a port is already in use
- **Close confirmation** — warns when quitting with running servers
- **File logging** — timestamped logs in `DrakonixAnvilData/logs/`
- **CI/CD** — GitHub Actions builds Linux, Windows, and macOS binaries on tagged releases

## Requirements

- [Docker](https://www.docker.com/) (uses [itzg/minecraft-server](https://github.com/itzg/docker-minecraft-server))
- 4GB+ RAM per server instance

## Quick Start

```bash
# Option 1: Install from crates.io
cargo install drakonix-anvil
drakonix-anvil

# Option 2: Download a pre-built binary from GitHub Releases
# https://github.com/meltingscales/DrakonixAnvil/releases

# Option 3: Build from source
git clone https://github.com/meltingscales/DrakonixAnvil
cd DrakonixAnvil
cargo build --release
./target/release/drakonix-anvil
```

## Command Line

Running the binary with no arguments launches the GUI. Passing a subcommand runs
the CLI instead — it shares the same data (`servers.json`, settings, backups) as
the GUI, so you can mix the two.

```bash
# Create a server (config only — the container is built on first start)
drakonix-anvil create atm9 --port 25565 --memory 8192 curseforge all-the-mods-9
drakonix-anvil create sky --mc-version 1.20.1 modrinth cobblemon-fabric <version-id>

# Lifecycle
drakonix-anvil list                 # all servers + container state
drakonix-anvil status atm9
drakonix-anvil start atm9           # pulls image + creates container on first run
drakonix-anvil stop atm9
drakonix-anvil restart atm9
drakonix-anvil delete atm9 --purge  # --purge also removes data + backups

# Backups
drakonix-anvil backup atm9
drakonix-anvil backups atm9
drakonix-anvil restore atm9 20260811_120000.zip   # OVERWRITES current data
drakonix-anvil export atm9 atm9-bundle.zip
drakonix-anvil import atm9-bundle.zip

# Live server console (RCON)
drakonix-anvil rcon atm9 list
drakonix-anvil rcon atm9 say hello everyone
```

Run `drakonix-anvil <command> --help` for the full option list. Progress and
diagnostic output goes to stderr; command results go to stdout.

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

## Roadmap

- **World viewer** — browse a topographical map of your server's world from the GUI
- **Player list** — show connected players for running servers
- **Resource monitoring** — CPU/memory usage per container from Docker stats
- **Scheduled backups** — automatic backups on a timer or before server restarts
- **Modpack auto-update** — detect when a newer CurseForge/Modrinth pack version is available
- ~~Container logs auto-refresh~~ — done in v0.7.2
- ~~"Open Server Folder" button~~ — done
- ~~Server export progress bar~~ — done
- ~~**Prep for transit**~~ — done in v0.6.0
- ~~**Cargo Release**~~ — done in v0.6.1

## Releasing

Releases are triggered by pushing a `v`-prefixed tag. GitHub Actions CI/CD builds Linux, Windows, and macOS binaries automatically.

```bash
# Bump version in Cargo.toml, commit, then:
git tag v0.X.0
git push origin v0.X.0
```

## Related Projects

- [itzg/docker-minecraft-server](https://github.com/itzg/docker-minecraft-server) — the Docker image that powers every server
- [CurseForge API](https://docs.curseforge.com/)
- [Modrinth API](https://docs.modrinth.com/)
- [Crafty Controller](https://craftycontrol.com/) — web-based Minecraft server management panel
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
