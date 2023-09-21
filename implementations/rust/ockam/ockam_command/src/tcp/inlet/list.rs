use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::address::extract_address_value;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::portal::InletList;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Inlets
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node.at_node);
    let node_name = extract_address_value(&node_name)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_inlets = async {
        let inlets: InletList = node.ask(&ctx, Request::get("/node/inlet")).await?;
        *is_finished.lock().await = true;
        Ok(inlets)
    };

    let output_messages = vec![format!(
        "Listing TCP Inlets on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (inlets, _) = try_join!(get_inlets, progress_output)?;

    let plain = opts.terminal.build_list(
        &inlets.list,
        "Inlets",
        &format!("No TCP Inlets found on {node_name}"),
    )?;
    let json = serde_json::to_string_pretty(&inlets.list).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
