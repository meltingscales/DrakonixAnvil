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
            version: "1.1.3".to_string(),
            minecraft_version: "1.7.10".to_string(),
            loader: ModLoader::Forge,
            source: ModpackSource::CurseForge {
                slug: "agrarian-skies-2".to_string(),
                file_id: 0, // 0 = latest
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
                // Resource Loader is client-only (references IResourcePack) and crashes dedicated servers
                "CF_EXCLUDE_MODS=resource-loader".to_string(),
                // Re-evaluate mod exclusions on each start (removes already-downloaded client mods)
                "CF_FORCE_SYNCHRONIZE=true".to_string(),
            ],
        }
    }

    pub fn builtin_templates() -> Vec<Self> {
        vec![
            Self::agrarian_skies_2(),
            Self::ftb_stoneblock_4(),
            Self::all_the_mods_9(),
            Self::vanilla(),
        ]
    }
}
