use clap::Args;
use colorful::Colorful;
use miette::miette;
use tracing::trace;

use ockam::Context;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::get_node_name;
use crate::terminal::tui::DeleteMode;
use crate::util::{node_rpc, parse_node_name};
use crate::{docs, fmt_err, fmt_ok, fmt_warn, CommandGlobalOpts, OckamColor};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = false,
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name assigned to Relay that will be deleted
    #[arg(display_order = 900)]
    relay_name: Option<String>,

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
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_relays = async {
        let relay_infos: Vec<RelayInfo> = node.ask(&ctx, Request::get("/node/forwarder")).await?;
        *is_finished.lock().await = true;
        Ok(relay_infos)
    };

    let output_messages = vec![format!(
        "Listing Relays on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (relays, _) = try_join!(get_relays, progress_output)?;
    trace!(?relays, "Relays retrieved");

    if relays.is_empty() {
        return Err(miette!("There are no relays to choose from"));
    }

    let delete_mode = if let Some(relay_name) = cmd.relay_name {
        DeleteMode::Single(relay_name)
    } else if opts.terminal.can_ask_for_user_input() {
        DeleteMode::Selected(
            opts.terminal.select_multiple(
                "Select one or more relays that you want to delete".to_string(),
                relays
                    .iter()
                    .map(|r| r.remote_address().to_string())
                    .collect(),
            ),
        )
    } else {
        DeleteMode::Default
    };

    match delete_mode {
        DeleteMode::Selected(selected_relay_names) => {
            if selected_relay_names.is_empty() {
                opts.terminal
                    .stdout()
                    .plain("No relays selected for deletion")
                    .write_line()?;
                return Ok(());
            }

            if opts.terminal.confirm_interactively(format!(
                "Would you like to delete these items : {:?}?",
                selected_relay_names
            )) {
                for relay in selected_relay_names {
                    let msg = match node
                        .tell(&ctx, Request::delete(format!("/node/forwarder/{relay}")))
                        .await
                    {
                        Ok(_) => {
                            fmt_ok!("✅ Relay `{relay}` deleted")
                        }
                        Err(_) => fmt_warn!("⚠️ Failed to delete Node '{relay}'\n"),
                    };
                    opts.terminal.clone().stdout().plain(msg).write_line()?;
                }
            }
        }
        DeleteMode::Single(relay) => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                format!("Are you sure you want to delete the relay {relay}?"),
            )? {
                let msg = match node
                    .tell(&ctx, Request::delete(format!("/node/forwarder/{relay}")))
                    .await
                {
                    Ok(_) => {
                        fmt_ok!("✅ Relay `{relay}` deleted")
                    }
                    Err(_) => fmt_warn!("⚠️ Failed to delete Node '{relay}'\n"),
                };

                opts.terminal.clone().stdout().plain(msg).write_line()?;
            }
        }
        DeleteMode::Default | DeleteMode::All => {
            fmt_err!("Delete mode not supported");
        }
    }

    Ok(())
}
