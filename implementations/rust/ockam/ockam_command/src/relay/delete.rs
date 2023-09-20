use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_core::api::Request;

use crate::node::get_node_name;
use crate::util::{node_rpc, parse_node_name, Rpc};
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
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this relay?")?
    {
        let relay_name = cmd.relay_name.clone();
        let at = get_node_name(&opts.state, &cmd.at);
        let node = parse_node_name(&at)?;
        let mut rpc = Rpc::background(&ctx, &opts.state, &node).await?;
        rpc.tell(Request::delete(format!("/node/forwarder/{relay_name}",)))
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Relay with name {} on Node {} has been deleted.",
                relay_name,
                node
            ))
            .machine(&relay_name)
            .json(serde_json::json!({ "forwarder": { "name": relay_name,
                "node": node } }))
            .write_line()
            .unwrap();
    }
    Ok(())
}
