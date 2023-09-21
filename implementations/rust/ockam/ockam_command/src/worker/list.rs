use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::workers::{WorkerList, WorkerStatus};
use ockam_api::nodes::RemoteNode;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List workers on a node
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Node at which to lookup workers
    #[arg(value_name = "NODE", long, display_order = 800)]
    at: Option<String>,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.at);
        node_rpc(run_impl, (opts, self))
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

    let node = RemoteNode::create(&ctx, &opts.state, &node_name).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_workers = async {
        let workers: WorkerList = node.ask(&ctx, api::list_workers()).await?;
        *is_finished.lock().await = true;
        Ok(workers)
    };

    let output_messages = vec![format!(
        "Listing Workers on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (workers, _) = try_join!(get_workers, progress_output)?;

    let list = opts.terminal.build_list(
        &workers.list,
        &format!("Workers on {node_name}"),
        &format!("No workers found on {node_name}."),
    )?;
    opts.terminal.stdout().plain(list).write_line()?;

    Ok(())
}

impl Output for WorkerStatus {
    fn output(&self) -> crate::Result<String> {
        Ok(format!(
            "Worker {}",
            self.addr
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))
    }
}
