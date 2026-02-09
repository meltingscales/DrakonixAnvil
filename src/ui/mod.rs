mod cf_browse;
mod dashboard;
mod server_create;
mod server_edit;

pub use cf_browse::{CfBrowseWidget, CfCallbacks, CfSearchState};
pub use dashboard::{DashboardCallbacks, DashboardView};
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
    Console(String), // Server name - RCON console
    Logs,
    DockerLogs,
    Settings,
    Help,
}
