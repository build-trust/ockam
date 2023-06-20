use clap::Args;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_core::api::Request;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::get_node_name;
use crate::terminal::OckamColor;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Node to list relays from
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,
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
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = extract_address_value(&at)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        rpc.request(Request::get("/node/forwarder")).await?;

        *is_finished.lock().await = true;
        rpc.parse_response::<Vec<ForwarderInfo>>()
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

    let (relays, _) = try_join!(send_req, progress_output)?;

    let plain = opts.terminal.build_list(
        &relays,
        &format!("Relays on Node {node_name}"),
        &format!("No Relays found on node {node_name}."),
    )?;
    let json = serde_json::to_string_pretty(&relays).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
