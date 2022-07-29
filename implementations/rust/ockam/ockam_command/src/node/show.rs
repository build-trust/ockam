use crate::util::{self, api, connect_to};
use crate::CommandGlobalOpts;
use anyhow::Context;
use clap::Args;
use ockam::Route;
use ockam_api::nodes::{models::base::NodeStatus, NODEMANAGER_ADDR};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the node.
    #[clap(default_value = "default")]
    node_name: String,
}

impl ShowCommand {
    pub fn run(opts: CommandGlobalOpts, command: ShowCommand) {
        let cfg = &opts.config;
        let port = match cfg.get_inner().nodes.get(&command.node_name) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, (), query_status);
    }
}

pub async fn query_status(ctx: ockam::Context, _: (), mut base_route: Route) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::query_status()?,
        )
        .await
        .context("Failed to process request")?;

    let NodeStatus {
        node_name,
        status,
        workers,
        pid,
        transports,
        ..
    } = api::parse_status(&resp)?;

    println!(
        "Node: {}, Status: {}, Worker count: {}, Pid: {}, Transport count: {}",
        node_name, status, workers, pid, transports,
    );

    util::stop_node(ctx).await
}
