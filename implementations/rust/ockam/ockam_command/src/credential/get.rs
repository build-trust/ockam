use clap::Args;

use ockam::Context;

use crate::{CommandGlobalOpts, docs};
use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, Rpc};

const LONG_ABOUT: &str = include_str!("./static/get/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/get/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
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
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::credentials::get_credential(
        cmd.overwrite,
        cmd.identity,
    ))
    .await?;
    Ok(())
}
