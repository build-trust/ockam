use anyhow::anyhow;
use clap::Args;

use ockam::identity::IdentityTrait;
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct RejectCommand {
    #[clap(display_order = 1002)]
    invitation: String,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl RejectCommand {
    pub fn run(command: RejectCommand, cloud_addr: MultiAddr) {
        embedded_node(reject, (cloud_addr, command));
    }
}

async fn reject(mut ctx: Context, args: (MultiAddr, RejectCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier().await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api
        .reject_invitations(identifier.key_id(), &cmd.invitation)
        .await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
