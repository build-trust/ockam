use crate::node::util::{delete_all_nodes, delete_node};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// Delete a node
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct DeleteCommand {
    /// Name of the node.
    #[arg(default_value = "default", group = "nodes")]
    node_name: String,

    /// Terminate all nodes
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Clean up config directories and all nodes state directories
    #[arg(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    if cmd.all {
        delete_all_nodes(opts, cmd.force)?;
    } else {
        delete_node(&opts, &cmd.node_name, cmd.force);
        println!("Deleted node '{}'", &cmd.node_name);
    }
    Ok(())
}
