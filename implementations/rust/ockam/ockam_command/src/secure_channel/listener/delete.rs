use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::nodes::models::secure_channel::DeleteSecureChannelListenerResponse;
use ockam_core::Address;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, parse_node_name, Rpc};
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
    let mut rpc = Rpc::background(ctx, &opts.state, &node_name).await?;
    let req = api::delete_secure_channel_listener(&cmd.address);
    let response: DeleteSecureChannelListenerResponse = rpc.ask(req).await?;
    let addr = response.addr;
    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Deleted secure-channel listener with address '{addr}' on node '{node_name}'"
        ))
        .machine(addr)
        .write_line()?;
    Ok(())
}
