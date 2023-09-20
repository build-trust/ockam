use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, PresentCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: PresentCommand,
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let mut rpc = Rpc::background(ctx, &opts.state, &node_name).await?;
    rpc.tell(api::credentials::present_credential(&cmd.to, cmd.oneway))
        .await?;
    Ok(())
}
