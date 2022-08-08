use crate::util::{connect_to, stop_node};
use crate::CommandGlobalOpts;
use crate::{node::NodeOpts, util::api};
use clap::Args;
use ockam::{Context, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::Status;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,
    #[clap(short, long, action)]
    full: bool,
}

impl ShowCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, show_identity);

        Ok(())
    }
}

pub async fn show_identity(
    ctx: Context,
    cmd: ShowCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    if cmd.full {
        let resp: Vec<u8> = ctx
            .send_and_receive(
                base_route.modify().append(NODEMANAGER_ADDR),
                api::long_identity()?,
            )
            .await?;

        let (response, result) = api::parse_long_identity_response(&resp)?;

        match response.status() {
            Some(Status::Ok) => {
                println!("{}", hex::encode(result.identity.0.as_ref()))
            }
            _ => {
                eprintln!("An error occurred while exporting Identity",)
            }
        }

        stop_node(ctx).await
    } else {
        let resp: Vec<u8> = ctx
            .send_and_receive(
                base_route.modify().append(NODEMANAGER_ADDR),
                api::short_identity()?,
            )
            .await?;

        let (response, result) = api::parse_short_identity_response(&resp)?;

        match response.status() {
            Some(Status::Ok) => {
                println!("{}", result.identity_id)
            }
            _ => {
                eprintln!("An error occurred while getting Identity",)
            }
        }

        stop_node(ctx).await
    }
}
