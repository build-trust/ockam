use crate::util::{api, exitcode, node_rpc, stop_node, ConfigError, RpcAlt, RpcCaller};

use crate::CommandGlobalOpts;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};

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

    #[clap(skip)]
    pub global_opts: Option<CommandGlobalOpts>,
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
    pub fn run(mut self, opts: CommandGlobalOpts) {
        self.global_opts = Some(opts.clone());
        node_rpc(rpc, (opts, self));
    }
}

impl<'a> RpcCaller<'a> for CreateCommand {
    type Req = CreateSecureChannelRequest<'a>;
    type Resp = CreateSecureChannelResponse<'a>;

    fn req(&self) -> ockam_core::api::RequestBuilder<'_, Self::Req> {
        let opts = self.global_opts.clone().unwrap();

        let (addr, _meta) =
            clean_multiaddr(&self.addr, &opts.config.get_lookup()).unwrap_or_else(|| {
                eprintln!("failed to normalize MultiAddr route");
                std::process::exit(exitcode::USAGE);
            });
        api::create_secure_channel(addr, self.authorized_identifier.clone())
    }
}

async fn rpc(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let res = rpc_callback(cmd, &ctx, opts).await;
    stop_node(ctx).await?;
    res
}

async fn rpc_callback(
    mut cmd: CreateCommand,
    ctx: &ockam::Context,
    opts: CommandGlobalOpts,
) -> crate::Result<()> {
    // We apply the inverse transformation done in the `create` command.
    let from = cmd.node_opts.from.clone();

    let res = RpcAlt::new(ctx, &opts, &from)?
        .request_then_response(&mut cmd)
        .await?;

    let parsed = res.parse_body()?;

    route_to_multiaddr(&route![parsed.addr.to_string()])
        .map(|addr| println!("{}", addr))
        .ok_or_else(|| ConfigError::InvalidSecureChannelAddress(parsed.addr.to_string()).into())
}
