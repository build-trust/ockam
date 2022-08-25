use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::{stop_node, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct SetAuthorityCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(value_name = "AUTHORITY")]
    pub authority: Vec<String>,
}

impl SetAuthorityCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: SetAuthorityCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SetAuthorityCommand),
) -> crate::Result<()> {
    let res = run_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
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
