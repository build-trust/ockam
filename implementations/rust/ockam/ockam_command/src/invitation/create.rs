use clap::Args;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;
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

    #[clap(display_order = 1101, long)]
    overwrite: bool,
}

impl<'a> From<&'a CreateCommand> for IdentityOpts {
    fn from(other: &'a CreateCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

impl CreateCommand {
    pub fn run(command: CreateCommand, cloud_addr: MultiAddr) {
        embedded_node(create, (cloud_addr, command));
    }
}

async fn create(mut ctx: Context, args: (MultiAddr, CreateCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&cmd), &ctx).await?;

    let r = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![r.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let pubkey = identity.id.to_string();
    let request =
        CreateInvitation::new(&pubkey, &cmd.email, &cmd.space_id, cmd.project_id.as_ref());
    let res = api.create_invitation(request).await?;
    println!("{res:?}");
    ctx.stop().await?;
    Ok(())
}
