use clap::Args;
use colorful::Colorful;

use ockam_api::nodes::{models, BackgroundNode};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::node_rpc;
use crate::{docs, fmt_ok, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP connection
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// TCP connection ID
    pub address: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this TCP connection?",
    )? {
        let address = cmd.address;
        let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
        let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
        let req = Request::delete("/node/tcp/connection")
            .body(models::transport::DeleteTransport::new(address.clone()));
        node.tell(&ctx, req).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "TCP connection {address} has been successfully deleted"
            ))
            .json(serde_json::json!({ "address": address }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
