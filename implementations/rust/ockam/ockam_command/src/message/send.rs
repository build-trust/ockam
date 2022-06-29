use crate::util::embedded_node;
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;
use std::time::Duration;

#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    pub address: MultiAddr,
    pub message: String,
}

impl SendCommand {
    pub fn run(command: SendCommand) {
        embedded_node(send_message, command)
    }
}

async fn send_message(mut ctx: Context, command: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    if let Some(route) = ockam_api::multiaddr_to_route(&command.address) {
        ctx.send(route, command.message).await?
    }

    // TODO: find a way to wait for send to complete
    ctx.sleep(Duration::from_millis(500)).await;

    ctx.stop().await?;

    Ok(())
}
