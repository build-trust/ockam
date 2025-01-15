use clap::Args;

use ockam::Context;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::api;
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
    pub fn name(&self) -> String {
        "add flowcontrol consumer".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        node.tell(
            ctx,
            api::add_consumer(self.flow_control_id.clone(), self.address.clone()),
        )
        .await?;

        Ok(())
    }
}
