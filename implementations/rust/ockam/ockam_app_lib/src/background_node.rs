use crate::cli::cli_bin;
use crate::Result;
use ockam::compat::tokio::task::spawn_blocking;
use ockam_core::async_trait;
use std::process::Command;
use tracing::{debug, info};

pub trait BackgroundNodeClient: Send + Sync + 'static {
    fn nodes(&self) -> Box<dyn Nodes>;
    fn projects(&self) -> Box<dyn Projects>;
}

#[async_trait]
pub trait Nodes: Send + Sync + 'static {
    async fn create(&mut self, node_name: &str) -> Result<()>;
}

#[async_trait]
pub trait Projects: Send + Sync + 'static {
    async fn enroll(&self, node_name: &str, hex_encoded_ticket: &str) -> Result<()>;
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
}
