use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::nodes::service::relay::Relays;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::trace;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::BackgroundNode;

use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    before_help = docs::before_help(PREVIEW_TAG),
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Get the list of Relays at the given node
    #[arg(global = true, long, value_name = "NODE", value_parser = extract_address_value)]
    pub to: Option<String>,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.to).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_relays = async {
        let relay_infos = node.list_relay(&ctx).await?;
        *is_finished.lock().await = true;
        Ok(relay_infos)
    };

    let output_messages = vec![format!(
        "Listing Relays on {}...\n",
        node.node_name().color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (relays, _) = try_join!(get_relays, progress_output)?;
    trace!(?relays, "Relays retrieved");

    let plain = opts.terminal.build_list(
        &relays,
        &format!("Relays on Node {}", node.node_name()),
        &format!("No Relays found on node {}.", node.node_name()),
    )?;
    let json = serde_json::to_string_pretty(&relays).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
