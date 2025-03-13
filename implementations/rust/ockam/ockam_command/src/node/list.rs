use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::NodeProcessStatus;
use serde::Serialize;

use ockam_api::cli_state::nodes::NodeInfo;
use ockam_api::colors::{color_primary, OckamColor};

use crate::{docs, Command, CommandGlobalOpts, Result};
use ockam_api::output::Output;
use ockam_node::Context;

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

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "node list";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message("Retrieving nodes...");
        }

        let nodes: Vec<NodeListOutput> = opts
            .state
            .get_nodes()
            .await?
            .iter()
            .map(NodeListOutput::from_node_info)
            .collect();

        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }

        let plain = opts
            .terminal
            .build_list(&nodes, "No nodes found on this system")?;

        opts.terminal
            .to_stdout()
            .plain(plain)
            .json_obj(&nodes)?
            .write_line()?;

        Ok(())
    }
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
                format!("With PID {}", color_primary(pid)),
            ),
            NodeProcessStatus::Zombie(pid) => (
                "ZOMBIE".color(OckamColor::Failure.color()),
                format!("With PID {}", color_primary(pid)),
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

        let output = format!(
            "Node {}{} is {}\n{}",
            color_primary(&self.node_name),
            default,
            status,
            process
        );

        Ok(output)
    }
}
