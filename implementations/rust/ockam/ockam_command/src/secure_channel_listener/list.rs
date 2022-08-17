use crate::util::{api, connect_to, exitcode, stop_node};
use crate::CommandGlobalOpts;

use clap::Args;

use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;
use ockam_core::Route;

use super::with_at_opt;

#[derive(Clone, Debug, Args)]

pub struct ListCommand {
    #[clap(flatten)]
    node_opts: with_at_opt::WithAtNodeOpt,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.at) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, command, secure_channel_listener_list);

        Ok(())
    }
}

pub async fn secure_channel_listener_list(
    ctx: ockam::Context,
    _cmd: ListCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::secure_channel_listener_list()?,
        )
        .await?;
    let (response, wrapped_list) = api::parse_secure_channel_listener_list_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("/secure channel listeners/{:?}", wrapped_list.list)
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",)
        }
    }

    stop_node(ctx).await
}
