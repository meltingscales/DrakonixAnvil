use crate::server::{ModLoader, ModpackSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackTemplate {
    pub name: String,
    pub description: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub source: ModpackSource,
    pub recommended_memory_mb: u64,
    pub java_version: u8,
    pub default_java_args: Vec<String>,
    /// Extra Docker env vars for pack-specific needs (e.g. CF_EXCLUDE_MODS for client-only mods)
    pub default_extra_env: Vec<String>,
}

impl ModpackTemplate {
    pub fn ftb_stoneblock_4() -> Self {
        Self {
            name: "FTB StoneBlock 4".to_string(),
            description: "A skyblock-style modpack where you start in a world of stone".to_string(),
            version: "1.0.0".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: ModLoader::NeoForge,
            source: ModpackSource::Ftb {
                pack_id: 0, // TODO: Get actual FTB pack ID
                version_id: 0,
            },
            recommended_memory_mb: 6144,
            java_version: 17,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=30".to_string(),
                "-XX:G1MaxNewSizePercent=40".to_string(),
                "-XX:G1HeapRegionSize=8M".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:G1HeapWastePercent=5".to_string(),
                "-XX:G1MixedGCCountTarget=4".to_string(),
                "-XX:InitiatingHeapOccupancyPercent=15".to_string(),
                "-XX:G1MixedGCLiveThresholdPercent=90".to_string(),
                "-XX:G1RSetUpdatingPauseTimePercent=5".to_string(),
                "-XX:SurvivorRatio=32".to_string(),
                "-XX:+PerfDisableSharedMem".to_string(),
                "-XX:MaxTenuringThreshold=1".to_string(),
            ],
            default_extra_env: vec![],
        }
    }

    pub fn all_the_mods_9() -> Self {
        Self {
            name: "All The Mods 9".to_string(),
            description: "A massive kitchen-sink modpack".to_string(),
            version: "0.2.0".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: ModLoader::NeoForge,
            source: ModpackSource::CurseForge {
                slug: "all-the-mods-9".to_string(),
                file_id: 0, // Latest
            },
            recommended_memory_mb: 8192,
            java_version: 17,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
            ],
            default_extra_env: vec![],
        }
    }

    pub fn vanilla() -> Self {
        Self {
            name: "Vanilla".to_string(),
            description: "Pure Minecraft experience".to_string(),
            version: "1.21".to_string(),
            minecraft_version: "1.21".to_string(),
            loader: ModLoader::Vanilla,
            source: ModpackSource::DirectDownload {
                url: "https://piston-data.mojang.com/v1/objects/450698d1863ab5180c25d7c804ef0fe6369dd1ba/server.jar".to_string(),
            },
            recommended_memory_mb: 2048,
            java_version: 21,
            default_java_args: vec![],
            default_extra_env: vec![],
        }
    }

    pub fn agrarian_skies_2() -> Self {
        Self {
            name: "Agrarian Skies 2".to_string(),
            description: "Classic FTB skyblock with quests and HQM. A beloved classic!".to_string(),
            version: "2.0.6".to_string(),
            minecraft_version: "1.7.10".to_string(),
            loader: ModLoader::Forge,
            // Use ForgeWithPack: itzg installs Forge, then overlays the server pack
            // (mods, configs). The server pack zip lacks a Forge jar/start script,
            // so AUTO_CURSEFORGE and TYPE=CURSEFORGE both fail on this old pack.
            source: ModpackSource::ForgeWithPack {
                forge_version: "10.13.4.1614".to_string(),
                pack_url: "https://mediafilez.forgecdn.net/files/3016/706/Agrarian%2BSkies%2B2%2B%282.0.6%29-Server.zip".to_string(),
            },
            recommended_memory_mb: 4096,
            java_version: 8,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            default_extra_env: vec![
                // Skyblock pack: use the included starting platform map instead of a generated world
                "LEVEL=maps/Default Platform - Normal".to_string(),
            ],
        }
    }

    pub fn atm9_to_the_sky() -> Self {
        Self {
            name: "ATM 9: To the Sky".to_string(),
            description: "All The Mods skyblock variant — tech, magic, and exploration in the sky"
                .to_string(),
            version: "1.0.3".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::ForgeWithPack {
                forge_version: "47.2.20".to_string(),
                pack_url: "https://mediafilez.forgecdn.net/files/5410/874/server-1.0.3.zip"
                    .to_string(),
            },
            recommended_memory_mb: 8192,
            java_version: 17,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=30".to_string(),
                "-XX:G1MaxNewSizePercent=40".to_string(),
                "-XX:G1HeapRegionSize=8M".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
            ],
            // SkyblockBuilder + DefaultWorldType mods handle skyblock world gen via config
            default_extra_env: vec![],
        }
    }

    pub fn regrowth() -> Self {
        Self {
            name: "Regrowth".to_string(),
            description: "Quest-driven FTB pack: regrow a barren world with magic and botany"
                .to_string(),
            version: "1.0.2".to_string(),
            minecraft_version: "1.7.10".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::ForgeWithPack {
                forge_version: "10.13.4.1614".to_string(),
                pack_url:
                    "https://dist.creeper.host/FTB2/modpacks/Regrowth/1_0_2/RegrowthServer.zip"
                        .to_string(),
            },
            recommended_memory_mb: 4096,
            java_version: 8,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            default_extra_env: vec![],
        }
    }

    pub fn project_ozone_lite() -> Self {
        Self {
            name: "Project Ozone Lite".to_string(),
            description: "Lightweight skyblock with quests, tech, and magic".to_string(),
            version: "1.3.6".to_string(),
            minecraft_version: "1.10.2".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::ForgeWithPack {
                forge_version: "12.18.3.2511".to_string(),
                pack_url: "https://mediafilez.forgecdn.net/files/2522/475/PO%20Lite%20Server%20v.1.3.6.zip".to_string(),
            },
            recommended_memory_mb: 4096,
            java_version: 8,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            default_extra_env: vec![],
        }
    }

    pub fn skyfactory_4() -> Self {
        Self {
            name: "SkyFactory 4".to_string(),
            description: "Popular skyblock with prestige system, tech trees, and automation"
                .to_string(),
            version: "4.2.4".to_string(),
            minecraft_version: "1.12.2".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::ForgeWithPack {
                forge_version: "14.23.5.2860".to_string(),
                pack_url:
                    "https://mediafilez.forgecdn.net/files/3565/687/SkyFactory-4_Server_4_2_4.zip"
                        .to_string(),
            },
            recommended_memory_mb: 4096,
            java_version: 8,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            default_extra_env: vec![],
        }
    }

    pub fn seaopolis_submerged() -> Self {
        Self {
            name: "Seaopolis: Submerged".to_string(),
            description: "Ocean-themed skyblock with underwater exploration and tech".to_string(),
            version: "B7.0".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::ForgeWithPack {
                forge_version: "47.2.20".to_string(),
                pack_url:
                    "https://mediafilez.forgecdn.net/files/5420/427/Submerged_server_pack.zip"
                        .to_string(),
            },
            recommended_memory_mb: 8192,
            java_version: 17,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:G1NewSizePercent=30".to_string(),
                "-XX:G1MaxNewSizePercent=40".to_string(),
                "-XX:G1HeapRegionSize=8M".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
            ],
            // SkyblockBuilder + DefaultWorldType mods handle skyblock world gen via config
            default_extra_env: vec![],
        }
    }

    pub fn builtin_templates() -> Vec<Self> {
        vec![
            Self::agrarian_skies_2(),
            Self::atm9_to_the_sky(),
            Self::ftb_stoneblock_4(),
            Self::all_the_mods_9(),
            Self::project_ozone_lite(),
            Self::regrowth(),
            Self::seaopolis_submerged(),
            Self::skyfactory_4(),
            Self::vanilla(),
        ]
    }
}
