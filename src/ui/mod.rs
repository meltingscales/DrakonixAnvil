mod dashboard;
mod server_create;

pub use dashboard::DashboardView;
pub use server_create::ServerCreateView;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Dashboard,
    CreateServer,
    #[allow(dead_code)] // Will be used when server details view is implemented
    ServerDetails(String),
    Logs,
    Settings,
}
