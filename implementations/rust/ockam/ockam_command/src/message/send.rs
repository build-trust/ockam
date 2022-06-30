use std::time::Duration;

use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    addr: MultiAddr,
    message: String,
}

impl SendCommand {
    pub fn run(cmd: SendCommand) {
        embedded_node(send_message, cmd)
    }
}

async fn send_message(mut ctx: Context, cmd: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    if let Some(route) = ockam_api::multiaddr_to_route(&cmd.addr) {
        ctx.send(route, cmd.message).await?
    }

    // TODO: find a way to wait for send to complete
    ctx.sleep(Duration::from_millis(500)).await;

    ctx.stop().await?;

    Ok(())
}
