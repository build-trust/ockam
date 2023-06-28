use clap::Args;
use colorful::Colorful;
use miette::miette;

use crate::{CommandGlobalOpts, docs};
use crate::node::{get_node_name, initialize_node_if_default};
use crate::node::util::{delete_all_nodes, delete_node};
use crate::terminal::ConfirmResult;
use crate::util::local_cmd;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete nodes
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the node.
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
        initialize_node_if_default(&opts, &self.node_name);
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_name);
    if cmd.all {
        delete_all_nodes(opts, cmd.force)?;
    } else {
        if cmd.yes {
            //call helper method for deleting single node By name
            delete_node(&opts, &node_name, cmd.force)?;
            // print message
            opts.terminal
                .stdout()
                .plain(format!(
                    "{} Node with name '{}' has been deleted.",
                    "✔︎".light_green(),
                    &node_name
                ))
                .machine(&node_name)
                .json(serde_json::json!({ "node": { "name": &node_name } }))
                .write_line()?;
        } else {
            // If it exists, proceed
            // If yes is not provided make sure using TTY
            match opts.terminal.confirm("This will delete the selected Node. Are you sure?")? {
                ConfirmResult::Yes => {}
                ConfirmResult::No => {
                    return Ok(());
                }
                ConfirmResult::NonTTY => {
                    return Err(miette!("Use --yes to confirm").into());
                }
            }
            //call helper method for deleting single node by name
            delete_node(&opts, &node_name, cmd.force)?;
            // print message
            opts.terminal
                .stdout()
                .plain(format!(
                    "{} Node with name '{}' has been deleted.",
                    "✔︎".light_green(),
                    &node_name
                ))
                .machine(&node_name)
                .json(serde_json::json!({ "node": { "name": &node_name } }))
                .write_line()?;
        }
    }
    Ok(())
}
