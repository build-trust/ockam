use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::Status;
use ockam_core::Route;

#[derive(Clone, Debug, Args)]
pub struct ExportCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl ExportCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, export_identity);

        Ok(())
    }
}

pub async fn export_identity(
    ctx: Context,
    _cmd: ExportCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::export_identity()?,
        )
        .await?;

    let (response, result) = api::parse_export_identity_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!("Identity is: {}", hex::encode(result.identity.0.as_ref()))
        }
        _ => {
            eprintln!("An error occurred while exporting Identity",)
        }
    }

    stop_node(ctx).await
}
