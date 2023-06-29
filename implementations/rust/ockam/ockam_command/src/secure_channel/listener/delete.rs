use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_core::Address;

use crate::{CommandGlobalOpts, docs, fmt_ok};
use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, parse_node_name, Rpc};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    /// Address at which the channel listener to be deleted is running
    address: Address,

    #[command(flatten)]
    node_opts: NodeOpts,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, (opts, cmd)).await
}

async fn run_impl(
    ctx: &Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let at = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&at)?;
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    let req = api::delete_secure_channel_listener(&cmd.address);
    rpc.request(req).await?;
    rpc.is_ok()?;

    let addr = format!("/service/{}", cmd.address.address());
    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Deleted secure-channel listener with address '{addr}' on node '{node_name}'"
        ))
        .machine(addr)
        .write_line()?;
    Ok(())
}
