use clap::Args;

use crate::util::{embedded_node, multiaddr_to_route};
use anyhow::anyhow;
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct AcceptCommand {
    #[clap(display_order = 1001)]
    email: String,

    #[clap(display_order = 1002)]
    invitation: String,
}

impl AcceptCommand {
    pub fn run(command: AcceptCommand, cloud_addr: MultiAddr) {
        embedded_node(accept, (cloud_addr, command));
    }
}

async fn accept(mut ctx: Context, args: (MultiAddr, AcceptCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api.accept_invitations(&cmd.email, &cmd.invitation).await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
