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
}

/// Generate a memorable 4-word RCON password (like "correct-horse-battery-staple")
fn generate_rcon_password() -> String {
    use rand::seq::SliceRandom;

    // Simple word list - Minecraft themed for fun
    const WORDS: &[&str] = &[
        "creeper", "diamond", "redstone", "enderman", "nether", "obsidian",
        "pickaxe", "zombie", "skeleton", "spider", "blaze", "ghast",
        "emerald", "villager", "golem", "beacon", "enchant", "potion",
        "anvil", "furnace", "chest", "portal", "dragon", "wither",
        "trident", "elytra", "shulker", "phantom", "pillager", "ravager",
        "copper", "amethyst", "deepslate", "warden", "sculk", "allay",
    ];

    let mut rng = rand::thread_rng();
    let words: Vec<&str> = WORDS.choose_multiple(&mut rng, 4).copied().collect();
    words.join("-")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub source: ModpackSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModLoader {
    Forge,
    Fabric,
    NeoForge,
    Vanilla,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModpackSource {
    CurseForge { project_id: u64, file_id: u64 },
    FTB { pack_id: u64, version_id: u64 },
    Modrinth { project_id: String, version_id: String },
    DirectDownload { url: String },
    Local { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerProperties {
    pub motd: String,
    pub max_players: u32,
    pub difficulty: Difficulty,
    pub gamemode: GameMode,
    pub pvp: bool,
    pub online_mode: bool,
    pub white_list: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Difficulty {
    Peaceful,
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
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
    Pulling,       // Pulling Docker image
    Starting,      // Docker container starting
    Initializing,  // Container running, MC server initializing (not yet accepting connections)
    Running,       // MC server accepting connections
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
            ModpackSource::FTB { pack_id, version_id } => {
                env.push("TYPE=FTB".to_string());
                env.push(format!("FTB_MODPACK_ID={}", pack_id));
                env.push(format!("FTB_MODPACK_VERSION_ID={}", version_id));
            }
            ModpackSource::CurseForge { project_id, file_id } => {
                env.push("TYPE=AUTO_CURSEFORGE".to_string());
                env.push(format!("CF_PAGE_URL=https://www.curseforge.com/minecraft/modpacks/{}", project_id));
                env.push(format!("CF_FILE_ID={}", file_id));
                // Note: CF_API_KEY should be set via global config, not here
            }
            ModpackSource::Modrinth { project_id, version_id } => {
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

        env
    }
}

impl ServerProperties {
    #[allow(dead_code)] // Will be used when generating server.properties files
    pub fn to_properties_string(&self) -> String {
        let difficulty = match self.difficulty {
            Difficulty::Peaceful => "peaceful",
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        };

        let gamemode = match self.gamemode {
            GameMode::Survival => "survival",
            GameMode::Creative => "creative",
            GameMode::Adventure => "adventure",
            GameMode::Spectator => "spectator",
        };

        format!(
            "motd={}\nmax-players={}\ndifficulty={}\ngamemode={}\npvp={}\nonline-mode={}\nwhite-list={}\n",
            self.motd,
            self.max_players,
            difficulty,
            gamemode,
            self.pvp,
            self.online_mode,
            self.white_list
        )
    }
}
