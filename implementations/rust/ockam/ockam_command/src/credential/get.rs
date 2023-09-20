use clap::Args;

use ockam::Context;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[arg(long)]
    pub overwrite: bool,

    #[arg(long = "identity", value_name = "IDENTITY")]
    identity: Option<String>,
}

impl GetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, GetCommand)) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: GetCommand,
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let mut rpc = Rpc::background(ctx, &opts.state, &node_name).await?;
    rpc.tell(api::credentials::get_credential(
        cmd.overwrite,
        cmd.identity,
    ))
    .await?;
    Ok(())
}
