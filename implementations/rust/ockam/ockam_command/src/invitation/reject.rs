use clap::Args;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;
use anyhow::anyhow;
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct RejectCommand {
    #[clap(display_order = 1002)]
    invitation: String,

    #[clap(display_order = 1101, long)]
    overwrite: bool,
}

impl RejectCommand {
    pub fn run(command: RejectCommand, cloud_addr: MultiAddr) {
        embedded_node(reject, (cloud_addr, command));
    }
}

impl<'a> From<&'a RejectCommand> for IdentityOpts {
    fn from(other: &'a RejectCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

async fn reject(mut ctx: Context, args: (MultiAddr, RejectCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&cmd), &ctx).await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api
        .reject_invitations(&identity.id.to_string(), &cmd.invitation)
        .await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
