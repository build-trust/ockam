use crate::util::node_rpc;
use crate::{color, docs, fmt_info, fmt_ok, fmt_warn, CommandGlobalOpts, OckamColor};

use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_node::Context;

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

    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[arg(short, long)]
    force: bool,
}

impl StopCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StopCommand),
) -> miette::Result<()> {
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

    if cmd.node_name.is_some() || !opts.terminal.can_ask_for_user_input() {
        let node_name = opts.state.get_node_or_default(&cmd.node_name).await?.name();
        if !running_nodes.contains(&node_name) {
            return Err(miette!(
                "The node {} was not found",
                node_name.light_magenta()
            ));
        }
        stop_node(opts, &node_name, cmd.force).await?;
        return Ok(());
    }

    match running_nodes.len() {
        0 => {
            unreachable!("this case is already handled above");
        }
        1 => {
            let node_name = running_nodes[0].as_str();
            stop_node(opts, node_name, cmd.force).await?;
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
                    stop_node(opts, node_name, cmd.force).await?;
                }
                _ => {
                    for item_name in selected_item_names {
                        stop_node(opts.clone(), &item_name, cmd.force).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn stop_node(opts: CommandGlobalOpts, node_name: &str, force: bool) -> miette::Result<()> {
    let res = opts.state.stop_node(node_name, force).await;
    let output = if res.is_ok() {
        fmt_ok!(
            "Node with name {} was stopped",
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
