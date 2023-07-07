use clap::Args;
use colorful::Colorful;

use crate::node::get_node_name;
use crate::node::util::{delete_all_nodes, delete_node};

use crate::util::local_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
    let node_name = get_node_name(&opts.state, &cmd.node_name);
    let prompt_msg = if cmd.all {
        "Are you sure you want to delete all nodes?"
    } else {
        "Are you sure you want to delete this node?"
    };
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, prompt_msg)?
    {
        if cmd.all {
            delete_all_nodes(&opts, cmd.force)?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!("All nodes have been deleted"))
                .write_line()?;
        } else {
            delete_node(&opts, &node_name, cmd.force)?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!("Node with name '{}' has been deleted", &node_name))
                .machine(&node_name)
                .json(serde_json::json!({ "node": { "name": &node_name } }))
                .write_line()?;
        }
    }
    Ok(())
}
