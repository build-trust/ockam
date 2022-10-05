use crate::node::NodeOpts;
use crate::util::{api, connect_to, exitcode};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_core::api::Status;
use ockam_core::Route;

/// Create vaults
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Path to the Vault storage file
    #[arg(short, long)]
    pub path: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) -> anyhow::Result<()> {
        let cfg = options.config;
        let port = cfg.get_node_port(&self.node_opts.api_node).unwrap();

        connect_to(port, self, create_vault);

        Ok(())
    }
}

pub async fn create_vault(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodemanager"),
            api::create_vault(cmd.path)?,
        )
        .await?;

    let response = api::parse_create_vault_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("Vault created!")
        }
        _ => {
            eprintln!("An error occurred while creating Vault",);
            std::process::exit(exitcode::CANTCREAT);
        }
    }

    Ok(())
}
