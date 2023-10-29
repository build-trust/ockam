use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::nodes::models::portal::{InletList, InletStatus};
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::{alias_parser, fetch_list};
use crate::util::{node_rpc, parse_node_name};
use crate::{docs, CommandGlobalOpts};
use crate::{fmt_err, fmt_ok};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Inlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Delete the inlet with this alias
    #[arg(display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

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
    let mut selected_aliases = Vec::with_capacity(1);
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

    if let Some(alias) = cmd.alias.as_ref() {
        // Check if alias exists
        let inlet_staus = node
            .ask_and_get_reply::<_, InletStatus>(&ctx, Request::get(format!("/node/inlet/{alias}")))
            .await?
            .found()
            .into_diagnostic()?
            .ok_or(miette!(
                "TCP inlet with alias {alias} was not found on Node {node_name}"
            ))?;
        selected_aliases.push(inlet_staus.alias);
    } else {
        if !opts.terminal.can_ask_for_user_input() {
            return Err(miette!("No alias was provided"));
        }
        // Get all the avilable aliases on the node
        let aliases: Vec<String> =
            fetch_list::<_, InletList>("/node/inlet", &ctx, &node, &opts.terminal)
                .await?
                .list
                .into_iter()
                .map(|status| status.alias)
                .collect();
        if aliases.is_empty() {
            return Err(miette!("There's no TCP inlents to choose from"));
        }
        // Show the multible selection menu
        selected_aliases = opts
            .terminal
            .select_multiple("Select an inlet/s to delete".to_string(), aliases);
        if selected_aliases.is_empty() {
            return Err(miette!("User did not select anything"));
        };
    }

    // Confirm that the user know what he do
    let know = if cmd.alias.is_none() {
        opts.terminal.confirm_interactively(formatdoc!(
            "Are you sure that you like to delete these items : {:?}?",
            selected_aliases
        ))
    } else {
        opts.terminal.confirmed_with_flag_or_prompt(
            cmd.yes,
            "Are you sure that you want to delete this TCP inlet?",
        )?
    };

    if know {
        for alias in selected_aliases {
            let msg = match node
                .tell(&ctx, Request::delete(format!("/node/inlet/{alias}")))
                .await
            {
                Ok(_) => fmt_ok!(
                    "✅ TCP inlet with alias `{alias}` on Node {node_name} has been deleted"
                ),
                Err(e) => fmt_err!("⚠️ Can't delete `{alias}` becuse: {e}"),
            };
            opts.terminal.clone().stdout().plain(msg).write_line()?;
        }
    }
    Ok(())
}
