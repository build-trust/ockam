use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::fmt_ok;
use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{node_rpc, parse_node_name};
use crate::{docs, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Delete the outlet with this alias name
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp outlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,

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

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let alias_name = cmd.alias.clone();
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    // Check if there an outlet with the provided alias/name exists
    node.ask_and_get_reply::<_, OutletStatus>(
        &ctx,
        Request::get(format!("/node/outlet/{alias_name}")),
    )
    .await?
    .found()
    .into_diagnostic()?
    .ok_or(miette!(
        "There's no Outlet with name/alias '{}'",
        alias_name
    ))?;
    // Proceed with the deletion
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this TCP outlet?",
    )? {
        node.tell(&ctx, Request::delete(format!("/node/outlet/{alias_name}")))
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "TCP outlet with alias {alias_name} on node {node_name} has been deleted."
            ))
            .machine(&alias_name)
            .json(serde_json::json!({ "tcp-outlet": { "alias": alias_name, "node": node_name } }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
