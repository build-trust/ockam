use anyhow::anyhow;
use clap::Args;

use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct AcceptCommand {
    #[clap(display_order = 1002)]
    invitation: String,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl AcceptCommand {
    pub fn run(command: AcceptCommand, cloud_addr: MultiAddr) {
        embedded_node(accept, (cloud_addr, command));
    }
}

async fn accept(mut ctx: Context, args: (MultiAddr, AcceptCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier()?;

    let r = ockam_api::multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api
        .accept_invitations(identifier.key_id(), &cmd.invitation)
        .await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
