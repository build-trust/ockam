use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, NodeOpts};
use crate::util::{api, extract_address_value, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;
use crate::Result;

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

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, AddConsumerCommand)) -> Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: AddConsumerCommand,
) -> Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = extract_address_value(&node_name)?;
    let tcp = TcpTransport::create(ctx).await?;

    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).tcp(&tcp)?.build();
    rpc.request(api::add_consumer(cmd.flow_control_id, cmd.address))
        .await?;

    Ok(())
}
