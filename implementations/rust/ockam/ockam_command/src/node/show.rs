use crate::util::{self, api, connect_to, OckamConfig};
use clap::Args;
use ockam::{Context, Route};
use ockam_api::nodes::{types::NodeStatus, NODEMAN_ADDR};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the node.
    pub node_name: String,
}

impl ShowCommand {
    pub fn run(cfg: &mut OckamConfig, command: ShowCommand) {
        let port = match cfg.get_nodes().get(&command.node_name) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, (), query_status);
    }
}

pub async fn query_status(ctx: Context, _: (), mut base_route: Route) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::query_status()?,
        )
        .await
        .unwrap();

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
