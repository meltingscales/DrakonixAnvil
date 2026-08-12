use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub modpack: ModpackInfo,
    pub port: u16,
    pub memory_mb: u64,
    pub java_args: Vec<String>,
    pub server_properties: ServerProperties,
    /// RCON password for remote console access
    #[serde(default = "generate_rcon_password")]
    pub rcon_password: String,
    /// Java version to use (8, 17, 21, etc.) — selects the Docker image tag
    #[serde(default = "default_java_version")]
    pub java_version: u8,
    /// Extra Docker environment variables (e.g. CF_EXCLUDE_MODS, CF_FORCE_SYNCHRONIZE)
    #[serde(default)]
    pub extra_env: Vec<String>,
}

fn default_java_version() -> u8 {
    21
}

/// Generate a memorable 4-word RCON password (like "correct-horse-battery-staple")
fn generate_rcon_password() -> String {
    use rand::seq::SliceRandom;

    // Simple word list - Minecraft themed for fun
    const WORDS: &[&str] = &[
        "creeper",
        "diamond",
        "redstone",
        "enderman",
        "nether",
        "obsidian",
        "pickaxe",
        "zombie",
        "skeleton",
        "spider",
        "blaze",
        "ghast",
        "emerald",
        "villager",
        "golem",
        "beacon",
        "enchant",
        "potion",
        "anvil",
        "furnace",
        "chest",
        "portal",
        "dragon",
        "wither",
        "trident",
        "elytra",
        "shulker",
        "phantom",
        "pillager",
        "ravager",
        "copper",
        "amethyst",
        "deepslate",
        "warden",
        "sculk",
        "allay",
    ];

    let mut rng = rand::thread_rng();
    let words: Vec<&str> = WORDS.choose_multiple(&mut rng, 4).copied().collect();
    words.join("-")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub source: ModpackSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModLoader {
    Forge,
    Fabric,
    NeoForge,
    Vanilla,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModpackSource {
    /// AUTO_CURSEFORGE: downloads from CurseForge client manifest by slug
    CurseForge {
        slug: String,
        file_id: u64,
    },
    /// Installs a specific Forge version, then overlays a server pack zip (mods, configs, etc.)
    /// via GENERIC_PACK_URL. For older packs whose server zips lack a Forge jar or start script.
    ForgeWithPack {
        forge_version: String,
        pack_url: String,
    },
    #[serde(alias = "FTB")]
    Ftb {
        pack_id: u64,
        version_id: u64,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
    DirectDownload {
        url: String,
    },
    Local {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ServerProperties {
    pub motd: String,
    pub max_players: u32,
    pub difficulty: Difficulty,
    pub gamemode: GameMode,
    pub pvp: bool,
    pub online_mode: bool,
    pub white_list: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 4] = [
        Difficulty::Peaceful,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
    ];
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Peaceful => write!(f, "peaceful"),
            Difficulty::Easy => write!(f, "easy"),
            Difficulty::Normal => write!(f, "normal"),
            Difficulty::Hard => write!(f, "hard"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub const ALL: [GameMode; 4] = [
        GameMode::Survival,
        GameMode::Creative,
        GameMode::Adventure,
        GameMode::Spectator,
    ];
}

impl std::fmt::Display for GameMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameMode::Survival => write!(f, "survival"),
            GameMode::Creative => write!(f, "creative"),
            GameMode::Adventure => write!(f, "adventure"),
            GameMode::Spectator => write!(f, "spectator"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInstance {
    pub config: ServerConfig,
    pub container_id: Option<String>,
    pub status: ServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ServerStatus {
    #[default]
    Stopped,
    Pulling,      // Pulling Docker image
    Starting,     // Docker container starting
    Initializing, // Container running, MC server initializing (not yet accepting connections)
    Running,      // MC server accepting connections
    Stopping,
    Error(String),
}

impl ServerConfig {
    pub fn new(name: String, modpack: ModpackInfo) -> Self {
        Self {
            name,
            modpack,
            port: 25565,
            memory_mb: 4096,
            java_args: vec![],
            server_properties: ServerProperties::default(),
            rcon_password: generate_rcon_password(),
            java_version: default_java_version(),
            extra_env: vec![],
        }
    }

    /// Get the Docker image to use based on the configured Java version.
    /// See https://docker-minecraft-server.readthedocs.io/en/latest/versions/java/
    pub fn docker_image(&self) -> String {
        match self.java_version {
            8 => "itzg/minecraft-server:java8".to_string(),
            11 => "itzg/minecraft-server:java11".to_string(),
            17 => "itzg/minecraft-server:java17".to_string(),
            21 => "itzg/minecraft-server:java21".to_string(),
            _ => "itzg/minecraft-server:latest".to_string(),
        }
    }

    /// Get the RCON port (always 25575 inside container, but we expose it on host)
    pub fn rcon_port(&self) -> u16 {
        // RCON port is game port + 10 to avoid conflicts between servers
        self.port + 10
    }
}

impl ServerConfig {
    /// Build Docker environment variables for the itzg/minecraft-server image
    pub fn build_docker_env(&self) -> Vec<String> {
        let mut env = vec![
            "EULA=TRUE".to_string(),
            format!("MEMORY={}M", self.memory_mb),
        ];

        // Set TYPE and loader-specific vars based on ModpackSource
        match &self.modpack.source {
            ModpackSource::Ftb {
                pack_id,
                version_id,
            } => {
                env.push("TYPE=FTBA".to_string());
                env.push(format!("FTB_MODPACK_ID={}", pack_id));
                if *version_id != 0 {
                    env.push(format!("FTB_MODPACK_VERSION_ID={}", version_id));
                }
            }
            ModpackSource::CurseForge { slug, file_id } => {
                env.push("TYPE=AUTO_CURSEFORGE".to_string());
                env.push(format!("CF_SLUG={}", slug));
                if *file_id != 0 {
                    env.push(format!("CF_FILE_ID={}", file_id));
                }
                // Note: CF_API_KEY should be set via global config, not here
            }
            ModpackSource::ForgeWithPack { forge_version, .. } => {
                // Pack extraction is handled on the host by pack_installer before
                // the container starts. We only tell itzg to install Forge.
                env.push("TYPE=FORGE".to_string());
                env.push(format!("FORGE_VERSION={}", forge_version));
            }
            ModpackSource::Modrinth {
                project_id,
                version_id,
            } => {
                env.push("TYPE=MODRINTH".to_string());
                env.push(format!("MODRINTH_PROJECT={}", project_id));
                env.push(format!("MODRINTH_VERSION={}", version_id));
            }
            ModpackSource::DirectDownload { url } => {
                // Determine TYPE from mod loader
                let type_str = match self.modpack.loader {
                    ModLoader::Forge => "FORGE",
                    ModLoader::Fabric => "FABRIC",
                    ModLoader::NeoForge => "NEOFORGE",
                    ModLoader::Vanilla => "VANILLA",
                };
                env.push(format!("TYPE={}", type_str));
                env.push(format!("MODPACK={}", url));
            }
            ModpackSource::Local { path } => {
                // For local modpacks, set type based on loader
                let type_str = match self.modpack.loader {
                    ModLoader::Forge => "FORGE",
                    ModLoader::Fabric => "FABRIC",
                    ModLoader::NeoForge => "NEOFORGE",
                    ModLoader::Vanilla => "VANILLA",
                };
                env.push(format!("TYPE={}", type_str));
                // Local path should be relative to /data in container
                env.push(format!("MODPACK=/data/{}", path));
            }
        }

        // Set VERSION (Minecraft version, not modpack version) if available
        if !self.modpack.minecraft_version.is_empty() {
            env.push(format!("VERSION={}", self.modpack.minecraft_version));
        }

        // Set JVM_OPTS if java_args are configured
        if !self.java_args.is_empty() {
            env.push(format!("JVM_OPTS={}", self.java_args.join(" ")));
        }

        // RCON settings (enabled by default in itzg/minecraft-server)
        env.push("ENABLE_RCON=true".to_string());
        env.push(format!("RCON_PASSWORD={}", self.rcon_password));

        // Server properties
        let sp = &self.server_properties;
        if !sp.motd.is_empty() {
            env.push(format!("MOTD={}", sp.motd));
        }
        env.push(format!("DIFFICULTY={}", sp.difficulty));
        env.push(format!("MODE={}", sp.gamemode));
        env.push(format!("MAX_PLAYERS={}", sp.max_players));
        env.push(format!("PVP={}", sp.pvp));
        env.push(format!("ONLINE_MODE={}", sp.online_mode));
        env.push(format!("ENABLE_WHITELIST={}", sp.white_list));

        // Extra env vars (e.g. CF_EXCLUDE_MODS for client-only mods)
        env.extend(self.extra_env.iter().cloned());

        env
    }
}


