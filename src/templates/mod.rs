use serde::{Deserialize, Serialize};
use crate::server::{ModLoader, ModpackSource};

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
}

impl ModpackTemplate {
    pub fn ftb_stoneblock_4() -> Self {
        Self {
            name: "FTB StoneBlock 4".to_string(),
            description: "A skyblock-style modpack where you start in a world of stone".to_string(),
            version: "1.0.0".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: ModLoader::NeoForge,
            source: ModpackSource::FTB {
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
                project_id: 715572,
                file_id: 0, // Latest
            },
            recommended_memory_mb: 8192,
            java_version: 17,
            default_java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
            ],
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
        }
    }

    pub fn builtin_templates() -> Vec<Self> {
        vec![
            Self::ftb_stoneblock_4(),
            Self::all_the_mods_9(),
            Self::vanilla(),
        ]
    }
}
