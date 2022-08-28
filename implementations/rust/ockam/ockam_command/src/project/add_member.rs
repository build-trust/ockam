use clap::Args;

use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::clean_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{node_rpc, RpcBuilder};
use crate::{CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct AddMemberCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, short)]
    member: IdentityIdentifier,

    #[clap(long, short)]
    to: MultiAddr,
}

impl AddMemberCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, AddMemberCommand)) -> Result<()> {
    async fn go(ctx: &mut Context, opts: &CommandGlobalOpts, cmd: AddMemberCommand) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let (to, meta) = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.route_to_controller,
            &cmd.node_opts.api_node,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        let req = Request::post("/members").body(AddMember::new(cmd.member));
        let mut rpc = RpcBuilder::new(ctx, opts, &cmd.node_opts.api_node)
            .tcp(&tcp)
            .to(&to)?
            .build()?;
        rpc.request(req).await?;
        rpc.is_ok()?;
        Ok(())
    }
    go(&mut ctx, &opts, cmd).await
}
