use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;

use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::{models, BackgroundNode};
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::node_rpc;
use crate::util::parse_node_name;
use crate::{docs, fmt_ok, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP listener
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Listener ID
    pub address: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

    // Check if there an TCP listener with the provided address exists
    let address = cmd.address;
    node.ask_and_get_reply::<_, TransportStatus>(
        &ctx,
        Request::get(format!("/node/tcp/listener/{address}")),
    )
    .await?
    .found()
    .into_diagnostic()?
    .ok_or(miette!(
        "TCP listener with address {address} was not found on Node {node_name}"
    ))?;

    // Proceed with the deletion
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this TCP listener?",
    )? {
        let req = Request::delete("/node/tcp/listener")
            .body(models::transport::DeleteTransport::new(address.clone()));
        node.tell(&ctx, req).await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "TCP listener with address {address} on Node {node_name} has been deleted"
            ))
            .json(serde_json::json!({ "tcp-listener": {"node": node_name } }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
