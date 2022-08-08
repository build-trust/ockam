use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::Status;
use ockam_core::Route;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    /// Path to the Vault storage file
    #[clap(short, long)]
    pub path: Option<String>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, create_vault);

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
            eprintln!("An error occurred while creating Vault",)
        }
    }

    stop_node(ctx).await
}
