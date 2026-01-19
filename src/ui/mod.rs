mod dashboard;
mod server_create;

pub use dashboard::DashboardView;
pub use server_create::ServerCreateView;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Dashboard,
    CreateServer,
    ServerDetails(String),
    Settings,
}
