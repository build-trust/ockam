use crate::cli::cli_bin;
use crate::error::Error;
use crate::Result;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::async_trait;
use std::net::SocketAddr;
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
        duct::cmd!(
            &self.bin,
            "--no-input",
            "node",
            "create",
            node_name,
            "--trust-context",
            node_name
        )
        .stderr_null()
        .stdout_capture()
        .run()?;
        debug!(node = %node_name, "Node created");
        Ok(())
    }

    async fn delete(&mut self, node_name: &str) -> Result<()> {
        debug!(node = %node_name, "Deleting node");
        let _ = duct::cmd!(
            &self.bin,
            "--no-input",
            "node",
            "delete",
            "--yes",
            node_name
        )
        .stderr_null()
        .stdout_capture()
        .run();
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
        duct::cmd!(
            &self.bin,
            "--no-input",
            "tcp-inlet",
            "create",
            "--at",
            node_name,
            "--from",
            &from_str,
            "--to",
            service_route,
            "--alias",
            service_name,
            "--retry-wait",
            "0",
        )
        .stderr_null()
        .stdout_capture()
        .run()?;
        info!(
            from = from_str,
            to = service_route,
            "Created TCP inlet for accepted invitation"
        );
        Ok(())
    }

    async fn show(&self, node_name: &str, inlet_name: &str) -> Result<InletStatus> {
        debug!(node = %node_name, "Checking TCP inlet status");
        match duct::cmd!(
            &self.bin,
            "--no-input",
            "tcp-inlet",
            "show",
            inlet_name,
            "--at",
            node_name,
            "--output",
            "json"
        )
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
    }

    async fn delete(&mut self, node_name: &str, alias: &str) -> Result<()> {
        debug!(node = %node_name, %alias, "Deleting TCP inlet");
        let _ = duct::cmd!(
            &cli_bin()?,
            "--no-input",
            "tcp-inlet",
            "delete",
            alias,
            "--at",
            node_name,
            "--yes"
        )
        .stderr_null()
        .stdout_capture()
        .run()
        .map_err(|e| warn!(%e, node = %node_name, alias = %alias, "Failed to delete TCP inlet"));
        info!(
            node = %node_name, alias = %alias,
            "Disconnected TCP inlet for accepted invitation"
        );
        Ok(())
    }
}

#[async_trait]
impl Projects for Cli {
    async fn enroll(&self, node_name: &str, hex_encoded_ticket: &str) -> Result<()> {
        let _ = duct::cmd!(
            &self.bin,
            "--no-input",
            "project",
            "enroll",
            "--new-trust-context-name",
            node_name,
            hex_encoded_ticket,
        )
        .stderr_null()
        .stdout_capture()
        .run();
        debug!(node = %node_name, "Node enrolled using enrollment ticket");
        Ok(())
    }

    async fn ticket(&self, project_name: &str) -> Result<String> {
        Ok(duct::cmd!(
            &self.bin,
            "project",
            "ticket",
            "--quiet",
            "--project",
            project_name,
            "--expires-in",
            DEFAULT_ENROLLMENT_TICKET_EXPIRY.to_string(),
            "--to",
            &format!("/project/{project_name}")
        )
        .read()
        .map_err(|err| {
            error!(?err, "Could not create enrollment ticket");
            Error::App("Could not create enrollment ticket".to_string())
        })?)
    }
}
