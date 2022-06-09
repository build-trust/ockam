use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route, DEFAULT_CLOUD_ADDRESS};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    address: MultiAddr,

    #[clap(display_order = 1101, long)]
    overwrite: bool,
}

impl<'a> From<&'a ListCommand> for IdentityOpts {
    fn from(other: &'a ListCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

impl ListCommand {
    pub fn run(command: ListCommand) {
        embedded_node(list, command);
    }
}

async fn list(mut ctx: Context, cmd: ListCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&cmd), &ctx).await?;

    let route =
        multiaddr_to_route(&cmd.address).ok_or_else(|| anyhow!("failed to parse address"))?;
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api.list_spaces(&identity.id.to_string()).await?;
    println!("{res:#?}");

    ctx.stop().await?;
    Ok(())
}
