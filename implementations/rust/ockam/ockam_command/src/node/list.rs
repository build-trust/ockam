use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::Context as _;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::base::NodeStatus;
use ockam_api::nodes::BackgroundNode;

use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List nodes
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, _cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    // Before printing node states we verify them.
    // We send a QueryStatus request to every node on
    // record. If the response yields a different pid to the
    // one in config, we update the pid stored in the config.
    // This should only happen if the node has failed in the past,
    // and has been restarted by something that is not this CLI.
    let mut default = String::new();
    let node_names: Vec<_> = {
        let nodes_states = opts.state.nodes.list()?;
        // default node
        if let Ok(state) = opts.state.nodes.default() {
            default = state.name().to_string();
        }
        nodes_states.iter().map(|s| s.name().to_string()).collect()
    };

    let mut nodes: Vec<NodeListOutput> = Vec::new();
    for node_name in node_names {
        let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_node_status = async {
            let result: miette::Result<NodeStatus> = node.ask(&ctx, api::query_status()).await;
            let node_status = match result {
                Ok(node_status) => {
                    if let Ok(node_state) = opts.state.nodes.get(&node_name) {
                        // Update the persisted configuration data with the pids
                        // responded by nodes.
                        if node_state.pid()? != Some(node_status.pid) {
                            node_state
                                .set_pid(node_status.pid)
                                .context("Failed to update pid for node {node_name}")?;
                        }
                    }
                    node_status
                }
                Err(_) => NodeStatus::new(node_name.to_string(), "Not running".to_string(), 0, 0),
            };
            *is_finished.lock().await = true;
            Ok(node_status)
        };

        let output_messages = vec![format!(
            "Retrieving node {}...\n",
            node_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (node_status, _) = try_join!(get_node_status, progress_output)?;

        nodes.push(NodeListOutput::new(
            node_status.node_name.to_string(),
            node_status.status.to_string(),
            node_status.pid,
            node_status.node_name == default,
        ));
    }

    let list = opts
        .terminal
        .build_list(&nodes, "Nodes", "No nodes found on this system.")?;
    opts.terminal.stdout().plain(list).write_line()?;
    Ok(())
}

pub struct NodeListOutput {
    pub node_name: String,
    pub status: String,
    pub pid: i32,
    pub is_default: bool,
}

impl NodeListOutput {
    pub fn new(node_name: String, status: String, pid: i32, is_default: bool) -> Self {
        Self {
            node_name,
            status,
            pid,
            is_default,
        }
    }
}

impl Output for NodeListOutput {
    fn output(&self) -> Result<String> {
        let (status, pid) = match self.status.as_str() {
            "Running" => (
                "UP".color(OckamColor::Success.color()),
                format!(
                    "Process id {}",
                    self.pid
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ),
            ),
            _ => (
                "DOWN".color(OckamColor::Failure.color()),
                "No process running".to_string(),
            ),
        };
        let default = match self.is_default {
            true => " (default)".to_string(),
            false => "".to_string(),
        };

        let output = formatdoc! {"
        Node {node_name}{default} {status}
        {pid}",
        node_name = self
            .node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        };

        Ok(output)
    }
}
