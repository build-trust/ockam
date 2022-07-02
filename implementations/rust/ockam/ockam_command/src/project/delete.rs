use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, DEFAULT_CLOUD_ADDRESS};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    space_id: String,

    /// Id of the project.
    #[clap(display_order = 1002)]
    project_id: String,

    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    address: MultiAddr,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl DeleteCommand {
    pub fn run(command: DeleteCommand) {
        embedded_node(delete, command);
    }
}

async fn delete(mut ctx: Context, cmd: DeleteCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier()?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address"))?;
    let mut api = MessagingClient::new(route, &ctx).await?;
    api.delete_project(&cmd.space_id, &cmd.project_id, identifier.key_id())
        .await?;
    println!("Project deleted");

    ctx.stop().await?;
    Ok(())
}
