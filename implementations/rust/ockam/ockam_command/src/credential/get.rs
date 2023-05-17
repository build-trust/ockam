use clap::Args;

use ockam::Context;

use crate::node::{get_node_name, NodeOpts};
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, GetCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: GetCommand,
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, cmd.node_opts.api_node.clone())?;
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::credentials::get_credential(
        cmd.overwrite,
        cmd.identity,
    ))
    .await?;
    Ok(())
}
