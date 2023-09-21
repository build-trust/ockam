use clap::Args;

use ockam::Context;
use ockam_api::nodes::RemoteNode;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::api::{self};
use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct PresentCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[arg(long, display_order = 900, id = "ROUTE")]
    pub to: MultiAddr,

    #[arg(short, long)]
    pub oneway: bool,
}

impl PresentCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, PresentCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: PresentCommand,
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = RemoteNode::create(ctx, &opts.state, &node_name).await?;
    node.tell(
        ctx,
        api::credentials::present_credential(&cmd.to, cmd.oneway),
    )
    .await?;
    Ok(())
}
