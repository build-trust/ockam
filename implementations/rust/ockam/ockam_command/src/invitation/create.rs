use anyhow::anyhow;
use clap::Args;

use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::{invitation::CreateInvitation, MessagingClient};
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Id of the space to invite for
    #[clap(display_order = 1001)]
    space_id: String,

    /// Email to sent the invite
    #[clap(display_order = 1003)]
    email: String,

    /// project id to invite to, optional.
    #[clap(display_order = 1002, long)]
    project_id: Option<String>,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl CreateCommand {
    pub fn run(cmd: CreateCommand, cloud_addr: MultiAddr) {
        embedded_node(create, (cloud_addr, cmd));
    }
}

async fn create(mut ctx: Context, args: (MultiAddr, CreateCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;

    let r = ockam_api::multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, identity, &ctx).await?;
    let request = CreateInvitation::new(cmd.email, cmd.space_id, cmd.project_id);
    let res = api.create_invitation(request).await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
