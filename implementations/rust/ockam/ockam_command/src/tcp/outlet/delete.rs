use clap::Args;
use colorful::Colorful;

use ockam::Context;
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
    /// Name assigned to outlet that will be deleted
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
    let alias = cmd.alias.clone();
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    // Check if the alias exists
    let alias_exists = check_alias_existence(&ctx, &opts.state, &node_name, &alias).await?;
    if alias_exists {
        if opts.terminal.confirmed_with_flag_or_prompt(
            cmd.yes,
            "Are you sure you want to delete this TCP outlet?",
        )? {
            let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
            node.tell(&ctx, Request::delete(format!("/node/outlet/{alias}")))
                .await?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!(
                    "TCP outlet with alias {alias} on node {node_name} has been deleted."
                ))
                .machine(&alias)
                .json(serde_json::json!({ "tcp-outlet": { "alias": alias, "node": node_name } }))
                .write_line()
                .unwrap();
        }
    } else {
        opts.terminal
            .stderr()
            .plain(fmt_err!("TCP outlet with alias {alias} does not exist."))
            .machine(&alias)
            .write_line()
            .unwrap();
    }
    Ok(())
}

// Helper function to check alias existence
async fn check_alias_existence(
    ctx: &Context,
    state: &State,
    node_name: &str,
    alias: &str,
) -> Result<bool, miette::Error> {
    let node = BackgroundNode::create(ctx, state, node_name).await?;
    let response = node
        .tell(ctx, Request::get(format!("/node/outlet/{alias}")))
        .await?;

    Ok(response.status().is_success())
}
