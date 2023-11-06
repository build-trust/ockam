use crate::cli::cli_bin;
use crate::Result;
use ockam::compat::tokio::task::spawn_blocking;
use ockam_core::async_trait;
use std::process::Command;
use tracing::{debug, info};

pub trait BackgroundNodeClient: Send + Sync + 'static {
    fn nodes(&self) -> Box<dyn Nodes>;
}

#[async_trait]
pub trait Nodes: Send + Sync + 'static {
    async fn create(&mut self, node_name: &str) -> Result<()>;
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
