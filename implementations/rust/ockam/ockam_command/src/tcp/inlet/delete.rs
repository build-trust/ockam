use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::alias_parser;

use crate::fmt_ok;
use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_core::api::Request;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Inlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Name assigned to inlet that will be deleted
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp inlet. If none are provided, the default node will be used
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
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this TCP inlet?")?
    {
        let alias = cmd.alias.clone();
        let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
        let node = parse_node_name(&node_name)?;
        let mut rpc = Rpc::background(&ctx, &opts, &node)?;
        rpc.request(Request::delete(format!("/node/inlet/{alias}")))
            .await?;
        rpc.is_ok()?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "TCP inlet with alias {alias} on Node {node} has been deleted."
            ))
            .machine(&alias)
            .json(serde_json::json!({ "tcp-inlet": { "alias": alias, "node": node } }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
