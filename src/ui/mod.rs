mod dashboard;
mod server_create;
mod server_edit;

pub use dashboard::DashboardView;
pub use server_create::ServerCreateView;
pub use server_edit::ServerEditView;

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
    Logs,
    Settings,
}
