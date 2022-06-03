use clap::Args;

use crate::util::{embedded_node, multiaddr_to_route};
use anyhow::anyhow;
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::Client;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct RejectCommand {
    #[clap(display_order = 1001)]
    email: String,

    #[clap(display_order = 1002)]
    invitation: String,
}

impl RejectCommand {
    pub fn run(command: RejectCommand, cloud_addr: MultiAddr) {
        embedded_node(reject, (cloud_addr, command));
    }
}

async fn reject(mut ctx: Context, args: (MultiAddr, RejectCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = Client::new(route, &ctx).await?;
    let res = api.reject_invitations(&cmd.email, &cmd.invitation).await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
