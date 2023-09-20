use clap::Args;

use ockam::Context;
use ockam_core::Address;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, parse_node_name, Rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channel Listener
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Address of the channel listener
    address: Address,

    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    run_impl(&ctx, (opts, cmd)).await
}

async fn run_impl(
    ctx: &Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let at = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&at)?;
    let address = &cmd.address;

    let mut rpc = Rpc::background(ctx, &opts.state, &node_name).await?;
    let req = api::show_secure_channel_listener(address);
    rpc.tell(req).await?;
    opts.terminal
        .stdout()
        .plain(format!("/service/{}", cmd.address.address()))
        .write_line()?;
    Ok(())
}
