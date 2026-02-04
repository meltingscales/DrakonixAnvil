#![allow(dead_code)] // Docker API methods will be used when container management is wired up

use anyhow::Result;
use bollard::Docker;
use bollard::container::{ListContainersOptions, Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions, LogsOptions};
use bollard::image::CreateImageOptions;
use bollard::models::ContainerSummary;
use std::collections::HashMap;
use std::path::Path;
use futures_util::StreamExt;

pub struct DockerManager {
    client: Docker,
}

impl DockerManager {
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()?;
        Ok(Self { client })
    }

    pub async fn check_connection(&self) -> Result<bool> {
        match self.client.ping().await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Docker connection failed: {}", e);
                Ok(false)
            }
        }
    }

    pub async fn get_version(&self) -> Result<String> {
        let version = self.client.version().await?;
        Ok(version.version.unwrap_or_else(|| "unknown".to_string()))
    }

    pub async fn list_minecraft_containers(&self) -> Result<Vec<ContainerSummary>> {
        let mut filters = HashMap::new();
        filters.insert("label", vec!["drakonix.managed=true"]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.client.list_containers(Some(options)).await?;
        Ok(containers)
    }

    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        match self.client.inspect_image(image).await {
            Ok(_) => {
                tracing::info!("Image {} found locally", image);
                Ok(true)
            }
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
                tracing::info!("Image {} not found locally (404)", image);
                Ok(false)
            }
            Err(e) => {
                tracing::warn!("Error checking image {}: {:?}", image, e);
                // If we can't check, assume it doesn't exist and try to pull
                Ok(false)
            }
        }
    }

    pub async fn pull_image(&self, image: &str) -> Result<()> {
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(options), None, None);
        let mut last_status: Option<String> = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = &info.status {
                        // Only log meaningful status changes, skip progress spam
                        // Skip "Downloading", "Extracting" progress updates (they have progress field)
                        let dominated = info.progress.is_some()
                            && (status == "Downloading" || status == "Extracting");

                        // Also skip duplicate status messages
                        let is_duplicate = last_status.as_ref() == Some(status);

                        if !dominated && !is_duplicate {
                            tracing::info!("Pull: {}", status);
                            last_status = Some(status.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Pull error: {}", e);
                    return Err(anyhow::anyhow!("Failed to pull image: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Ensure an image exists locally, pulling it if necessary
    pub async fn ensure_image(&self, image: &str) -> Result<()> {
        if !self.image_exists(image).await? {
            tracing::info!("Image {} not found locally, pulling...", image);
            self.pull_image(image).await?;
        }
        Ok(())
    }

    pub async fn create_minecraft_container(
        &self,
        container_name: &str,
        server_name: &str,
        image: &str,
        port: u16,
        memory_mb: u64,
        env_vars: Vec<String>,
        data_path: &Path,
    ) -> Result<String> {
        let mut labels = HashMap::new();
        labels.insert("drakonix.managed", "true");
        labels.insert("drakonix.type", "minecraft-server");
        labels.insert("drakonix.server-name", server_name);

        // Convert data_path to absolute path for Docker bind mount
        let data_path_abs = std::fs::canonicalize(data_path)
            .unwrap_or_else(|_| data_path.to_path_buf());
        let bind_mount = format!("{}:/data", data_path_abs.display());

        let host_config = bollard::models::HostConfig {
            port_bindings: Some({
                let mut bindings = HashMap::new();
                bindings.insert(
                    "25565/tcp".to_string(),
                    Some(vec![bollard::models::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(port.to_string()),
                    }]),
                );
                bindings
            }),
            binds: Some(vec![bind_mount]),
            memory: Some((memory_mb * 1024 * 1024) as i64),
            ..Default::default()
        };

        let config = Config {
            image: Some(image.to_string()),
            env: Some(env_vars),
            labels: Some(labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptions { name: container_name, ..Default::default() };
        let response = self.client.create_container(Some(options), config).await?;

        Ok(response.id)
    }

    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.client.start_container(id, None::<StartContainerOptions<String>>).await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> Result<()> {
        self.client.stop_container(id, Some(StopContainerOptions { t: 30 })).await?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str) -> Result<()> {
        self.client.remove_container(id, None).await?;
        Ok(())
    }

    /// Check if a container is currently running
    /// Returns Ok(true) if running, Ok(false) if stopped/exited, Err if container not found
    pub async fn is_container_running(&self, id: &str) -> Result<bool> {
        let info = self.client.inspect_container(id, None).await?;
        let running = info.state
            .and_then(|s| s.running)
            .unwrap_or(false);
        Ok(running)
    }

    pub async fn get_container_logs(&self, id: &str, tail_lines: usize) -> Result<String> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail_lines.to_string(),
            ..Default::default()
        };

        let mut stream = self.client.logs(id, Some(options));
        let mut output = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(log) => {
                    output.push_str(&log.to_string());
                }
                Err(e) => {
                    tracing::warn!("Error reading logs: {}", e);
                    break;
                }
            }
        }

        Ok(output)
    }
}
