use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{stop_node, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    pub id: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: DeleteCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let res = run_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::space::delete(&cmd)).await?;
    rpc.check_response()
}
