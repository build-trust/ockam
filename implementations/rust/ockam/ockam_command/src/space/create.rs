use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::cloud::{space::CreateSpace, MessagingClient};
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, DEFAULT_CLOUD_ADDRESS};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space.
    #[clap(display_order = 1001)]
    name: String,

    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    address: MultiAddr,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl CreateCommand {
    pub fn run(command: CreateCommand) {
        embedded_node(create, command);
    }
}

async fn create(mut ctx: Context, cmd: CreateCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier()?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address"))?;
    let mut api = MessagingClient::new(route, &ctx).await?;
    let request = CreateSpace::new(cmd.name);
    let res = api.create_space(request, identifier.key_id()).await?;
    println!("{res:#?}");

    ctx.stop().await?;
    Ok(())
}
