use clap::Args;
use ockam::Context;

use crate::{
    node::NodeOpts,
    util::{api, node_rpc, stop_node, Rpc},
    CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    pub alias: String,

    #[clap(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(tcp_inlet_delete_rcp, (options, self))
    }
}

async fn tcp_inlet_delete_rcp(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let res = tcp_inlet_delete_rcp_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn tcp_inlet_delete_rcp_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::background(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::tcp::inlet::delete(&cmd)).await?;
    rpc.is_ok()?;
    Ok(())
}
