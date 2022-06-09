use anyhow::anyhow;
use clap::Args;

use crate::old::identity::load_or_create_identity;
use crate::IdentityOpts;
use ockam::{Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::util::{embedded_node, multiaddr_to_route, DEFAULT_CLOUD_ADDRESS};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    id: String,

    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    address: MultiAddr,

    #[clap(display_order = 1101, long)]
    overwrite: bool,
}

impl ShowCommand {
    pub fn run(command: ShowCommand) {
        embedded_node(show, command);
    }
}

impl<'a> From<&'a ShowCommand> for IdentityOpts {
    fn from(other: &'a ShowCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

async fn show(mut ctx: Context, cmd: ShowCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&cmd), &ctx).await?;

    let route =
        multiaddr_to_route(&cmd.address).ok_or_else(|| anyhow!("failed to parse address"))?;
    let mut api = MessagingClient::new(route, &ctx).await?;
    let res = api.get_space(&cmd.id, &identity.id.to_string()).await?;
    println!("{res:#?}");

    ctx.stop().await?;
    Ok(())
}
