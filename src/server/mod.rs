use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub modpack: ModpackInfo,
    pub port: u16,
    pub memory_mb: u64,
    pub java_args: Vec<String>,
    pub server_properties: ServerProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
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
    Starting,
    Running,
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
        }
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
