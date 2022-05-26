use clap::Args;

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
pub struct StartCommand {
    /// Name of the node.
    #[clap(default_value_t = String::from("default"))]
    pub node_name: String,
}

impl StartCommand {
    pub fn run(command: StartCommand) {
        embedded_node(setup, command)
    }
}

async fn setup(ctx: Context, c: StartCommand) -> anyhow::Result<()> {
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
