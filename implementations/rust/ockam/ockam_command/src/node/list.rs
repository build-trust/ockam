use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;
use ockam_api::cli_state::NodeProcessStatus;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::cli_state::nodes::NodeInfo;
use ockam_api::colors::OckamColor;

use crate::{docs, CommandGlobalOpts, Result};
use ockam_api::output::Output;

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
    pub fn name(&self) -> String {
        "node list".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        // Before printing node states we verify them.
        // We send a QueryStatus request to every node on
        // record. If the response yields a different pid to the
        // one in config, we update the pid stored in the config.
        // This should only happen if the node has failed in the past,
        // and has been restarted by something that is not this CLI.
        let node_names: Vec<_> = {
            let nodes = opts.state.get_nodes().await?;
            nodes.iter().map(|n| n.name()).collect()
        };

        let nodes = get_nodes_info(&opts, node_names).await?;
        print_nodes_info(&opts, nodes)?;
        Ok(())
    }
}

pub async fn get_nodes_info(
    opts: &CommandGlobalOpts,
    node_names: Vec<String>,
) -> Result<Vec<NodeListOutput>> {
    let mut nodes: Vec<NodeListOutput> = Vec::new();

    for node_name in node_names {
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_node_status = async {
            let node = opts.state.get_node(&node_name).await?;
            *is_finished.lock().await = true;
            Ok(node)
        };

        let output_messages = vec![format!(
            "Retrieving node {}...\n",
            node_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (node, _) = try_join!(get_node_status, progress_output)?;

        nodes.push(NodeListOutput::from_node_info(&node));
    }

    Ok(nodes)
}

pub fn print_nodes_info(
    opts: &CommandGlobalOpts,
    nodes: Vec<NodeListOutput>,
) -> miette::Result<()> {
    let plain = opts
        .terminal
        .build_list(&nodes, "No nodes found on this system.")?;

    let json = serde_json::to_string(&nodes).into_diagnostic()?;

    opts.terminal
        .clone()
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}

#[derive(Serialize)]
pub struct NodeListOutput {
    pub node_name: String,
    pub status: NodeProcessStatus,
    pub pid: Option<u32>,
    pub is_default: bool,
}

impl NodeListOutput {
    pub fn new(
        node_name: String,
        status: NodeProcessStatus,
        pid: Option<u32>,
        is_default: bool,
    ) -> Self {
        Self {
            node_name,
            status,
            pid,
            is_default,
        }
    }

    pub fn from_node_info(node_info: &NodeInfo) -> Self {
        Self::new(
            node_info.name(),
            node_info.status(),
            node_info.pid(),
            node_info.is_default(),
        )
    }
}

impl Output for NodeListOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let (status, process) = match self.status {
            NodeProcessStatus::Running(pid) => (
                "UP".color(OckamColor::Success.color()),
                format!(
                    "Process id {}",
                    pid.to_string().color(OckamColor::PrimaryResource.color())
                ),
            ),
            NodeProcessStatus::Zombie(pid) => (
                "ZOMBIE".color(OckamColor::Failure.color()),
                format!(
                    "Process id {}",
                    pid.to_string().color(OckamColor::PrimaryResource.color())
                ),
            ),
            NodeProcessStatus::Stopped => (
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
        {process}",
        node_name = self
            .node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        };

        Ok(output)
    }
}
