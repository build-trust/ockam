use crate::cli::cli_bin;
use crate::error::Error;
use crate::Result;
use ockam::compat::tokio::task::spawn_blocking;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::async_trait;
use std::net::SocketAddr;
use std::process::Command;
use tracing::{debug, error, info, trace, warn};

// Matches backend default of 14 days
const DEFAULT_ENROLLMENT_TICKET_EXPIRY: &str = "14d";

pub trait BackgroundNodeClient: Send + Sync + 'static {
    fn nodes(&self) -> Box<dyn Nodes>;
    fn inlets(&self) -> Box<dyn Inlets>;
    fn projects(&self) -> Box<dyn Projects>;
}

#[async_trait]
pub trait Nodes: Send + Sync + 'static {
    async fn create(&mut self, node_name: &str) -> Result<()>;
    async fn delete(&mut self, node_name: &str) -> Result<()>;
}

#[async_trait]
pub trait Inlets: Send + Sync + 'static {
    async fn create(
        &mut self,
        node_name: &str,
        from: &SocketAddr,
        service_route: &str,
        service_name: &str,
    ) -> Result<()>;
    async fn show(&self, node_name: &str, inlet_name: &str) -> Result<InletStatus>;
    async fn delete(&mut self, node_name: &str, alias: &str) -> Result<()>;
}

#[async_trait]
pub trait Projects: Send + Sync + 'static {
    async fn enroll(&self, node_name: &str, hex_encoded_ticket: &str) -> Result<()>;

    /// Returns the hex-encoded enrollment ticket
    async fn ticket(&self, project_name: &str) -> Result<String>;
}

#[derive(Clone)]
pub struct Cli {
    bin: String,
}

impl Cli {
    pub fn new() -> Self {
        Self {
            bin: cli_bin().expect("OCKAM env variable is not valid"),
        }
    }
}

fn log_command(cmd: &mut Command) -> std::io::Result<()> {
    info!(
        "Executing command: {} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|arg| arg.to_string_lossy())
            .fold(String::new(), |acc, arg| acc + " " + &arg)
    );

    Ok(())
}

impl BackgroundNodeClient for Cli {
    fn nodes(&self) -> Box<dyn Nodes> {
        Box::new(self.clone())
    }

    fn inlets(&self) -> Box<dyn Inlets> {
        Box::new(self.clone())
    }

    fn projects(&self) -> Box<dyn Projects> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl Nodes for Cli {
    async fn create(&mut self, node_name: &str) -> Result<()> {
        let bin = self.bin.clone();
        let node_name = node_name.to_string();
        spawn_blocking(move || {
            let _ = duct::cmd!(
                &bin,
                "--no-input",
                "node",
                "create",
                &node_name,
                "--trust-context",
                &node_name
            )
            .before_spawn(log_command)
            .stderr_null()
            .stdout_capture()
            .run()
            .map(|_| debug!(node = %node_name, "Node created"));
        })
        .await?;

        Ok(())
    }

    async fn delete(&mut self, node_name: &str) -> Result<()> {
        debug!(node = %node_name, "Deleting node");
        let bin = self.bin.clone();
        let node_name = node_name.to_string();
        spawn_blocking(move || {
            let _ = duct::cmd!(&bin, "--no-input", "node", "delete", "--yes", &node_name)
                .before_spawn(log_command)
                .stderr_null()
                .stdout_capture()
                .run();
        })
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Inlets for Cli {
    async fn create(
        &mut self,
        node_name: &str,
        from: &SocketAddr,
        service_route: &str,
        service_name: &str,
    ) -> Result<()> {
        let from_str = from.to_string();
        let bin = self.bin.clone();
        let node_name = node_name.to_string();
        let service_route = service_route.to_string();
        let service_name = service_name.to_string();
        spawn_blocking(move || {
            duct::cmd!(
                &bin,
                "--no-input",
                "tcp-inlet",
                "create",
                "--at",
                &node_name,
                "--from",
                &from_str,
                "--to",
                &service_route,
                "--alias",
                &service_name,
                "--retry-wait",
                "0",
            )
            .before_spawn(log_command)
            .stderr_null()
            .stdout_capture()
            .run()
            .map(|_| {
                info!(
                    from = from_str,
                    to = service_route,
                    "Created TCP inlet for accepted invitation"
                );
            })
        })
        .await?
        .map_err(|err| err.into())
    }

    async fn show(&self, node_name: &str, inlet_name: &str) -> Result<InletStatus> {
        debug!(node = %node_name, "Checking TCP inlet status");
        let inlet_name = inlet_name.to_string();
        let node_name = node_name.to_string();
        let bin = self.bin.clone();
        spawn_blocking(move || {
            match duct::cmd!(
                &bin,
                "--no-input",
                "tcp-inlet",
                "show",
                &inlet_name,
                "--at",
                &node_name,
                "--output",
                "json"
            )
            .before_spawn(log_command)
            .env("OCKAM_LOG", "off")
            .stderr_null()
            .stdout_capture()
            .run()
            {
                Ok(cmd) => {
                    trace!(output = ?String::from_utf8_lossy(&cmd.stdout), "TCP inlet status");
                    let inlet: InletStatus = serde_json::from_slice(&cmd.stdout)?;
                    debug!(
                        at = ?inlet.bind_addr,
                        alias = inlet.alias,
                        "TCP inlet running"
                    );
                    Ok(inlet)
                }
                Err(_) => Err(Error::App(format!(
                    "TCP inlet {} is not running",
                    inlet_name
                ))),
            }
        })
        .await?
    }

    async fn delete(&mut self, node_name: &str, alias: &str) -> Result<()> {
        debug!(node = %node_name, %alias, "Deleting TCP inlet");
        let bin = self.bin.clone();
        let node_name = node_name.to_string();
        let alias = alias.to_string();
        spawn_blocking(move || {
            let _ = duct::cmd!(
                &bin,
                "--no-input",
                "tcp-inlet",
                "delete",
                &alias,
                "--at",
                &node_name,
                "--yes"
            )
            .before_spawn(log_command)
            .stderr_null()
            .stdout_capture()
            .run()
            .map(|_| {
                info!(
                    node = %node_name, alias = %alias,
                    "Disconnected TCP inlet for accepted invitation"
                );
            })
            .map_err(
                |e| warn!(%e, node = %node_name, alias = %alias, "Failed to delete TCP inlet"),
            );
        })
        .await
        .map_err(|err| err.into())
    }
}

#[async_trait]
impl Projects for Cli {
    async fn enroll(&self, node_name: &str, hex_encoded_ticket: &str) -> Result<()> {
        let node_name = node_name.to_string();
        let hex_encoded_ticket = hex_encoded_ticket.to_string();
        let bin = self.bin.clone();
        spawn_blocking(move || {
            let _ = duct::cmd!(
                &bin,
                "--no-input",
                "project",
                "enroll",
                "--new-trust-context-name",
                &node_name,
                &hex_encoded_ticket,
            )
            .before_spawn(log_command)
            .stderr_null()
            .stdout_capture()
            .run()
            .map(|_| {
                debug!(node = %node_name, "Node enrolled using enrollment ticket");
            });
        })
        .await
        .map_err(|err| err.into())
    }

    async fn ticket(&self, project_name: &str) -> Result<String> {
        let bin = self.bin.clone();
        let project_name = project_name.to_string();
        spawn_blocking(move || {
            duct::cmd!(
                &bin,
                "project",
                "ticket",
                "--quiet",
                "--project",
                &project_name,
                "--expires-in",
                DEFAULT_ENROLLMENT_TICKET_EXPIRY.to_string(),
                "--to",
                &format!("/project/{project_name}")
            )
            .before_spawn(log_command)
            .read()
            .map_err(|err| {
                error!(?err, "Could not create enrollment ticket");
                Error::App("Could not create enrollment ticket".to_string())
            })
        })
        .await?
    }
}
