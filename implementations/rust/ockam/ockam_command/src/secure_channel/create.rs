use crate::util::{api, exitcode, node_rpc, stop_node, ConfigError, Rpc};

use crate::CommandGlobalOpts;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_api::{clean_multiaddr, route_to_multiaddr};
use ockam_core::route;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    pub node_opts: SecureChannelNodeOpts,

    /// Route to a secure channel listener (required)
    #[clap(name = "to", short, long, value_name = "ROUTE")]
    pub addr: MultiAddr,

    /// Pre-known Identifiers of the other side
    #[clap(short, long)]
    pub authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

#[derive(Clone, Debug, Args)]
pub struct SecureChannelNodeOpts {
    /// Node that will initiate the secure channel
    #[clap(
        global = true,
        short,
        long,
        value_name = "NODE",
        default_value = "default"
    )]
    pub from: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }

    async fn rpc_callback(
        self,
        ctx: &ockam::Context,
        opts: CommandGlobalOpts,
    ) -> crate::Result<()> {
        let addr = clean_multiaddr(&self.addr, &opts.config.get_lookup()).unwrap_or_else(|| {
            eprintln!("failed to normalize MultiAddr route");
            std::process::exit(exitcode::USAGE);
        });

        let mut rpc = Rpc::new(ctx, &opts, &self.node_opts.from)?;

        rpc.request(api::create_secure_channel(addr, self.authorized_identifier))
            .await?;

        let res = rpc.parse_response::<CreateSecureChannelResponse>()?;

        route_to_multiaddr(&route![res.addr.to_string()])
            .map(|addr| println!("{}", addr))
            .ok_or_else(|| ConfigError::InvalidSecureChannelAddress(res.addr.to_string()).into())
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> crate::Result<()> {
    let res = cmd.rpc_callback(&ctx, opts).await;
    stop_node(ctx).await?;
    res
}
