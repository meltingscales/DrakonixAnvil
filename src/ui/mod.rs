mod cf_browse;
mod dashboard;
mod mr_browse;
mod server_create;
mod server_edit;

pub use cf_browse::{CfBrowseWidget, CfCallbacks, CfSearchState};
pub use dashboard::{DashboardCallbacks, DashboardView};
pub use mr_browse::{MrBrowseWidget, MrCallbacks, MrSearchState};
pub use server_create::{CreateViewCallbacks, ServerCreateView};
pub use server_edit::{ServerEditResult, ServerEditView};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Dashboard,
    CreateServer,
    EditServer(String),
    #[allow(dead_code)] // Will be used when server details view is implemented
    ServerDetails(String),
    ContainerLogs(String),
    ConfirmDelete(String),
    Backups(String), // Server name - list and restore backups
    ConfirmRestore(String, std::path::PathBuf), // Server name, backup path
    ConfirmDeleteBackup(String, std::path::PathBuf), // Server name, backup path
    ConfirmRemoveContainer(String), // Server name - confirm old container removal before recreate
    ConfirmImport(std::path::PathBuf), // Path to .drakonixanvil-server.zip to preview and import
    Console(String), // Server name - RCON console
    Logs,
    DockerLogs,
    Settings,
    Help,
}
