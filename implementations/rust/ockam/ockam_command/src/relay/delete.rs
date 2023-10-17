use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::get_node_name;
use crate::util::{node_rpc, parse_node_name};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = false,
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name assigned to Relay that will be deleted
    #[arg(display_order = 900, required = true)]
    relay_name: String,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let relay_name = cmd.relay_name.clone();
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

    // Check if relay exists
    node.ask_and_get_reply::<_, RelayInfo>(
        &ctx,
        Request::get(format!("/node/forwarder/{relay_name}")),
    )
    .await?
    .found()
    .into_diagnostic()?
    .ok_or(miette!("Relay with name '{}' does not exist", relay_name))?;

    // Proceed with the deletion
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this relay?")?
    {
        node.tell(
            &ctx,
            Request::delete(format!("/node/forwarder/{relay_name}",)),
        )
        .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Relay with name {} on Node {} has been deleted.",
                relay_name,
                node_name
            ))
            .machine(&relay_name)
            .json(serde_json::json!({ "name": relay_name, "node": node_name }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
