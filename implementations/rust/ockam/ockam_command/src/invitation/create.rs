use clap::Args;

use crate::util::{embedded_node, multiaddr_to_route};
use anyhow::anyhow;
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::{invitation::CreateInvitation, MessagingClient};
use ockam_multiaddr::MultiAddr;

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
}

impl CreateCommand {
    pub fn run(command: CreateCommand, cloud_addr: MultiAddr) {
        embedded_node(create, (cloud_addr, command));
    }
}

async fn create(mut ctx: Context, args: (MultiAddr, CreateCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let request = CreateInvitation::new("1", &cmd.email, &cmd.space_id, cmd.project_id.as_deref());
    let res = api.create_invitation(request).await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
