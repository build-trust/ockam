use clap::Args;
use std::{env::current_exe, process::Command, time::Duration};

use crate::{
    config::OckamConfig,
    util::{self, connect_to, embedded_node, DEFAULT_TCP_PORT},
};
use ockam::{Context, NodeMan, NodeManMessage, NodeManReply, Route, TcpTransport};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    #[clap(default_value_t = String::from("default"))]
    node_name: String,

    /// Spawn a node in the background.
    #[clap(display_order = 900, long, short)]
    spawn: bool,

    #[clap(default_value_t = DEFAULT_TCP_PORT, long, short)]
    port: u16,
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) {
        if command.spawn {
            // On systems with non-obvious path setups (or during
            // development) re-executing the current binary is a more
            // deterministic way of starting a node.
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());
            Command::new(ockam)
                .args(["node", "create", &command.node_name])
                .spawn()
                .expect("could not spawn node");

            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));

            // Then query the node manager for the status
            connect_to(command.port, (), query_status);
        } else {
            if let Err(e) = cfg.create_node(&command.node_name, DEFAULT_TCP_PORT) {
                eprintln!(
                    "failed to spawn node with name '{}': {:?}",
                    command.node_name, e
                );
                std::process::exit(-1);
            }
            embedded_node(setup, command);
        }
    }
}

async fn query_status(ctx: Context, _: (), mut base_route: Route) -> anyhow::Result<()> {
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
        } => println!(
            "Node: {}, Status: {}, Worker count: {}",
            node_name, status, workers
        ),
        // _ => eprintln!("Received invalid reply format!"),
    }

    util::stop_node(ctx).await
}

async fn setup(ctx: Context, c: CreateCommand) -> anyhow::Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(format!("127.0.0.1:{}", DEFAULT_TCP_PORT))
        .await?;

    ctx.start_worker("_internal.nodeman", NodeMan::new(c.node_name))
        .await?;

    Ok(())
}
