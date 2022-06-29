use crate::util::embedded_node;
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

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

async fn send_message(ctx: Context, command: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    if let Some(route) = ockam_api::multiaddr_to_route(&command.address) {
        ctx.send(route, command.message).await?
    }

    // TODO: find a way to wait for send to complete
    // ctx.stop().await
    Ok(())
}
