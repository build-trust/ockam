use crate::{
    config::OckamConfig,
    util::{self, connect_to},
};
use clap::Args;
use ockam::{Context, NodeManMessage, NodeManReply, Route};

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
    let reply: NodeManReply = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            NodeManMessage::Status,
        )
        .await
        .unwrap();

    match reply {
        NodeManReply::Status {
            node_name,
            status,
            workers,
            ..
        } => println!(
            "Node: {}, Status: {}, Worker count: {}",
            node_name, status, workers
        ),
        // _ => eprintln!("Received invalid reply format!"),
    }

    util::stop_node(ctx).await
}
