use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::fmt_ok;
use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Delete the outlet with this alias
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
        node_rpc(run_impl, (opts, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;

    // Check if there an outlet with the provided alias/name exists
    let alias = cmd.alias;
    node.ask_and_get_reply::<_, OutletStatus>(&ctx, Request::get(format!("/node/outlet/{alias}")))
        .await?
        .found()
        .into_diagnostic()?
        .ok_or(miette!(
            "TCP outlet with alias {alias} was not found on Node {}",
            node.node_name()
        ))?;

    // Proceed with the deletion
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this TCP outlet?",
    )? {
        node.tell(&ctx, Request::delete(format!("/node/outlet/{alias}")))
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "TCP outlet with alias {alias} on Node {} has been deleted",
                node.node_name()
            ))
            .machine(&alias)
            .json(serde_json::json!({ "alias": alias, "node": node.node_name() }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
