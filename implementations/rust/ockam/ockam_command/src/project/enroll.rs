use clap::Args;

use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::clean_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::util::api::CloudOpts;
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts, Result};

/// An authorised enroller can add members to a project.
#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide())]
pub struct EnrollCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(long, short)]
    member: IdentityIdentifier,

    #[clap(long, short)]
    to: MultiAddr,
}

impl EnrollCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> Result<()> {
    async fn go(ctx: &mut Context, opts: &CommandGlobalOpts, cmd: EnrollCommand) -> Result<()> {
        let node_name = start_embedded_node(ctx, &opts.config).await?;

        let (to, meta) = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &meta,
            &cmd.cloud_opts.route_to_controller,
            &node_name,
            None,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        let req = Request::post("/members").body(AddMember::new(cmd.member));
        let mut rpc = RpcBuilder::new(ctx, opts, &node_name).to(&to)?.build();
        rpc.request(req).await?;
        delete_embedded_node(&opts.config, rpc.node_name()).await;
        rpc.is_ok()?;
        Ok(())
    }
    go(&mut ctx, &opts, cmd).await
}
