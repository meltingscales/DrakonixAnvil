# 🔨 DrakonixAnvil - Minecraft Server Management Made Simple

A cross-platform GUI tool for deploying, managing, and backing up multiple Minecraft servers with Docker. No command-line experience required!

## ✨ Features

- 🖱️ **Point-and-click server deployment** - No more editing YAML files
- 🔄 **Multi-instance management** - Run dozens of modpacks simultaneously
- 💾 **Automated backups** - Schedule backups with configurable retention
- 🌐 **Port forwarding wizard** - Step-by-step guide for router configuration
- 📊 **Resource monitoring** - CPU, RAM, and player count at a glance
- 🔍 **Built-in log viewer** - No more SSH or `docker logs` commands
- 🎯 **Modpack templates** - Pre-configured for popular modpacks (ATM9, SkyFactory, etc.)
- ⚡ **One-click updates** - Upgrade to new modpack versions with ease

## 🎯 Who Is This For?

- **Homelabbers** who want to consolidate their Minecraft infrastructure
- **Server hosts** managing multiple modpacks for different friend groups
- **Parents** setting up servers for their kids without touching the terminal
- **Gamers** who are tired of Ansible playbooks (we've all been there)

## 🛠️ Current Complexity This Solves

DrakonixAnvil migrates complexity from [meltingscales/VirtualMachineConfigs](https://github.com/meltingscales/VirtualMachineConfigs) by:

- Abstracting Ansible playbook variables into form fields
- Handling the differences between modpack installation types:
  - Forge installer invocation
  - Install script execution
  - Direct server JAR launch
  - Startup script wrappers
  - Directory flattening from ZIP files
- Automating port conflict detection
- Providing visual feedback for deployment progress

## 📋 Requirements

- **Docker** (or Podman)
- **4GB+ RAM** per server instance
- **Linux/macOS/Windows** (cross-platform Rust GUI)

## 🚀 Quick Start
```bash
# Download the latest release from GitHub Releases
# Extract and run
./drakonix-anvil

# Or build from source
git clone https://github.com/meltingscales/DrakonixAnvil
cd drakonix-anvil
cargo build --release
./target/release/drakonix-anvil
```

## 🎮 Supported Modpacks

Pre-configured templates for:
- ✅ All The Mods 9 (To The Sky)
- ✅ All The Forge 10
- ✅ SkyFactory 4
- ✅ Project Ozone Lite
- ✅ Regrowth
- ✅ Seaopolis Submerged
- ✅ Vanilla Minecraft

Custom modpacks supported via manual configuration!

## 🌐 Port Forwarding Guide

Built-in wizard walks you through:
1. Finding your router's IP address
2. Accessing router admin panel (common router brands)
3. Creating port forwarding rules (25565, or custom ports)
4. Testing external connectivity
5. Sharing your server with friends (dynamic DNS options)

## 🏗️ Architecture
```
DrakonixAnvil
├── GUI (Rust - egui/Tauri)
│   ├── Dashboard View
│   ├── Server Creation Wizard
│   ├── Backup Manager
│   └── Port Forwarding Guide
├── Backend (Rust)
│   ├── Docker API Integration
│   ├── Server Lifecycle Management
│   ├── Backup/Restore Engine
│   └── Template System
└── Templates
    ├── Modpack Configurations (TOML)
    └── Ansible Playbook Migrations
```

## 📸 Screenshots

*Coming soon!*

## 🗺️ Roadmap

- [ ] **v0.1**: Basic server CRUD operations
- [ ] **v0.2**: Backup/restore functionality
- [ ] **v0.3**: Port forwarding wizard
- [ ] **v0.4**: Modpack update detection
- [ ] **v0.5**: Player whitelist management
- [ ] **v0.6**: Performance metrics dashboard
- [ ] **v0.7**: Scheduled task automation
- [ ] **v1.0**: Stable release with all core features

## 🎨 Design Philosophy

**The Anvil Way:**
- **Forge complexity into simplicity** - Complex Ansible → Simple GUI
- **Temper with reliability** - Battle-tested playbooks → Stable templates
- **Craft with care** - Beginner-friendly UX without sacrificing power-user features
- **Shape with flexibility** - Docker-based, not locked to specific infrastructure

## 🤝 Contributing

We welcome contributions! Areas that need help:
- Additional modpack templates
- Router-specific port forwarding guides
- UI/UX improvements
- Documentation
- Testing on various operating systems

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 🐛 Bug Reports & Feature Requests

Please use [GitHub Issues](https://github.com/yourusername/drakonix-anvil/issues) to report bugs or request features.

## 📄 License

MIT License - See [LICENSE](LICENSE) file for details

## 🙏 Acknowledgments

- Built on years of Ansible automation wisdom from the homelab community
- Inspired by frustration with `vim server.properties` at 2 AM
- Special thanks to the maintainers of the original VirtualMachineConfigs repository
- Modpack creators and the Minecraft modding community

## 🔗 Related Projects

- [Original Ansible Playbooks](https://github.com/meltingscales/VirtualMachineConfigs)
- [Docker](https://www.docker.com/)
- [Prism Launcher](https://prismlauncher.org/) - Recommended client for playing

---

**Note:** DrakonixAnvil is a complete rewrite and migration from Ansible-based deployment to a user-friendly GUI. The original Ansible playbooks are preserved in `/templates` for reference and as the foundation for our server deployment system.

**Why "Drakonix"?** Dragons forge legends, and this anvil forges Minecraft servers. 🐉⚒️
