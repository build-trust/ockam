use clap::Args;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::NodeOpts;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{stop_node, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: ListCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> crate::Result<()> {
    let res = run_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::space::list(&cmd)).await?;
    rpc.print_response::<Vec<Space>>()
}
