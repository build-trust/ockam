use clap::Args;

use ockam::Context;
use ockam_api::nodes::{BackgroundNode, Credentials};

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[arg(long)]
    pub overwrite: bool,

    /// Name of the Identity for which the credential was issued.
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,
}

impl GetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, GetCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: GetCommand) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = BackgroundNode::create(ctx, &opts.state, &node_name).await?;
    node.get_credential(ctx, cmd.overwrite, cmd.identity)
        .await?;
    Ok(())
}
