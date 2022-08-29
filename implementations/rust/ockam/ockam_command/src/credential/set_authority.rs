use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct SetAuthorityCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

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
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::credentials::set_authority(&cmd.authority))
        .await?;
    Ok(())
}
