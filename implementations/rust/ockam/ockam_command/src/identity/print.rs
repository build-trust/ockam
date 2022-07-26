use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::NODEMAN_ADDR;
use ockam_api::Status;
use ockam_core::Route;

#[derive(Clone, Debug, Args)]
pub struct PrintCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl PrintCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, print_identity);

        Ok(())
    }
}

pub async fn print_identity(
    ctx: Context,
    _cmd: PrintCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::print_identity()?,
        )
        .await?;

    let (response, result) = api::parse_print_identity_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!("Identity id is: {}!", result.identity_id)
        }
        _ => {
            eprintln!("An error occurred while getting Identity",)
        }
    }

    stop_node(ctx).await
}
