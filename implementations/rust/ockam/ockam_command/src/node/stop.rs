use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::colors::OckamColor;
use ockam_api::{color, fmt_info, fmt_ok, fmt_warn};

use crate::util::print_warning_for_deprecated_flag_no_effect;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/stop/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/stop/after_long_help.txt");

/// Stop a running node
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StopCommand {
    /// Name of the node.
    node_name: Option<String>,

    /// [DEPRECATED] Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[arg(short, long)]
    force: bool,
}

impl StopCommand {
    pub fn name(&self) -> String {
        "node stop".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if self.force {
            print_warning_for_deprecated_flag_no_effect(&opts, "--force")?;
        }

        let running_nodes = opts
            .state
            .get_nodes()
            .await?
            .iter()
            .filter(|node| node.is_running())
            .map(|node| node.name())
            .collect::<Vec<String>>();
        if running_nodes.is_empty() {
            opts.terminal
                .stdout()
                .plain(fmt_info!("There are no nodes running"))
                .write_line()?;
            return Ok(());
        }

        if self.node_name.is_some() || !opts.terminal.can_ask_for_user_input() {
            let node_name = opts
                .state
                .get_node_or_default(&self.node_name)
                .await?
                .name();
            if !running_nodes.contains(&node_name) {
                return Err(miette!(
                    "The node {} was not found",
                    node_name.light_magenta()
                ));
            }
            stop_node(opts, &node_name).await?;
            return Ok(());
        }

        match running_nodes.len() {
            0 => {
                unreachable!("this case is already handled above");
            }
            1 => {
                let node_name = running_nodes[0].as_str();
                stop_node(opts, node_name).await?;
            }
            _ => {
                let selected_item_names = opts.terminal.select_multiple(
                    "Select one or more nodes that you want to stop".to_string(),
                    running_nodes,
                );
                match selected_item_names.len() {
                    0 => {
                        opts.terminal
                            .stdout()
                            .plain(fmt_info!("No nodes selected to stop"))
                            .write_line()?;
                    }
                    1 => {
                        let node_name = selected_item_names[0].as_str();
                        stop_node(opts, node_name).await?;
                    }
                    _ => {
                        for item_name in selected_item_names {
                            stop_node(opts.clone(), &item_name).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

async fn stop_node(opts: CommandGlobalOpts, node_name: &str) -> miette::Result<()> {
    let res = opts.state.stop_node(node_name).await;
    let output = if res.is_ok() {
        fmt_ok!(
            "The node with name {} was stopped",
            color!(node_name, OckamColor::PrimaryResource)
        )
    } else {
        fmt_warn!(
            "Failed to delete node with name {}",
            color!(node_name, OckamColor::PrimaryResource)
        )
    };
    opts.terminal.stdout().plain(output).write_line()?;
    Ok(())
}
