use clap::Args;

use ockam::Context;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct SetAuthorityCommand {
    #[clap(value_name = "AUTHORITY")]
    pub authority: Vec<String>,
}

impl SetAuthorityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SetAuthorityCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: SetAuthorityCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::credentials::set_authority(&cmd.authority))
        .await?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
