use clap::Args;

use ockam::identity::credential::Credential;
use ockam::{Context, TcpTransport};
use ockam_api::clean_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{node_rpc, RpcBuilder};
use crate::{stop_node, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct GetCredentialCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, short)]
    to: MultiAddr,
}

impl GetCredentialCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, GetCredentialCommand),
) -> Result<()> {
    async fn go(
        ctx: &mut Context,
        opts: &CommandGlobalOpts,
        cmd: &GetCredentialCommand,
    ) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let (to, meta) = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let projects_sc = crate::project::util::lookup_projects(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.addr,
            &cmd.node_opts.api_node,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        let mut rpc = RpcBuilder::new(ctx, opts, &cmd.node_opts.api_node)
            .tcp(&tcp)
            .to(&to)?
            .build()?;
        rpc.request(Request::post("/credential")).await?;
        rpc.print_response::<Credential<'_>>()?;
        Ok(())
    }
    let result = go(&mut ctx, &opts, &cmd).await;
    stop_node(ctx).await?;
    result
}
