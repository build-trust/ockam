use anyhow::anyhow;
use clap::Args;

use ockam::{AsyncTryClone, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, DEFAULT_CLOUD_ADDRESS};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    id: String,

    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    addr: MultiAddr,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl ShowCommand {
    pub fn run(cmd: ShowCommand) {
        embedded_node(show, cmd);
    }
}

//TODO: there is lot of repetition between this and the delete command for example.
//      Ideally, would want to do something like
//          through_space_gateway(ctx, cmd.identity_opts, |api| api.show(&cmd.id))
//      on one, and
//          through_space_gateway(ctx, cmd.identity_opts, |api| api.delete(&cmd.id))
//      on the other, but hit a problem about "async clousures are unstable", so here is it.
async fn show(mut ctx: Context, cmd: ShowCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;

    let route = ockam_api::multiaddr_to_route(&cmd.addr)
        .ok_or_else(|| anyhow!("failed to parse address"))?;

    // We built two different messaging clients, because the route they connect
    // to is different.
    // Both had to access the identity.
    // Had to clone the identity here this is 100% ugly and should be a better way.
    let identity_copy = identity.async_try_clone().await?;
    let mut api = MessagingClient::new(route, identity_copy, &ctx).await?;
    let spaces = api.list_spaces().await?;
    match spaces.iter().find(|s| s.id == cmd.id) {
        None => println!("Space {} not found", &cmd.id),
        Some(space) => {
            // TODO: Also use the provided identity_id on the space' list response
            //       to authenticate the other end
            let r: &str = space.gateway_route.as_ref();
            let addr = MultiAddr::try_from(r)?;
            let route = ockam_api::multiaddr_to_route(&addr)
                .ok_or_else(|| anyhow!("failed to parse project route"))?;
            let mut api = MessagingClient::new(route, identity, &ctx).await?;
            let res = api.get_space(&cmd.id).await?;
            println!("{res:#?}");
        }
    }
    ctx.stop().await?;
    Ok(())
}
