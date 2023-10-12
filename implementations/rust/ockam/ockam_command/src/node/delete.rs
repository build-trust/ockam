use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::StateDirTrait;

use crate::node::get_default_node_name;
use crate::node::util::{delete_all_nodes, delete_node};
use crate::terminal::tui::DeleteMode;
use crate::util::local_cmd;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete nodes
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the node to be deleted
    #[arg(group = "nodes")]
    node_name: Option<String>,

    /// Terminate all node processes and delete all node configurations
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Terminate node process(es) immediately (uses SIGKILL instead of SIGTERM)
    #[arg(display_order = 901, long, short)]
    force: bool,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
    let nodes_names = opts.state.nodes.list_items_names()?;
    if nodes_names.is_empty() {
        opts.terminal
            .stdout()
            .plain("There are no nodes to delete")
            .write_line()?;
        return Ok(());
    }

    let delete_mode = if cmd.all {
        DeleteMode::All
    } else if let Some(node_name) = cmd.node_name {
        DeleteMode::Single(node_name)
    } else if nodes_names.len() == 1 {
        DeleteMode::Default
    } else if opts.terminal.can_ask_for_user_input() {
        DeleteMode::Selected(opts.terminal.select_multiple(
            "Select one or more nodes that you want to delete".to_string(),
            nodes_names,
        ))
    } else {
        DeleteMode::Default
    };

    match delete_mode {
        DeleteMode::All => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                "Are you sure you want to delete all nodes?",
            )? {
                delete_all_nodes(&opts, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("All nodes have been deleted"))
                    .write_line()?;
            }
        }
        DeleteMode::Selected(selected_node_names) => {
            if selected_node_names.is_empty() {
                opts.terminal
                    .stdout()
                    .plain("No nodes selected for deletion")
                    .write_line()?;
                return Ok(());
            }

            if opts.terminal.confirm_interactively(format!(
                "Would you like to delete these items : {:?}?",
                selected_node_names
            )) {
                let output = selected_node_names
                    .iter()
                    .map(|name| {
                        if opts.state.nodes.delete_sigkill(name, cmd.force).is_ok() {
                            fmt_ok!("Node '{name}' deleted\n")
                        } else {
                            fmt_warn!("Failed to delete Node '{name}'\n")
                        }
                    })
                    .collect::<String>();

                opts.terminal.stdout().plain(output).write_line()?;
            }
        }
        DeleteMode::Single(node_name) => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                format!("Are you sure you want to delete the node {node_name}?"),
            )? {
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Node with name '{node_name}' has been deleted"))
                    .machine(&node_name)
                    .json(serde_json::json!({ "name": &node_name }))
                    .write_line()?;
            }
        }
        DeleteMode::Default => {
            let node_name = get_default_node_name(&opts.state);
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                format!("Are you sure you want to delete the default node '{node_name}'?"),
            )? {
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Node with name '{node_name}' has been deleted"))
                    .machine(&node_name)
                    .json(serde_json::json!({ "name": &node_name }))
                    .write_line()?;
            }
        }
    };
    Ok(())
}
