use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::{CommandGlobalOpts, docs};
use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{node_rpc, Rpc};
use crate::util::api::{self};

const LONG_ABOUT: &str = include_str!("./static/present/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/present/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
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
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: PresentCommand,
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::credentials::present_credential(&cmd.to, cmd.oneway))
        .await?;
    Ok(())
}
