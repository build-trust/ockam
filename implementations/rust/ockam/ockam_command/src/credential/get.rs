use clap::Args;

use ockam::Context;
use ockam_api::nodes::{BackgroundNodeClient, Credentials};

use crate::node::NodeOpts;
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
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, GetCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: GetCommand) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
    node.get_credential(ctx, cmd.overwrite, cmd.identity)
        .await?;
    Ok(())
}
