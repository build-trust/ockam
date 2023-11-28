use clap::Args;

use ockam::Context;
use ockam_api::nodes::BackgroundNode;
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{api, node_rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct AddConsumerCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Corresponding FlowControlId value
    flow_control_id: FlowControlId,

    /// Address of the Consumer
    address: MultiAddr,
}

impl AddConsumerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self))
    }
}

async fn rpc(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddConsumerCommand),
) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: AddConsumerCommand,
) -> miette::Result<()> {
    let node = BackgroundNode::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
    node.tell(ctx, api::add_consumer(cmd.flow_control_id, cmd.address))
        .await?;

    Ok(())
}
