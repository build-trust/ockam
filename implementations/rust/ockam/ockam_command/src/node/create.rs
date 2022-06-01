use clap::Args;
use std::{env::current_exe, process::Command};

use crate::util::embedded_node;
use ockam::{Context, Result, Routed, TcpTransport, Worker};

struct Status {
    node_name: String,
}

#[ockam::worker]
impl Worker for Status {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!(
            "\n[✓] Node: {}, Address: {}, Received: {}",
            self.node_name,
            ctx.address(),
            msg
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    #[clap(default_value_t = String::from("default"))]
    pub node_name: String,

    /// Spawn a node in the background.
    #[clap(display_order = 900, long, short)]
    spawn: bool,
}

impl CreateCommand {
    pub fn run(command: CreateCommand) {
        if command.spawn {
            // On systems with non-obvious path setups (or during
            // development) re-executing the current binary is a more
            // deterministic way of starting a node.
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());
            Command::new(ockam)
                .args(["node", "create", &command.node_name])
                .spawn()
                .expect("could not spawn node");
        } else {
            embedded_node(setup, command)
        }
    }
}

async fn setup(ctx: Context, c: CreateCommand) -> anyhow::Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:62526").await?;

    ctx.start_worker(
        "status",
        Status {
            node_name: c.node_name,
        },
    )
    .await?;

    Ok(())
}
