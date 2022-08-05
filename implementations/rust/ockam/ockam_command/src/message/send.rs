use clap::Args;
use std::io;
use std::str::FromStr;

use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    addr: String,
    message: String,
}

impl SendCommand {
    pub fn run(cmd: SendCommand) {
        embedded_node(send_message, cmd)
    }
}

async fn send_message(mut ctx: Context, cmd: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let addr = MultiAddr::from_str(&cmd.addr)?;
    if let Some(route) = ockam_api::multiaddr_to_route(&addr) {
        ctx.send(route, cmd.message).await?;
        let message = ctx.receive::<String>().await?;
        println!("{}", message);
    }

    ctx.stop().await?;

    Ok(())
}
